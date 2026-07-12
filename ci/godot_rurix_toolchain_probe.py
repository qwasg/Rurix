#!/usr/bin/env python3
"""Probe the local Godot/Rurix toolchain without mutating the machine."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import pathlib
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass

from godot_rurix_patch_stack import (
    evaluate_followup_patch_applyability,
    evaluate_patch_stack,
    evaluate_stacked_patch_applyability,
)


ROOT = pathlib.Path(__file__).resolve().parents[1]
EXTERNAL_GODOT = ROOT / "external" / "godot-master"
SCONSTRUCT = EXTERNAL_GODOT / "SConstruct"
RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"
LOCAL_LOG_DIR = ROOT / "target" / "grx"
JSON_REPORT = LOCAL_LOG_DIR / "godot_toolchain_probe.json"
DXIL_TOOLCHAIN_REPORT = LOCAL_LOG_DIR / "dxil_toolchain_probe.json"
BUILD_SUMMARY_REPORT = LOCAL_LOG_DIR / "godot_scons_build_summary.json"
LOAD_SMOKE_SUMMARY_REPORT = LOCAL_LOG_DIR / "godot_load_smoke_summary.json"
BENCH_SMOKE_SUMMARY_REPORT = LOCAL_LOG_DIR / "godot_bench_project_smoke_summary.json"
BENCH_RUNNER_SUMMARY_REPORT = LOCAL_LOG_DIR / "godot_bench_runner_summary.json"
BENCH_DIR = ROOT / "spike" / "godot-rurix" / "bench"
GRX006_SCHEMA_SAMPLE_FILES = (
    BENCH_DIR / "schemas" / "baseline_evidence.schema.json",
    BENCH_DIR / "schemas" / "perf_gate_input.schema.json",
    BENCH_DIR / "samples" / "baseline_smoke_example.json",
    BENCH_DIR / "samples" / "perf_gate_failing_example.json",
)
GRX006_PERF_GATE_SCRIPT = BENCH_DIR / "perf_gate.py"
GRX006_BASELINE_SMOKE_SAMPLE = BENCH_DIR / "samples" / "baseline_smoke_example.json"
GRX006_FORBIDDEN_SKIP_SAMPLE = (
    BENCH_DIR / "samples" / "perf_gate_forbidden_skip_example.json"
)
GRX006_MISSING_SAMPLE_COUNT_SAMPLE = (
    BENCH_DIR / "samples" / "baseline_missing_sample_count_example.json"
)
GRX007_VISUAL_DIFF_SCRIPT = BENCH_DIR / "visual_diff.py"
GRX007_VISUAL_SCHEMA = BENCH_DIR / "schemas" / "visual_diff_evidence.schema.json"
GRX007_VISUAL_PLACEHOLDER_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_placeholder.json"
)
GRX007_VISUAL_LDR_PASS_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_ldr_pass_example.json"
)
GRX007_VISUAL_MISSING_LDR_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_pass_missing_ldr_example.json"
)
GRX007_VISUAL_MISMATCH_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_mismatch_example.json"
)
GRX007_VISUAL_SKIP_FAKE_LDR_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_skip_with_fake_ldr_example.json"
)
GRX007_VISUAL_MISSING_FRAME_ARTIFACT_SAMPLE = (
    BENCH_DIR / "samples" / "visual_diff_pass_missing_frame_artifact_example.json"
)
GRX008_FALLBACK_TELEMETRY_SCRIPT = BENCH_DIR / "fallback_telemetry.py"
GRX008_FALLBACK_SCHEMA = BENCH_DIR / "schemas" / "fallback_telemetry.schema.json"
GRX008_FALLBACK_PLACEHOLDER_SAMPLE = (
    BENCH_DIR / "samples" / "fallback_telemetry_placeholder.json"
)
GRX008_FALLBACK_FULL_NULL_TIMESTAMP_SAMPLE = (
    BENCH_DIR / "samples" / "fallback_telemetry_full_null_timestamp_example.json"
)
GRX008_FALLBACK_SCAFFOLD_INACTIVE_SAMPLE = (
    BENCH_DIR / "samples" / "fallback_telemetry_scaffold_fallback_inactive_example.json"
)
GRX009_PASS_DIR_ENV = "RURIX_GRX009_PASS_DIR"
DEFAULT_GRX009_PASS_DIR = BENCH_DIR.parent / "passes" / "luminance_reduction"


def grx009_pass_dir_from_env(env: dict[str, str] | None = None) -> pathlib.Path:
    environ = os.environ if env is None else env
    candidate = environ.get(GRX009_PASS_DIR_ENV)
    if not candidate:
        return DEFAULT_GRX009_PASS_DIR
    return pathlib.Path(candidate).expanduser()


GRX009_PASS_DIR = grx009_pass_dir_from_env()
GRX009_PASS_CONTRACT = GRX009_PASS_DIR / "PASS_CONTRACT.md"
GRX009_PASS_MANIFEST = GRX009_PASS_DIR / "pass_manifest.json"
GRX009_PATCH_0002 = (
    BENCH_DIR.parent / "patches" / "0002-rurix-accel-luminance-pass-gate.patch"
)
GRX009_PATCH_0001 = (
    BENCH_DIR.parent / "patches" / "0001-rurix-accel-module-scaffold.patch"
)
GRX009_PATCH_0003 = (
    BENCH_DIR.parent
    / "patches"
    / "0003-rurix-accel-luminance-core-callsite-wiring.patch"
)
GRX009_PATCH_0004 = (
    BENCH_DIR.parent
    / "patches"
    / "0004-rurix-accel-luminance-resource-mapping-scaffold.patch"
)
GRX009_PATCH_0005 = (
    BENCH_DIR.parent
    / "patches"
    / "0005-rurix-accel-luminance-runtime-binding-preflight.patch"
)
GRX009_PATCH_0006 = (
    BENCH_DIR.parent
    / "patches"
    / "0006-rurix-accel-luminance-gated-dispatch-bringup.patch"
)
GRX009_PATCH_0007 = (
    BENCH_DIR.parent
    / "patches"
    / "0007-rurix-accel-luminance-native-resource-handle-mapping.patch"
)
GRX009_PATCH_0008 = (
    BENCH_DIR.parent
    / "patches"
    / "0008-rurix-accel-luminance-godot-runtime-bridge-recording-smoke.patch"
)
GRX009_PATCH_0009 = (
    BENCH_DIR.parent / "patches" / "0009-rurix-accel-luminance-real-pass-optin.patch"
)
GRX009_PATCH_0010 = (
    BENCH_DIR.parent
    / "patches"
    / "0010-rurix-accel-luminance-real-pass-result-writeback.patch"
)
GRX009_RESOURCE_MAPPING = GRX009_PASS_DIR / "resource_mapping.md"
GRX009_DESCRIPTOR_LAYOUT = (
    GRX009_PASS_DIR
    / "artifacts"
    / "luminance_reduction_descriptor_layout.json"
)
GRX009_DXIL_ARTIFACT = (
    GRX009_PASS_DIR / "artifacts" / "luminance_reduction.dxil"
)
GRX009_ROOT_SIGNATURE_ARTIFACT = (
    GRX009_PASS_DIR / "artifacts" / "luminance_reduction.rts0.bin"
)
GRX009_REAL_D3D12_DISPATCH_SMOKE = (
    GRX009_PASS_DIR / "real_d3d12_dispatch_smoke.json"
)
GRX009_BRIDGE_RECORDING_EVIDENCE = (
    GRX009_PASS_DIR / "bridge_dispatch_recording_evidence.json"
)
GRX009_GODOT_RUNTIME_RECORDING_EVIDENCE = (
    GRX009_PASS_DIR / "godot_runtime_bridge_recording_evidence.json"
)
# Historical measured success artifact: only ever written on a strict
# status=success runtime smoke and never overwritten by a later SKIP/FAIL run.
# The segment 4f readiness gate advances off THIS file, not the reproducible
# SKIP-by-default latest evidence above.
GRX009_GODOT_RUNTIME_RECORDING_SUCCESS_EVIDENCE = (
    GRX009_PASS_DIR / "godot_runtime_bridge_recording_success_evidence.json"
)
# Segment 4h gated real-pass enablement gate: latest evidence is rewritten on
# every run (reproducible-default SKIP without the 0001..0009 scratch exe; a
# completed measured run records skip_kind=measured_prerequisite_blocked and
# the first missing prerequisite). The readiness gate advances only off the
# historical measured success artifact, which is unreachable with the tracked
# segment 4i texture-capable artifact by design (single 8x8 reduction level:
# no multi-level pyramid cascade, no EMA feedback, no previous-luminance double
# buffering, no final-level WRITE_LUMINANCE clamp gating).
GRX009_REAL_PASS_ENABLEMENT_EVIDENCE = (
    GRX009_PASS_DIR / "real_pass_enablement_evidence.json"
)
GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE = (
    GRX009_PASS_DIR / "real_pass_enablement_success_evidence.json"
)
GRX009_REAL_PASS_ENABLEMENT_SCHEMA = (
    GRX009_PASS_DIR / "real_pass_enablement_evidence.schema.json"
)
GRX009_REAL_PASS_ENABLEMENT_TELEMETRY = (
    GRX009_PASS_DIR / "real_pass_enablement_telemetry.json"
)
# Stage A5 owner default-enable decision: written only after the segment 4h
# strict measured success; records keep_default_disabled with the rationale
# (no per-pass FPS evidence, patch 0010 writeback scaffold, level-0-only math
# parity) and the re-evaluation conditions.
GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE = (
    GRX009_PASS_DIR / "real_pass_default_enable_decision.json"
)
GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION_DOC = (
    GRX009_PASS_DIR / "real_pass_default_enable_decision.md"
)
GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION = "keep_default_disabled"
GRX010_NEXT_ACTION = "start_grx010_tonemap_pass_contract"

# --- GRX-010 tonemap pass (segment A: contract + offline hlsl_bridge kernel +
# fail-closed bridge TonemapGate + patch 0011 call-site gate + standalone real
# D3D12 dispatch smoke). The pass stays default disabled and fallback-only;
# these gates never imply real_gpu_pass=true or any performance claim. ---
GRX010_PASS_DIR_ENV = "RURIX_GRX010_PASS_DIR"
DEFAULT_GRX010_PASS_DIR = BENCH_DIR.parent / "passes" / "tonemap"


def grx010_pass_dir_from_env(env: dict[str, str] | None = None) -> pathlib.Path:
    environ = os.environ if env is None else env
    candidate = environ.get(GRX010_PASS_DIR_ENV)
    if not candidate:
        return DEFAULT_GRX010_PASS_DIR
    return pathlib.Path(candidate).expanduser()


GRX010_PASS_DIR = grx010_pass_dir_from_env()
GRX010_PATCH_0011 = (
    BENCH_DIR.parent
    / "patches"
    / "0011-rurix-accel-tonemap-pass-gate-and-callsite.patch"
)
# Next actions along the GRX-010 chain: contract slice ready -> patch 0011
# applyable -> standalone dispatch smoke measured -> runtime resource binding
# slice (0005/0007-level native handle wiring, then 4f/4g/4h-level segments).
GRX010_FIX_PATCH_0011_ACTION = "fix_grx010_tonemap_patch_0011_applyability"
GRX010_PROVIDE_DISPATCH_SMOKE_ACTION = "provide_grx010_tonemap_real_d3d12_dispatch_smoke"
GRX010_NEXT_ACTION_AFTER_CONTRACT = "start_grx010_tonemap_runtime_resource_binding"
# Stage A5 manifest runtime state once the 4h strict measured success exists:
# the DEFAULT runtime path stays fallback-only; the suffix records that the
# opt-in real-pass arm has a measured success.
GRX009_MANIFEST_OPTIN_MEASURED_RUNTIME_STATE = (
    "fallback_only_by_default_real_pass_optin_measured"
)

# Cache for the (expensive) strict 4h success audit, keyed by the resolved
# success-evidence path so monkeypatched fixture pass dirs get their own entry.
_GRX009_REAL_PASS_SUCCESS_AUDIT_CACHE: dict[str, bool] = {}

# --- GRX-010 tonemap stage-A5-equivalent close-out constants. Patch 0012
# (runtime resource binding) and 0013 (recording smoke + real-pass opt-in)
# complete the 0001..0013 stack; the tonemap opt-in real-pass arm has a strict
# measured success (real_pass_enablement_success_evidence.json). The default
# runtime path stays fallback-only, default_enable_state stays disabled, and no
# performance/FPS/GPU-timestamp claim is ever made. Mirrors GRX-009 segment 4h
# + segment 4m. ---
GRX010_PATCH_0012 = (
    BENCH_DIR.parent
    / "patches"
    / "0012-rurix-accel-tonemap-runtime-resource-binding.patch"
)
GRX010_PATCH_0013 = (
    BENCH_DIR.parent
    / "patches"
    / "0013-rurix-accel-tonemap-recording-smoke-and-real-pass-optin.patch"
)
# The full tonemap patch stack 0001..0013 (GRX-009 0001..0010 + tonemap
# 0011..0013). Reused for the success-evidence patch-stack-identity and scratch
# source provenance audits.
GRX010_PATCH_STACK_ID = "0001..0013"
GRX010_PATCH_STACK_FILES = (
    GRX009_PATCH_0001,
    GRX009_PATCH_0002,
    GRX009_PATCH_0003,
    GRX009_PATCH_0004,
    GRX009_PATCH_0005,
    GRX009_PATCH_0006,
    GRX009_PATCH_0007,
    GRX009_PATCH_0008,
    GRX009_PATCH_0009,
    GRX009_PATCH_0010,
    GRX010_PATCH_0011,
    GRX010_PATCH_0012,
    GRX010_PATCH_0013,
)
GRX010_OFFLINE_COMPILE_EVIDENCE = GRX010_PASS_DIR / "offline_compile_evidence.json"
GRX010_DXIL_ARTIFACT = GRX010_PASS_DIR / "artifacts" / "tonemap.dxil"
GRX010_ROOT_SIGNATURE_ARTIFACT = GRX010_PASS_DIR / "artifacts" / "tonemap.rts0.bin"
GRX010_DESCRIPTOR_LAYOUT = (
    GRX010_PASS_DIR / "artifacts" / "tonemap_descriptor_layout.json"
)
# Tonemap real-pass enablement gate (mirror of GRX-009 segment 4h): the latest
# evidence is reproducible-default SKIP without the 0001..0013 scratch exe; the
# readiness gate advances only off the historical measured success artifact.
GRX010_REAL_PASS_ENABLEMENT_EVIDENCE = (
    GRX010_PASS_DIR / "real_pass_enablement_evidence.json"
)
GRX010_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE = (
    GRX010_PASS_DIR / "real_pass_enablement_success_evidence.json"
)
GRX010_REAL_PASS_ENABLEMENT_SCHEMA = (
    GRX010_PASS_DIR / "real_pass_enablement_evidence.schema.json"
)
GRX010_REAL_PASS_ENABLEMENT_TELEMETRY = (
    GRX010_PASS_DIR / "real_pass_enablement_telemetry.json"
)
# Tonemap visual real-pass frame artifacts (committed only on a strict
# status=success run; hash-pinned in the success evidence).
GRX010_VISUAL_REAL_PASS_REFERENCE_FRAME = (
    GRX010_PASS_DIR / "artifacts" / "visual" / "tonemap_real_pass_reference.rgb8"
)
GRX010_VISUAL_REAL_PASS_CANDIDATE_FRAME = (
    GRX010_PASS_DIR / "artifacts" / "visual" / "tonemap_real_pass_candidate.rgb8"
)
GRX010_VISUAL_REAL_PASS_DIFF_ARTIFACT = (
    GRX010_PASS_DIR / "artifacts" / "visual" / "tonemap_real_pass_diff.rgb8"
)
# Owner default-enable decision (mirror of GRX-009 segment 4m): written only
# after the tonemap real-pass strict measured success; records
# keep_default_disabled with the rationale (no per-pass FPS evidence, patch
# 0013 writeback scaffold, LINEAR + sRGB-only math subset) and the
# re-evaluation conditions.
GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE = (
    GRX010_PASS_DIR / "real_pass_default_enable_decision.json"
)
GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION_DOC = (
    GRX010_PASS_DIR / "real_pass_default_enable_decision.md"
)
GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION = "keep_default_disabled"
GRX010_DECISION_SEGMENT = "grx010_real_pass_default_enable_decision"
# Stage-A5 pins mirrored from ci/grx010_tonemap_real_pass_enablement_smoke.py;
# the validation-failed regression test asserts probe/harness parity.
GRX010_REAL_PASS_SUBJECT = "grx010_tonemap_real_pass_enablement_smoke"
GRX010_REAL_PASS_SEGMENT = "grx010_real_pass_enablement"
GRX010_SUCCESS_EVIDENCE_KIND = "historical_measured_success"
GRX010_LATEST_EVIDENCE_REL_PATH = (
    "spike/godot-rurix/passes/tonemap/real_pass_enablement_evidence.json"
)
GRX010_EXPECTED_FIRST_MISSING_PREREQUISITE = "real_dispatch_recording_failed"
GRX010_FALLBACK_MARKER = (
    "RurixAccel: tonemap native resource handle mapping fallback rc="
)
GRX010_REAL_PASS_BLOCKED_MARKER = "RXGD_TONEMAP_REAL_PASS_BLOCKED"
GRX010_REAL_PASS_MARKER = "RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS"
GRX010_WRITEBACK_MARKER = "RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS_WRITEBACK"
GRX010_RECORD_MARKER = "RXGD_GODOT_RUNTIME_TONEMAP_RECORD"
GRX010_MANIFEST_OPTIN_MEASURED_RUNTIME_STATE = (
    "fallback_only_by_default_real_pass_optin_measured"
)
# The 22 checks the tonemap real-pass enablement smoke emits: all must be True
# on a strict success EXCEPT the two candidate-fallback checks, which must be
# False (a success candidate leg never falls back / never prints the blocked
# diagnostic).
GRX010_REQUIRED_TRUE_CHECKS = (
    "artifact_hashes_match_offline_evidence",
    "reference_run_exit_zero",
    "candidate_run_exit_zero",
    "forced_fallback_run_exit_zero",
    "session_ready_all_runs",
    "markers_absent_reference",
    "fallback_marker_observed_forced_fallback",
    "real_pass_blocked_marker_observed_forced_fallback",
    "record_marker_absent_all_runs",
    "frames_captured",
    "dimensions_match",
    "capture_frame_indices_match",
    "runtime_log_audit_clean",
    "diff_within_threshold_candidate",
    "diff_within_threshold_forced_fallback",
    "telemetry_document_valid",
    "telemetry_entries_coherent",
    "scratch_source_provenance_ok",
    "native_continuation_writeback_scaffold",
    "real_pass_dispatched_and_completed",
)
GRX010_REQUIRED_FALSE_CHECKS = (
    "fallback_marker_observed_candidate",
    "real_pass_blocked_marker_observed_candidate",
)
# Next actions along the GRX-010 close-out chain, consulted only once the
# segment A slice (contract + patch 0011 + standalone dispatch smoke) is ready.
GRX010_FIX_PATCH_0012_ACTION = "fix_grx010_patch_0012_applyability"
GRX010_FIX_PATCH_0013_ACTION = "fix_grx010_patch_0013_applyability"
GRX010_PROVIDE_ENABLEMENT_ACTION = (
    "provide_grx010_tonemap_real_pass_enablement_success"
)
GRX010_DESIGN_DECISION_ACTION = (
    "design_grx010_tonemap_real_pass_default_enable_decision"
)
GRX011_NEXT_ACTION = "start_grx011_ssao_blur_godot_patch_0014"

# --- GRX gate sequence (table-driven per-pass registration) ----------------
# Industrialized GRX-011+ scaffolding. From GRX-011 onward every pass ships one
# gate module under ci/grx_gates/ exporting `evaluate() -> dict` (see
# ci/grx_gates/_common.py for the interface contract). Registered gates are
# listed here in order; `walk_grx_gate_sequence` (below) consults them, fail
# closed, ONLY once the legacy grx010 chain has closed out and handed off to
# grx011 (next_action == GRX011_NEXT_ACTION).
#
# The table is EMPTY until Wave 2 registers the first downstream gate (grx011);
# while it is empty the probe's next_action is computed exactly as before (this
# is a HARD regression requirement — do not add behaviour on the empty path).
# Each entry is a dict describing one gate module:
#   {"gate_id": "grx011", "module": "grx011_ssao_blur"}    # ci/grx_gates/<module>.py
#   {"gate_id": "grx011", "module_path": "/abs/path.py"}   # explicit path (tests)
GRX_GATES_DIR = ROOT / "ci" / "grx_gates"
# Wave 2 registers the first downstream gate (grx011 ssao_blur). The gate is
# consulted fail-closed by walk_grx_gate_sequence: until its contract + patch
# applyability + standalone dispatch smoke + real-pass enablement + owner
# default-enable decision are all green it reports not-ready and the probe
# leaves next_action UNCHANGED (a recorded grx_gate_module_error, never a
# silent advance).
GRX_GATE_SEQUENCE: list[dict[str, object]] = [
    {"gate_id": "grx011", "module": "grx011_ssao_blur"},
    {"gate_id": "grx012", "module": "grx012_taa_resolve"},
    {"gate_id": "grx013", "module": "grx013_particles_copy"},
    {"gate_id": "grx014", "module": "grx014_cluster_store"},
]
GRX_GATE_REQUIRED_KEYS = (
    "gate_id",
    "contract_ready",
    "patch_applyability",
    "dispatch_smoke_ready",
    "enablement_ready",
    "decision_ready",
    "first_issue",
    "next_action",
)
GRX_GATE_READINESS_KEYS = (
    "contract_ready",
    "patch_applyability",
    "dispatch_smoke_ready",
    "enablement_ready",
    "decision_ready",
)

# Cache for the (expensive) strict tonemap real-pass success audit.
_GRX010_REAL_PASS_SUCCESS_AUDIT_CACHE: dict[str, bool] = {}


def grx009_real_pass_measured_success_active() -> bool:
    """Stage A5 fail-closed switch: True only when the segment 4h strict
    measured success artifact ``real_pass_enablement_success_evidence.json``
    exists AND passes the full strict audit
    (``grx009_segment4h_real_pass_enablement_issue``). A missing artifact, a
    SKIP/FAIL document, or a hand-edited placeholder never activates the
    relaxed stage A5 manifest acceptance (implemented=true,
    real_gpu_pass=true, runtime_state=fallback_only_by_default_real_pass_optin_measured)."""
    path = GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE
    if not path.exists():
        return False
    key = str(path)
    cached = _GRX009_REAL_PASS_SUCCESS_AUDIT_CACHE.get(key)
    if cached is None:
        cached = grx009_segment4h_real_pass_enablement_issue() is None
        _GRX009_REAL_PASS_SUCCESS_AUDIT_CACHE[key] = cached
    return cached


def grx009_real_pass_success_evidence_conflict() -> bool:
    """True when a 4h success artifact exists but FAILS the strict audit.

    Pre-A5 gates required the success artifact to not exist at all; stage A5
    relaxes that to "may exist only when it is a real audited strict success",
    keeping the fail-closed rejection for placeholders/tampered documents."""
    return (
        GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE.exists()
        and not grx009_real_pass_measured_success_active()
    )


def grx009_manifest_implemented_ok(manifest: dict[str, object]) -> bool:
    """implemented=false is always accepted; implemented=true only under the
    audited stage A5 measured success (fail-closed otherwise)."""
    value = manifest.get("implemented")
    if value is False:
        return True
    return value is True and grx009_real_pass_measured_success_active()


def grx009_manifest_runtime_state_ok(
    implementation_status: dict[str, object],
) -> bool:
    """runtime_state=fallback_only is always accepted; the stage A5
    fallback_only_by_default_real_pass_optin_measured value only under the
    audited measured success (fail-closed otherwise)."""
    runtime_state = implementation_status.get("runtime_state")
    if runtime_state == "fallback_only":
        return True
    return (
        runtime_state == GRX009_MANIFEST_OPTIN_MEASURED_RUNTIME_STATE
        and grx009_real_pass_measured_success_active()
    )


def grx009_manifest_real_gpu_pass_ok(
    implementation_status: dict[str, object],
) -> bool:
    """real_gpu_pass=false is always accepted; true only under the audited
    stage A5 measured success (fail-closed otherwise)."""
    value = implementation_status.get("real_gpu_pass")
    if value is False:
        return True
    return value is True and grx009_real_pass_measured_success_active()
GRX009_VISUAL_REAL_PASS_REFERENCE_FRAME = (
    GRX009_PASS_DIR / "artifacts" / "visual" / "luminance_real_pass_reference.rgb8"
)
GRX009_VISUAL_REAL_PASS_CANDIDATE_FRAME = (
    GRX009_PASS_DIR / "artifacts" / "visual" / "luminance_real_pass_candidate.rgb8"
)
GRX009_VISUAL_REAL_PASS_DIFF_ARTIFACT = (
    GRX009_PASS_DIR / "artifacts" / "visual" / "luminance_real_pass_diff.rgb8"
)
GRX009_BRIDGE_LIB = ROOT / "src" / "rurix-godot" / "src" / "lib.rs"
GRX009_DISABLED_TELEMETRY_SAMPLE = (
    BENCH_DIR / "samples" / "fallback_telemetry_luminance_disabled_example.json"
)
GRX009_CALLSITE_WIRED_TELEMETRY_SAMPLE = (
    BENCH_DIR
    / "samples"
    / "fallback_telemetry_luminance_callsite_wired_disabled_example.json"
)
GRX009_COMPILE_EVIDENCE = GRX009_PASS_DIR / "offline_compile_evidence.json"
GRX009_TEXTURE_DXC_FEASIBILITY_EVIDENCE = (
    GRX009_PASS_DIR / "texture_dxc_feasibility_evidence.json"
)
GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC = (
    GRX009_PASS_DIR / "dxc_texture_artifact_bridge.md"
)
GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DESIGN = (
    GRX009_PASS_DIR / "dxc_texture_artifact_bridge_design.json"
)
GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_SCAFFOLD_EVIDENCE = (
    GRX009_PASS_DIR / "dxc_texture_artifact_bridge_scaffold_evidence.json"
)
GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR = (
    GRX009_PASS_DIR / "artifacts" / "dxc_texture_bridge"
)
GRX009_DXC_TEXTURE_BRIDGE_DESCRIPTOR_LAYOUT = (
    GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR / "descriptor_layout.json"
)
GRX009_DXC_TEXTURE_BRIDGE_ROOT_SIGNATURE_METADATA = (
    GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR / "root_signature_scaffold.json"
)
GRX009_DXC_TEXTURE_BRIDGE_RTS0_ARTIFACT = (
    GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR / "root_signature.rts0.bin"
)
GRX009_DXC_TEXTURE_DESCRIPTOR_RTS0_CROSSCHECK_EVIDENCE = (
    GRX009_PASS_DIR / "dxc_texture_descriptor_rts0_crosscheck_evidence.json"
)
GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_EVIDENCE = (
    GRX009_PASS_DIR / "texture_artifact_provenance_policy.json"
)
GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC = (
    GRX009_PASS_DIR / "texture_artifact_provenance_policy.md"
)
# Segment 4i historical raw-buffer fixture: the original segment 3a artifact
# chain (raw_buffer_view kernel) that first proved segment 3a offline compile
# success. The canonical GRX009_COMPILE_EVIDENCE now reports the *newer*
# texture-capable compile attempt, which is expected to fail closed
# (status=compile_failed) until a patched llc supports texture intrinsics;
# this file lets segment 3a+/4a+ readiness re-verify the still-valid
# historical success instead of requiring the canonical file to literally
# read status=success.
GRX009_RAW_BUFFER_COMPILE_EVIDENCE = (
    GRX009_PASS_DIR / "offline_compile_evidence_raw_buffer.json"
)
GRX009_COMPILE_SCHEMA = GRX009_PASS_DIR / "compile_evidence.schema.json"
GRX009_VISUAL_FALLBACK_SCHEMA = (
    GRX009_PASS_DIR / "visual_fallback_evidence.schema.json"
)
# The *latest* segment 4g run evidence: rewritten on every run, honestly SKIP
# when the tracked Godot exe is unavailable; never advances the gate on its own.
GRX009_VISUAL_FALLBACK_EVIDENCE = GRX009_PASS_DIR / "visual_fallback_evidence.json"
# The *historical measured success* segment 4g artifact: only ever written on a
# strict status=success run; the segment 4g readiness gate advances off THIS file.
GRX009_VISUAL_FALLBACK_SUCCESS_EVIDENCE = (
    GRX009_PASS_DIR / "visual_fallback_success_evidence.json"
)
GRX009_MEASURED_FALLBACK_TELEMETRY = (
    GRX009_PASS_DIR / "measured_fallback_telemetry.json"
)
GRX009_VISUAL_REFERENCE_FRAME = (
    GRX009_PASS_DIR / "artifacts" / "visual" / "luminance_fallback_reference.rgb8"
)
GRX009_VISUAL_CANDIDATE_FRAME = (
    GRX009_PASS_DIR / "artifacts" / "visual" / "luminance_fallback_candidate.rgb8"
)
GRX009_VISUAL_DIFF_ARTIFACT = (
    GRX009_PASS_DIR / "artifacts" / "visual" / "luminance_fallback_diff.rgb8"
)
# Segment 4g visual gate pins. These MUST stay in sync with the constants in
# ci/grx009_segment4g_visual_fallback_smoke.py; the regression test asserts it.
GRX009_SEGMENT4G_METRIC_KIND = "ldr_absolute_diff"
GRX009_SEGMENT4G_FRAME_FORMAT = "R8G8B8_raw"
GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD = 2
GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD = 0.25
GRX009_SEGMENT4G_MIN_FRAME_DIMENSION = 64
GRX009_SEGMENT4G_MEAN_ABS_EPSILON = 1e-9
# The ONLY Godot `ERROR:` line the segment 4g runtime log audit tolerates
# (with a recorded rationale). Pin parity with the smoke's ALLOWED_GODOT_ERROR
# is asserted by the validation-failed regression test.
GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR = "Could not load global script cache"
# Segment 4h gate pins. These MUST stay in sync with the constants in
# ci/grx009_segment4h_real_pass_enablement_smoke.py; the regression test
# asserts equality. The visual pins (metric/format/thresholds/min dimension)
# are shared with segment 4g above.
GRX009_SEGMENT4H_EXPECTED_FIRST_MISSING_PREREQUISITE = "real_dispatch_recording_failed"
GRX009_SEGMENT4H_FALLBACK_MARKER = (
    "RurixAccel: luminance pyramid real-pass fallback rc="
)
GRX009_SEGMENT4H_BLOCKED_MARKER = "RXGD_REAL_PASS_BLOCKED"
GRX009_SEGMENT4H_REAL_PASS_MARKER = "RXGD_GODOT_RUNTIME_LUMINANCE_REAL_PASS"
GODOT_EXE_ENV_SEGMENT4H = "RURIX_GRX009_SEGMENT4H_GODOT_EXE"
GRX009_SEGMENT3A_BLOCKED_COMPILE_STATUSES = {
    "compile_failed",
    "toolchain_missing",
    "validation_failed",
}
RURIX_LLC_ENV = "RURIX_LLC"
RURIX_DXC_DIR_ENV_KEYS = ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR")
DXC_VALIDATOR_SUITE_FILES = ("dxc.exe", "dxv.exe", "dxil.dll")
PROBE_TIMEOUT_SECONDS = 10
DXC_IDENTITY_MARKERS = ("dxc", "dxcompiler", "directx shader compiler")
DXV_IDENTITY_MARKERS = ("dxil validator", "dxil", "validator")
LOCAL_SCONS_VENV = LOCAL_LOG_DIR / "scons-venv"
LOCAL_SCONS_PYTHON = LOCAL_SCONS_VENV / "Scripts" / "python.exe"
LOCAL_GODOT_LOCALAPPDATA = LOCAL_LOG_DIR / "localappdata"
LOCAL_GODOT_BUILD_DEPS = LOCAL_GODOT_LOCALAPPDATA / "Godot" / "build_deps"
GODOT_INSTALL_ACCESSKIT = (
    ROOT / "external" / "godot-master" / "misc" / "scripts" / "install_accesskit.py"
)
GODOT_INSTALL_D3D12_DEPS = (
    ROOT / "external" / "godot-master" / "misc" / "scripts" / "install_d3d12_sdk_windows.py"
)
VSWHERE = pathlib.Path(
    os.environ.get(
        "RURIX_VSWHERE",
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    )
)
VCVARSALL_REL = pathlib.Path("VC") / "Auxiliary" / "Build" / "vcvarsall.bat"

DEFAULT_SDK_INCLUDE_ROOTS = [
    pathlib.Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)"))
    / "Windows Kits"
    / "10"
    / "Include",
]
DEFAULT_SDK_BIN_ROOTS = [
    pathlib.Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)"))
    / "Windows Kits"
    / "10"
    / "bin",
]
HEADER_CANDIDATES = ("d3d12.h", "dxgi1_6.h")
TOOL_CANDIDATES = ("dxc.exe", "dxv.exe")
SCONS_BUILD_ARGS = (
    "platform=windows target=template_debug d3d12=yes "
    "module_rurix_accel_enabled=yes disable_path_overrides=no"
)
SCONS_ICE_ARGS = (
    SCONS_BUILD_ARGS + " num_jobs=1 verbose=yes angle=no silence_msvc=no"
)
PROBE_COMMAND = "py -3 ci/godot_rurix_toolchain_probe.py"
LOAD_SMOKE_COMMAND = r"py -3 ci\godot_rurix_load_smoke.py"
TOOLCHAIN_ENV_KEYS = (
    "VSINSTALLDIR",
    "VCINSTALLDIR",
    "VCToolsInstallDir",
    "VCTOOLSINSTALLDIR",
    "VisualStudioVersion",
)
REQUIRED_BUILD_ARTIFACT_KEYS = (
    "godot_exe",
    "godot_console_exe",
    "module_rurix_accel_lib",
)
REQUIRED_SCONS_ARGS = ("disable_path_overrides=no",)
VS_INSTALL_RE = re.compile(
    r"(?i)([A-Z]:\\[^:\n\r]*?Microsoft Visual Studio\\\d{4}\\[^\\\n\r]+)"
)

HOST_MACHINE = platform.machine().lower()
if HOST_MACHINE in ("amd64", "x86_64", "x64"):
    GODOT_WINDOWS_ARCH = "x86_64"
    ACCESSKIT_WINDOWS_ARCH = "x86_64"
    PIX_WINDOWS_ARCH = "x64"
    AGILITY_WINDOWS_ARCH = "x64"
elif HOST_MACHINE in ("arm64", "aarch64"):
    GODOT_WINDOWS_ARCH = "arm64"
    ACCESSKIT_WINDOWS_ARCH = "arm64"
    PIX_WINDOWS_ARCH = "ARM64"
    AGILITY_WINDOWS_ARCH = "arm64"
elif HOST_MACHINE in ("x86", "i386", "i686"):
    GODOT_WINDOWS_ARCH = "x86_32"
    ACCESSKIT_WINDOWS_ARCH = "x86"
    PIX_WINDOWS_ARCH = "x86"
    AGILITY_WINDOWS_ARCH = "x86"
else:
    GODOT_WINDOWS_ARCH = "x86_64"
    ACCESSKIT_WINDOWS_ARCH = "x86_64"
    PIX_WINDOWS_ARCH = "x64"
    AGILITY_WINDOWS_ARCH = "x64"


@dataclass
class ProbeResult:
    name: str
    status: str
    reason: str
    details: dict[str, object]


def print_result(result: ProbeResult) -> None:
    print(f"[godot-toolchain] {result.name}: {result.status} - {result.reason}")
    if result.details:
        for key, value in sorted(result.details.items()):
            print(f"[godot-toolchain]   {key}: {value}")


def completed_output(proc: subprocess.CompletedProcess[str]) -> str:
    parts = []
    stdout = (proc.stdout or "").strip()
    stderr = (proc.stderr or "").strip()
    if stdout:
        parts.append(stdout)
    if stderr:
        parts.append(stderr)
    return " | ".join(parts)


def timeout_output(exc: subprocess.TimeoutExpired) -> str:
    parts = [f"command timed out after {exc.timeout} seconds"]
    for value in (exc.stdout, exc.stderr):
        if isinstance(value, bytes):
            value = value.decode("utf-8", errors="ignore")
        if isinstance(value, str) and value.strip():
            parts.append(value.strip())
    return " | ".join(parts)


def cleaned_lines(text: str) -> list[str]:
    return [line.strip() for line in text.splitlines() if line.strip()]


def normalize_string(value: object) -> str | None:
    if not isinstance(value, str):
        return None
    stripped = value.strip()
    return stripped or None


def evaluate_grx_gate_entry(entry: object) -> dict[str, object]:
    """Load and evaluate one GRX gate-module entry, fail closed.

    Returns a normalized record with keys ``gate_id``, ``all_ready``,
    ``next_action``, ``first_issue``, ``module_error`` and ``evaluation``.
    ``module_error`` is non-None on ANY failure (non-dict entry, missing/broken
    module file, import error, missing/non-callable ``evaluate``, ``evaluate``
    raising, a non-dict result, or an interface violation). ``all_ready`` is
    True only when every readiness key is True and ``first_issue`` is None; the
    probe never advances ``next_action`` from a module that is not ``all_ready``.
    See ci/grx_gates/_common.py for the gate-module interface contract.
    """
    gate_id = "unknown"
    if isinstance(entry, dict):
        gate_id = normalize_string(entry.get("gate_id")) or "unknown"
    result: dict[str, object] = {
        "gate_id": gate_id,
        "all_ready": False,
        "next_action": None,
        "first_issue": None,
        "module_error": None,
        "evaluation": None,
    }
    if not isinstance(entry, dict):
        result["module_error"] = "gate entry is not a dict"
        return result

    module_path_text = normalize_string(entry.get("module_path"))
    if module_path_text:
        module_file = pathlib.Path(module_path_text)
    else:
        module_name = normalize_string(entry.get("module"))
        if not module_name:
            result["module_error"] = "gate entry missing both 'module' and 'module_path'"
            return result
        module_file = GRX_GATES_DIR / f"{module_name}.py"
    if not module_file.is_file():
        result["module_error"] = f"gate module file not found: {module_file}"
        return result

    # Make the shared gate helpers importable (`import _common`) regardless of
    # how the probe itself was launched (script vs in-process import).
    gates_dir = str(GRX_GATES_DIR)
    if gates_dir not in sys.path:
        sys.path.insert(0, gates_dir)

    try:
        spec = importlib.util.spec_from_file_location(
            f"grx_gate_{gate_id}_{module_file.stem}", module_file
        )
        if spec is None or spec.loader is None:
            result["module_error"] = f"could not build import spec for {module_file}"
            return result
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
    except Exception as exc:  # noqa: BLE001 - fail closed on any load error
        result["module_error"] = f"gate module import failed: {type(exc).__name__}: {exc}"
        return result

    evaluate = getattr(module, "evaluate", None)
    if not callable(evaluate):
        result["module_error"] = "gate module does not export a callable evaluate()"
        return result
    try:
        evaluation = evaluate()
    except Exception as exc:  # noqa: BLE001 - fail closed on any evaluate error
        result["module_error"] = f"evaluate() raised {type(exc).__name__}: {exc}"
        return result
    if not isinstance(evaluation, dict):
        result["module_error"] = "evaluate() did not return a dict"
        return result

    missing = [key for key in GRX_GATE_REQUIRED_KEYS if key not in evaluation]
    if missing:
        result["module_error"] = f"evaluate() result missing required keys: {missing}"
        return result
    for key in GRX_GATE_READINESS_KEYS:
        if not isinstance(evaluation.get(key), bool):
            result["module_error"] = f"evaluate() key {key!r} must be a bool"
            return result
    first_issue_value = evaluation.get("first_issue")
    if first_issue_value is not None and not isinstance(first_issue_value, str):
        result["module_error"] = "evaluate() key 'first_issue' must be str or None"
        return result
    next_action_value = evaluation.get("next_action")
    if next_action_value is not None and not isinstance(next_action_value, str):
        result["module_error"] = "evaluate() key 'next_action' must be str or None"
        return result

    result["evaluation"] = evaluation
    result["first_issue"] = normalize_string(first_issue_value)
    result["next_action"] = normalize_string(next_action_value)
    result["all_ready"] = (
        all(evaluation.get(key) is True for key in GRX_GATE_READINESS_KEYS)
        and result["first_issue"] is None
    )
    return result


def walk_grx_gate_sequence(
    sequence: object,
    base_next_action: str | None,
    base_next_action_reason: str | None = None,
    base_next_command: str | None = None,
) -> dict[str, object]:
    """Table-driven walk over the GRX gate sequence, fail closed.

    Starts from the base ``next_action`` produced by the legacy grx010 chain.
    For each registered gate in order: if the module fails to load, violates the
    ``evaluate()`` interface, or reports a non-empty ``first_issue`` / any
    readiness key false, the walk records a ``grx_gate_module_error``, leaves
    ``next_action`` UNCHANGED, and stops (does not consult later gates). Only a
    fully-ready gate advances ``next_action`` to its gate-provided value and
    lets the walk continue. An EMPTY sequence is a pure no-op, so the returned
    ``next_action`` equals ``base_next_action`` (hard regression requirement).
    """
    next_action = base_next_action
    next_action_reason = base_next_action_reason
    next_command = base_next_command
    evaluations: list[dict[str, object]] = []
    module_errors: list[dict[str, object]] = []
    for entry in sequence:
        evaluation = evaluate_grx_gate_entry(entry)
        module_error = evaluation.get("module_error")
        if module_error is None and evaluation.get("all_ready") is not True:
            # A conforming module that is simply not complete yet: fail closed
            # (do not advance) and surface the first_issue as a gate error.
            module_error = "gate not ready: first_issue=" + (
                normalize_string(evaluation.get("first_issue")) or "unknown"
            )
        record = {
            "gate_id": evaluation.get("gate_id"),
            "all_ready": evaluation.get("all_ready"),
            "first_issue": evaluation.get("first_issue"),
            "next_action": evaluation.get("next_action"),
            "module_error": module_error,
        }
        evaluations.append(record)
        if module_error is not None:
            module_errors.append(
                {"gate_id": evaluation.get("gate_id"), "reason": module_error}
            )
            break
        candidate = normalize_string(evaluation.get("next_action"))
        if candidate:
            next_action = candidate
            next_action_reason = (
                f"GRX gate {evaluation.get('gate_id')} reports contract, patch, "
                "dispatch, enablement and decision all ready with no first_issue; "
                f"advancing next_action to {candidate}."
            )
            next_command = None
    return {
        "next_action": next_action,
        "next_action_reason": next_action_reason,
        "next_command": next_command,
        "evaluations": evaluations,
        "module_errors": module_errors,
    }


def infer_vs_installation_root(raw_path: object) -> str | None:
    candidate = normalize_string(raw_path)
    if not candidate:
        return None
    match = VS_INSTALL_RE.search(candidate)
    if match:
        return match.group(1)
    return None


def run_cmd_chain(
    command: str,
    *,
    vcvarsall: str | None = None,
) -> subprocess.CompletedProcess[str]:
    if vcvarsall:
        with tempfile.NamedTemporaryFile(
            "w",
            suffix=".bat",
            delete=False,
            encoding="utf-8",
            newline="\r\n",
        ) as handle:
            handle.write("@echo off\r\n")
            handle.write(f'call "{vcvarsall}" x64 >nul\r\n')
            handle.write("if errorlevel 1 exit /b %errorlevel%\r\n")
            handle.write(command + "\r\n")
            temp_path = handle.name
        try:
            return subprocess.run(
                ["cmd.exe", "/d", "/c", temp_path],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
                timeout=PROBE_TIMEOUT_SECONDS,
            )
        finally:
            try:
                pathlib.Path(temp_path).unlink(missing_ok=True)
            except OSError:
                pass
    return subprocess.run(
        ["cmd.exe", "/d", "/c", command],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
        timeout=PROBE_TIMEOUT_SECONDS,
    )


def read_env_var_from_cmd(var_name: str, vcvarsall: str | None = None) -> str | None:
    proc = run_cmd_chain(f"set {var_name}", vcvarsall=vcvarsall)
    if proc.returncode != 0:
        return None
    for line in cleaned_lines(proc.stdout):
        prefix = f"{var_name}="
        if line.startswith(prefix):
            return line[len(prefix) :]
    return None


def collect_msvc_shell_evidence(vcvarsall: str | None = None) -> dict[str, object]:
    details: dict[str, object] = {
        "mode": "vcvarsall_x64" if vcvarsall else "current_shell",
    }
    if vcvarsall:
        details["vcvarsall_bat"] = vcvarsall

    for env_key in TOOLCHAIN_ENV_KEYS:
        value = (
            read_env_var_from_cmd(env_key, vcvarsall)
            if vcvarsall
            else normalize_string(os.environ.get(env_key))
        )
        if value:
            details[env_key] = value

    where_proc = run_cmd_chain("where cl", vcvarsall=vcvarsall)
    where_lines = cleaned_lines(where_proc.stdout)
    if where_lines:
        details["where_cl"] = where_lines
        details["compiler_path"] = where_lines[0]
        install_root = infer_vs_installation_root(where_lines[0])
        if install_root:
            details["compiler_installation_root"] = install_root
    if where_proc.returncode != 0:
        details["where_cl_error"] = completed_output(where_proc) or str(where_proc.returncode)

    cl_bv_proc = run_cmd_chain("cl /Bv", vcvarsall=vcvarsall)
    cl_bv_output = completed_output(cl_bv_proc)
    if cl_bv_output:
        details["cl_bv"] = cl_bv_output
    details["cl_bv_exit_code"] = cl_bv_proc.returncode
    return details


def load_json_report(path: pathlib.Path) -> dict[str, object] | None:
    if not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def load_json_file(path: pathlib.Path) -> dict[str, object] | None:
    if not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def file_contains_all(path: pathlib.Path, needles: list[str]) -> bool:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return False
    return all(needle in text for needle in needles)


def validate_fallback_telemetry_sample(sample_path: pathlib.Path) -> bool:
    if not sample_path.exists() or not GRX008_FALLBACK_TELEMETRY_SCRIPT.exists():
        return False
    return (
        _bench_script_exit_code(
            GRX008_FALLBACK_TELEMETRY_SCRIPT,
            ["--validate-only", str(sample_path)],
        )
        == 0
    )


def grx009_patch_stack_result() -> dict[str, object]:
    return evaluate_patch_stack(
        ROOT,
        EXTERNAL_GODOT,
        GRX009_PATCH_0001,
        GRX009_PATCH_0002,
        GRX009_PATCH_0003,
    )


def grx009_patch_stack_ready(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx009_patch_stack_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx009_patch_0004_applyability_result() -> dict[str, object]:
    return evaluate_followup_patch_applyability(
        ROOT,
        EXTERNAL_GODOT,
        GRX009_PATCH_0004,
        "0004",
    )


def grx009_patch_0004_applyable(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx009_patch_0004_applyability_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx009_patch_0005_applyability_result() -> dict[str, object]:
    """0005 stacks on 0004, which is forward-applicable but not applied to the
    snapshot, so the check runs in a temporary scratch copy (the snapshot
    working tree is never modified)."""
    return evaluate_stacked_patch_applyability(
        ROOT,
        EXTERNAL_GODOT,
        [GRX009_PATCH_0004],
        GRX009_PATCH_0005,
        "0005",
    )


def grx009_patch_0005_applyable(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx009_patch_0005_applyability_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx009_patch_0006_applyability_result() -> dict[str, object]:
    """0006 stacks on 0004+0005, which are forward-applicable but not applied
    to the snapshot, so the check runs in a temporary scratch copy (the
    snapshot working tree is never modified)."""
    return evaluate_stacked_patch_applyability(
        ROOT,
        EXTERNAL_GODOT,
        [GRX009_PATCH_0004, GRX009_PATCH_0005],
        GRX009_PATCH_0006,
        "0006",
    )


def grx009_patch_0006_applyable(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx009_patch_0006_applyability_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx009_patch_0007_applyability_result() -> dict[str, object]:
    """0007 stacks on 0004+0005+0006, which are forward-applicable but not
    applied to the snapshot, so the check runs in a temporary scratch copy (the
    snapshot working tree is never modified)."""
    return evaluate_stacked_patch_applyability(
        ROOT,
        EXTERNAL_GODOT,
        [GRX009_PATCH_0004, GRX009_PATCH_0005, GRX009_PATCH_0006],
        GRX009_PATCH_0007,
        "0007",
    )


def grx009_patch_0007_applyable(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx009_patch_0007_applyability_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx009_patch_0008_applyability_result() -> dict[str, object]:
    """0008 stacks on 0004+0005+0006+0007, which are forward-applicable but not
    applied to the snapshot, so the check runs in a temporary scratch copy (the
    snapshot working tree is never modified)."""
    return evaluate_stacked_patch_applyability(
        ROOT,
        EXTERNAL_GODOT,
        [GRX009_PATCH_0004, GRX009_PATCH_0005, GRX009_PATCH_0006, GRX009_PATCH_0007],
        GRX009_PATCH_0008,
        "0008",
    )


def grx009_patch_0008_applyable(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx009_patch_0008_applyability_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx009_patch_0009_applyability_result() -> dict[str, object]:
    """0009 stacks on 0004+0005+0006+0007+0008, which are forward-applicable
    but not applied to the snapshot, so the check runs in a temporary scratch
    copy (the snapshot working tree is never modified)."""
    return evaluate_stacked_patch_applyability(
        ROOT,
        EXTERNAL_GODOT,
        [
            GRX009_PATCH_0004,
            GRX009_PATCH_0005,
            GRX009_PATCH_0006,
            GRX009_PATCH_0007,
            GRX009_PATCH_0008,
        ],
        GRX009_PATCH_0009,
        "0009",
    )


def grx009_patch_0009_applyable(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx009_patch_0009_applyability_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx010_patch_0011_applyability_result() -> dict[str, object]:
    """0011 stacks on 0004..0010, which are forward-applicable but not applied
    to the snapshot, so the check runs in a temporary scratch copy (the
    snapshot working tree is never modified)."""
    return evaluate_stacked_patch_applyability(
        ROOT,
        EXTERNAL_GODOT,
        [
            GRX009_PATCH_0004,
            GRX009_PATCH_0005,
            GRX009_PATCH_0006,
            GRX009_PATCH_0007,
            GRX009_PATCH_0008,
            GRX009_PATCH_0009,
            GRX009_PATCH_0010,
        ],
        GRX010_PATCH_0011,
        "0011",
    )


def grx010_patch_0011_applyable(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx010_patch_0011_applyability_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def _grx010_evidence_artifact_issue(
    pass_dir: pathlib.Path, evidence: dict[str, object]
) -> str | None:
    """Recompute the on-disk canonical tonemap artifact digests and require
    them to match the offline compile evidence byte for byte (anti-fabrication:
    a hand-edited evidence JSON without matching artifacts never passes)."""
    artifacts = evidence.get("artifacts")
    if not isinstance(artifacts, dict):
        return "grx010_offline_evidence_artifacts_missing"
    expected_names = {
        "dxil": pass_dir / "artifacts" / "tonemap.dxil",
        "root_signature": pass_dir / "artifacts" / "tonemap.rts0.bin",
        "descriptor_layout": pass_dir / "artifacts" / "tonemap_descriptor_layout.json",
    }
    for key, disk_path in expected_names.items():
        entry = artifacts.get(key)
        if not isinstance(entry, dict):
            return f"grx010_offline_evidence_artifact_{key}_missing"
        recorded = normalize_string(entry.get("sha256"))
        actual = sha256_of_file(disk_path)
        if recorded is None or actual is None or recorded != actual:
            return "grx010_offline_artifact_hash_mismatch"
    return None


def grx010_tonemap_contract_issue(
    pass_dir: pathlib.Path | None = None,
) -> str | None:
    """GRX-010 segment A contract gate audit. Returns None when the tonemap
    pass contract slice is coherent, otherwise the first blocking issue.

    Checks (evidence-based, from disk bytes): the contract trio
    (PASS_CONTRACT.md / pass_manifest.json / resource_mapping.md) carries the
    tonemap markers; the manifest stays fail-closed (default disabled,
    implemented=false, runtime_state=fallback_only, real_gpu_pass=false); the
    offline compile evidence records a success under the owner-approved
    hlsl_bridge_workaround provenance with runtime_mappable=true and NOT
    rurix_owned; the canonical artifacts exist on disk and hash to the
    evidence digests; the bridge carries the fail-closed TonemapGate markers;
    and patch 0011 carries the per-pass setting + call-site gate markers.
    """
    base = GRX010_PASS_DIR if pass_dir is None else pass_dir
    contract = base / "PASS_CONTRACT.md"
    manifest_path = base / "pass_manifest.json"
    mapping = base / "resource_mapping.md"
    evidence_path = base / "offline_compile_evidence.json"
    math_parity_path = base / "math_parity_evidence.json"

    if not file_contains_all(
        contract,
        [
            "pass_id = tonemap",
            "RXGD_PASS_TONEMAP",
            "TONEMAPPER_LINEAR",
            "RXGD_CAP_TONEMAP_REAL_PASS",
            "disabled",
        ],
    ):
        return "grx010_pass_contract_markers_missing"
    if not file_contains_all(
        mapping,
        ["src_color", "dst_color", "b0 space0", "28 bytes", "texture2d", "rwtexture2d"],
    ):
        return "grx010_resource_mapping_markers_missing"

    manifest = load_json_file(manifest_path)
    if manifest is None:
        return "grx010_pass_manifest_missing"
    if manifest.get("pass_id") != "tonemap":
        return "grx010_manifest_pass_id_mismatch"
    if manifest.get("default_enable_state") != "disabled":
        return "grx010_manifest_not_default_disabled"
    # Stage-A5 fail-closed relaxation (mirror of GRX-009): implemented=false is
    # always accepted; implemented=true only under the audited tonemap real-pass
    # measured success. A tampered/absent success artifact fails closed back to
    # requiring the pre-close-out shape.
    if not grx010_manifest_implemented_ok(manifest):
        return "grx010_manifest_implemented_must_stay_false"
    if manifest.get("offline_compile_status") != "success":
        return "grx010_manifest_offline_compile_status_mismatch"
    if manifest.get("offline_compile_provenance") != "hlsl_bridge_workaround":
        return "grx010_manifest_provenance_mismatch"
    implementation_status = manifest.get("implementation_status")
    if not isinstance(implementation_status, dict):
        return "grx010_manifest_implementation_status_missing"
    if not grx010_manifest_runtime_state_ok(implementation_status):
        return "grx010_manifest_runtime_state_mismatch"
    if not grx010_manifest_real_gpu_pass_ok(implementation_status):
        return "grx010_manifest_real_gpu_pass_must_stay_false"
    if not grx010_manifest_dispatch_recorded_ok(implementation_status):
        return "grx010_manifest_dispatch_recorded_must_stay_false"
    # The default Godot runtime tonemap path stays disabled even after the
    # measured opt-in success (only the opt-in arm ran a real dispatch).
    if implementation_status.get("godot_runtime_tonemap_path_enabled") is not False:
        return "grx010_manifest_runtime_path_must_stay_disabled"

    evidence = load_json_file(evidence_path)
    if evidence is None:
        return "grx010_offline_evidence_missing"
    if evidence.get("status") != "success":
        return "grx010_offline_evidence_status_not_success"
    if evidence.get("provenance") != "hlsl_bridge_workaround":
        return "grx010_offline_evidence_provenance_mismatch"
    if evidence.get("rurix_owned") is not False:
        return "grx010_offline_evidence_rurix_owned_must_stay_false"
    if evidence.get("runtime_mappable") is not True:
        return "grx010_offline_evidence_not_runtime_mappable"
    validation = evidence.get("dxil_provenance")
    validation = validation.get("validation") if isinstance(validation, dict) else None
    if not isinstance(validation, dict) or validation.get("status") != "pass":
        return "grx010_offline_evidence_dxv_validation_not_pass"
    artifact_issue = _grx010_evidence_artifact_issue(base, evidence)
    if artifact_issue is not None:
        return artifact_issue

    math_parity = load_json_file(math_parity_path)
    if math_parity is None:
        return "grx010_math_parity_evidence_missing"
    if (
        normalize_string(math_parity.get("math_status"))
        != "linear_srgb_cpu_reference_proven_pending_gpu_dispatch"
    ):
        return "grx010_math_parity_status_mismatch"

    if not file_contains_all(
        GRX009_BRIDGE_LIB,
        [
            "TonemapGate",
            "RXGD_CAP_TONEMAP_REAL_PASS",
            "TonemapDispatchPackage",
            "RXGD_TONEMAP_REAL_PASS_BLOCKED",
        ],
    ):
        return "grx010_bridge_tonemap_gate_markers_missing"
    if not file_contains_all(
        GRX010_PATCH_0011,
        [
            "rendering/rurix_accel/passes/tonemap/enabled",
            "try_record_tonemap",
            "RXGD_PASS_TONEMAP",
            "tone_mapper->tonemapper",
            "RXGD_STATUS_FALLBACK",
        ],
    ):
        return "grx010_patch_0011_markers_missing"
    return None


def grx010_tonemap_d3d12_dispatch_smoke_status(
    pass_dir: pathlib.Path | None = None,
) -> str | None:
    base = GRX010_PASS_DIR if pass_dir is None else pass_dir
    doc = load_json_file(base / "real_d3d12_dispatch_smoke.json")
    if doc is None:
        return None
    return normalize_string(doc.get("status"))


def grx010_tonemap_d3d12_dispatch_smoke_issue(
    pass_dir: pathlib.Path | None = None,
) -> str | None:
    """GRX-010 standalone real D3D12 dispatch smoke gate audit. Returns None
    when the measured smoke evidence is a strict success whose artifact
    digests still match the on-disk artifacts and the offline compile
    evidence, otherwise the first blocking issue. SKIP never advances the
    gate."""
    base = GRX010_PASS_DIR if pass_dir is None else pass_dir
    evidence_path = base / "real_d3d12_dispatch_smoke.json"
    doc = load_json_file(evidence_path)
    if doc is None:
        return "grx010_dispatch_smoke_evidence_missing"
    status = normalize_string(doc.get("status"))
    if status != "success":
        return f"grx010_dispatch_smoke_status_{status or 'missing'}"
    if doc.get("subject") != "grx010_tonemap_real_d3d12_dispatch_smoke":
        return "grx010_dispatch_smoke_subject_mismatch"
    if doc.get("runtime_state") != "fallback_only":
        return "grx010_dispatch_smoke_runtime_state_mismatch"
    if doc.get("real_gpu_pass") is not False:
        return "grx010_dispatch_smoke_real_gpu_pass_must_stay_false"
    if doc.get("artifact_hashes_match_offline_evidence") is not True:
        return "grx010_dispatch_smoke_offline_hash_flag_false"
    artifacts = doc.get("artifacts")
    if not isinstance(artifacts, dict):
        return "grx010_dispatch_smoke_artifacts_missing"
    offline = load_json_file(base / "offline_compile_evidence.json")
    offline_artifacts = offline.get("artifacts") if isinstance(offline, dict) else None
    if not isinstance(offline_artifacts, dict):
        return "grx010_dispatch_smoke_offline_evidence_missing"
    disk_paths = {
        "dxil": base / "artifacts" / "tonemap.dxil",
        "root_signature": base / "artifacts" / "tonemap.rts0.bin",
        "descriptor_layout": base / "artifacts" / "tonemap_descriptor_layout.json",
    }
    for key, disk_path in disk_paths.items():
        entry = artifacts.get(key)
        recorded = (
            normalize_string(entry.get("sha256")) if isinstance(entry, dict) else None
        )
        actual = sha256_of_file(disk_path)
        offline_entry = offline_artifacts.get(key)
        offline_sha = (
            normalize_string(offline_entry.get("sha256"))
            if isinstance(offline_entry, dict)
            else None
        )
        if recorded is None or actual is None or recorded != actual:
            return "grx010_dispatch_smoke_artifact_hash_mismatch"
        if offline_sha is None or offline_sha != actual:
            return "grx010_dispatch_smoke_offline_hash_mismatch"
    checks = doc.get("checks")
    if not isinstance(checks, dict):
        return "grx010_dispatch_smoke_checks_missing"
    for check_name in (
        "artifact_hashes_match_offline_evidence",
        "descriptor_layout_matches_resource_mapping",
        "root_signature_create_from_rurix_rts0",
        "compute_pso_from_rurix_dxil",
        "srv_uav_root_constants_bound_from_layout",
        "dispatch_executed",
        "fence_completed",
        "dst_uav_readback",
        "dst_matches_cpu_reference",
    ):
        if checks.get(check_name) is not True:
            return f"grx010_dispatch_smoke_check_failed_{check_name}"
    return None


def grx009_compile_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_COMPILE_EVIDENCE)


def grx009_texture_dxc_feasibility_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_TEXTURE_DXC_FEASIBILITY_EVIDENCE)


def grx009_dxc_texture_descriptor_rts0_crosscheck_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_DXC_TEXTURE_DESCRIPTOR_RTS0_CROSSCHECK_EVIDENCE)


def grx009_texture_artifact_provenance_policy_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_EVIDENCE)


def grx009_texture_dxc_feasibility_status(
    evidence: dict[str, object] | None = None,
) -> str:
    candidate = evidence if evidence is not None else grx009_texture_dxc_feasibility_evidence()
    if not isinstance(candidate, dict):
        return "missing"
    return normalize_string(candidate.get("status")) or "malformed"


def grx009_texture_dxc_evidence_path(path_text: str | None) -> pathlib.Path | None:
    if not path_text:
        return None
    candidate = pathlib.Path(path_text)
    if candidate.is_absolute():
        return candidate
    return ROOT / candidate


def grx009_texture_dxc_feasibility_issue(
    evidence: dict[str, object] | None = None,
) -> str | None:
    candidate = evidence if evidence is not None else grx009_texture_dxc_feasibility_evidence()
    if not isinstance(candidate, dict):
        return "missing"
    status = normalize_string(candidate.get("status"))
    if status not in {"success", "fail", "skip"}:
        return "malformed"
    issue = normalize_string(candidate.get("issue"))
    if candidate.get("ready") is not True or status != "success":
        return issue or status
    dxil = candidate.get("dxil_container")
    if not isinstance(dxil, dict):
        return "dxil_container_missing"
    dxil_path = grx009_texture_dxc_evidence_path(normalize_string(dxil.get("path")))
    if dxil_path is None or not dxil_path.is_file():
        return "dxil_container_missing"
    recorded_sha = normalize_string(dxil.get("sha256"))
    actual_sha = sha256_of_file(dxil_path)
    if not recorded_sha or actual_sha != recorded_sha:
        return "dxil_hash_mismatch"
    if dxil.get("artifact_kind") != "dxil_container":
        return "dxil_artifact_kind_mismatch"
    if dxil.get("produced_by_current_run") is not True:
        return "dxil_not_produced_by_current_run"
    descriptor = candidate.get("descriptor_binding_expectation")
    if not isinstance(descriptor, dict):
        return "descriptor_binding_expectation_missing"
    resources = descriptor.get("resources")
    if not isinstance(resources, list):
        return "descriptor_binding_expectation_missing"
    binding_kinds = {
        normalize_string(resource.get("binding_kind"))
        for resource in resources
        if isinstance(resource, dict)
    }
    if not {"texture2d", "rwtexture2d"}.issubset(binding_kinds):
        return "descriptor_binding_kind_mismatch"
    comparison = candidate.get("rurix_artifact_contract_comparison")
    if not isinstance(comparison, dict):
        return "contract_comparison_missing"
    if comparison.get("satisfies_current_bridge_descriptor_layout_contract") is not False:
        return "contract_comparison_overclaims_bridge_ready"
    missing_work = comparison.get("missing_work")
    if not isinstance(missing_work, list) or not {
        "root_signature_extraction",
        "descriptor_layout_synthesis",
        "binding_kind_mapping",
        "DXIL_validation_integration",
        "Rurix_source_provenance",
    }.issubset(set(str(item) for item in missing_work)):
        return "contract_comparison_missing_work_incomplete"
    validation = candidate.get("validation")
    if not isinstance(validation, dict) or validation.get("status") != "pass":
        return "dxv_validation_not_passed"
    return None


def grx009_texture_dxc_feasibility_ready(
    evidence: dict[str, object] | None = None,
) -> bool:
    candidate = evidence if evidence is not None else grx009_texture_dxc_feasibility_evidence()
    if not isinstance(candidate, dict):
        return False
    if candidate.get("ready") is not True:
        return False
    if normalize_string(candidate.get("status")) != "success":
        return False
    return grx009_texture_dxc_feasibility_issue(candidate) is None


def grx009_dxc_texture_artifact_bridge_design_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DESIGN)


def grx009_canonical_descriptor_binding_kinds() -> list[str] | None:
    descriptor = load_json_file(GRX009_DESCRIPTOR_LAYOUT)
    if not isinstance(descriptor, dict):
        return None
    resources = descriptor.get("resources")
    if not isinstance(resources, list):
        return None
    binding_kinds: list[str] = []
    for resource in resources:
        if not isinstance(resource, dict):
            return None
        binding_kind = normalize_string(resource.get("binding_kind"))
        if not binding_kind:
            return None
        binding_kinds.append(binding_kind)
    return binding_kinds


# GRX-009 stage A3: SHA-256 digests of the raw-buffer-era canonical package
# (the bytes that sat at the canonical artifacts/ paths before the
# owner-approved hlsl_bridge_workaround canonical texture switch). The
# historical measured smokes (segments 4c/4d/4f/4g and the latest 4h run)
# recorded THESE digests; they stay valid measured evidence of that package
# after the canonical switch, so the measured-evidence gates accept this
# pinned digest trio — but ONLY while the switch itself is owner-approved
# and recorded (see grx009_canonical_texture_switch_active).
GRX009_RAW_BUFFER_ERA_CANONICAL_DIGESTS: dict[str, str] = {
    "dxil": "c77a54ded13417a8d27400abdad38a95454cb4de269cf2d4c9633011967abb9b",
    "root_signature": "f08794f9886e1ebc4c905e3006732e572ec913a75255c5b488cf4877a1391f03",
    "descriptor_layout": "673da613157ada7a264019d4b2071feb7995b8a47206e0bae807ac885f07269b",
}


def grx009_canonical_texture_switch_active(
    compile_evidence: dict[str, object] | None = None,
    policy_evidence: dict[str, object] | None = None,
) -> bool:
    """GRX-009 stage A3: the owner-approved hlsl_bridge_workaround canonical
    texture switch is active.

    True only when BOTH sides of the explicit, owner-approved advance are
    recorded: the tracked texture artifact provenance policy evidence
    (``texture_artifact_provenance_policy.json``) records
    ``policy_ready=true`` with the exact owner decision, AND the canonical
    offline compile evidence records ``status=success`` with
    ``provenance=hlsl_bridge_workaround``. A hand-edited canonical evidence
    without the owner policy (or vice versa) never activates the switch, so
    the historical fail-closed gates keep rejecting silent advances.
    """
    policy = (
        policy_evidence
        if policy_evidence is not None
        else grx009_texture_artifact_provenance_policy_evidence()
    )
    if not isinstance(policy, dict):
        return False
    if policy.get("policy_ready") is not True:
        return False
    if normalize_string(policy.get("status")) != "success":
        return False
    owner_decision = policy.get("owner_decision")
    if not isinstance(owner_decision, dict):
        return False
    if normalize_string(owner_decision.get("decision")) != (
        "approve_hlsl_bridge_workaround_as_temporary_runtime_mappable_canonical"
    ):
        return False
    evidence = (
        compile_evidence if compile_evidence is not None else grx009_compile_evidence()
    )
    if not isinstance(evidence, dict):
        return False
    if normalize_string(evidence.get("status")) != "success":
        return False
    if normalize_string(evidence.get("provenance")) != "hlsl_bridge_workaround":
        return False
    return True


def grx009_canonical_descriptor_binding_kinds_issue(
    compile_evidence: dict[str, object] | None = None,
) -> str | None:
    """Canonical descriptor binding-kind gate shared by the segment 4k
    design/scaffold/RTS0/cross-check/policy gates.

    The design-slice semantics were "no silent canonical advance": the
    canonical descriptor had to stay ``raw_buffer_view`` while only
    design/scaffold evidence existed. Stage A3 is the explicit,
    owner-approved advance, so the canonical descriptor may now be either
    the historical ``["raw_buffer_view", "raw_buffer_view"]`` shape or the
    approved ``["texture2d", "rwtexture2d"]`` shape — the latter ONLY when
    the owner-approved canonical texture switch is active. Anything else is
    still rejected.
    """
    binding_kinds = grx009_canonical_descriptor_binding_kinds()
    if binding_kinds == ["raw_buffer_view", "raw_buffer_view"]:
        return None
    if binding_kinds == ["texture2d", "rwtexture2d"] and (
        grx009_canonical_texture_switch_active(compile_evidence)
    ):
        return None
    return "canonical_descriptor_binding_kind_must_remain_raw_buffer_view"


def grx009_offline_compile_canonical_state_issue(
    compile_doc: dict[str, object] | None,
) -> str | None:
    """Offline-compile-state gate shared by the segment 4k gates.

    During the design slices the canonical offline compile evidence had to
    stay ``compile_failed`` / ``runtime_mappable=false`` (no silent
    advance). Under the owner-approved canonical texture switch the
    canonical evidence records ``status=success`` with
    ``runtime_mappable=true`` — accepted ONLY when the switch is active.
    """
    if not isinstance(compile_doc, dict):
        return "offline_compile_status_must_remain_compile_failed"
    status = normalize_string(compile_doc.get("status"))
    if status == "compile_failed":
        if compile_doc.get("runtime_mappable") is not False:
            return "offline_compile_runtime_mappable_must_remain_false"
        return None
    if grx009_canonical_texture_switch_active(compile_doc):
        return None
    return "offline_compile_status_must_remain_compile_failed"


def grx009_measured_evidence_digests_ok(
    recorded: dict[str, str | None],
    current: dict[str, str | None],
) -> bool:
    """A measured-evidence artifact digest trio is accepted when it matches
    the current canonical offline package, or — only while the
    owner-approved hlsl_bridge_workaround canonical texture switch is
    active — the pinned raw-buffer-era canonical package those historical
    smokes actually measured (the canonical artifact bytes were replaced by
    the approved switch; the measured runs remain valid historical evidence
    of the package they ran against)."""
    if recorded == current:
        return True
    if recorded == GRX009_RAW_BUFFER_ERA_CANONICAL_DIGESTS and (
        grx009_canonical_texture_switch_active()
    ):
        return True
    return False


def grx009_evidence_artifact_digests(
    evidence_artifacts: object,
) -> dict[str, str | None] | None:
    """Extract the {dxil, root_signature, descriptor_layout} SHA-256 trio a
    measured-evidence artifacts block records, or None when malformed."""
    if not isinstance(evidence_artifacts, dict):
        return None
    recorded: dict[str, str | None] = {}
    for key in ("dxil", "root_signature", "descriptor_layout"):
        entry = evidence_artifacts.get(key)
        if not isinstance(entry, dict):
            return None
        recorded[key] = normalize_string(entry.get("sha256"))
    return recorded


def grx009_dxc_texture_artifact_bridge_design_issue(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
) -> str | None:
    candidate = evidence if evidence is not None else grx009_dxc_texture_artifact_bridge_design_evidence()
    if not isinstance(candidate, dict):
        return "design_evidence_missing"
    if not grx009_texture_dxc_feasibility_ready(texture_feasibility_evidence):
        return "texture_dxc_feasibility_not_ready"
    if not GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC.is_file():
        return "design_document_missing"
    required_doc_needles = [
        "## Root Signature Strategy",
        "## Descriptor Layout Synthesis",
        "## Binding Kind Mapping",
        "## DXIL Validation Metadata",
        "## Rurix Provenance",
        "## Canonical Switch Conditions",
        "## Fail Closed Conditions",
    ]
    if not file_contains_all(GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC, required_doc_needles):
        return "design_document_required_sections_missing"
    if normalize_string(candidate.get("status")) != "design_ready":
        return "design_evidence_status_not_ready"
    if candidate.get("design_ready") is not True:
        return "design_evidence_design_ready_not_true"
    if candidate.get("runtime_mappable") is not False:
        return "design_evidence_runtime_mappable_must_be_false"
    if candidate.get("real_gpu_pass") is not False:
        return "design_evidence_real_gpu_pass_must_be_false"
    if candidate.get("canonical_artifact_replaced") is not False:
        return "design_evidence_canonical_artifact_replaced_must_be_false"
    if candidate.get("offline_compile_status_changed") is not False:
        return "design_evidence_offline_compile_status_changed_must_be_false"
    if normalize_string(candidate.get("contract_document")) != (
        "spike/godot-rurix/passes/luminance_reduction/dxc_texture_artifact_bridge.md"
    ):
        return "design_evidence_contract_document_mismatch"
    if normalize_string(candidate.get("source_feasibility_evidence")) != (
        "spike/godot-rurix/passes/luminance_reduction/texture_dxc_feasibility_evidence.json"
    ):
        return "design_evidence_source_feasibility_mismatch"
    required_sections = candidate.get("required_contract_sections")
    if not isinstance(required_sections, list) or not {
        "root_signature_strategy",
        "descriptor_layout_synthesis",
        "binding_kind_mapping",
        "dxil_validation_metadata",
        "rurix_provenance",
        "canonical_switch_conditions",
        "fail_closed_conditions",
    }.issubset(set(str(section) for section in required_sections)):
        return "design_evidence_required_sections_incomplete"
    if normalize_string(candidate.get("next_action_if_design_ready")) != (
        "implement_grx009_dxc_texture_artifact_bridge_scaffold"
    ):
        return "design_evidence_next_action_mismatch"
    blocked_implications = candidate.get("does_not_imply")
    if not isinstance(blocked_implications, list) or not {
        "offline_compile_success",
        "runtime_mappable=true",
        "real_gpu_pass=true",
        "canonical artifact replacement",
        "visual success",
        "performance claim",
    }.issubset(set(str(item) for item in blocked_implications)):
        return "design_evidence_does_not_imply_incomplete"
    compile_doc = compile_evidence if compile_evidence is not None else grx009_compile_evidence()
    if not isinstance(compile_doc, dict):
        return "offline_compile_evidence_missing"
    compile_state_issue = grx009_offline_compile_canonical_state_issue(compile_doc)
    if compile_state_issue is not None:
        return compile_state_issue
    manifest_doc = manifest if manifest is not None else grx009_manifest()
    if not isinstance(manifest_doc, dict):
        return "manifest_missing"
    if not grx009_manifest_implemented_ok(manifest_doc):
        return "manifest_implemented_must_remain_false"
    if manifest_doc.get("default_enable_state") != "disabled":
        return "manifest_default_enable_state_must_remain_disabled"
    implementation_status = manifest_doc.get("implementation_status")
    if not isinstance(implementation_status, dict):
        return "manifest_implementation_status_missing"
    if not grx009_manifest_runtime_state_ok(implementation_status):
        return "manifest_runtime_state_must_remain_fallback_only"
    if not grx009_manifest_real_gpu_pass_ok(implementation_status):
        return "manifest_real_gpu_pass_must_remain_false"
    design_status = implementation_status.get("segment_4k_dxc_texture_artifact_bridge_design")
    if not isinstance(design_status, dict):
        return "manifest_design_status_missing"
    if design_status.get("status") != "design_ready":
        return "manifest_design_status_not_ready"
    if design_status.get("document") != (
        "spike/godot-rurix/passes/luminance_reduction/dxc_texture_artifact_bridge.md"
    ):
        return "manifest_design_document_mismatch"
    if design_status.get("evidence") != (
        "spike/godot-rurix/passes/luminance_reduction/dxc_texture_artifact_bridge_design.json"
    ):
        return "manifest_design_evidence_mismatch"
    if design_status.get("next_action_when_ready") != (
        "implement_grx009_dxc_texture_artifact_bridge_scaffold"
    ):
        return "manifest_design_next_action_mismatch"
    if grx009_real_pass_success_evidence_conflict():
        return "real_pass_enablement_success_evidence_must_not_exist"
    binding_kinds_issue = grx009_canonical_descriptor_binding_kinds_issue(compile_doc)
    if binding_kinds_issue is not None:
        return binding_kinds_issue
    return None


def grx009_dxc_texture_artifact_bridge_design_ready(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
) -> bool:
    return (
        grx009_dxc_texture_artifact_bridge_design_issue(
            evidence,
            manifest,
            compile_evidence,
            texture_feasibility_evidence,
        )
        is None
    )


def grx009_dxc_texture_artifact_bridge_scaffold_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_SCAFFOLD_EVIDENCE)


def grx009_repo_path(path_text: str | None) -> pathlib.Path | None:
    if not path_text:
        return None
    candidate = pathlib.Path(path_text)
    if candidate.is_absolute():
        return candidate
    return ROOT / candidate


def grx009_scaffold_artifact_entry_ok(entry: object) -> bool:
    if not isinstance(entry, dict):
        return False
    path = grx009_repo_path(normalize_string(entry.get("path")))
    recorded_sha = normalize_string(entry.get("sha256"))
    if path is None or not path.is_file() or not recorded_sha:
        return False
    if sha256_of_file(path) != recorded_sha:
        return False
    size_bytes = entry.get("size_bytes")
    if isinstance(size_bytes, int) and size_bytes != path.stat().st_size:
        return False
    return True


def grx009_scaffold_descriptor_resources_ok(resources: object) -> bool:
    if not isinstance(resources, list) or len(resources) != 2:
        return False
    expected = [
        ("src_luminance", "t0 space0", "SRV", 0, 0, "texture2d"),
        ("dst_luminance", "u0 space0", "UAV", 0, 0, "rwtexture2d"),
    ]
    for resource, values in zip(resources, expected):
        if not isinstance(resource, dict):
            return False
        name, binding, resource_class, register, space, binding_kind = values
        if resource.get("name") != name:
            return False
        if resource.get("binding") != binding:
            return False
        if resource.get("class") != resource_class:
            return False
        if resource.get("register") != register or resource.get("space") != space:
            return False
        if resource.get("binding_kind") != binding_kind:
            return False
    return True


def grx009_scaffold_descriptor_file_ok(entry: dict[str, object]) -> bool:
    if not grx009_scaffold_artifact_entry_ok(entry):
        return False
    path = grx009_repo_path(normalize_string(entry.get("path")))
    descriptor = load_json_file(path) if path is not None else None
    if not isinstance(descriptor, dict):
        return False
    if descriptor.get("root_constants") != "none":
        return False
    if descriptor.get("canonical_artifact_eligible") is not False:
        return False
    return grx009_scaffold_descriptor_resources_ok(descriptor.get("resources"))


def grx009_dxc_texture_artifact_bridge_scaffold_status(
    evidence: dict[str, object] | None = None,
) -> str:
    candidate = evidence if evidence is not None else grx009_dxc_texture_artifact_bridge_scaffold_evidence()
    if not isinstance(candidate, dict):
        return "missing"
    return normalize_string(candidate.get("status")) or "malformed"


def grx009_dxc_texture_artifact_bridge_scaffold_issue(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
    design_evidence: dict[str, object] | None = None,
) -> str | None:
    candidate = evidence if evidence is not None else grx009_dxc_texture_artifact_bridge_scaffold_evidence()
    if not isinstance(candidate, dict):
        return "scaffold_evidence_missing"
    manifest_doc = manifest if manifest is not None else grx009_manifest()
    compile_doc = compile_evidence if compile_evidence is not None else grx009_compile_evidence()
    feasibility_doc = texture_feasibility_evidence if texture_feasibility_evidence is not None else grx009_texture_dxc_feasibility_evidence()
    design_doc = design_evidence if design_evidence is not None else grx009_dxc_texture_artifact_bridge_design_evidence()
    design_issue = grx009_dxc_texture_artifact_bridge_design_issue(
        design_doc,
        manifest_doc,
        compile_doc,
        feasibility_doc,
    )
    if design_issue is not None:
        return f"design_gate_not_ready:{design_issue}"
    if normalize_string(candidate.get("status")) != "success":
        return "scaffold_evidence_status_not_success"
    if candidate.get("scaffold_ready") is not True:
        return "scaffold_ready_must_be_true"
    if candidate.get("runtime_mappable") is not False:
        return "scaffold_runtime_mappable_must_be_false"
    if candidate.get("real_gpu_pass") is not False:
        return "scaffold_real_gpu_pass_must_be_false"
    if candidate.get("canonical_artifact_replaced") is not False:
        return "scaffold_canonical_artifact_replaced_must_be_false"
    if candidate.get("offline_compile_status_changed") is not False:
        return "scaffold_offline_compile_status_changed_must_be_false"
    if normalize_string(candidate.get("provenance")) != "hlsl_bridge_workaround":
        return "scaffold_provenance_must_be_hlsl_bridge_workaround"
    if candidate.get("rurix_owned") is not False:
        return "scaffold_hlsl_workaround_rurix_owned_must_be_false"
    if candidate.get("design_or_scaffold_only") is not True:
        return "scaffold_design_or_scaffold_only_must_be_true"
    if candidate.get("canonical_artifact_eligible") is not False:
        return "scaffold_canonical_artifact_eligible_must_be_false"
    if normalize_string(candidate.get("next_action_if_ready")) not in {
        "prepare_grx009_texture_artifact_rurix_provenance_or_rts0_integration",
        "prepare_grx009_texture_artifact_descriptor_rts0_crosscheck_or_provenance_policy",
        "define_grx009_texture_artifact_provenance_policy",
    }:
        return "scaffold_next_action_mismatch"
    dxil_metadata = candidate.get("dxil_container_metadata")
    if not isinstance(dxil_metadata, dict):
        return "scaffold_dxil_metadata_missing"
    for tool_name in ("dxc", "dxv"):
        tool = dxil_metadata.get(tool_name)
        if not isinstance(tool, dict):
            return f"scaffold_{tool_name}_metadata_missing"
        if not normalize_string(tool.get("path")):
            return f"scaffold_{tool_name}_path_missing"
        if not normalize_string(tool.get("version_output")):
            return f"scaffold_{tool_name}_version_missing"
    for command_name in ("compile", "validation"):
        command = dxil_metadata.get(command_name)
        if not isinstance(command, dict):
            return f"scaffold_{command_name}_metadata_missing"
        if not isinstance(command.get("argv"), list) or not command.get("argv"):
            return f"scaffold_{command_name}_argv_missing"
        if command.get("exit_code") != 0:
            return f"scaffold_{command_name}_exit_must_be_zero"
        if not normalize_string(command.get("stdout_path")):
            return f"scaffold_{command_name}_stdout_missing"
        if not normalize_string(command.get("stderr_path")):
            return f"scaffold_{command_name}_stderr_missing"
    validation = dxil_metadata.get("validation")
    if not isinstance(validation, dict) or validation.get("status") != "pass":
        return "scaffold_dxv_validation_metadata_missing"
    if dxil_metadata.get("target_profile") != "cs_6_0":
        return "scaffold_target_profile_mismatch"
    if dxil_metadata.get("entry_point") != "main":
        return "scaffold_entry_point_mismatch"
    container = dxil_metadata.get("container")
    if not grx009_scaffold_artifact_entry_ok(container):
        return "scaffold_dxil_container_artifact_mismatch"
    if isinstance(container, dict) and container.get("artifact_kind") != "dxil_container":
        return "scaffold_dxil_container_kind_mismatch"
    descriptor = candidate.get("descriptor_layout_artifact")
    if not isinstance(descriptor, dict):
        return "scaffold_descriptor_layout_artifact_missing"
    if descriptor.get("root_constants") != "none":
        return "scaffold_descriptor_root_constants_must_be_none"
    if descriptor.get("canonical_artifact_eligible") is not False:
        return "scaffold_descriptor_canonical_eligible_must_be_false"
    if not grx009_scaffold_descriptor_resources_ok(descriptor.get("resources")):
        return "scaffold_descriptor_binding_kind_missing_or_mismatch"
    if not grx009_scaffold_descriptor_file_ok(descriptor):
        return "scaffold_descriptor_layout_artifact_mismatch"
    root_signature = candidate.get("root_signature_scaffold")
    if not isinstance(root_signature, dict):
        return "scaffold_root_signature_missing"
    if not grx009_scaffold_artifact_entry_ok(root_signature):
        return "scaffold_root_signature_artifact_mismatch"
    if root_signature.get("rurix_owned_rts0_generated") is True:
        if root_signature.get("root_signature_status") != "rurix_synthesized":
            return "scaffold_root_signature_status_must_be_rurix_synthesized"
        rts0 = root_signature.get("rts0_artifact")
        if not isinstance(rts0, dict):
            return "scaffold_rts0_artifact_missing"
        rts0_path_text = normalize_string(rts0.get("path"))
        if not rts0_path_text or "artifacts/dxc_texture_bridge/" not in rts0_path_text:
            return "scaffold_rts0_artifact_must_be_independent"
        if not grx009_scaffold_artifact_entry_ok(rts0):
            return "scaffold_rts0_artifact_mismatch"
    else:
        if root_signature.get("root_signature_status") != "scaffold_only":
            return "scaffold_root_signature_status_must_be_scaffold_only"
        if not normalize_string(root_signature.get("unavailable_reason")):
            return "scaffold_root_signature_unavailable_reason_missing"
    if root_signature.get("canonical_artifact_eligible") is not False:
        return "scaffold_root_signature_canonical_eligible_must_be_false"
    mapping = candidate.get("binding_kind_mapping")
    if not isinstance(mapping, dict):
        return "scaffold_binding_kind_mapping_missing"
    texture_mapping = mapping.get("RXGD_RESOURCE_TEXTURE")
    if not isinstance(texture_mapping, dict):
        return "scaffold_texture_mapping_missing"
    if texture_mapping.get("src_luminance") != "texture2d":
        return "scaffold_texture_mapping_src_mismatch"
    if texture_mapping.get("dst_luminance") != "rwtexture2d":
        return "scaffold_texture_mapping_dst_mismatch"
    if texture_mapping.get("rule") != "by descriptor slot":
        return "scaffold_texture_mapping_rule_mismatch"
    if mapping.get("RXGD_RESOURCE_BUFFER") != "raw_buffer_view":
        return "scaffold_buffer_mapping_mismatch"
    if mapping.get("canonical_descriptor_binding_kind") != "raw_buffer_view":
        return "scaffold_canonical_descriptor_mapping_mismatch"
    if mapping.get("canonical_descriptor_replaced") is not False:
        return "scaffold_canonical_descriptor_replaced_must_be_false"
    compile_state_issue = grx009_offline_compile_canonical_state_issue(
        compile_doc if isinstance(compile_doc, dict) else None
    )
    if compile_state_issue is not None:
        return compile_state_issue
    if grx009_real_pass_success_evidence_conflict():
        return "real_pass_enablement_success_evidence_must_not_exist"
    binding_kinds_issue = grx009_canonical_descriptor_binding_kinds_issue(
        compile_doc if isinstance(compile_doc, dict) else None
    )
    if binding_kinds_issue is not None:
        return binding_kinds_issue
    return None


def grx009_dxc_texture_artifact_bridge_scaffold_ready(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
    design_evidence: dict[str, object] | None = None,
) -> bool:
    return (
        grx009_dxc_texture_artifact_bridge_scaffold_issue(
            evidence,
            manifest,
            compile_evidence,
            texture_feasibility_evidence,
            design_evidence,
        )
        is None
    )


def grx009_dxc_texture_rts0_integration_issue(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
    design_evidence: dict[str, object] | None = None,
) -> str | None:
    candidate = evidence if evidence is not None else grx009_dxc_texture_artifact_bridge_scaffold_evidence()
    manifest_doc = manifest if manifest is not None else grx009_manifest()
    compile_doc = compile_evidence if compile_evidence is not None else grx009_compile_evidence()
    feasibility_doc = texture_feasibility_evidence if texture_feasibility_evidence is not None else grx009_texture_dxc_feasibility_evidence()
    design_doc = design_evidence if design_evidence is not None else grx009_dxc_texture_artifact_bridge_design_evidence()
    scaffold_issue = grx009_dxc_texture_artifact_bridge_scaffold_issue(
        candidate,
        manifest_doc,
        compile_doc,
        feasibility_doc,
        design_doc,
    )
    if scaffold_issue is not None:
        return f"scaffold_gate_not_ready:{scaffold_issue}"
    if not isinstance(candidate, dict):
        return "rts0_scaffold_evidence_missing"
    if candidate.get("runtime_mappable") is not False:
        return "rts0_runtime_mappable_must_be_false"
    if candidate.get("real_gpu_pass") is not False:
        return "rts0_real_gpu_pass_must_be_false"
    if candidate.get("canonical_artifact_replaced") is not False:
        return "rts0_canonical_artifact_replaced_must_be_false"
    if candidate.get("offline_compile_status_changed") is not False:
        return "rts0_offline_compile_status_changed_must_be_false"
    if normalize_string(candidate.get("provenance")) != "hlsl_bridge_workaround":
        return "rts0_provenance_must_be_hlsl_bridge_workaround"
    if candidate.get("rurix_owned") is not False:
        return "rts0_hlsl_workaround_rurix_owned_must_be_false"
    if candidate.get("rts0_integration_ready") is not True:
        return "rts0_integration_ready_must_be_true"
    if normalize_string(candidate.get("rts0_integration_status")) != "success":
        return "rts0_integration_status_must_be_success"
    root_signature = candidate.get("root_signature_scaffold")
    if not isinstance(root_signature, dict):
        return "rts0_root_signature_metadata_missing"
    if root_signature.get("root_signature_status") != "rurix_synthesized":
        return "rts0_root_signature_status_must_be_rurix_synthesized"
    if root_signature.get("rts0_integration_ready") is not True:
        return "rts0_root_signature_ready_must_be_true"
    if normalize_string(root_signature.get("rts0_integration_status")) != "success":
        return "rts0_root_signature_status_field_must_be_success"
    if root_signature.get("rurix_owned_rts0_generated") is not True:
        return "rts0_root_signature_generation_flag_must_be_true"
    descriptor_entry = root_signature.get("descriptor_layout_artifact")
    if not isinstance(descriptor_entry, dict):
        return "rts0_descriptor_artifact_missing"
    if not grx009_scaffold_artifact_entry_ok(descriptor_entry):
        return "rts0_descriptor_artifact_hash_mismatch"
    descriptor_path = grx009_repo_path(normalize_string(descriptor_entry.get("path")))
    if descriptor_path != GRX009_DXC_TEXTURE_BRIDGE_DESCRIPTOR_LAYOUT:
        return "rts0_descriptor_path_mismatch"
    descriptor_doc = load_json_file(descriptor_path) if descriptor_path is not None else None
    if not isinstance(descriptor_doc, dict):
        return "rts0_descriptor_file_missing"
    if descriptor_doc.get("root_constants") != "none":
        return "rts0_descriptor_root_constants_must_be_none"
    if descriptor_doc.get("canonical_artifact_eligible") is not False:
        return "rts0_descriptor_canonical_eligible_must_be_false"
    if not grx009_scaffold_descriptor_resources_ok(descriptor_doc.get("resources")):
        return "rts0_descriptor_texture_binding_kind_missing_or_mismatch"
    rts0 = root_signature.get("rts0_artifact")
    if not isinstance(rts0, dict):
        return "rts0_artifact_missing"
    rts0_path = grx009_repo_path(normalize_string(rts0.get("path")))
    if rts0_path != GRX009_DXC_TEXTURE_BRIDGE_RTS0_ARTIFACT:
        return "rts0_artifact_path_mismatch"
    if not grx009_scaffold_artifact_entry_ok(rts0):
        return "rts0_artifact_hash_mismatch"
    if root_signature.get("canonical_artifact_eligible") is not False:
        return "rts0_root_signature_canonical_eligible_must_be_false"
    compile_state_issue = grx009_offline_compile_canonical_state_issue(
        compile_doc if isinstance(compile_doc, dict) else None
    )
    if compile_state_issue is not None:
        return compile_state_issue
    if grx009_real_pass_success_evidence_conflict():
        return "real_pass_enablement_success_evidence_must_not_exist"
    binding_kinds_issue = grx009_canonical_descriptor_binding_kinds_issue(
        compile_doc if isinstance(compile_doc, dict) else None
    )
    if binding_kinds_issue is not None:
        return binding_kinds_issue
    if isinstance(manifest_doc, dict):
        implementation_status = grx009_manifest_implementation_status(manifest_doc)
        if not isinstance(implementation_status, dict):
            return "manifest_implementation_status_missing"
        if not grx009_manifest_runtime_state_ok(implementation_status):
            return "manifest_runtime_state_must_remain_fallback_only"
        if not grx009_manifest_real_gpu_pass_ok(implementation_status):
            return "manifest_real_gpu_pass_must_remain_false"
        integration = implementation_status.get("segment_4k_dxc_texture_artifact_rts0_integration")
        if not isinstance(integration, dict):
            return "manifest_rts0_integration_status_missing"
        if integration.get("rts0_integration_ready") is not True:
            return "manifest_rts0_integration_ready_must_be_true"
        if integration.get("runtime_mappable") is not False:
            return "manifest_rts0_runtime_mappable_must_be_false"
        if integration.get("real_gpu_pass") is not False:
            return "manifest_rts0_real_gpu_pass_must_be_false"
        if integration.get("canonical_artifact_replaced") is not False:
            return "manifest_rts0_canonical_artifact_replaced_must_be_false"
        if normalize_string(integration.get("provenance")) != "hlsl_bridge_workaround":
            return "manifest_rts0_provenance_must_be_hlsl_bridge_workaround"
        if integration.get("rurix_owned") is not False:
            return "manifest_rts0_rurix_owned_must_be_false"
        if normalize_string(integration.get("next_action_when_ready")) != (
            "prepare_grx009_texture_artifact_descriptor_rts0_crosscheck_or_provenance_policy"
        ):
            return "manifest_rts0_next_action_mismatch"
    return None


def grx009_dxc_texture_rts0_integration_ready(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
    design_evidence: dict[str, object] | None = None,
) -> bool:
    return (
        grx009_dxc_texture_rts0_integration_issue(
            evidence,
            manifest,
            compile_evidence,
            texture_feasibility_evidence,
            design_evidence,
        )
        is None
    )


def grx009_crosscheck_descriptor_resources_ok(resources: object) -> bool:
    if not isinstance(resources, list):
        return False
    by_name = {resource.get("name"): resource for resource in resources if isinstance(resource, dict)}
    expected = {
        "src_luminance": {"class": "SRV", "register": 0, "space": 0, "count": 1, "binding_kind": "texture2d"},
        "dst_luminance": {"class": "UAV", "register": 0, "space": 0, "count": 1, "binding_kind": "rwtexture2d"},
    }
    if set(by_name) != set(expected):
        return False
    for name, fields in expected.items():
        resource = by_name.get(name)
        if not isinstance(resource, dict):
            return False
        for key, value in fields.items():
            if resource.get(key) != value:
                return False
    return True


def grx009_dxc_texture_descriptor_rts0_crosscheck_status(
    evidence: dict[str, object] | None = None,
) -> str:
    candidate = evidence if evidence is not None else grx009_dxc_texture_descriptor_rts0_crosscheck_evidence()
    if not isinstance(candidate, dict):
        return "missing"
    return normalize_string(candidate.get("cross_check_status")) or "malformed"


def grx009_dxc_texture_descriptor_rts0_crosscheck_issue(
    crosscheck_evidence: dict[str, object] | None = None,
    scaffold_evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
    design_evidence: dict[str, object] | None = None,
) -> str | None:
    candidate = crosscheck_evidence if crosscheck_evidence is not None else grx009_dxc_texture_descriptor_rts0_crosscheck_evidence()
    scaffold_doc = scaffold_evidence if scaffold_evidence is not None else grx009_dxc_texture_artifact_bridge_scaffold_evidence()
    manifest_doc = manifest if manifest is not None else grx009_manifest()
    compile_doc = compile_evidence if compile_evidence is not None else grx009_compile_evidence()
    feasibility_doc = texture_feasibility_evidence if texture_feasibility_evidence is not None else grx009_texture_dxc_feasibility_evidence()
    design_doc = design_evidence if design_evidence is not None else grx009_dxc_texture_artifact_bridge_design_evidence()
    rts0_issue = grx009_dxc_texture_rts0_integration_issue(
        scaffold_doc,
        manifest_doc,
        compile_doc,
        feasibility_doc,
        design_doc,
    )
    if rts0_issue is not None:
        return f"rts0_gate_not_ready:{rts0_issue}"
    if not isinstance(candidate, dict):
        return "descriptor_rts0_crosscheck_evidence_missing"
    if normalize_string(candidate.get("cross_check_status")) != "success":
        return "descriptor_rts0_crosscheck_status_must_be_success"
    if candidate.get("descriptor_rts0_crosscheck_ready") is not True:
        return "descriptor_rts0_crosscheck_ready_must_be_true"
    if candidate.get("runtime_mappable") is not False:
        return "descriptor_rts0_crosscheck_runtime_mappable_must_be_false"
    if candidate.get("real_gpu_pass") is not False:
        return "descriptor_rts0_crosscheck_real_gpu_pass_must_be_false"
    if candidate.get("canonical_artifact_replaced") is not False:
        return "descriptor_rts0_crosscheck_canonical_artifact_replaced_must_be_false"
    if candidate.get("offline_compile_status_changed") is not False:
        return "descriptor_rts0_crosscheck_offline_compile_status_changed_must_be_false"
    if normalize_string(candidate.get("provenance")) != "hlsl_bridge_workaround":
        return "descriptor_rts0_crosscheck_provenance_must_be_hlsl_bridge_workaround"
    if candidate.get("rurix_owned") is not False:
        return "descriptor_rts0_crosscheck_hlsl_workaround_rurix_owned_must_be_false"
    if candidate.get("byte_for_byte_match") is not True:
        return "descriptor_rts0_crosscheck_byte_for_byte_match_must_be_true"
    if candidate.get("root_constants") != "none":
        return "descriptor_rts0_crosscheck_root_constants_must_be_none"
    descriptor = candidate.get("descriptor_layout_artifact")
    if not isinstance(descriptor, dict):
        return "descriptor_rts0_crosscheck_descriptor_artifact_missing"
    descriptor_path = grx009_repo_path(normalize_string(descriptor.get("path")))
    if descriptor_path != GRX009_DXC_TEXTURE_BRIDGE_DESCRIPTOR_LAYOUT:
        return "descriptor_rts0_crosscheck_descriptor_path_mismatch"
    descriptor_sha = normalize_string(descriptor.get("sha256"))
    if not descriptor_sha or sha256_of_file(GRX009_DXC_TEXTURE_BRIDGE_DESCRIPTOR_LAYOUT) != descriptor_sha:
        return "descriptor_rts0_crosscheck_descriptor_hash_mismatch"
    descriptor_doc = load_json_file(GRX009_DXC_TEXTURE_BRIDGE_DESCRIPTOR_LAYOUT)
    if not isinstance(descriptor_doc, dict):
        return "descriptor_rts0_crosscheck_descriptor_file_missing"
    if descriptor_doc.get("root_constants") != "none":
        return "descriptor_rts0_crosscheck_descriptor_root_constants_must_be_none"
    if not grx009_crosscheck_descriptor_resources_ok(descriptor_doc.get("resources")):
        return "descriptor_rts0_crosscheck_descriptor_resources_mismatch"
    rts0 = candidate.get("rts0_artifact")
    if not isinstance(rts0, dict):
        return "descriptor_rts0_crosscheck_rts0_artifact_missing"
    rts0_path = grx009_repo_path(normalize_string(rts0.get("path")))
    if rts0_path != GRX009_DXC_TEXTURE_BRIDGE_RTS0_ARTIFACT:
        return "descriptor_rts0_crosscheck_rts0_path_mismatch"
    rts0_sha = normalize_string(rts0.get("sha256"))
    if not rts0_sha or sha256_of_file(GRX009_DXC_TEXTURE_BRIDGE_RTS0_ARTIFACT) != rts0_sha:
        return "descriptor_rts0_crosscheck_rts0_hash_mismatch"
    reserialized = candidate.get("reserialized_rts0_artifact")
    if not isinstance(reserialized, dict):
        return "descriptor_rts0_crosscheck_reserialized_rts0_missing"
    reserialized_sha = normalize_string(reserialized.get("sha256"))
    if not reserialized_sha or reserialized_sha != rts0_sha:
        return "descriptor_rts0_crosscheck_reserialized_rts0_hash_mismatch"
    root_signature = load_json_file(GRX009_DXC_TEXTURE_BRIDGE_ROOT_SIGNATURE_METADATA)
    if not isinstance(root_signature, dict):
        return "descriptor_rts0_crosscheck_root_signature_metadata_missing"
    if normalize_string(root_signature.get("cross_check_status")) != "success":
        return "descriptor_rts0_crosscheck_root_signature_status_mismatch"
    if root_signature.get("descriptor_rts0_crosscheck_ready") is not True:
        return "descriptor_rts0_crosscheck_root_signature_ready_mismatch"
    if normalize_string(root_signature.get("descriptor_sha256")) not in {"", descriptor_sha}:
        return "descriptor_rts0_crosscheck_root_signature_descriptor_hash_mismatch"
    if normalize_string(root_signature.get("rts0_sha256")) not in {"", rts0_sha}:
        return "descriptor_rts0_crosscheck_root_signature_rts0_hash_mismatch"
    if normalize_string(root_signature.get("reserialized_rts0_sha256")) not in {"", reserialized_sha}:
        return "descriptor_rts0_crosscheck_root_signature_reserialized_hash_mismatch"
    if root_signature.get("byte_for_byte_match") is not True:
        return "descriptor_rts0_crosscheck_root_signature_byte_match_mismatch"
    if isinstance(scaffold_doc, dict):
        root_signature_scaffold = scaffold_doc.get("root_signature_scaffold")
        if not isinstance(root_signature_scaffold, dict):
            return "descriptor_rts0_crosscheck_scaffold_root_signature_missing"
        if normalize_string(root_signature_scaffold.get("cross_check_status")) != "success":
            return "descriptor_rts0_crosscheck_scaffold_status_mismatch"
        if root_signature_scaffold.get("descriptor_rts0_crosscheck_ready") is not True:
            return "descriptor_rts0_crosscheck_scaffold_ready_mismatch"
        if normalize_string(root_signature_scaffold.get("descriptor_sha256")) not in {"", descriptor_sha}:
            return "descriptor_rts0_crosscheck_scaffold_descriptor_hash_mismatch"
        if normalize_string(root_signature_scaffold.get("rts0_sha256")) not in {"", rts0_sha}:
            return "descriptor_rts0_crosscheck_scaffold_rts0_hash_mismatch"
        if normalize_string(root_signature_scaffold.get("reserialized_rts0_sha256")) not in {"", reserialized_sha}:
            return "descriptor_rts0_crosscheck_scaffold_reserialized_hash_mismatch"
    compile_state_issue = grx009_offline_compile_canonical_state_issue(
        compile_doc if isinstance(compile_doc, dict) else None
    )
    if compile_state_issue is not None:
        return compile_state_issue
    if grx009_real_pass_success_evidence_conflict():
        return "real_pass_enablement_success_evidence_must_not_exist"
    binding_kinds_issue = grx009_canonical_descriptor_binding_kinds_issue(
        compile_doc if isinstance(compile_doc, dict) else None
    )
    if binding_kinds_issue is not None:
        return binding_kinds_issue
    if isinstance(manifest_doc, dict):
        implementation_status = grx009_manifest_implementation_status(manifest_doc)
        if not isinstance(implementation_status, dict):
            return "manifest_implementation_status_missing"
        if not grx009_manifest_runtime_state_ok(implementation_status):
            return "manifest_runtime_state_must_remain_fallback_only"
        if not grx009_manifest_real_gpu_pass_ok(implementation_status):
            return "manifest_real_gpu_pass_must_remain_false"
        crosscheck = implementation_status.get("segment_4k_dxc_texture_descriptor_rts0_crosscheck")
        if not isinstance(crosscheck, dict):
            return "manifest_descriptor_rts0_crosscheck_status_missing"
        if crosscheck.get("descriptor_rts0_crosscheck_ready") is not True:
            return "manifest_descriptor_rts0_crosscheck_ready_must_be_true"
        if crosscheck.get("runtime_mappable") is not False:
            return "manifest_descriptor_rts0_runtime_mappable_must_be_false"
        if crosscheck.get("real_gpu_pass") is not False:
            return "manifest_descriptor_rts0_real_gpu_pass_must_be_false"
        if crosscheck.get("canonical_artifact_replaced") is not False:
            return "manifest_descriptor_rts0_canonical_artifact_replaced_must_be_false"
        if normalize_string(crosscheck.get("provenance")) != "hlsl_bridge_workaround":
            return "manifest_descriptor_rts0_provenance_must_be_hlsl_bridge_workaround"
        if crosscheck.get("rurix_owned") is not False:
            return "manifest_descriptor_rts0_rurix_owned_must_be_false"
        if normalize_string(crosscheck.get("next_action_when_ready")) != "define_grx009_texture_artifact_provenance_policy":
            return "manifest_descriptor_rts0_next_action_mismatch"
    return None


def grx009_dxc_texture_descriptor_rts0_crosscheck_ready(
    crosscheck_evidence: dict[str, object] | None = None,
    scaffold_evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
    design_evidence: dict[str, object] | None = None,
) -> bool:
    return (
        grx009_dxc_texture_descriptor_rts0_crosscheck_issue(
            crosscheck_evidence,
            scaffold_evidence,
            manifest,
            compile_evidence,
            texture_feasibility_evidence,
            design_evidence,
        )
        is None
    )


def grx009_texture_artifact_provenance_policy_status(
    evidence: dict[str, object] | None = None,
) -> str:
    candidate = evidence if evidence is not None else grx009_texture_artifact_provenance_policy_evidence()
    if not isinstance(candidate, dict):
        return "missing"
    return normalize_string(candidate.get("status")) or "malformed"


def grx009_texture_artifact_provenance_policy_issue(
    policy_evidence: dict[str, object] | None = None,
    crosscheck_evidence: dict[str, object] | None = None,
    scaffold_evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
    design_evidence: dict[str, object] | None = None,
) -> str | None:
    manifest_doc = manifest if manifest is not None else grx009_manifest()
    compile_doc = compile_evidence if compile_evidence is not None else grx009_compile_evidence()
    crosscheck_issue = grx009_dxc_texture_descriptor_rts0_crosscheck_issue(
        crosscheck_evidence,
        scaffold_evidence,
        manifest_doc,
        compile_doc,
        texture_feasibility_evidence,
        design_evidence,
    )
    if crosscheck_issue is not None:
        return f"crosscheck_gate_not_ready:{crosscheck_issue}"
    candidate = policy_evidence if policy_evidence is not None else grx009_texture_artifact_provenance_policy_evidence()
    if not isinstance(candidate, dict):
        return "texture_artifact_provenance_policy_evidence_missing"
    if normalize_string(candidate.get("status")) != "success":
        return "provenance_policy_status_must_be_success"
    if candidate.get("policy_ready") is not True:
        return "provenance_policy_ready_must_be_true"
    if normalize_string(candidate.get("segment")) != "4l_texture_artifact_provenance_policy":
        return "provenance_policy_segment_mismatch"
    if candidate.get("runtime_mappable") is not False:
        return "provenance_policy_runtime_mappable_must_be_false"
    if candidate.get("real_gpu_pass") is not False:
        return "provenance_policy_real_gpu_pass_must_be_false"
    if candidate.get("canonical_artifact_replaced") is not False:
        return "provenance_policy_canonical_artifact_replaced_must_be_false"
    if candidate.get("offline_compile_status_changed") is not False:
        return "provenance_policy_offline_compile_status_changed_must_be_false"
    owner_decision = candidate.get("owner_decision")
    if not isinstance(owner_decision, dict):
        return "provenance_policy_owner_decision_missing"
    if normalize_string(owner_decision.get("decision")) != (
        "approve_hlsl_bridge_workaround_as_temporary_runtime_mappable_canonical"
    ):
        return "provenance_policy_owner_decision_mismatch"
    if not normalize_string(owner_decision.get("approved_by")):
        return "provenance_policy_owner_approved_by_missing"
    provenance_policy = candidate.get("provenance_policy")
    if not isinstance(provenance_policy, dict):
        return "provenance_policy_block_missing"
    if normalize_string(provenance_policy.get("provenance")) != "hlsl_bridge_workaround":
        return "provenance_policy_provenance_must_be_hlsl_bridge_workaround"
    if provenance_policy.get("rurix_owned") is not False:
        return "provenance_policy_rurix_owned_must_be_false"
    if provenance_policy.get("rurix_owned_rts0") is not True:
        return "provenance_policy_rurix_owned_rts0_must_be_true"
    if normalize_string(provenance_policy.get("canonical_switch_exception")) != (
        "owner_approved_hlsl_bridge_workaround"
    ):
        return "provenance_policy_canonical_switch_exception_mismatch"
    revert_conditions = provenance_policy.get("revert_to_rurix_owned_when")
    if not isinstance(revert_conditions, list) or not revert_conditions:
        return "provenance_policy_revert_conditions_missing"
    policy_doc_path = grx009_repo_path(normalize_string(candidate.get("policy_document")))
    if policy_doc_path != GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC:
        return "provenance_policy_document_path_mismatch"
    if not GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC.is_file():
        return "provenance_policy_document_missing"
    required_doc_needles = [
        "## Owner Decision",
        "## Exception to Canonical Switch Conditions",
        "## Revert / Re-cut Conditions",
        "## Fail-Closed Invariants",
    ]
    if not file_contains_all(GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC, required_doc_needles):
        return "provenance_policy_document_required_sections_missing"
    bridge_doc_path = grx009_repo_path(normalize_string(candidate.get("bridge_contract_document")))
    if bridge_doc_path != GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC:
        return "provenance_policy_bridge_contract_document_mismatch"
    if not file_contains_all(
        GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC,
        ["texture_artifact_provenance_policy.md"],
    ):
        return "bridge_contract_document_missing_owner_exception_reference"
    if normalize_string(candidate.get("next_action_if_ready")) != (
        "provide_grx009_runtime_mappable_luminance_kernel_artifact"
    ):
        return "provenance_policy_next_action_mismatch"
    compile_state_issue = grx009_offline_compile_canonical_state_issue(
        compile_doc if isinstance(compile_doc, dict) else None
    )
    if compile_state_issue is not None:
        return compile_state_issue
    if grx009_real_pass_success_evidence_conflict():
        return "real_pass_enablement_success_evidence_must_not_exist"
    binding_kinds_issue = grx009_canonical_descriptor_binding_kinds_issue(
        compile_doc if isinstance(compile_doc, dict) else None
    )
    if binding_kinds_issue is not None:
        return binding_kinds_issue
    if isinstance(manifest_doc, dict):
        implementation_status = grx009_manifest_implementation_status(manifest_doc)
        if not isinstance(implementation_status, dict):
            return "manifest_implementation_status_missing"
        if not grx009_manifest_runtime_state_ok(implementation_status):
            return "manifest_runtime_state_must_remain_fallback_only"
        if not grx009_manifest_real_gpu_pass_ok(implementation_status):
            return "manifest_real_gpu_pass_must_remain_false"
        policy = implementation_status.get("segment_4l_texture_artifact_provenance_policy")
        if not isinstance(policy, dict):
            return "manifest_provenance_policy_status_missing"
        if normalize_string(policy.get("status")) != "success":
            return "manifest_provenance_policy_status_must_be_success"
        if policy.get("policy_ready") is not True:
            return "manifest_provenance_policy_ready_must_be_true"
        if policy.get("runtime_mappable") is not False:
            return "manifest_provenance_policy_runtime_mappable_must_be_false"
        if policy.get("real_gpu_pass") is not False:
            return "manifest_provenance_policy_real_gpu_pass_must_be_false"
        if policy.get("canonical_artifact_replaced") is not False:
            return "manifest_provenance_policy_canonical_artifact_replaced_must_be_false"
        if normalize_string(policy.get("provenance")) != "hlsl_bridge_workaround":
            return "manifest_provenance_policy_provenance_must_be_hlsl_bridge_workaround"
        if policy.get("rurix_owned") is not False:
            return "manifest_provenance_policy_rurix_owned_must_be_false"
        if normalize_string(policy.get("next_action_when_ready")) != (
            "provide_grx009_runtime_mappable_luminance_kernel_artifact"
        ):
            return "manifest_provenance_policy_next_action_mismatch"
    return None


def grx009_texture_artifact_provenance_policy_ready(
    policy_evidence: dict[str, object] | None = None,
    crosscheck_evidence: dict[str, object] | None = None,
    scaffold_evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
    compile_evidence: dict[str, object] | None = None,
    texture_feasibility_evidence: dict[str, object] | None = None,
    design_evidence: dict[str, object] | None = None,
) -> bool:
    return (
        grx009_texture_artifact_provenance_policy_issue(
            policy_evidence,
            crosscheck_evidence,
            scaffold_evidence,
            manifest,
            compile_evidence,
            texture_feasibility_evidence,
            design_evidence,
        )
        is None
    )


def grx009_raw_buffer_compile_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_RAW_BUFFER_COMPILE_EVIDENCE)


def grx009_segment4i_fail_closed_active(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> bool:
    """The segment 4i texture-capable compile attempt is failing closed.

    True only when the manifest records the segment 4h/4i fail-closed marker
    AND the canonical evidence's own status/blocker are exactly the expected
    fail-closed shape (a hand-edited canonical evidence claiming some other
    status must NOT be treated as fail-closed). This does not by itself prove
    the canonical artifacts are trustworthy; callers must still re-verify
    artifact hashes against the raw-buffer historical evidence.
    """
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    if implementation_status.get("segment_4h_fail_closed_texture_intrinsic_unsupported") is not True:
        return False
    if normalize_string(evidence.get("status")) != "compile_failed":
        return False
    if normalize_string(evidence.get("blocker_category")) != "dxil_container_missing":
        return False
    return True


def grx009_segment4i_raw_buffer_backing_ok(evidence: dict[str, object]) -> bool:
    """The fail-closed canonical evidence's artifacts really are the raw-buffer
    historical bytes: cross-checks the canonical artifact SHA-256 values (and
    the files on disk) against the tracked raw-buffer historical evidence, so
    a tampered or stale canonical evidence cannot be waved through just
    because it claims the fail-closed shape."""
    raw_evidence = grx009_raw_buffer_compile_evidence()
    if raw_evidence is None:
        return False
    if normalize_string(raw_evidence.get("status")) != "success":
        return False
    if raw_evidence.get("manifest_segment_after_run") != 3:
        return False
    canonical_artifacts_root = evidence.get("artifacts")
    raw_artifacts = raw_evidence.get("artifacts")
    if not isinstance(canonical_artifacts_root, dict) or not isinstance(raw_artifacts, dict):
        return False
    # Segment 4i restructured canonical artifacts: the raw-buffer fallback
    # bytes live under artifacts.bridge_tracked_fallback.{dxil,...}; the
    # raw_buffer_historical evidence still has them at top level.
    canonical_artifacts = canonical_artifacts_root.get("bridge_tracked_fallback")
    if not isinstance(canonical_artifacts, dict):
        return False
    for key in ("dxil", "root_signature", "descriptor_layout"):
        canonical_entry = canonical_artifacts.get(key)
        raw_entry = raw_artifacts.get(key)
        if not isinstance(canonical_entry, dict) or not isinstance(raw_entry, dict):
            return False
        canonical_path = normalize_string(canonical_entry.get("path"))
        raw_sha = normalize_string(raw_entry.get("sha256"))
        canonical_sha = normalize_string(canonical_entry.get("sha256"))
        if not canonical_path or not raw_sha or not canonical_sha:
            return False
        if canonical_sha != raw_sha:
            return False
        candidate = ROOT / canonical_path
        actual_sha = sha256_of_file(candidate)
        if actual_sha is None or actual_sha != raw_sha:
            return False
    return True


def sha256_of_file(path: pathlib.Path) -> str | None:
    """真实读取文件内容重算 SHA-256(evidence 中记录值须与之匹配,防造假)。"""
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(65536), b""):
                digest.update(chunk)
    except OSError:
        return None
    return digest.hexdigest()


def read_text_prefix(path: pathlib.Path, limit: int = 65536) -> str:
    if not path.is_file():
        return ""
    try:
        return path.read_bytes()[:limit].decode("utf-8", errors="ignore")
    except OSError:
        return ""


def file_evidence(path: pathlib.Path) -> dict[str, object]:
    return {
        "path": str(path),
        "exists": path.exists(),
        "is_file": path.is_file(),
        "sha256": sha256_of_file(path),
    }


def probe_validator_executable(
    path: pathlib.Path,
    args: list[str],
    identity_markers: tuple[str, ...],
    identity_failure_reason: str,
) -> dict[str, object]:
    command = [str(path), *args]
    result: dict[str, object] = {
        "probe_command": command,
        "probe_exit_code": None,
        "probe_output": None,
        "probe_passed": False,
        "probe_timed_out": False,
        "probe_timeout_seconds": PROBE_TIMEOUT_SECONDS,
        "probe_identity_passed": False,
        "probe_identity_reason": None,
    }
    try:
        proc = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
            timeout=PROBE_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as exc:
        result["probe_timed_out"] = True
        result["probe_output"] = timeout_output(exc)
        return result
    except OSError as exc:
        result["probe_output"] = f"{type(exc).__name__}: {exc}"
        return result
    result["probe_exit_code"] = proc.returncode
    result["probe_output"] = completed_output(proc)
    output = str(result["probe_output"] or "").lower()
    identity_passed = any(marker in output for marker in identity_markers)
    result["probe_identity_passed"] = identity_passed
    if proc.returncode == 0 and not identity_passed:
        result["probe_identity_reason"] = identity_failure_reason
    result["probe_passed"] = proc.returncode == 0 and identity_passed
    return result


def probe_rurix_llc(env: dict[str, str] | None = None) -> dict[str, object]:
    environ = os.environ if env is None else env
    raw_path = normalize_string(environ.get(RURIX_LLC_ENV))
    result: dict[str, object] = {
        "env_key": RURIX_LLC_ENV,
        "path": raw_path,
        "status": "MISSING",
        "missing_reason": None,
        "file": None,
        "version_output": None,
        "version_exit_code": None,
        "version_timed_out": False,
        "version_timeout_seconds": PROBE_TIMEOUT_SECONDS,
    }
    if raw_path is None:
        result["missing_reason"] = "RURIX_LLC_not_set"
        return result
    path = pathlib.Path(raw_path).expanduser()
    result["path"] = str(path)
    result["file"] = file_evidence(path)
    if not path.exists():
        result["missing_reason"] = "RURIX_LLC_path_missing"
        return result
    if not path.is_file():
        result["missing_reason"] = "RURIX_LLC_not_file"
        return result
    try:
        proc = subprocess.run(
            [str(path), "--version"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
            timeout=PROBE_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as exc:
        result["status"] = "INVALID"
        result["missing_reason"] = "RURIX_LLC_version_timeout"
        result["version_timed_out"] = True
        result["version_output"] = timeout_output(exc)
        return result
    except OSError as exc:
        result["missing_reason"] = "RURIX_LLC_not_executable"
        result["error"] = f"{type(exc).__name__}: {exc}"
        return result
    result["version_exit_code"] = proc.returncode
    result["version_output"] = completed_output(proc)
    if proc.returncode != 0:
        result["status"] = "INVALID"
        result["missing_reason"] = "RURIX_LLC_version_failed"
        return result
    result["status"] = "PASS"
    return result


def probe_signed_dxc_validator_suite(
    env: dict[str, str] | None = None,
) -> dict[str, object]:
    environ = os.environ if env is None else env
    candidates: list[dict[str, object]] = []
    for env_key in RURIX_DXC_DIR_ENV_KEYS:
        raw_path = normalize_string(environ.get(env_key))
        if raw_path is None:
            continue
        path = pathlib.Path(raw_path).expanduser()
        files = {
            name: file_evidence(path / name) for name in DXC_VALIDATOR_SUITE_FILES
        }
        missing_files = [name for name, data in files.items() if data["is_file"] is not True]
        probe_failures: list[str] = []
        identity_failures: list[str] = []
        if "dxc.exe" not in missing_files:
            dxc_probe = probe_validator_executable(
                path / "dxc.exe",
                ["--version"],
                DXC_IDENTITY_MARKERS,
                "dxc_identity_marker_missing",
            )
            files["dxc.exe"].update(dxc_probe)
            if dxc_probe["probe_passed"] is not True:
                probe_failures.append("dxc_probe_failed")
            reason = normalize_string(dxc_probe.get("probe_identity_reason"))
            if reason:
                identity_failures.append(reason)
        if "dxv.exe" not in missing_files:
            dxv_probe = probe_validator_executable(
                path / "dxv.exe",
                ["--help"],
                DXV_IDENTITY_MARKERS,
                "dxv_identity_marker_missing",
            )
            files["dxv.exe"].update(dxv_probe)
            if dxv_probe["probe_passed"] is not True:
                probe_failures.append("dxv_probe_failed")
            reason = normalize_string(dxv_probe.get("probe_identity_reason"))
            if reason:
                identity_failures.append(reason)
        suite_passed = (
            path.is_dir()
            and not missing_files
            and not probe_failures
            and not identity_failures
        )
        candidate: dict[str, object] = {
            "env_key": env_key,
            "path": str(path),
            "exists": path.exists(),
            "is_dir": path.is_dir(),
            "files": files,
            "missing_files": missing_files,
            "status": "PASS" if suite_passed else "MISSING",
            "missing_reasons": [],
        }
        missing_reasons: list[str] = []
        if not path.exists():
            missing_reasons.append("validator_suite_dir_missing")
        elif not path.is_dir():
            missing_reasons.append("validator_suite_not_dir")
        missing_reasons.extend(f"missing_{name}" for name in missing_files)
        missing_reasons.extend(probe_failures)
        missing_reasons.extend(identity_failures)
        candidate["missing_reasons"] = missing_reasons
        candidates.append(candidate)
    if not candidates:
        return {
            "env_keys": list(RURIX_DXC_DIR_ENV_KEYS),
            "status": "MISSING",
            "selected_env_key": None,
            "path": None,
            "missing_files": list(DXC_VALIDATOR_SUITE_FILES),
            "missing_reasons": ["validator_suite_env_not_set"],
            "candidates": [],
        }
    passing = next(
        (candidate for candidate in candidates if candidate["status"] == "PASS"),
        None,
    )
    selected = passing or candidates[0]
    return {
        "env_keys": list(RURIX_DXC_DIR_ENV_KEYS),
        "status": selected["status"],
        "selected_env_key": selected["env_key"],
        "path": selected["path"],
        "missing_files": selected["missing_files"],
        "missing_reasons": selected["missing_reasons"],
        "files": selected["files"],
        "candidates": candidates,
    }


def dxil_toolchain_next_command() -> str:
    return (
        "$env:RURIX_LLC='H:\\path\\to\\patched\\llc.exe'; "
        "$env:RURIX_DXC_DIR='H:\\path\\to\\signed-dxc-suite'; "
        r"py -3 ci\godot_rurix_toolchain_probe.py"
    )


def build_dxil_toolchain_preflight(
    env: dict[str, str] | None = None,
) -> dict[str, object]:
    llc = probe_rurix_llc(env)
    validator_suite = probe_signed_dxc_validator_suite(env)
    missing_reasons: list[str] = []
    llc_reason = normalize_string(llc.get("missing_reason"))
    if llc_reason:
        missing_reasons.append(llc_reason)
    suite_reasons = validator_suite.get("missing_reasons")
    if isinstance(suite_reasons, list):
        missing_reasons.extend(str(reason) for reason in suite_reasons)
    ready = llc.get("status") == "PASS" and validator_suite.get("status") == "PASS"
    if llc.get("status") != "PASS":
        next_action = "provide_or_locate_patched_dxil_llc"
    elif validator_suite.get("status") != "PASS":
        next_action = "provide_signed_dxc_validator_suite"
    else:
        next_action = None
    return {
        "schema_version": 1,
        "generated_by": "ci/godot_rurix_toolchain_probe.py",
        "report_path": str(DXIL_TOOLCHAIN_REPORT),
        "rurix_llc": llc,
        "signed_dxc_validator_suite": validator_suite,
        "ready": ready,
        "missing_reasons": missing_reasons,
        "next_action": next_action,
        "next_command": None if ready else dxil_toolchain_next_command(),
        "notes": [
            "PATH dxc.exe/dxv.exe is not treated as the signed validator suite.",
            "This preflight reports local gaps only and does not install or download toolchains.",
        ],
    }


def grx009_dxil_artifact_is_real_container(path: pathlib.Path) -> bool:
    text = read_text_prefix(path)
    if text.startswith("; ModuleID"):
        return False
    if "target triple = \"dxil-unknown-shadermodel" in text:
        return False
    if "entry:\n  ret void" in text or "entry:\r\n  ret void" in text:
        return False
    return path.is_file()


def grx009_compile_stderr_has_skip_marker(evidence: dict[str, object]) -> bool:
    commands = evidence.get("commands")
    if not isinstance(commands, list):
        return False
    for command in commands:
        if not isinstance(command, dict):
            continue
        stderr_path = normalize_string(command.get("stderr_path"))
        if not stderr_path:
            continue
        stderr_text = read_text_prefix(ROOT / stderr_path).lower()
        if any(
            marker in stderr_text
            for marker in ("patched llc not found", "dxc validator not found", "skipped")
        ):
            return True
    return False


def grx009_compile_manifest_consistency_issue(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> str | None:
    evidence_status = normalize_string(evidence.get("status"))
    evidence_blocker = normalize_string(evidence.get("blocker_category"))
    manifest_status = normalize_string(manifest.get("offline_compile_status"))
    # Red-check (applies to ALL paths, fail-closed or not): a failed compile
    # can never be runtime-mappable. This must run before the fail_closed
    # branch so that any compile_failed + runtime_mappable=true contradiction
    # is rejected regardless of other evidence shape.
    if evidence_status == "compile_failed" and evidence.get("runtime_mappable") is True:
        return (
            "GRX-009 evidence contradiction: status=compile_failed but "
            "runtime_mappable=true; a failed compile cannot be runtime-mappable"
        )
    fail_closed = grx009_segment4i_fail_closed_active(manifest, evidence)
    if fail_closed:
        # Segment 4i: the canonical evidence now honestly reports the newer
        # texture-capable compile attempt (compile_failed / dxil_container_missing)
        # while manifest.offline_compile_status/segment_3a_last_result still
        # record the historical raw-buffer segment 3a success. This divergence
        # is only accepted when the raw-buffer historical evidence backs it up
        # byte-for-byte; a hand-edited canonical evidence cannot fake this path.
        if not grx009_segment4i_raw_buffer_backing_ok(evidence):
            return (
                "GRX-009 segment 4i fail-closed manifest/evidence mismatch: "
                "canonical evidence claims the fail-closed shape but its artifacts "
                "do not match the tracked raw-buffer historical evidence"
            )
    elif manifest_status != evidence_status:
        return (
            "GRX-009 segment 3a manifest/evidence mismatch: "
            f"manifest offline_compile_status={manifest_status or 'missing'} "
            f"but latest evidence status={evidence_status or 'missing'}"
        )
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return "GRX-009 segment 3a manifest/evidence mismatch: implementation_status is missing"
    last_result = normalize_string(implementation_status.get("segment_3a_last_result"))
    if not fail_closed and last_result != evidence_status:
        return (
            "GRX-009 segment 3a manifest/evidence mismatch: "
            f"segment_3a_last_result={last_result or 'missing'} "
            f"but latest evidence status={evidence_status or 'missing'}"
        )
    if not fail_closed and evidence_status in GRX009_SEGMENT3A_BLOCKED_COMPILE_STATUSES:
        blockers = manifest.get("compile_blockers")
        blocker_categories: list[str] = []
        if isinstance(blockers, list):
            for blocker in blockers:
                if isinstance(blocker, dict):
                    category = normalize_string(blocker.get("category"))
                    if category:
                        blocker_categories.append(category)
        if evidence_blocker not in blocker_categories:
            return (
                "GRX-009 segment 3a manifest/evidence mismatch: "
                f"latest evidence blocker={evidence_blocker or 'missing'} "
                f"but manifest compile_blockers={blocker_categories or ['missing']}"
            )
    return None


def grx009_offline_compile_success_evidence_ok(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> bool:
    """Segment 3a success evidence is complete, consistent, and untampered.

    Segment-number pins live in the per-segment gates: segment 3a requires the
    manifest to still sit at segment 3, while later segments (4a+) reuse this
    helper to re-verify the same offline compile evidence after the manifest
    has advanced. The evidence itself always records
    ``manifest_segment_after_run == 3`` because that is the run that produced
    it; segment 4a does not re-run the offline compile.
    """
    if not GRX009_COMPILE_SCHEMA.exists():
        return False
    if evidence.get("pass_id") != "luminance_reduction":
        return False
    fail_closed = grx009_segment4i_fail_closed_active(manifest, evidence)
    if evidence.get("status") != "success" and not fail_closed:
        return False
    if evidence.get("runtime_state") != "fallback_only":
        return False
    if grx009_compile_manifest_consistency_issue(manifest, evidence) is not None:
        return False
    if fail_closed:
        # Segment 4i fail-closed: the canonical evidence honestly records the
        # newer texture-capable compile attempt's failure, but its artifacts
        # are the raw-buffer historical bytes (already cross-checked above via
        # grx009_compile_manifest_consistency_issue -> grx009_segment4i_raw_buffer_backing_ok).
        # manifest_segment_after_run stays 2 for this attempt; segment 3a's
        # historical success is what backs manifest.offline_compile_status.
        if evidence.get("manifest_segment_after_run") != 2:
            return False
    else:
        # success evidence 必须已把 manifest 推进到 segment 3(与 schema allOf 同口径)。
        if evidence.get("manifest_segment_after_run") != 3:
            return False
    if not fail_closed and manifest.get("offline_compile_status") != "success":
        return False
    if fail_closed and manifest.get("offline_compile_status") not in (
        "compile_failed",
        "segment_4i_fail_closed_texture_intrinsic_unsupported",
    ):
        return False
    artifacts_root = evidence.get("artifacts")
    if not isinstance(artifacts_root, dict):
        return False
    # Segment 4i restructured the compile_failed evidence's artifacts: the
    # raw-buffer fallback bytes live under
    # artifacts.bridge_tracked_fallback.{dxil,root_signature,descriptor_layout}
    # and attempted_texture_dxil describes the failed texture compile. Real
    # success evidence (status=success, e.g. the historical raw-buffer segment
    # 3a evidence or a synthetic segment 4b fixture derived from it) keeps the
    # original flat artifacts.{dxil,root_signature,descriptor_layout} layout per
    # the schema's success if/then block; only the fail-closed branch reads
    # from bridge_tracked_fallback.
    if fail_closed:
        artifacts = artifacts_root.get("bridge_tracked_fallback")
        if not isinstance(artifacts, dict):
            return False
    else:
        artifacts = artifacts_root
    # manifest 声明的 artifact 路径集合(evidence 路径须与其对应字段一致,防漂移)。
    manifest_artifacts = manifest.get("offline_compile_artifacts")
    if not isinstance(manifest_artifacts, dict):
        return False
    for key in ("dxil", "root_signature", "descriptor_layout"):
        artifact = artifacts.get(key)
        if not isinstance(artifact, dict):
            return False
        path_text = normalize_string(artifact.get("path"))
        if artifact.get("exists") is not True or not path_text:
            return False
        if artifact.get("produced_by_current_run") is not True and not fail_closed:
            return False
        # evidence 路径 == manifest.offline_compile_artifacts 对应字段(normalize 后)。
        manifest_path_text = normalize_string(manifest_artifacts.get(key))
        if manifest_path_text != path_text:
            return False
        candidate = ROOT / path_text
        if not candidate.is_file():
            return False
        # evidence 记录的 sha256 须为非空字符串,且与真实文件内容重算值匹配。
        recorded_sha = normalize_string(artifact.get("sha256"))
        if not recorded_sha:
            return False
        actual_sha = sha256_of_file(candidate)
        if actual_sha is None or actual_sha != recorded_sha:
            return False
        if key == "dxil":
            if artifact.get("artifact_kind") != "dxil_container":
                return False
            if artifact.get("semantic_status") == "entry_shell_only":
                return False
            if not grx009_dxil_artifact_is_real_container(candidate):
                return False
    if grx009_compile_stderr_has_skip_marker(evidence):
        return False
    return True


def grx009_segment3a_compile_ready() -> bool:
    """GRX-009 segment 3a offline compile evidence is intact.

    Historical/cumulative semantics: segment 3a stays ready as long as the
    manifest sits at segment 3 *or later* (3b/4a/4b/...) with the offline
    compile success evidence still complete, consistent, untampered, and
    ``real_gpu_pass == false``. This gate must not flip to false just because
    a later segment (e.g. 4b gated dispatch bring-up) has advanced the manifest;
    the underlying artifact-hash / DXIL-container checks are unchanged.
    """
    manifest = grx009_manifest()
    evidence = grx009_compile_evidence()
    if manifest is None or evidence is None:
        return False
    if not grx009_segment3a_artifacts_evidence_ready(manifest, evidence):
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    segment = implementation_status.get("segment")
    if not isinstance(segment, int) or segment < 3:
        return False
    if not grx009_manifest_real_gpu_pass_ok(implementation_status):
        return False
    return True


def grx009_segment3a_artifacts_evidence_ready(
    manifest: dict[str, object] | None = None,
    evidence: dict[str, object] | None = None,
) -> bool:
    candidate_manifest = manifest or grx009_manifest()
    candidate_evidence = evidence or grx009_compile_evidence()
    if candidate_manifest is None or candidate_evidence is None:
        return False
    return grx009_offline_compile_success_evidence_ok(
        candidate_manifest,
        candidate_evidence,
    )


def grx009_segment3b_resource_mapping_evidence_ready() -> bool:
    if not file_contains_all(
        GRX009_RESOURCE_MAPPING,
        [
            "resource mapping scaffold",
            "src_luminance = t0",
            "dst_luminance = u0",
            "b0",
            "64-bit integer shader capability",
            "fallback_only",
        ],
    ):
        return False
    descriptor = load_json_file(GRX009_DESCRIPTOR_LAYOUT)
    if descriptor is None:
        return False
    mapping = descriptor.get("segment3b_mapping")
    if not isinstance(mapping, dict):
        return False
    if mapping.get("status") != "resource_mapping_scaffold_only":
        return False
    if mapping.get("requires_64bit_integer_shader_capability") is not True:
        return False
    if mapping.get("runtime_state") != "fallback_only":
        return False
    if mapping.get("real_gpu_pass") is not False:
        return False
    if not file_contains_all(
        GRX009_PATCH_0004,
        [
            "resource mapping scaffold",
            "RXGD_PASS_LUMINANCE_REDUCTION",
            "src_luminance = t0",
            "dst_luminance = u0",
            "64-bit integer shader capability",
            "RXGD_STATUS_FALLBACK",
        ],
    ):
        return False
    if not file_contains_all(
        GRX009_BRIDGE_LIB,
        [
            "record_runtime_binding_preflight",
            "LUMINANCE_ROOT_CONSTANT_BYTES",
            "RXGD_CAP_SHADER_INT64",
            "RXGD_STATUS_FALLBACK",
        ],
    ):
        return False
    return True


def grx009_segment3b_resource_mapping_inputs_ready() -> bool:
    """GRX-009 segment 3b resource mapping scaffold evidence is intact.

    Historical/cumulative semantics: segment 3b stays ready as long as the
    manifest sits at segment 3 *or later* with the ``segment_3b_resource_mapping_scaffold``
    milestone flag still set, the resource mapping / descriptor layout / patch
    evidence still holding, and ``real_gpu_pass == false``. Relying on the
    cumulative milestone flag (instead of an exact ``segment_detail`` pin) keeps
    this predecessor ready after the manifest advances to segment 4a/4b; the
    resource-mapping and 3a artifact checks are unchanged.
    """
    manifest = grx009_manifest()
    if manifest is None:
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    segment = implementation_status.get("segment")
    if not isinstance(segment, int) or segment < 3:
        return False
    if implementation_status.get("segment_3b_resource_mapping_scaffold") is not True:
        return False
    if not grx009_manifest_real_gpu_pass_ok(implementation_status):
        return False
    if manifest.get("resource_mapping") != "spike/godot-rurix/passes/luminance_reduction/resource_mapping.md":
        return False
    if not grx009_segment3a_compile_ready():
        return False
    if not grx009_segment3b_resource_mapping_evidence_ready():
        return False
    return True


def grx009_segment3b_resource_mapping_ready() -> bool:
    if not grx009_segment3b_resource_mapping_inputs_ready():
        return False
    if not grx009_patch_0004_applyable():
        return False
    return True


def grx009_segment4a_runtime_binding_preflight_inputs_ready() -> bool:
    """GRX-009 segment 4a runtime binding preflight evidence is present.

    Preflight/fallback-only semantics: this gate never represents a real GPU
    pass. Historical/cumulative semantics: it stays ready as long as the
    manifest sits at segment 4 *or later* with the
    ``segment_4a_runtime_binding_preflight`` milestone flag still set (instead
    of an exact ``4a`` segment_detail pin), so it does not flip to false once
    the manifest advances to segment 4b gated dispatch bring-up. It requires
    ``runtime_state == fallback_only`` and ``real_gpu_pass == false``, the
    segment 3a offline compile success evidence to still be intact, the
    segment 3b resource mapping file evidence to still hold, the 0005 patch
    and bridge preflight markers to exist, and the shared patch stack to sit
    at 0001+0002+0003 with 0004 forward-applicable.
    """
    manifest = grx009_manifest()
    evidence = grx009_compile_evidence()
    if manifest is None or evidence is None:
        return False
    if not grx009_manifest_implemented_ok(manifest):
        return False
    if manifest.get("default_enable_state") != "disabled":
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    segment = implementation_status.get("segment")
    if not isinstance(segment, int) or segment < 4:
        return False
    if not grx009_manifest_real_gpu_pass_ok(implementation_status):
        return False
    if not grx009_manifest_runtime_state_ok(implementation_status):
        return False
    if implementation_status.get("segment_4a_runtime_binding_preflight") is not True:
        return False
    if implementation_status.get("godot_core_call_site_wired") is not True:
        return False
    if not grx009_segment3a_artifacts_evidence_ready(manifest, evidence):
        return False
    if manifest.get("resource_mapping") != "spike/godot-rurix/passes/luminance_reduction/resource_mapping.md":
        return False
    if not grx009_segment3b_resource_mapping_evidence_ready():
        return False
    if not file_contains_all(
        GRX009_RESOURCE_MAPPING,
        ["runtime binding preflight"],
    ):
        return False
    if not file_contains_all(
        GRX009_PATCH_0005,
        [
            "runtime binding preflight",
            "RXGD_PASS_LUMINANCE_REDUCTION",
            "RXGD_RESOURCE_TEXTURE",
            "try_record_luminance_reduction",
            "source_width",
            "src_luminance = t0",
            "dst_luminance = u0",
            "b0",
            "64-bit integer shader capability",
            "RXGD_STATUS_FALLBACK",
            "no D3D12 dispatch is recorded",
        ],
    ):
        return False
    if not file_contains_all(
        GRX009_BRIDGE_LIB,
        [
            "record_runtime_binding_preflight",
            "LUMINANCE_ROOT_CONSTANT_BYTES",
            "RXGD_CAP_SHADER_INT64",
            "RXGD_STATUS_FALLBACK",
        ],
    ):
        return False
    if not grx009_patch_stack_ready():
        return False
    if not grx009_patch_0004_applyable():
        return False
    return True


def grx009_segment4a_runtime_binding_preflight_ready(
    inputs_ready: bool | None = None,
    patch_0005_result: dict[str, object] | None = None,
) -> bool:
    if inputs_ready is None:
        inputs_ready = grx009_segment4a_runtime_binding_preflight_inputs_ready()
    if not inputs_ready:
        return False
    if not grx009_patch_0005_applyable(patch_0005_result):
        return False
    return True


def grx009_segment4b_gated_dispatch_bringup_inputs_ready() -> bool:
    """GRX-009 segment 4b gated dispatch bring-up evidence is present.

    Gated dispatch bring-up / fallback-only semantics: this gate never
    represents a real GPU pass or a real D3D12 dispatch. It requires the
    manifest to sit at segment 4 with ``runtime_state == fallback_only`` and
    ``real_gpu_pass == false``, the segment 3a offline compile success evidence
    to still be intact, the segment 3b resource mapping and segment 4a runtime
    binding preflight evidence to still hold, the 0006 patch and bridge dispatch
    bring-up markers to exist, and the shared patch stack to sit at
    0001+0002+0003 with 0004 and 0005 forward/stack-applicable.

    Historical/cumulative: this gate relies on the cumulative
    ``segment_4b_gated_dispatch_bringup`` milestone flag rather than pinning an
    exact ``segment_detail`` string, so it stays ready after the manifest's
    ``segment_detail`` advances (e.g. to 4e/4f). This mirrors the segment 4a
    inputs gate, which also keys off ``segment == 4`` plus its milestone flag.
    """
    manifest = grx009_manifest()
    evidence = grx009_compile_evidence()
    if manifest is None or evidence is None:
        return False
    if not grx009_manifest_implemented_ok(manifest):
        return False
    if manifest.get("default_enable_state") != "disabled":
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    if implementation_status.get("segment") != 4:
        return False
    if not grx009_manifest_real_gpu_pass_ok(implementation_status):
        return False
    if not grx009_manifest_runtime_state_ok(implementation_status):
        return False
    if implementation_status.get("segment_4a_runtime_binding_preflight") is not True:
        return False
    if implementation_status.get("segment_4b_gated_dispatch_bringup") is not True:
        return False
    if implementation_status.get("real_d3d12_dispatch_recorded") is not False:
        return False
    if implementation_status.get("godot_core_call_site_wired") is not True:
        return False
    if not grx009_segment3a_artifacts_evidence_ready(manifest, evidence):
        return False
    if manifest.get("resource_mapping") != "spike/godot-rurix/passes/luminance_reduction/resource_mapping.md":
        return False
    if not grx009_segment3b_resource_mapping_evidence_ready():
        return False
    if not file_contains_all(
        GRX009_RESOURCE_MAPPING,
        ["runtime binding preflight"],
    ):
        return False
    if not file_contains_all(
        GRX009_PATCH_0006,
        [
            "gated dispatch bring-up",
            "rendering/rurix_accel/passes/luminance_reduction/dispatch_bringup",
            "RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP",
            "caps.flags |= RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP",
            "RXGD_STATUS_FALLBACK",
            "no D3D12 dispatch is recorded",
        ],
    ):
        return False
    if not file_contains_all(
        GRX009_BRIDGE_LIB,
        [
            "record_gated_dispatch_bringup",
            "check_dispatch_eligibility",
            "RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP",
            "LuminanceDispatchPackage",
            "request_dispatch_bringup",
            "RXGD_STATUS_FALLBACK",
        ],
    ):
        return False
    if not grx009_patch_stack_ready():
        return False
    if not grx009_patch_0004_applyable():
        return False
    if not grx009_patch_0005_applyable():
        return False
    return True


def grx009_segment4b_gated_dispatch_bringup_ready(
    inputs_ready: bool | None = None,
    patch_0006_result: dict[str, object] | None = None,
) -> bool:
    if inputs_ready is None:
        inputs_ready = grx009_segment4b_gated_dispatch_bringup_inputs_ready()
    if not inputs_ready:
        return False
    if not grx009_patch_0006_applyable(patch_0006_result):
        return False
    return True


def grx009_real_d3d12_dispatch_smoke_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_REAL_D3D12_DISPATCH_SMOKE)


def grx009_offline_artifact_digests(
    evidence: dict[str, object] | None,
) -> dict[str, str | None]:
    """Extract the segment 3a offline compile artifact SHA-256 digests.

    Segment 4i restructured the compile_failed canonical artifacts: the
    raw-buffer fallback bytes live under
    artifacts.bridge_tracked_fallback.{dxil,...}. Success-shaped canonical
    evidence (the historical raw-buffer success and the stage A3
    owner-approved hlsl_bridge_workaround texture package) keeps the flat
    artifacts.{dxil,root_signature,descriptor_layout} layout per the schema.
    This helper is only ever called with canonical evidence
    (grx009_compile_evidence()).
    """
    out: dict[str, str | None] = {
        "dxil": None,
        "root_signature": None,
        "descriptor_layout": None,
    }
    artifacts_root = evidence.get("artifacts") if isinstance(evidence, dict) else None
    if isinstance(artifacts_root, dict):
        artifacts = artifacts_root.get("bridge_tracked_fallback")
        if not isinstance(artifacts, dict):
            artifacts = artifacts_root
        for key in out:
            entry = artifacts.get(key)
            if isinstance(entry, dict):
                out[key] = normalize_string(entry.get("sha256"))
    return out


def grx009_offline_evidence_records_texture_binding_kinds(
    evidence: dict[str, object] | None = None,
) -> bool:
    """Audit the canonical offline compile evidence for the segment 4i
    texture-capable kernel artifact round.

    Returns True only when the canonical ``offline_compile_evidence.json``
    records ``attempted_binding_kinds`` containing both ``"texture2d"`` and
    ``"rwtexture2d"`` (the SRV/UAV texture resource kinds emitted by
    ``src/lib_texture.rx``) and a non-empty ``math_parity_status`` field.
    This check is deliberately decoupled from ``runtime_mappable`` because
    the texture-capable compile attempt fails closed (runtime_mappable is
    false while the patched llc lacks texture intrinsic support); the
    attempted_binding_kinds field records the binding kinds targeted by
    the attempt regardless of whether it produced a runtime-mappable
    artifact. A missing or partial evidence file returns False; this
    helper never raises.
    """
    if evidence is None:
        evidence = grx009_compile_evidence()
    if not isinstance(evidence, dict):
        return False
    attempted_binding_kinds = evidence.get("attempted_binding_kinds")
    if not isinstance(attempted_binding_kinds, list):
        return False
    if "texture2d" not in attempted_binding_kinds or "rwtexture2d" not in attempted_binding_kinds:
        return False
    if not normalize_string(evidence.get("math_parity_status")):
        return False
    return True


def grx009_real_d3d12_dispatch_smoke_ready(
    evidence: dict[str, object] | None = None,
    segment4b_ready: bool | None = None,
) -> bool:
    """GRX-009 segment 4c real D3D12 dispatch smoke is measured and matches the
    tracked offline artifacts.

    Ready only when the segment 4b gated dispatch bring-up is already ready and
    the tracked ``real_d3d12_dispatch_smoke.json`` records ``status == success``
    against artifact digests that still match both the on-disk artifacts and the
    segment 3a offline compile evidence. A missing / SKIP / FAIL smoke, a stale
    hash, or a mismatched descriptor layout is not ready. Success here is smoke
    evidence only: it must keep ``runtime_state == fallback_only`` and
    ``real_gpu_pass == false`` and never implies a Godot runtime pass.
    """
    if segment4b_ready is None:
        segment4b_ready = grx009_segment4b_gated_dispatch_bringup_ready()
    if not segment4b_ready:
        return False
    if evidence is None:
        evidence = grx009_real_d3d12_dispatch_smoke_evidence()
    if not isinstance(evidence, dict):
        return False
    if evidence.get("status") != "success":
        return False
    if evidence.get("pass_id") != "luminance_reduction":
        return False
    if evidence.get("segment") != "4c":
        return False
    if evidence.get("runtime_state") != "fallback_only":
        return False
    if evidence.get("real_gpu_pass") is not False:
        return False
    if evidence.get("artifact_hashes_match_offline_evidence") is not True:
        return False

    # Re-verify recorded digests against the current offline evidence and the
    # on-disk artifacts so stale/tampered evidence cannot advance the gate.
    offline_digests = grx009_offline_artifact_digests(grx009_compile_evidence())
    current = {
        "dxil": sha256_of_file(GRX009_DXIL_ARTIFACT),
        "root_signature": sha256_of_file(GRX009_ROOT_SIGNATURE_ARTIFACT),
        "descriptor_layout": sha256_of_file(GRX009_DESCRIPTOR_LAYOUT),
    }
    if any(value is None for value in current.values()):
        return False
    if current != offline_digests:
        return False
    recorded = grx009_evidence_artifact_digests(evidence.get("artifacts"))
    if recorded is None or not grx009_measured_evidence_digests_ok(recorded, current):
        return False

    checks = evidence.get("checks")
    required_checks = (
        "artifact_hashes_match_offline_evidence",
        "descriptor_layout_matches_resource_mapping",
        "root_signature_create_from_rurix_rts0",
        "compute_pso_from_rurix_dxil",
        "srv_uav_root_constants_bound_from_layout",
        "dispatch_executed",
        "fence_completed",
        "dst_uav_readback",
    )
    if not isinstance(checks, dict):
        return False
    if any(checks.get(name) is not True for name in required_checks):
        return False

    dispatch = evidence.get("dispatch")
    if not isinstance(dispatch, dict):
        return False
    if not normalize_string(dispatch.get("fence_completed_value")):
        return False
    if not normalize_string(dispatch.get("dimensions")):
        return False

    device = evidence.get("device")
    if not isinstance(device, dict) or not normalize_string(device.get("adapter")):
        return False
    return True


def grx009_bridge_recording_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_BRIDGE_RECORDING_EVIDENCE)


def grx009_bridge_recording_evidence_dll_sha256(
    evidence: dict[str, object] | None,
) -> str | None:
    """SHA-256 of the feature-built DLL recorded by the segment 4d smoke run.

    This is the *historical* measured artifact fingerprint. It is intentionally
    NOT part of the 4d readiness gate: target/debug/rurix_godot.dll is a mutable
    build artifact, so a later feature-off build can overwrite it while the
    measured evidence stays valid."""
    if not isinstance(evidence, dict):
        return None
    fingerprint = evidence.get("dll_fingerprint")
    if isinstance(fingerprint, dict):
        return normalize_string(fingerprint.get("dll_sha256"))
    return None


def grx009_bridge_real_d3d12_dispatch_recording_ready(
    evidence: dict[str, object] | None = None,
    dispatch_smoke_ready: bool | None = None,
) -> bool:
    """GRX-009 segment 4d bridge real D3D12 dispatch recording smoke is measured
    and matches the tracked offline artifacts.

    Ready only when the segment 4c standalone dispatch smoke is already ready and
    the tracked ``bridge_dispatch_recording_evidence.json`` records
    ``status == success``: the bridge (rurix_godot.dll built with the
    ``d3d12-recording-shim`` feature) recorded ONE real luminance compute dispatch
    on a real D3D12 device/queue via its C ABI. Success here is bridge smoke
    evidence only — it must keep ``runtime_state == fallback_only``,
    ``real_gpu_pass == false``, ``godot_runtime_luminance_path_enabled == false``,
    ``default_enable_state == disabled``, ``gpu_timestamp_status == not_yet``, and
    it never implies a Godot runtime pass. A missing / SKIP / FAIL smoke, a stale
    hash, or a mismatched descriptor layout is not ready.
    """
    if dispatch_smoke_ready is None:
        dispatch_smoke_ready = grx009_real_d3d12_dispatch_smoke_ready()
    if not dispatch_smoke_ready:
        return False
    if evidence is None:
        evidence = grx009_bridge_recording_evidence()
    if not isinstance(evidence, dict):
        return False
    if evidence.get("status") != "success":
        return False
    if evidence.get("pass_id") != "luminance_reduction":
        return False
    if evidence.get("segment") != "4d":
        return False
    if evidence.get("runtime_state") != "fallback_only":
        return False
    if evidence.get("real_gpu_pass") is not False:
        return False
    if evidence.get("bridge_recorded_d3d12_dispatch") is not True:
        return False
    if evidence.get("godot_runtime_luminance_path_enabled") is not False:
        return False
    if evidence.get("default_enable_state") != "disabled":
        return False
    if evidence.get("gpu_timestamp_status") != "not_yet":
        return False
    if evidence.get("artifact_hashes_match_offline_evidence") is not True:
        return False

    # Re-verify recorded digests against the current offline evidence and the
    # on-disk artifacts so stale/tampered evidence cannot advance the gate.
    offline_digests = grx009_offline_artifact_digests(grx009_compile_evidence())
    current = {
        "dxil": sha256_of_file(GRX009_DXIL_ARTIFACT),
        "root_signature": sha256_of_file(GRX009_ROOT_SIGNATURE_ARTIFACT),
        "descriptor_layout": sha256_of_file(GRX009_DESCRIPTOR_LAYOUT),
    }
    if any(value is None for value in current.values()):
        return False
    if current != offline_digests:
        return False
    recorded = grx009_evidence_artifact_digests(evidence.get("artifacts"))
    if recorded is None or not grx009_measured_evidence_digests_ok(recorded, current):
        return False

    checks = evidence.get("checks")
    required_checks = (
        "artifact_hashes_match_offline_evidence",
        "descriptor_layout_matches_resource_mapping",
        "recording_shim_linked",
        "real_d3d12_device_queue_resource_handles",
        "dispatch_bringup_optin_and_record_arm",
        "int64_capability",
        "rxgd_record_pass_returned_ok",
        "bridge_recorded_one_pass",
        "no_fallback_passes",
        "gpu_time_ns_zero",
        "fence_completed",
        "dst_uav_readback",
    )
    if not isinstance(checks, dict):
        return False
    if any(checks.get(name) is not True for name in required_checks):
        return False

    stats = evidence.get("bridge_stats")
    if not isinstance(stats, dict):
        return False
    if normalize_string(stats.get("recorded_passes")) != "1":
        return False
    if normalize_string(stats.get("fallback_passes")) != "0":
        return False
    if normalize_string(stats.get("gpu_time_ns")) != "0":
        return False
    if normalize_string(stats.get("last_error")) != "0":
        return False

    dispatch = evidence.get("dispatch")
    if not isinstance(dispatch, dict):
        return False
    if not normalize_string(dispatch.get("fence_completion")):
        return False
    if not normalize_string(dispatch.get("dimensions")):
        return False

    device = evidence.get("device")
    if not isinstance(device, dict) or not normalize_string(device.get("adapter")):
        return False

    bridge = evidence.get("bridge")
    if not isinstance(bridge, dict) or normalize_string(bridge.get("shim_available")) != "1":
        return False
    return True


def grx009_segment4e_native_resource_handle_mapping_inputs_ready() -> bool:
    """GRX-009 segment 4e native resource handle mapping evidence is present.

    Native handle mapping preflight / fallback-only semantics: this gate never
    represents a real GPU pass, a real D3D12 dispatch, or an enabled Godot
    luminance Rurix path. It requires the manifest to still sit at segment 4
    with ``runtime_state == fallback_only``, ``real_gpu_pass == false``,
    ``real_d3d12_dispatch_recorded == false``, ``default_enable_state ==
    disabled``, and ``implemented == false``, the cumulative
    ``segment_4e_native_resource_handle_mapping`` milestone flag to be set, the
    segment 3a offline compile success evidence and the segment 3b resource
    mapping evidence to still hold, and the 0007 patch markers to prove the
    Godot side now resolves real D3D12 ID3D12Resource* native handles (through
    RenderingDevice::get_driver_resource) with a zero-handle fallback to the
    native Godot luminance path. The Godot module must still never advertise the
    harness-only RXGD_CAP_LUMINANCE_DISPATCH_RECORD flag.
    """
    manifest = grx009_manifest()
    evidence = grx009_compile_evidence()
    if manifest is None or evidence is None:
        return False
    if not grx009_manifest_implemented_ok(manifest):
        return False
    if manifest.get("default_enable_state") != "disabled":
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    if implementation_status.get("segment") != 4:
        return False
    if not grx009_manifest_real_gpu_pass_ok(implementation_status):
        return False
    if not grx009_manifest_runtime_state_ok(implementation_status):
        return False
    if implementation_status.get("real_d3d12_dispatch_recorded") is not False:
        return False
    if implementation_status.get("godot_runtime_luminance_path_enabled") is not False:
        return False
    if implementation_status.get("segment_4a_runtime_binding_preflight") is not True:
        return False
    if implementation_status.get("segment_4b_gated_dispatch_bringup") is not True:
        return False
    if (
        implementation_status.get("segment_4e_native_resource_handle_mapping")
        is not True
    ):
        return False
    if implementation_status.get("godot_core_call_site_wired") is not True:
        return False
    if not grx009_segment3a_artifacts_evidence_ready(manifest, evidence):
        return False
    if manifest.get("resource_mapping") != "spike/godot-rurix/passes/luminance_reduction/resource_mapping.md":
        return False
    if not grx009_segment3b_resource_mapping_evidence_ready():
        return False
    if not file_contains_all(
        GRX009_PATCH_0007,
        [
            "native resource handle mapping",
            "RenderingDevice::get_driver_resource",
            "DRIVER_RESOURCE_TEXTURE",
            "p_source_native_handle",
            "p_dest_native_handle",
            "ID3D12Resource*",
            "rb->get_internal_texture()",
            "luminance_buffers->reduce[0]",
            "native Godot luminance path",
            "RXGD_STATUS_FALLBACK",
            "does not set RXGD_CAP_LUMINANCE_DISPATCH_RECORD",
        ],
    ):
        return False
    if not grx009_patch_stack_ready():
        return False
    if not grx009_patch_0004_applyable():
        return False
    if not grx009_patch_0005_applyable():
        return False
    if not grx009_patch_0006_applyable():
        return False
    return True


def grx009_segment4e_native_resource_handle_mapping_ready(
    inputs_ready: bool | None = None,
    patch_0007_result: dict[str, object] | None = None,
    bridge_recording_ready: bool | None = None,
) -> bool:
    """GRX-009 segment 4e native resource handle mapping is ready.

    Ready only when the segment 4d bridge real D3D12 dispatch recording smoke is
    already ready, the 4e manifest/patch inputs hold, and the 0007 patch stacks
    cleanly on 0004+0005+0006. This is native handle mapping preflight only: it
    keeps ``runtime_state == fallback_only``, ``real_gpu_pass == false``, and the
    default Godot luminance path active, and never implies a Godot runtime pass,
    a real GPU dispatch, visual/telemetry, or a performance claim.
    """
    if bridge_recording_ready is None:
        bridge_recording_ready = grx009_bridge_real_d3d12_dispatch_recording_ready()
    if not bridge_recording_ready:
        return False
    if inputs_ready is None:
        inputs_ready = grx009_segment4e_native_resource_handle_mapping_inputs_ready()
    if not inputs_ready:
        return False
    if not grx009_patch_0007_applyable(patch_0007_result):
        return False
    return True


def grx009_godot_runtime_recording_evidence() -> dict[str, object] | None:
    """The *latest* runtime smoke evidence. Reproducible-default SKIP when the
    scratch Godot exe env var is absent; never advances the gate on its own."""
    return load_json_file(GRX009_GODOT_RUNTIME_RECORDING_EVIDENCE)


def grx009_godot_runtime_recording_success_evidence() -> dict[str, object] | None:
    """The *historical measured success* artifact. Written only on a strict
    status=success run and never overwritten by a later SKIP/FAIL run; the
    segment 4f readiness gate advances off this file."""
    return load_json_file(GRX009_GODOT_RUNTIME_RECORDING_SUCCESS_EVIDENCE)


def grx009_segment4f_inputs_ready() -> bool:
    """GRX-009 segment 4f Godot-runtime bridge recording harness inputs are wired.

    This is a harness/preflight gate: it requires the segment 4e native resource
    handle mapping inputs to still hold, the 0008 patch markers (the harness-only
    dispatch_recording_smoke opt-in, the RXGD_CAP_LUMINANCE_DISPATCH_RECORD
    record-arm, the RXGD_GODOT_RUNTIME_LUMINANCE_RECORD marker, and the
    test-only/default-off/fallback discipline), and the 0008 patch to stack
    cleanly on 0004+0005+0006+0007. It never implies a real GPU pass, an enabled
    default Godot luminance path, or a measured runtime recording on its own.
    """
    if not grx009_segment4e_native_resource_handle_mapping_inputs_ready():
        return False
    if not file_contains_all(
        GRX009_PATCH_0008,
        [
            "rendering/rurix_accel/passes/luminance_reduction/dispatch_recording_smoke",
            "RXGD_CAP_LUMINANCE_DISPATCH_RECORD",
            "caps.flags |= RXGD_CAP_LUMINANCE_DISPATCH_RECORD",
            "d3d12-recording-shim",
            "RXGD_GODOT_RUNTIME_LUMINANCE_RECORD",
            "RXGD_STATUS_FALLBACK",
            "test-only",
        ],
    ):
        return False
    if not grx009_patch_0008_applyable():
        return False
    return True


GRX009_SEGMENT4F_SUCCESS_EVIDENCE_KIND = "historical_measured_success"
GRX009_SEGMENT4F_LATEST_EVIDENCE_REL_PATH = (
    "spike/godot-rurix/passes/luminance_reduction/"
    "godot_runtime_bridge_recording_evidence.json"
)
GRX009_SEGMENT4F_PATCH_STACK_ID = "0001..0008"
GRX009_SEGMENT4F_PATCH_STACK_FILES = (
    GRX009_PATCH_0001,
    GRX009_PATCH_0002,
    GRX009_PATCH_0003,
    GRX009_PATCH_0004,
    GRX009_PATCH_0005,
    GRX009_PATCH_0006,
    GRX009_PATCH_0007,
    GRX009_PATCH_0008,
)
GRX009_SEGMENT4F_RECORDING_SHIM_FEATURE = "d3d12-recording-shim"
GRX009_SEGMENT4H_PATCH_STACK_ID = "0001..0010"
GRX009_SEGMENT4H_PATCH_STACK_FILES = (
    *GRX009_SEGMENT4F_PATCH_STACK_FILES,
    GRX009_PATCH_0009,
    GRX009_PATCH_0010,
)
GRX009_SEGMENT4H_SUCCESS_EVIDENCE_KIND = "historical_measured_success"
GRX009_SEGMENT4H_LATEST_EVIDENCE_REL_PATH = (
    "spike/godot-rurix/passes/luminance_reduction/real_pass_enablement_evidence.json"
)


def _is_positive_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value > 0


def _is_sha256_hex(value: object) -> bool:
    return isinstance(value, str) and re.fullmatch(r"[0-9a-fA-F]{64}", value) is not None


def _is_git_oid(value: object) -> bool:
    return isinstance(value, str) and re.fullmatch(r"[0-9a-fA-F]{40}", value) is not None


def grx009_segment4f_patch_queue_contains(marker: str) -> bool:
    for patch_path in GRX009_SEGMENT4F_PATCH_STACK_FILES:
        try:
            if marker in patch_path.read_text(encoding="utf-8", errors="ignore"):
                return True
        except OSError:
            return False
    return False


def grx009_segment4f_patch_stack_identity_ok(
    stack: object,
    stack_files: tuple = GRX009_SEGMENT4F_PATCH_STACK_FILES,
    stack_id: str = GRX009_SEGMENT4F_PATCH_STACK_ID,
) -> bool:
    if not isinstance(stack, dict):
        return False
    if stack.get("stack") != stack_id:
        return False
    patches = stack.get("patches")
    if not isinstance(patches, list) or len(patches) != len(stack_files):
        return False
    by_name = {path.name: path for path in stack_files}
    seen: set[str] = set()
    for entry in patches:
        if not isinstance(entry, dict):
            return False
        name = normalize_string(entry.get("patch"))
        if name is None or name not in by_name or name in seen:
            return False
        seen.add(name)
        patch_path = by_name[name]
        if not patch_path.is_file():
            return False
        actual_sha = sha256_of_file(patch_path)
        if actual_sha is None or normalize_string(entry.get("sha256")) != actual_sha:
            return False
        recorded_size = entry.get("size_bytes")
        if (
            not isinstance(recorded_size, int)
            or isinstance(recorded_size, bool)
            or recorded_size != patch_path.stat().st_size
        ):
            return False
    return seen == set(by_name)


def grx009_segment4f_ordered_patch_audit_ok(
    audit: object,
    stack_files: tuple = GRX009_SEGMENT4F_PATCH_STACK_FILES,
) -> bool:
    if not isinstance(audit, list) or len(audit) != len(stack_files):
        return False
    for index, (entry, patch_path) in enumerate(zip(audit, stack_files), start=1):
        if not isinstance(entry, dict):
            return False
        if entry.get("order") != index:
            return False
        if normalize_string(entry.get("patch")) != patch_path.name:
            return False
        actual_sha = sha256_of_file(patch_path)
        if actual_sha is None or normalize_string(entry.get("sha256")) != actual_sha:
            return False
        recorded_size = entry.get("size_bytes")
        if (
            not isinstance(recorded_size, int)
            or isinstance(recorded_size, bool)
            or recorded_size != patch_path.stat().st_size
        ):
            return False
        if not _is_git_oid(entry.get("commit")):
            return False
        if not _is_git_oid(entry.get("tree")):
            return False
    return True


def grx009_segment4f_scratch_source_provenance_ok(
    evidence: dict[str, object],
    stack_files: tuple = GRX009_SEGMENT4F_PATCH_STACK_FILES,
    stack_id: str = GRX009_SEGMENT4F_PATCH_STACK_ID,
) -> bool:
    provenance = evidence.get("scratch_source_provenance")
    if not isinstance(provenance, dict):
        return False
    if provenance.get("source_audit_supported") is not True:
        return False
    if provenance.get("expected_stack_ok") is not True:
        return False
    if provenance.get("source_clean") is not True:
        return False
    source_status = provenance.get("source_status")
    if not isinstance(source_status, list) or len(source_status) != 0:
        return False
    if provenance.get("tracked_patch_stack_only") is not True:
        return False
    if normalize_string(provenance.get("base_snapshot")) != "external/godot-master":
        return False
    for key in ("base_commit", "base_tree", "final_head", "final_tree", "actual_head", "actual_tree"):
        if not _is_git_oid(provenance.get(key)):
            return False
    if provenance.get("actual_tree") != provenance.get("final_tree"):
        return False
    if not grx009_segment4f_patch_stack_identity_ok(
        provenance.get("applied_patch_stack"), stack_files, stack_id
    ):
        return False
    patch_audit = provenance.get("patch_application_audit")
    if not grx009_segment4f_ordered_patch_audit_ok(patch_audit, stack_files):
        return False
    if not isinstance(patch_audit, list) or len(patch_audit) == 0:
        return False
    last_patch_audit = patch_audit[-1]
    if not isinstance(last_patch_audit, dict):
        return False
    if last_patch_audit.get("commit") != provenance.get("final_head"):
        return False
    if last_patch_audit.get("tree") != provenance.get("final_tree"):
        return False
    source_errors = provenance.get("source_audit_errors")
    if source_errors is not None and (not isinstance(source_errors, list) or len(source_errors) != 0):
        return False
    godot_exe = provenance.get("godot_exe")
    if not isinstance(godot_exe, dict):
        return False
    if not normalize_string(godot_exe.get("path_at_run")):
        return False
    if not _is_sha256_hex(godot_exe.get("sha256")):
        return False
    if not _is_positive_int(godot_exe.get("size_bytes")):
        return False
    build = provenance.get("build")
    if build is not None and not isinstance(build, dict):
        return False
    return True


def grx009_segment4f_runtime_log_audit_ok(evidence: dict[str, object]) -> bool:
    audit = evidence.get("runtime_log_audit")
    if not isinstance(audit, dict):
        return False
    if audit.get("unexpected_rxgd_diag_count") != 0:
        return False
    if audit.get("unexpected_godot_error_count") != 0:
        return False
    stdout = normalize_string(evidence.get("stdout")) or ""
    if "RXGD_DIAG" in stdout:
        allowed_by_queue = grx009_segment4f_patch_queue_contains("RXGD_DIAG")
        if not allowed_by_queue or audit.get("rxgd_diag_allowed_by_tracked_patch_queue") is not True:
            return False
    if "ERROR:" in stdout:
        allowed_errors = audit.get("allowed_godot_errors")
        if not isinstance(allowed_errors, list):
            return False
        allowed_global_cache = False
        for entry in allowed_errors:
            if not isinstance(entry, dict):
                continue
            message = normalize_string(entry.get("message")) or ""
            rationale = normalize_string(entry.get("rationale")) or ""
            if "Could not load global script cache" in message and rationale:
                allowed_global_cache = True
        for line in stdout.splitlines():
            if line.strip().startswith("ERROR:") and "Could not load global script cache" not in line:
                return False
            if line.strip().startswith("ERROR:") and "Could not load global script cache" in line and not allowed_global_cache:
                return False
    return True


def grx009_segment4f_success_audit_ok(evidence: dict[str, object]) -> bool:
    """Validate the historical measured success artifact's audit provenance.

    A stale, hand-edited, or partially-populated success JSON must NOT advance
    the segment 4f readiness gate. This re-verifies the audit fields the smoke
    records only on a strict ``status=success`` run:

      * ``evidence_kind == "historical_measured_success"``,
      * ``latest_evidence_path`` points at the reproducible-default latest
        evidence file,
      * ``godot_exe_fingerprint`` pins a real, uncommitted scratch exe
        (nonempty ``exe_sha256``, ``exe_size_bytes > 0``, ``committed == false``),
      * ``patch_stack_identity`` records the full ``0001..0008`` stack and every
        patch entry still matches the current patch file's sha256/size,
      * ``dll_fingerprint`` pins the feature-built recording-shim DLL (nonempty
        ``dll_sha256``, ``dll_size_bytes > 0``, ``features`` carries
        ``d3d12-recording-shim``).
    """
    if not isinstance(evidence, dict):
        return False
    if evidence.get("evidence_kind") != GRX009_SEGMENT4F_SUCCESS_EVIDENCE_KIND:
        return False
    if (
        normalize_string(evidence.get("latest_evidence_path"))
        != GRX009_SEGMENT4F_LATEST_EVIDENCE_REL_PATH
    ):
        return False

    exe_fp = evidence.get("godot_exe_fingerprint")
    if not isinstance(exe_fp, dict):
        return False
    if not _is_sha256_hex(exe_fp.get("exe_sha256")):
        return False
    if not _is_positive_int(exe_fp.get("exe_size_bytes")):
        return False
    if exe_fp.get("committed") is not False:
        return False

    dll_fp = evidence.get("dll_fingerprint")
    if not isinstance(dll_fp, dict):
        return False
    if not _is_sha256_hex(dll_fp.get("dll_sha256")):
        return False
    if not _is_positive_int(dll_fp.get("dll_size_bytes")):
        return False
    snapshot_sha = dll_fp.get("snapshot_dll_sha256")
    if snapshot_sha is not None and snapshot_sha != dll_fp.get("dll_sha256"):
        return False
    features = dll_fp.get("features")
    if (
        not isinstance(features, list)
        or GRX009_SEGMENT4F_RECORDING_SHIM_FEATURE not in features
    ):
        return False

    if not grx009_segment4f_patch_stack_identity_ok(evidence.get("patch_stack_identity")):
        return False
    if not grx009_segment4f_scratch_source_provenance_ok(evidence):
        return False
    if not grx009_segment4f_runtime_log_audit_ok(evidence):
        return False
    return True


def grx009_segment4f_godot_runtime_bridge_recording_ready(
    evidence: dict[str, object] | None = None,
    segment4e_ready: bool | None = None,
) -> bool:
    """GRX-009 segment 4f Godot-runtime bridge dispatch recording smoke is ready.

    Ready only when the segment 4e native resource handle mapping is already
    ready AND the *historical measured success* artifact
    ``godot_runtime_bridge_recording_success_evidence.json`` records
    ``status=success`` — the patched Godot runtime luminance call site (via the
    harness-only dispatch_recording_smoke opt-in and a d3d12-recording-shim
    ``rurix_godot.dll``) drove at least one real bridge-recorded
    RXGD_PASS_LUMINANCE_REDUCTION dispatch through the real native handles.

    The gate deliberately reads the historical success artifact, NOT the
    reproducible-default *latest* evidence file: the latest file honestly
    records ``status=skip`` whenever the scratch Godot exe env var is absent, so
    keying the gate off it would regress readiness every time the smoke reruns
    without the scratch build. A missing success artifact, a stale artifact
    hash, a mismatched discipline flag, or a stale/tampered audit provenance
    (see ``grx009_segment4f_success_audit_ok``: evidence_kind, latest evidence
    pointer, Godot exe / recording-shim DLL fingerprints, and the 0001..0008
    patch-stack hashes) still does NOT advance readiness. Even
    a success keeps ``runtime_state == fallback_only``, ``real_gpu_pass ==
    false``, ``real_d3d12_dispatch_recorded == false`` (default Godot runtime
    meaning), ``godot_runtime_luminance_path_enabled == false``, and
    ``default_enable_state == disabled``.
    """
    if segment4e_ready is None:
        segment4e_ready = grx009_segment4e_native_resource_handle_mapping_ready()
    if not segment4e_ready:
        return False
    if not grx009_segment4f_inputs_ready():
        return False
    if evidence is None:
        evidence = grx009_godot_runtime_recording_success_evidence()
    if not isinstance(evidence, dict):
        return False
    if not grx009_segment4f_success_audit_ok(evidence):
        return False
    if evidence.get("status") != "success":
        return False
    if evidence.get("pass_id") != "luminance_reduction":
        return False
    if evidence.get("segment") != "4f":
        return False
    if evidence.get("runtime_state") != "fallback_only":
        return False
    if evidence.get("real_gpu_pass") is not False:
        return False
    if evidence.get("real_d3d12_dispatch_recorded") is not False:
        return False
    if evidence.get("godot_runtime_bridge_recorded_dispatch") is not True:
        return False
    if evidence.get("godot_runtime_luminance_path_enabled") is not False:
        return False
    if evidence.get("default_enable_state") != "disabled":
        return False
    if evidence.get("gpu_timestamp_status") != "not_yet":
        return False
    if evidence.get("artifact_hashes_match_offline_evidence") is not True:
        return False

    # Re-verify recorded digests against the current offline evidence and the
    # on-disk artifacts so stale/tampered evidence cannot advance the gate.
    offline_digests = grx009_offline_artifact_digests(grx009_compile_evidence())
    current = {
        "dxil": sha256_of_file(GRX009_DXIL_ARTIFACT),
        "root_signature": sha256_of_file(GRX009_ROOT_SIGNATURE_ARTIFACT),
        "descriptor_layout": sha256_of_file(GRX009_DESCRIPTOR_LAYOUT),
    }
    if any(value is None for value in current.values()):
        return False
    if current != offline_digests:
        return False
    recorded = grx009_evidence_artifact_digests(evidence.get("artifacts"))
    if recorded is None or not grx009_measured_evidence_digests_ok(recorded, current):
        return False

    checks = evidence.get("checks")
    required_checks = (
        "artifact_hashes_match_offline_evidence",
        "descriptor_layout_matches_resource_mapping",
        "recording_shim_linked",
        "godot_runtime_session_ready",
        "godot_runtime_call_site_recorded",
        "recorded_one_pass",
        "godot_exit_code_zero",
    )
    if not isinstance(checks, dict):
        return False
    if any(checks.get(name) is not True for name in required_checks):
        return False

    recording = evidence.get("recording")
    if not isinstance(recording, dict):
        return False
    if normalize_string(recording.get("recorded")) != "1":
        return False
    return True


def grx009_segment4f_godot_runtime_bridge_recording_issue(
    evidence: dict[str, object] | None = None,
    segment4e_ready: bool | None = None,
) -> str | None:
    """First concrete missing prerequisite for the segment 4f gate, or None.

    The readiness gate ``grx009_segment4f_godot_runtime_bridge_recording_ready``
    returns ``False`` whenever any prerequisite fails; this companion returns a
    human-readable reason string for the same failure (or ``None`` when the gate
    is ready) so the probe output can honestly report *why* readiness is false
    instead of an empty string. The check order mirrors the readiness function
    exactly so the two functions stay in lockstep.
    """
    if segment4e_ready is None:
        segment4e_ready = grx009_segment4e_native_resource_handle_mapping_ready()
    if not segment4e_ready:
        return "segment 4e native resource handle mapping not ready"
    if not grx009_segment4f_inputs_ready():
        return "segment 4f inputs not ready (tracked artifacts missing or malformed)"
    if evidence is None:
        evidence = grx009_godot_runtime_recording_success_evidence()
    if not isinstance(evidence, dict):
        return (
            "historical success evidence missing "
            "(godot_runtime_bridge_recording_success_evidence.json not found or malformed)"
        )
    if not grx009_segment4f_success_audit_ok(evidence):
        return (
            "success audit failed: evidence_kind/latest evidence pointer/Godot exe/"
            "recording-shim DLL fingerprints or 0001..0008 patch-stack hashes mismatch"
        )
    if evidence.get("status") != "success":
        return f"status != success (got: {evidence.get('status')!r})"
    if evidence.get("pass_id") != "luminance_reduction":
        return f"pass_id mismatch (got: {evidence.get('pass_id')!r})"
    if evidence.get("segment") != "4f":
        return f"segment mismatch (got: {evidence.get('segment')!r})"
    if evidence.get("runtime_state") != "fallback_only":
        return f"runtime_state mismatch (got: {evidence.get('runtime_state')!r})"
    if evidence.get("real_gpu_pass") is not False:
        return f"real_gpu_pass must be false (got: {evidence.get('real_gpu_pass')!r})"
    if evidence.get("real_d3d12_dispatch_recorded") is not False:
        return (
            f"real_d3d12_dispatch_recorded must be false "
            f"(got: {evidence.get('real_d3d12_dispatch_recorded')!r})"
        )
    if evidence.get("godot_runtime_bridge_recorded_dispatch") is not True:
        return (
            f"godot_runtime_bridge_recorded_dispatch must be true "
            f"(got: {evidence.get('godot_runtime_bridge_recorded_dispatch')!r})"
        )
    if evidence.get("godot_runtime_luminance_path_enabled") is not False:
        return (
            f"godot_runtime_luminance_path_enabled must be false "
            f"(got: {evidence.get('godot_runtime_luminance_path_enabled')!r})"
        )
    if evidence.get("default_enable_state") != "disabled":
        return f"default_enable_state mismatch (got: {evidence.get('default_enable_state')!r})"
    if evidence.get("gpu_timestamp_status") != "not_yet":
        return f"gpu_timestamp_status mismatch (got: {evidence.get('gpu_timestamp_status')!r})"
    if evidence.get("artifact_hashes_match_offline_evidence") is not True:
        return (
            "artifact_hashes_match_offline_evidence must be true "
            "(historical success evidence hashes do not match current offline "
            "evidence; rerun ci/grx009_godot_runtime_bridge_recording_smoke.py "
            "with full 0001..0008 scratch Godot exe to re-sign evidence)"
        )

    # Re-verify recorded digests against the current offline evidence and the
    # on-disk artifacts so stale/tampered evidence cannot advance the gate.
    offline_digests = grx009_offline_artifact_digests(grx009_compile_evidence())
    current = {
        "dxil": sha256_of_file(GRX009_DXIL_ARTIFACT),
        "root_signature": sha256_of_file(GRX009_ROOT_SIGNATURE_ARTIFACT),
        "descriptor_layout": sha256_of_file(GRX009_DESCRIPTOR_LAYOUT),
    }
    if any(value is None for value in current.values()):
        return (
            "current artifact file hash missing "
            "(dxil/root_signature/descriptor_layout file not found)"
        )
    if current != offline_digests:
        return "current offline evidence digests do not match on-disk artifact file hashes"
    recorded = grx009_evidence_artifact_digests(evidence.get("artifacts"))
    if recorded is None:
        return "evidence artifacts field is missing or malformed"
    if not grx009_measured_evidence_digests_ok(recorded, current):
        return (
            "historical success evidence artifact digests match neither the "
            "current canonical package nor the pinned raw-buffer-era canonical "
            "package under the owner-approved texture switch"
        )

    checks = evidence.get("checks")
    required_checks = (
        "artifact_hashes_match_offline_evidence",
        "descriptor_layout_matches_resource_mapping",
        "recording_shim_linked",
        "godot_runtime_session_ready",
        "godot_runtime_call_site_recorded",
        "recorded_one_pass",
        "godot_exit_code_zero",
    )
    if not isinstance(checks, dict):
        return "checks field is not a dict"
    missing_names = [name for name in required_checks if checks.get(name) is not True]
    if missing_names:
        return f"required checks missing or false: {missing_names}"

    recording = evidence.get("recording")
    if not isinstance(recording, dict):
        return "recording field is not a dict"
    if normalize_string(recording.get("recorded")) != "1":
        return f"recording.recorded must be '1' (got: {recording.get('recorded')!r})"
    return None


def grx009_segment4g_visual_fallback_evidence() -> dict[str, object] | None:
    """The *latest* segment 4g run evidence. Reproducible-default SKIP when the
    tracked Godot exe is unavailable; never advances the gate on its own."""
    return load_json_file(GRX009_VISUAL_FALLBACK_EVIDENCE)


def grx009_segment4g_visual_fallback_success_evidence() -> dict[str, object] | None:
    """The *historical measured success* segment 4g artifact. Written only on a
    strict status=success run of ci/grx009_segment4g_visual_fallback_smoke.py;
    the segment 4g readiness gate advances off this file."""
    return load_json_file(GRX009_VISUAL_FALLBACK_SUCCESS_EVIDENCE)


GRX009_SEGMENT4G_REQUIRED_CHECKS = (
    "artifact_hashes_match_offline_evidence",
    "reference_run_exit_zero",
    "candidate_run_exit_zero",
    "session_ready_both_runs",
    "fallback_marker_observed_candidate",
    "fallback_marker_absent_reference",
    "frames_captured",
    "dimensions_match",
    "capture_frame_indices_match",
    "runtime_log_audit_clean",
    "diff_within_threshold",
    "telemetry_document_valid",
    "telemetry_entry_coherent",
)


def grx009_segment4g_runtime_log_audit_issue(
    evidence: dict[str, object],
) -> str | None:
    """First runtime-log-audit issue in the segment 4g success evidence, or None.

    Both matrix legs' recorded stdout must be free of unexpected Godot
    ``ERROR:`` lines. The only tolerated error line is the known
    global-script-cache warning, and the recorded audit must explicitly allow
    it with a rationale (segment 4f policy)."""
    audit = evidence.get("runtime_log_audit")
    if not isinstance(audit, dict):
        return "success evidence is missing the runtime_log_audit section"
    for leg_name, stdout_key in (
        ("reference", "stdout_reference"),
        ("candidate", "stdout_candidate"),
    ):
        leg_audit = audit.get(leg_name)
        if not isinstance(leg_audit, dict):
            return f"runtime_log_audit is missing the {leg_name} leg"
        if leg_audit.get("unexpected_rxgd_diag_count") != 0:
            return (
                f"runtime_log_audit {leg_name} leg records unexpected RXGD_DIAG lines"
            )
        if leg_audit.get("unexpected_godot_error_count") != 0:
            return (
                f"runtime_log_audit {leg_name} leg records unexpected Godot "
                "ERROR lines"
            )
        allowed_entries = leg_audit.get("allowed_godot_errors")
        if not isinstance(allowed_entries, list):
            return (
                f"runtime_log_audit {leg_name} leg allowed_godot_errors is malformed"
            )
        allowed_known = False
        for entry in allowed_entries:
            if not isinstance(entry, dict):
                return (
                    f"runtime_log_audit {leg_name} leg allowed_godot_errors "
                    "entry is malformed"
                )
            message = normalize_string(entry.get("message")) or ""
            rationale = normalize_string(entry.get("rationale")) or ""
            if GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR not in message:
                return (
                    f"runtime_log_audit {leg_name} leg allows a Godot error other "
                    f"than '{GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR}'"
                )
            if not rationale:
                return (
                    f"runtime_log_audit {leg_name} leg allowed error lacks a rationale"
                )
            allowed_known = True
        stdout_text = evidence.get(stdout_key)
        if not isinstance(stdout_text, str):
            return f"success evidence is missing {stdout_key}"
        for line in stdout_text.splitlines():
            if not line.strip().startswith("ERROR:"):
                continue
            if GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR not in line:
                return f"{stdout_key} contains an unexpected Godot ERROR line"
            if not allowed_known:
                return (
                    f"{stdout_key} contains the known cache warning but the "
                    "runtime_log_audit does not allow it with a rationale"
                )
    return None


def _grx009_is_frame_dimension(value: object) -> bool:
    return (
        isinstance(value, int)
        and not isinstance(value, bool)
        and value >= GRX009_SEGMENT4G_MIN_FRAME_DIMENSION
    )


def grx009_segment4g_resign_hint() -> str:
    descriptor_sha = sha256_of_file(GRX009_DESCRIPTOR_LAYOUT)
    if descriptor_sha:
        return (
            "rerun ci/grx009_segment4g_visual_fallback_smoke.py to re-sign "
            f"evidence against current descriptor_layout sha256 {descriptor_sha}"
        )
    return "rerun ci/grx009_segment4g_visual_fallback_smoke.py to re-sign evidence"


def grx009_segment4g_visual_fallback_issue(
    evidence: dict[str, object] | None = None,
    segment4f_ready: bool | None = None,
) -> str | None:
    """First concrete missing prerequisite for the segment 4g gate, or None.

    The gate only accepts REAL measured evidence: a status=success historical
    artifact whose raw RGB8 frame artifacts exist on disk with matching SHA-256
    digests and width*height*3 sizes, whose LDR absolute diff numbers match a
    fresh in-process recompute of |reference - candidate| (the tracked diff
    artifact bytes included), whose diff sits within the pinned thresholds, and
    whose measured fallback telemetry records the observed fallback path with
    no real_gpu_pass or performance claim. A SKIP is never ready; placeholder
    or estimated evidence (measured_local != true) is never ready.

    The segment 4f Godot-runtime bridge recording smoke is a hard prerequisite:
    when it is not ready the 4g gate cannot advance, and the returned issue
    text records the Path B re-sign action that unblocks it.
    """
    if not GRX009_VISUAL_FALLBACK_SCHEMA.exists():
        return "visual_fallback_evidence.schema.json is missing"
    if segment4f_ready is None:
        segment4f_ready = grx009_segment4f_godot_runtime_bridge_recording_ready()
    if not segment4f_ready:
        return (
            "segment 4f Godot-runtime bridge recording is not ready; "
            "rerun ci/grx009_godot_runtime_bridge_recording_smoke.py with full "
            "0001..0008 scratch Godot exe to re-sign 4f evidence"
        )
    if evidence is None:
        evidence = grx009_segment4g_visual_fallback_success_evidence()
    if not isinstance(evidence, dict):
        return (
            "historical measured success artifact "
            "visual_fallback_success_evidence.json is missing or unreadable; "
            "run ci/grx009_segment4g_visual_fallback_smoke.py to produce a "
            "measured visual/fallback run"
        )
    status = normalize_string(evidence.get("status"))
    if status != "success":
        return (
            f"visual/fallback success evidence status is {status or 'missing'}, "
            "not success (a SKIP/FAIL run is never ready)"
        )
    if evidence.get("pass_id") != "luminance_reduction":
        return "success evidence pass_id is not luminance_reduction"
    if evidence.get("segment") != "4g":
        return "success evidence segment is not 4g"
    if evidence.get("evidence_kind") != "historical_measured_success":
        return "success evidence evidence_kind is not historical_measured_success"
    if evidence.get("runtime_state") != "fallback_only":
        return "success evidence does not keep runtime_state=fallback_only"
    if evidence.get("real_gpu_pass") is not False:
        return "success evidence does not keep real_gpu_pass=false"
    if evidence.get("real_d3d12_dispatch_recorded") is not False:
        return "success evidence does not keep real_d3d12_dispatch_recorded=false"
    if evidence.get("godot_runtime_luminance_path_enabled") is not False:
        return (
            "success evidence does not keep godot_runtime_luminance_path_enabled=false"
        )
    if evidence.get("default_enable_state") != "disabled":
        return "success evidence does not keep default_enable_state=disabled"
    if normalize_string(evidence.get("performance_claim")) != "none":
        return (
            "success evidence performance_claim is not 'none'; no performance or "
            "FPS improvement claim is allowed at this gate"
        )

    checks = evidence.get("checks")
    if not isinstance(checks, dict) or any(
        checks.get(name) is not True for name in GRX009_SEGMENT4G_REQUIRED_CHECKS
    ):
        return "success evidence checks are not all green"

    visual = evidence.get("visual")
    if not isinstance(visual, dict):
        return "success evidence is missing the visual section"
    if visual.get("measured_local") is not True:
        return (
            "visual evidence is not measured_local=true; placeholder/estimated "
            "visual evidence is never ready"
        )
    if visual.get("metric_kind") != GRX009_SEGMENT4G_METRIC_KIND:
        return f"visual metric_kind is not {GRX009_SEGMENT4G_METRIC_KIND}"
    if visual.get("format") != GRX009_SEGMENT4G_FRAME_FORMAT:
        return f"visual frame format is not {GRX009_SEGMENT4G_FRAME_FORMAT}"
    width = visual.get("width")
    height = visual.get("height")
    if not _grx009_is_frame_dimension(width) or not _grx009_is_frame_dimension(height):
        return (
            "visual width/height are malformed or below the "
            f"{GRX009_SEGMENT4G_MIN_FRAME_DIMENSION}px minimum"
        )
    capture_frame_index = visual.get("capture_frame_index")
    if (
        not isinstance(capture_frame_index, int)
        or isinstance(capture_frame_index, bool)
        or capture_frame_index < 1
    ):
        return "visual capture_frame_index is missing or malformed"
    expected_size = width * height * 3

    frame_bytes: dict[str, bytes] = {}
    for key, path in (
        ("reference_frame", GRX009_VISUAL_REFERENCE_FRAME),
        ("candidate_frame", GRX009_VISUAL_CANDIDATE_FRAME),
        ("diff_artifact", GRX009_VISUAL_DIFF_ARTIFACT),
    ):
        entry = visual.get(key)
        if not isinstance(entry, dict):
            return f"visual {key} entry is missing"
        if not path.is_file():
            return f"visual {key} artifact is missing on disk ({path.name})"
        actual_sha = sha256_of_file(path)
        if actual_sha is None or normalize_string(entry.get("sha256")) != actual_sha:
            return (
                f"visual {key} sha256 does not match the on-disk artifact; "
                f"{grx009_segment4g_resign_hint()}"
            )
        try:
            raw = path.read_bytes()
        except OSError:
            return f"visual {key} artifact is unreadable ({path.name})"
        if len(raw) != expected_size:
            return (
                f"visual {key} size {len(raw)} does not equal width*height*3="
                f"{expected_size} (malformed dimensions)"
            )
        frame_bytes[key] = raw

    computed_diff = bytes(
        abs(a - b)
        for a, b in zip(frame_bytes["reference_frame"], frame_bytes["candidate_frame"])
    )
    if computed_diff != frame_bytes["diff_artifact"]:
        return (
            "diff artifact bytes do not equal the recomputed "
            "|reference - candidate| LDR diff"
        )
    computed_max = max(computed_diff) if computed_diff else 0
    computed_mean = (sum(computed_diff) / len(computed_diff)) if computed_diff else 0.0
    recorded_max = visual.get("max_abs_diff")
    if (
        not isinstance(recorded_max, (int, float))
        or isinstance(recorded_max, bool)
        or float(recorded_max) != float(computed_max)
    ):
        return "recorded max_abs_diff does not match the recomputed diff"
    recorded_mean = visual.get("mean_abs_diff")
    if (
        not isinstance(recorded_mean, (int, float))
        or isinstance(recorded_mean, bool)
        or abs(float(recorded_mean) - computed_mean) > GRX009_SEGMENT4G_MEAN_ABS_EPSILON
    ):
        return "recorded mean_abs_diff does not match the recomputed diff"
    if visual.get("max_abs_diff_threshold") != GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD:
        return "recorded max_abs_diff_threshold does not equal the pinned gate threshold"
    if (
        visual.get("mean_abs_diff_threshold")
        != GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD
    ):
        return "recorded mean_abs_diff_threshold does not equal the pinned gate threshold"
    if (
        computed_max > GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD
        or computed_mean > GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD
    ):
        return (
            "measured LDR absolute diff exceeds the pinned visual gate threshold "
            f"(max_abs={computed_max}, mean_abs={computed_mean:.6f})"
        )
    if visual.get("within_threshold") is not True:
        return "visual within_threshold is not true"

    telemetry = evidence.get("fallback_telemetry")
    if not isinstance(telemetry, dict):
        return "success evidence is missing the fallback_telemetry section"
    if telemetry.get("fallback_path_observed") is not True:
        return "fallback telemetry does not record fallback_path_observed=true"
    matrix = telemetry.get("pass_enable_matrix")
    if not isinstance(matrix, dict):
        return "fallback telemetry is missing the pass_enable_matrix"
    disabled_leg = matrix.get("disabled_default")
    enabled_leg = matrix.get("enabled_fallback")
    if not isinstance(disabled_leg, dict) or not isinstance(enabled_leg, dict):
        return (
            "pass_enable_matrix must record both the disabled_default and "
            "enabled_fallback legs"
        )
    if disabled_leg.get("exit_code") != 0 or enabled_leg.get("exit_code") != 0:
        return "pass_enable_matrix legs did not both exit 0"
    if (
        disabled_leg.get("session_ready") is not True
        or enabled_leg.get("session_ready") is not True
    ):
        return "pass_enable_matrix legs did not both observe a ready bridge session"
    if enabled_leg.get("bridge_fallback_marker_observed") is not True:
        return (
            "enabled_fallback leg did not observe the bridge fallback marker; "
            "the fallback path was not measured"
        )
    if disabled_leg.get("bridge_fallback_marker_observed") is not False:
        return (
            "disabled_default leg unexpectedly observed the bridge fallback "
            "marker; the disabled pass must never call the bridge"
        )

    telemetry_entry = telemetry.get("telemetry_document")
    if not isinstance(telemetry_entry, dict):
        return "fallback telemetry is missing the telemetry_document fingerprint"
    if not GRX009_MEASURED_FALLBACK_TELEMETRY.is_file():
        return "measured_fallback_telemetry.json is missing on disk"
    telemetry_sha = sha256_of_file(GRX009_MEASURED_FALLBACK_TELEMETRY)
    if (
        telemetry_sha is None
        or normalize_string(telemetry_entry.get("sha256")) != telemetry_sha
    ):
        return (
            "measured_fallback_telemetry.json sha256 does not match the recorded "
            f"fingerprint; {grx009_segment4g_resign_hint()}"
        )
    telemetry_doc = load_json_file(GRX009_MEASURED_FALLBACK_TELEMETRY)
    if telemetry_doc is None:
        return "measured_fallback_telemetry.json is unreadable"
    if telemetry_doc.get("evidence_level") != "measured_local":
        return "measured fallback telemetry is not evidence_level=measured_local"
    passes = telemetry_doc.get("passes")
    luminance_entry = None
    if isinstance(passes, list):
        for entry in passes:
            if isinstance(entry, dict) and entry.get("pass_id") == "luminance_reduction":
                luminance_entry = entry
    if luminance_entry is None:
        return "measured fallback telemetry has no luminance_reduction pass entry"
    if luminance_entry.get("godot_fallback_active") is not True:
        return "measured fallback telemetry does not record godot_fallback_active=true"
    if normalize_string(luminance_entry.get("enable_state")) != "enabled":
        return (
            "measured fallback telemetry does not record enable_state=enabled "
            "(the candidate leg enabled the pass)"
        )
    if normalize_string(luminance_entry.get("fallback_reason")) != "validation_failed":
        return (
            "measured fallback telemetry fallback_reason is not validation_failed "
            "(the 0002-level module call carries no resource bindings, so the "
            "bridge preflight must record validation_failed)"
        )
    telemetry_frame = luminance_entry.get("telemetry_frame")
    if (
        not isinstance(telemetry_frame, int)
        or isinstance(telemetry_frame, bool)
        or telemetry_frame != capture_frame_index
    ):
        return (
            "measured fallback telemetry telemetry_frame is stale: it does not "
            "equal the visual capture_frame_index"
        )
    if (
        _bench_script_exit_code(
            GRX008_FALLBACK_TELEMETRY_SCRIPT,
            ["--validate-only", str(GRX009_MEASURED_FALLBACK_TELEMETRY)],
        )
        != 0
    ):
        return (
            "measured_fallback_telemetry.json does not pass "
            "fallback_telemetry.py --validate-only"
        )

    if evidence.get("artifact_hashes_match_offline_evidence") is not True:
        return (
            "success evidence does not record "
            "artifact_hashes_match_offline_evidence=true"
        )
    offline_digests = grx009_offline_artifact_digests(grx009_compile_evidence())
    current = {
        "dxil": sha256_of_file(GRX009_DXIL_ARTIFACT),
        "root_signature": sha256_of_file(GRX009_ROOT_SIGNATURE_ARTIFACT),
        "descriptor_layout": sha256_of_file(GRX009_DESCRIPTOR_LAYOUT),
    }
    if any(value is None for value in current.values()):
        return "tracked offline compile artifacts are missing on disk"
    if current != offline_digests:
        return (
            "tracked offline compile artifacts no longer match the offline "
            "compile evidence digests"
        )
    recorded = grx009_evidence_artifact_digests(evidence.get("artifacts"))
    if recorded is None:
        return "success evidence artifacts block is missing"
    if not grx009_measured_evidence_digests_ok(recorded, current):
        return (
            "success evidence artifact digests match neither the current "
            "canonical package nor the pinned raw-buffer-era canonical package "
            f"under the owner-approved texture switch; {grx009_segment4g_resign_hint()}"
        )

    dll_fp = evidence.get("dll_fingerprint")
    if not isinstance(dll_fp, dict):
        return "success evidence dll_fingerprint is missing"
    dll_features = dll_fp.get("features")
    if not isinstance(dll_features, list) or dll_features:
        return (
            "success evidence dll_fingerprint must record a feature-off shipping "
            "bridge build (no cargo features)"
        )
    log_issue = grx009_segment4g_runtime_log_audit_issue(evidence)
    if log_issue is not None:
        return log_issue
    return None


def grx009_segment4g_visual_fallback_ready(
    evidence: dict[str, object] | None = None,
    segment4f_ready: bool | None = None,
) -> bool:
    """GRX-009 segment 4g real visual diff + measured fallback telemetry gate.

    Ready only when the segment 4f Godot-runtime bridge recording smoke is
    already ready AND the historical measured success artifact
    ``visual_fallback_success_evidence.json`` passes the full strict audit in
    ``grx009_segment4g_visual_fallback_issue`` (real hash-verified frame
    artifacts, recomputed LDR absolute diff within the pinned thresholds, and
    measured fallback telemetry with the observed bridge fallback marker).

    Success here is a fallback-path visual/telemetry gate ONLY: both frames
    were rendered by the native Godot luminance path (the enabled pass fell
    back through the shipping feature-off bridge), so it is NOT visual
    verification of a Rurix GPU pass. It keeps ``runtime_state ==
    fallback_only``, ``real_gpu_pass == false``, ``real_d3d12_dispatch_recorded
    == false``, ``default_enable_state == disabled``, and makes no performance,
    FPS, or GPU-timestamp claim.
    """
    if segment4f_ready is None:
        segment4f_ready = grx009_segment4f_godot_runtime_bridge_recording_ready()
    if not segment4f_ready:
        return False
    return grx009_segment4g_visual_fallback_issue(evidence, segment4f_ready=True) is None


def grx009_segment4h_real_pass_enablement_evidence() -> dict[str, object] | None:
    """The *latest* segment 4h run evidence. Reproducible-default SKIP without
    the 0001..0009 scratch exe; a completed measured run records
    skip_kind=measured_prerequisite_blocked plus the first missing
    prerequisite. Never advances the gate on its own."""
    return load_json_file(GRX009_REAL_PASS_ENABLEMENT_EVIDENCE)


def grx009_segment4h_latest_evidence_hash_chain_issue(
    evidence: dict[str, object] | None = None,
) -> str | None:
    if evidence is None:
        evidence = grx009_segment4h_real_pass_enablement_evidence()
    if not isinstance(evidence, dict):
        return "latest real_pass_enablement_evidence.json is missing or malformed"
    if evidence.get("artifact_hashes_match_offline_evidence") is not True:
        return "latest 4h evidence does not record artifact_hashes_match_offline_evidence=true"

    offline_digests = grx009_offline_artifact_digests(grx009_compile_evidence())
    current = {
        "dxil": sha256_of_file(GRX009_DXIL_ARTIFACT),
        "root_signature": sha256_of_file(GRX009_ROOT_SIGNATURE_ARTIFACT),
        "descriptor_layout": sha256_of_file(GRX009_DESCRIPTOR_LAYOUT),
    }
    if any(value is None for value in current.values()):
        return "tracked 4h artifact files are missing on disk"
    if current != offline_digests:
        return "current 4h artifact file hashes do not match canonical offline fallback evidence"

    recorded = grx009_evidence_artifact_digests(evidence.get("artifacts"))
    if recorded is None:
        return "latest 4h evidence artifacts block is missing"
    if not grx009_measured_evidence_digests_ok(recorded, current):
        return (
            "latest 4h evidence artifact sha256 hashes match neither the "
            "current canonical package nor the pinned raw-buffer-era canonical "
            "package under the owner-approved texture switch"
        )

    evidence_offline = evidence.get("offline_evidence")
    if not isinstance(evidence_offline, dict):
        return "latest 4h evidence offline_evidence block is missing"
    offline_fields = {
        "dxil": "dxil_sha256",
        "root_signature": "root_signature_sha256",
        "descriptor_layout": "descriptor_layout_sha256",
    }
    recorded_offline = {
        key: normalize_string(evidence_offline.get(field))
        for key, field in offline_fields.items()
    }
    if not grx009_measured_evidence_digests_ok(recorded_offline, offline_digests):
        return (
            "latest 4h evidence offline_evidence sha256 hashes match neither "
            "the current canonical package nor the pinned raw-buffer-era "
            "canonical package under the owner-approved texture switch"
        )
    return None


def grx009_segment4h_real_pass_enablement_success_evidence() -> dict[str, object] | None:
    """The *historical measured success* segment 4h artifact. Written only on
    a strict status=success run (opt-in real dispatch executed AND completed
    AND visual gate green); unreachable with the tracked segment 3a artifact
    by design. The segment 4h readiness gate advances off this file."""
    return load_json_file(GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE)


GRX009_SEGMENT4H_REQUIRED_CHECKS = (
    "artifact_hashes_match_offline_evidence",
    "reference_run_exit_zero",
    "candidate_run_exit_zero",
    "forced_fallback_run_exit_zero",
    "session_ready_all_runs",
    "markers_absent_reference",
    "fallback_marker_observed_forced_fallback",
    "real_pass_blocked_marker_observed_forced_fallback",
    "record_marker_absent_all_runs",
    "frames_captured",
    "dimensions_match",
    "capture_frame_indices_match",
    "runtime_log_audit_clean",
    "diff_within_threshold_candidate",
    "diff_within_threshold_forced_fallback",
    "telemetry_document_valid",
    "telemetry_entries_coherent",
    "scratch_source_provenance_ok",
    "real_pass_dispatched_and_completed",
)


def grx009_segment4h_runtime_log_audit_issue(
    evidence: dict[str, object],
) -> str | None:
    """First runtime-log-audit issue in segment 4h evidence, or None. All
    three matrix legs' recorded stdout must be free of unexpected Godot
    ``ERROR:`` lines (segment 4f/4g policy)."""
    audit = evidence.get("runtime_log_audit")
    if not isinstance(audit, dict):
        return "evidence is missing the runtime_log_audit section"
    for leg_name, stdout_key in (
        ("reference", "stdout_reference"),
        ("candidate", "stdout_candidate"),
        ("forced_fallback", "stdout_forced_fallback"),
    ):
        leg_audit = audit.get(leg_name)
        if not isinstance(leg_audit, dict):
            return f"runtime_log_audit is missing the {leg_name} leg"
        if leg_audit.get("unexpected_rxgd_diag_count") != 0:
            return (
                f"runtime_log_audit {leg_name} leg records unexpected RXGD_DIAG lines"
            )
        if leg_audit.get("unexpected_godot_error_count") != 0:
            return (
                f"runtime_log_audit {leg_name} leg records unexpected Godot "
                "ERROR lines"
            )
        allowed_entries = leg_audit.get("allowed_godot_errors")
        if not isinstance(allowed_entries, list):
            return (
                f"runtime_log_audit {leg_name} leg allowed_godot_errors is malformed"
            )
        allowed_known = False
        for entry in allowed_entries:
            if not isinstance(entry, dict):
                return (
                    f"runtime_log_audit {leg_name} leg allowed_godot_errors "
                    "entry is malformed"
                )
            message = normalize_string(entry.get("message")) or ""
            rationale = normalize_string(entry.get("rationale")) or ""
            if GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR not in message:
                return (
                    f"runtime_log_audit {leg_name} leg allows a Godot error other "
                    f"than '{GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR}'"
                )
            if not rationale:
                return (
                    f"runtime_log_audit {leg_name} leg allowed error lacks a rationale"
                )
            allowed_known = True
        stdout_text = evidence.get(stdout_key)
        if not isinstance(stdout_text, str):
            return f"evidence is missing {stdout_key}"
        for line in stdout_text.splitlines():
            if not line.strip().startswith("ERROR:"):
                continue
            if GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR not in line:
                return f"{stdout_key} contains an unexpected Godot ERROR line"
            if not allowed_known:
                return (
                    f"{stdout_key} contains the known cache warning but the "
                    "runtime_log_audit does not allow it with a rationale"
                )
    return None


def grx009_segment4h_real_pass_enablement_issue(
    evidence: dict[str, object] | None = None,
) -> str | None:
    """First concrete missing prerequisite for the segment 4h readiness gate,
    or None.

    The gate only accepts a REAL measured strict success: a status=success
    historical artifact where the opt-in real dispatch actually executed and
    completed (``RXGD_GODOT_RUNTIME_LUMINANCE_REAL_PASS`` marker line pinned),
    the raw RGB8 frame artifacts exist on disk with matching SHA-256 digests,
    a fresh recompute of |reference - candidate| matches the recorded diff and
    sits within the pinned LDR thresholds, the forced-failure red leg fell
    back with ``unsupported_device``, the 0001..0009 patch-stack identity and
    scratch source provenance audit hold, the runtime log audit is clean, and
    no performance/FPS/GPU-timestamp claim exists. With the tracked segment 4i
    artifact this success is unreachable BY DESIGN: the patched llc does not
    support the ``llvm.dx.resource.load.texture.2d`` intrinsic, so the
    texture-capable offline compile fails closed and the bridge tracked
    package stays raw-buffer while the Godot runtime provides Texture2D
    handles. The honest gate outcome today is the latest evidence's
    measured_prerequisite_blocked SKIP naming
    ``kernel_binding_kind_mismatch``, which never advances readiness. Only
    once a newer patched llc lands and the tracked package flips to
    texture-capable does the FIRST missing prerequisite advance to
    ``math_pyramid_parity_not_proven`` (single 8x8 reduction level: no
    multi-level pyramid cascade, no EMA feedback, no previous-luminance
    double buffering, no final-level WRITE_LUMINANCE clamp gating)."""
    if not GRX009_REAL_PASS_ENABLEMENT_SCHEMA.exists():
        return "real_pass_enablement_evidence.schema.json is missing"
    if evidence is None:
        evidence = grx009_segment4h_real_pass_enablement_success_evidence()
    if not isinstance(evidence, dict):
        return (
            "historical measured success artifact "
            "real_pass_enablement_success_evidence.json is missing; the "
            "opt-in real-pass gate has not measured a strict success (with "
            "the tracked segment 4i artifact this is the designed "
            "fail-closed state: the first missing prerequisite is "
            "kernel_binding_kind_mismatch — a runtime-mappable "
            "texture-capable kernel artifact round, blocked today by the "
            "patched llc not supporting texture intrinsics)"
        )
    status = normalize_string(evidence.get("status"))
    if status != "success":
        return (
            f"real-pass enablement success evidence status is {status or 'missing'}, "
            "not success (a SKIP/FAIL run is never ready)"
        )
    if evidence.get("pass_id") != "luminance_reduction":
        return "success evidence pass_id is not luminance_reduction"
    if evidence.get("segment") != "4h":
        return "success evidence segment is not 4h"
    if evidence.get("evidence_kind") != GRX009_SEGMENT4H_SUCCESS_EVIDENCE_KIND:
        return "success evidence evidence_kind is not historical_measured_success"
    if (
        normalize_string(evidence.get("latest_evidence_path"))
        != GRX009_SEGMENT4H_LATEST_EVIDENCE_REL_PATH
    ):
        return "success evidence latest_evidence_path is wrong"
    if evidence.get("runtime_state") != "fallback_only":
        return "success evidence does not keep runtime_state=fallback_only"
    if evidence.get("godot_runtime_luminance_path_enabled") is not False:
        return (
            "success evidence does not keep godot_runtime_luminance_path_enabled=false"
        )
    if evidence.get("default_enable_state") != "disabled":
        return "success evidence does not keep default_enable_state=disabled"
    if evidence.get("gpu_timestamp_status") != "not_yet":
        return "success evidence does not keep gpu_timestamp_status=not_yet"
    if normalize_string(evidence.get("performance_claim")) != "none":
        return (
            "success evidence performance_claim is not 'none'; no performance or "
            "FPS improvement claim is allowed at this gate"
        )
    if (
        normalize_string(evidence.get("expected_first_missing_prerequisite"))
        != GRX009_SEGMENT4H_EXPECTED_FIRST_MISSING_PREREQUISITE
    ):
        return "success evidence does not pin the expected first missing prerequisite"
    # A strict success is the ONLY document allowed to record a real GPU pass,
    # and it must carry the real-pass marker line as measured proof.
    if evidence.get("real_gpu_pass") is not True:
        return "success evidence does not record real_gpu_pass=true"
    if evidence.get("real_d3d12_dispatch_recorded") is not True:
        return "success evidence does not record real_d3d12_dispatch_recorded=true"
    marker_line = normalize_string(evidence.get("real_pass_marker_line"))
    if not marker_line or GRX009_SEGMENT4H_REAL_PASS_MARKER not in marker_line:
        return (
            "success evidence does not pin the observed "
            f"{GRX009_SEGMENT4H_REAL_PASS_MARKER} marker line"
        )

    checks = evidence.get("checks")
    if not isinstance(checks, dict) or any(
        checks.get(name) is not True for name in GRX009_SEGMENT4H_REQUIRED_CHECKS
    ):
        return "success evidence checks are not all green"

    matrix = evidence.get("pass_enable_matrix")
    if not isinstance(matrix, dict):
        return "success evidence is missing the pass_enable_matrix"
    reference_leg = matrix.get("disabled_default")
    candidate_leg = matrix.get("enabled_real_pass_optin")
    forced_leg = matrix.get("forced_capability_downgrade")
    if not all(
        isinstance(leg, dict) for leg in (reference_leg, candidate_leg, forced_leg)
    ):
        return (
            "pass_enable_matrix must record the disabled_default, "
            "enabled_real_pass_optin, and forced_capability_downgrade legs"
        )
    for name, leg in (
        ("disabled_default", reference_leg),
        ("enabled_real_pass_optin", candidate_leg),
        ("forced_capability_downgrade", forced_leg),
    ):
        if leg.get("exit_code") != 0:
            return f"pass_enable_matrix {name} leg did not exit 0"
        if leg.get("session_ready") is not True:
            return f"pass_enable_matrix {name} leg did not observe a ready session"
        if leg.get("record_marker_observed") is not False:
            return f"pass_enable_matrix {name} leg observed the recording marker"
    for marker_key in (
        "bridge_fallback_marker_observed",
        "real_pass_blocked_marker_observed",
        "real_pass_marker_observed",
    ):
        if reference_leg.get(marker_key) is not False:
            return (
                "disabled_default leg unexpectedly observed a bridge marker; "
                "the disabled pass must never invoke the bridge"
            )
    if candidate_leg.get("real_pass_marker_observed") is not True:
        return "enabled_real_pass_optin leg did not observe the real-pass marker"
    if candidate_leg.get("real_pass_blocked_marker_observed") is not False:
        return (
            "enabled_real_pass_optin leg observed the blocked diagnostic; the "
            "success outcome is contradictory"
        )
    if candidate_leg.get("bridge_fallback_marker_observed") is not False:
        return (
            "enabled_real_pass_optin leg observed the fallback marker; the "
            "success outcome is contradictory"
        )
    if forced_leg.get("bridge_fallback_marker_observed") is not True:
        return (
            "forced_capability_downgrade leg did not observe the fallback "
            "marker; the forced-failure red leg was not measured"
        )
    if forced_leg.get("real_pass_blocked_marker_observed") is not True:
        return (
            "forced_capability_downgrade leg did not observe the blocked "
            "diagnostic"
        )
    if forced_leg.get("real_pass_marker_observed") is not False:
        return "forced_capability_downgrade leg observed the real-pass marker"

    visual = evidence.get("visual")
    if not isinstance(visual, dict):
        return "success evidence is missing the visual section"
    if visual.get("measured_local") is not True:
        return "visual evidence is not measured_local=true"
    if visual.get("metric_kind") != GRX009_SEGMENT4G_METRIC_KIND:
        return f"visual metric_kind is not {GRX009_SEGMENT4G_METRIC_KIND}"
    if visual.get("format") != GRX009_SEGMENT4G_FRAME_FORMAT:
        return f"visual frame format is not {GRX009_SEGMENT4G_FRAME_FORMAT}"
    width = visual.get("width")
    height = visual.get("height")
    if not _grx009_is_frame_dimension(width) or not _grx009_is_frame_dimension(height):
        return (
            "visual width/height are malformed or below the "
            f"{GRX009_SEGMENT4G_MIN_FRAME_DIMENSION}px minimum"
        )
    capture_frame_index = visual.get("capture_frame_index")
    if (
        not isinstance(capture_frame_index, int)
        or isinstance(capture_frame_index, bool)
        or capture_frame_index < 1
    ):
        return "visual capture_frame_index is missing or malformed"
    if visual.get("max_abs_diff_threshold") != GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD:
        return "recorded max_abs_diff_threshold does not equal the pinned gate threshold"
    if (
        visual.get("mean_abs_diff_threshold")
        != GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD
    ):
        return "recorded mean_abs_diff_threshold does not equal the pinned gate threshold"
    expected_size = width * height * 3

    frame_bytes: dict[str, bytes] = {}
    for key, path in (
        ("reference_frame", GRX009_VISUAL_REAL_PASS_REFERENCE_FRAME),
        ("candidate_frame", GRX009_VISUAL_REAL_PASS_CANDIDATE_FRAME),
        ("diff_artifact", GRX009_VISUAL_REAL_PASS_DIFF_ARTIFACT),
    ):
        entry = visual.get(key)
        if not isinstance(entry, dict):
            return f"visual {key} entry is missing"
        if not path.is_file():
            return f"visual {key} artifact is missing on disk ({path.name})"
        actual_sha = sha256_of_file(path)
        if actual_sha is None or normalize_string(entry.get("sha256")) != actual_sha:
            return f"visual {key} sha256 does not match the on-disk artifact"
        try:
            raw = path.read_bytes()
        except OSError:
            return f"visual {key} artifact is unreadable ({path.name})"
        if len(raw) != expected_size:
            return (
                f"visual {key} size {len(raw)} does not equal width*height*3="
                f"{expected_size} (malformed dimensions)"
            )
        frame_bytes[key] = raw

    computed_diff = bytes(
        abs(a - b)
        for a, b in zip(frame_bytes["reference_frame"], frame_bytes["candidate_frame"])
    )
    if computed_diff != frame_bytes["diff_artifact"]:
        return (
            "diff artifact bytes do not equal the recomputed "
            "|reference - candidate| LDR diff"
        )
    computed_max = max(computed_diff) if computed_diff else 0
    computed_mean = (sum(computed_diff) / len(computed_diff)) if computed_diff else 0.0
    diffs = visual.get("diffs")
    if not isinstance(diffs, dict):
        return "visual diffs section is missing"
    candidate_diff = diffs.get("candidate")
    forced_diff = diffs.get("forced_fallback")
    if not isinstance(candidate_diff, dict) or not isinstance(forced_diff, dict):
        return "visual diffs must record candidate and forced_fallback entries"
    recorded_max = candidate_diff.get("max_abs_diff")
    if (
        not isinstance(recorded_max, (int, float))
        or isinstance(recorded_max, bool)
        or float(recorded_max) != float(computed_max)
    ):
        return "recorded candidate max_abs_diff does not match the recomputed diff"
    recorded_mean = candidate_diff.get("mean_abs_diff")
    if (
        not isinstance(recorded_mean, (int, float))
        or isinstance(recorded_mean, bool)
        or abs(float(recorded_mean) - computed_mean) > GRX009_SEGMENT4G_MEAN_ABS_EPSILON
    ):
        return "recorded candidate mean_abs_diff does not match the recomputed diff"
    if (
        computed_max > GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD
        or computed_mean > GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD
    ):
        return (
            "measured LDR absolute diff exceeds the pinned visual gate threshold "
            f"(max_abs={computed_max}, mean_abs={computed_mean:.6f})"
        )
    for name, entry in (("candidate", candidate_diff), ("forced_fallback", forced_diff)):
        if entry.get("within_threshold") is not True:
            return f"visual diffs {name} within_threshold is not true"

    telemetry = evidence.get("fallback_telemetry")
    if not isinstance(telemetry, dict):
        return "success evidence is missing the fallback_telemetry section"
    if telemetry.get("no_fps_claim") is not True:
        return "fallback telemetry does not record no_fps_claim=true"
    telemetry_entry = telemetry.get("telemetry_document")
    if not isinstance(telemetry_entry, dict):
        return "fallback telemetry is missing the telemetry_document fingerprint"
    if not GRX009_REAL_PASS_ENABLEMENT_TELEMETRY.is_file():
        return "real_pass_enablement_telemetry.json is missing on disk"
    telemetry_sha = sha256_of_file(GRX009_REAL_PASS_ENABLEMENT_TELEMETRY)
    if (
        telemetry_sha is None
        or normalize_string(telemetry_entry.get("sha256")) != telemetry_sha
    ):
        return (
            "real_pass_enablement_telemetry.json sha256 does not match the "
            "recorded fingerprint"
        )
    telemetry_doc = load_json_file(GRX009_REAL_PASS_ENABLEMENT_TELEMETRY)
    if telemetry_doc is None:
        return "real_pass_enablement_telemetry.json is unreadable"
    if telemetry_doc.get("evidence_level") != "measured_local":
        return "real-pass enablement telemetry is not evidence_level=measured_local"
    passes = telemetry_doc.get("passes")
    forced_entry = None
    if isinstance(passes, list):
        for entry in passes:
            if not isinstance(entry, dict):
                continue
            if entry.get("leg") == "enabled_real_pass_optin":
                return (
                    "telemetry document carries a candidate fallback entry "
                    "although the success evidence claims a real pass; the "
                    "outcome is contradictory"
                )
            if entry.get("leg") == "forced_capability_downgrade":
                forced_entry = entry
    if forced_entry is None:
        return "telemetry document has no forced_capability_downgrade entry"
    if forced_entry.get("pass_id") != "luminance_reduction":
        return "forced telemetry entry pass_id is not luminance_reduction"
    if normalize_string(forced_entry.get("enable_state")) != "enabled":
        return "forced telemetry entry does not record enable_state=enabled"
    if normalize_string(forced_entry.get("fallback_reason")) != "unsupported_device":
        return "forced telemetry entry fallback_reason is not unsupported_device"
    if forced_entry.get("godot_fallback_active") is not True:
        return "forced telemetry entry does not record godot_fallback_active=true"
    telemetry_frame = forced_entry.get("telemetry_frame")
    if (
        not isinstance(telemetry_frame, int)
        or isinstance(telemetry_frame, bool)
        or telemetry_frame != capture_frame_index
    ):
        return (
            "forced telemetry entry telemetry_frame is stale: it does not "
            "equal the visual capture_frame_index"
        )
    if (
        _bench_script_exit_code(
            GRX008_FALLBACK_TELEMETRY_SCRIPT,
            ["--validate-only", str(GRX009_REAL_PASS_ENABLEMENT_TELEMETRY)],
        )
        != 0
    ):
        return (
            "real_pass_enablement_telemetry.json does not pass "
            "fallback_telemetry.py --validate-only"
        )

    if evidence.get("artifact_hashes_match_offline_evidence") is not True:
        return (
            "success evidence does not record "
            "artifact_hashes_match_offline_evidence=true"
        )
    offline_digests = grx009_offline_artifact_digests(grx009_compile_evidence())
    current = {
        "dxil": sha256_of_file(GRX009_DXIL_ARTIFACT),
        "root_signature": sha256_of_file(GRX009_ROOT_SIGNATURE_ARTIFACT),
        "descriptor_layout": sha256_of_file(GRX009_DESCRIPTOR_LAYOUT),
    }
    if any(value is None for value in current.values()):
        return "tracked offline compile artifacts are missing on disk"
    if current != offline_digests:
        return (
            "tracked offline compile artifacts no longer match the offline "
            "compile evidence digests"
        )
    recorded = grx009_evidence_artifact_digests(evidence.get("artifacts"))
    if recorded is None:
        return "success evidence artifacts block is missing"
    if not grx009_measured_evidence_digests_ok(recorded, current):
        return (
            "success evidence artifact digests match neither the current "
            "canonical package nor the pinned raw-buffer-era canonical package "
            "under the owner-approved texture switch"
        )

    exe_fp = evidence.get("godot_exe_fingerprint")
    if not isinstance(exe_fp, dict):
        return "success evidence godot_exe_fingerprint is missing"
    if not _is_sha256_hex(exe_fp.get("exe_sha256")):
        return "success evidence godot_exe_fingerprint sha256 is malformed"
    if not _is_positive_int(exe_fp.get("exe_size_bytes")):
        return "success evidence godot_exe_fingerprint size is malformed"
    if exe_fp.get("committed") is not False:
        return "success evidence godot_exe_fingerprint must record committed=false"

    dll_fp = evidence.get("dll_fingerprint")
    if not isinstance(dll_fp, dict):
        return "success evidence dll_fingerprint is missing"
    if not _is_sha256_hex(dll_fp.get("dll_sha256")):
        return "success evidence dll_fingerprint sha256 is malformed"
    dll_features = dll_fp.get("features")
    # Stage A5: a strict real-pass success is only reachable through the
    # linked real dispatch path, which is compiled ONLY under the
    # d3d12-recording-shim feature. The success evidence must therefore
    # record exactly that feature build (the shipping feature-off bridge
    # stays fail-closed and can never produce a success document).
    if not isinstance(dll_features, list) or dll_features != ["d3d12-recording-shim"]:
        return (
            "success evidence dll_fingerprint must record the stage A5 "
            "d3d12-recording-shim bridge build (the only build with the "
            "linked real dispatch path)"
        )

    if not grx009_segment4f_patch_stack_identity_ok(
        evidence.get("patch_stack_identity"),
        GRX009_SEGMENT4H_PATCH_STACK_FILES,
        GRX009_SEGMENT4H_PATCH_STACK_ID,
    ):
        return (
            "success evidence patch_stack_identity does not match the tracked "
            f"{GRX009_SEGMENT4H_PATCH_STACK_ID} patch stack"
        )
    if not grx009_segment4f_scratch_source_provenance_ok(
        evidence,
        GRX009_SEGMENT4H_PATCH_STACK_FILES,
        GRX009_SEGMENT4H_PATCH_STACK_ID,
    ):
        return (
            "success evidence scratch source provenance does not pass the "
            f"{GRX009_SEGMENT4H_PATCH_STACK_ID} tracked-patch-stack-only audit"
        )

    log_issue = grx009_segment4h_runtime_log_audit_issue(evidence)
    if log_issue is not None:
        return log_issue
    return None


def grx009_segment4h_real_pass_enablement_ready(
    evidence: dict[str, object] | None = None,
    segment4g_ready: bool | None = None,
) -> bool:
    """GRX-009 segment 4h gated real-pass enablement gate.

    Ready only when the segment 4g visual/fallback gate is already ready AND
    the historical measured success artifact
    ``real_pass_enablement_success_evidence.json`` passes the full strict
    audit in ``grx009_segment4h_real_pass_enablement_issue`` (real dispatch
    executed and completed, hash-verified frames, recomputed LDR diff within
    thresholds, forced-failure red leg measured, 0001..0009 provenance, clean
    runtime log audit, no performance claim). With the tracked segment 4i
    artifact this gate stays not-ready BY DESIGN: the patched llc does not
    support the texture-load intrinsic, so the tracked bridge package stays
    raw-buffer, the bridge's kernel-binding-kind conformance check fails
    closed against the Godot runtime's Texture2D handles, and the latest
    evidence records the first missing prerequisite
    (``kernel_binding_kind_mismatch`` — a runtime-mappable texture-capable
    kernel artifact round) instead. Even a future success keeps
    ``default_enable_state == disabled`` and ``performance_claim == none``.
    """
    if segment4g_ready is None:
        segment4g_ready = grx009_segment4g_visual_fallback_ready()
    if not segment4g_ready:
        return False
    return grx009_segment4h_real_pass_enablement_issue(evidence) is None


GRX009_DEFAULT_ENABLE_DECISION_DOC_SECTIONS = [
    "## Owner Decision",
    "## Rationale",
    "## Re-evaluation Conditions",
    "## Fail-Closed Invariants",
]


def grx009_real_pass_default_enable_decision_evidence() -> dict[str, object] | None:
    return load_json_file(GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE)


def grx009_real_pass_default_enable_decision_status(
    evidence: dict[str, object] | None = None,
) -> str:
    candidate = (
        evidence
        if evidence is not None
        else grx009_real_pass_default_enable_decision_evidence()
    )
    if not isinstance(candidate, dict):
        return "missing"
    return normalize_string(candidate.get("status")) or "malformed"


def grx009_real_pass_default_enable_decision_issue(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
) -> str | None:
    """First missing prerequisite of the stage A5 owner default-enable
    decision gate, or None.

    Ready only when the segment 4h strict measured success is active (audited,
    fail-closed) AND the owner decision evidence records keep_default_disabled
    with a signed owner block, the rationale/re-evaluation lists, the decision
    document with its required sections, performance_claim=none, and the
    manifest keeps default_enable_state=disabled while referencing the
    decision files from the segment_4h_real_pass_measured_success block.
    Modeled on the segment 4l texture artifact provenance policy gate."""
    if not grx009_real_pass_measured_success_active():
        return "real_pass_measured_success_not_active"
    candidate = (
        evidence
        if evidence is not None
        else grx009_real_pass_default_enable_decision_evidence()
    )
    if not isinstance(candidate, dict):
        return "default_enable_decision_evidence_missing"
    if candidate.get("pass_id") != "luminance_reduction":
        return "default_enable_decision_pass_id_mismatch"
    if normalize_string(candidate.get("segment")) != (
        "4m_real_pass_default_enable_decision"
    ):
        return "default_enable_decision_segment_mismatch"
    if normalize_string(candidate.get("status")) != "success":
        return "default_enable_decision_status_must_be_success"
    if candidate.get("decision_ready") is not True:
        return "default_enable_decision_ready_must_be_true"
    if normalize_string(candidate.get("default_enable_state")) != "disabled":
        return "default_enable_decision_state_must_be_disabled"
    if normalize_string(candidate.get("performance_claim")) != "none":
        return "default_enable_decision_performance_claim_must_be_none"
    owner_decision = candidate.get("owner_decision")
    if not isinstance(owner_decision, dict):
        return "default_enable_decision_owner_block_missing"
    if normalize_string(owner_decision.get("decision")) != (
        GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION
    ):
        return "default_enable_decision_owner_decision_mismatch"
    if not normalize_string(owner_decision.get("approved_by")):
        return "default_enable_decision_owner_approved_by_missing"
    if not normalize_string(owner_decision.get("machine_role")):
        return "default_enable_decision_owner_machine_role_missing"
    rationale = candidate.get("decision_rationale")
    if not isinstance(rationale, list) or not rationale:
        return "default_enable_decision_rationale_missing"
    reevaluate = candidate.get("reevaluate_when")
    if not isinstance(reevaluate, list) or not reevaluate:
        return "default_enable_decision_reevaluate_conditions_missing"
    prerequisite = candidate.get("prerequisite_evidence")
    if not isinstance(prerequisite, dict):
        return "default_enable_decision_prerequisite_evidence_missing"
    success_path = grx009_repo_path(
        normalize_string(prerequisite.get("real_pass_enablement_success"))
    )
    if success_path != GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE:
        return "default_enable_decision_prerequisite_success_path_mismatch"
    decision_doc_path = grx009_repo_path(
        normalize_string(candidate.get("decision_document"))
    )
    if decision_doc_path != GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION_DOC:
        return "default_enable_decision_document_path_mismatch"
    if not GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION_DOC.is_file():
        return "default_enable_decision_document_missing"
    if not file_contains_all(
        GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION_DOC,
        GRX009_DEFAULT_ENABLE_DECISION_DOC_SECTIONS,
    ):
        return "default_enable_decision_document_required_sections_missing"
    if normalize_string(candidate.get("next_action_if_ready")) != GRX010_NEXT_ACTION:
        return "default_enable_decision_next_action_mismatch"
    manifest_doc = manifest if manifest is not None else grx009_manifest()
    if not isinstance(manifest_doc, dict):
        return "manifest_missing"
    if manifest_doc.get("default_enable_state") != "disabled":
        return "manifest_default_enable_state_must_remain_disabled"
    implementation_status = manifest_doc.get("implementation_status")
    if not isinstance(implementation_status, dict):
        return "manifest_implementation_status_missing"
    measured_success = implementation_status.get(
        "segment_4h_real_pass_measured_success"
    )
    if not isinstance(measured_success, dict):
        return "manifest_measured_success_block_missing"
    if normalize_string(measured_success.get("status")) != "success":
        return "manifest_measured_success_status_mismatch"
    if normalize_string(measured_success.get("decision")) != (
        GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION
    ):
        return "manifest_measured_success_decision_mismatch"
    if grx009_repo_path(
        normalize_string(measured_success.get("default_enable_decision"))
    ) != GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE:
        return "manifest_measured_success_decision_path_mismatch"
    if grx009_repo_path(
        normalize_string(measured_success.get("evidence"))
    ) != GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE:
        return "manifest_measured_success_evidence_path_mismatch"
    if normalize_string(measured_success.get("next_action_when_ready")) != (
        GRX010_NEXT_ACTION
    ):
        return "manifest_measured_success_next_action_mismatch"
    if normalize_string(measured_success.get("performance_claim")) != "none":
        return "manifest_measured_success_performance_claim_must_be_none"
    return None


def grx009_real_pass_default_enable_decision_ready(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
) -> bool:
    return (
        grx009_real_pass_default_enable_decision_issue(evidence, manifest) is None
    )


# =====================================================================
# GRX-010 tonemap stage-A5-equivalent close-out gates. Mirror of the
# GRX-009 segment 4h real-pass enablement + segment 4m owner default-enable
# decision, adapted to the tonemap markers, the 0001..0013 patch stack, and
# the tonemap-specific patch 0013 writeback-scaffold marker. Even a strict
# measured success keeps default_enable_state=disabled and performance_claim
# =none.
# =====================================================================
def grx010_manifest() -> dict[str, object] | None:
    return load_json_file(GRX010_PASS_DIR / "pass_manifest.json")


def grx010_real_pass_enablement_success_evidence() -> dict[str, object] | None:
    """The *historical measured success* tonemap real-pass artifact. Written
    only on a strict status=success run (opt-in real dispatch executed AND
    completed AND the LDR visual gate stayed within thresholds); never deleted
    or overwritten by a later SKIP/FAIL run. The readiness gate advances off
    this file."""
    return load_json_file(GRX010_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE)


def _grx010_enablement_artifact_digests_issue(
    evidence: dict[str, object],
) -> str | None:
    """The recorded evidence artifact digests must equal the on-disk canonical
    tonemap artifacts AND the tonemap offline compile evidence digests, byte
    for byte (anti-fabrication)."""
    artifacts = evidence.get("artifacts")
    if not isinstance(artifacts, dict):
        return "success evidence artifacts block is missing"
    offline = load_json_file(GRX010_OFFLINE_COMPILE_EVIDENCE)
    offline_artifacts = offline.get("artifacts") if isinstance(offline, dict) else None
    if not isinstance(offline_artifacts, dict):
        return "tonemap offline compile evidence is missing"
    disk_paths = {
        "dxil": GRX010_DXIL_ARTIFACT,
        "root_signature": GRX010_ROOT_SIGNATURE_ARTIFACT,
        "descriptor_layout": GRX010_DESCRIPTOR_LAYOUT,
    }
    for key, disk_path in disk_paths.items():
        entry = artifacts.get(key)
        recorded = (
            normalize_string(entry.get("sha256")) if isinstance(entry, dict) else None
        )
        actual = sha256_of_file(disk_path)
        offline_entry = offline_artifacts.get(key)
        offline_sha = (
            normalize_string(offline_entry.get("sha256"))
            if isinstance(offline_entry, dict)
            else None
        )
        if recorded is None or actual is None or recorded != actual:
            return (
                "success evidence artifact digests do not match the on-disk "
                "tonemap artifacts"
            )
        if offline_sha is None or offline_sha != actual:
            return (
                "on-disk tonemap artifacts no longer match the offline compile "
                "evidence digests"
            )
    return None


def grx010_real_pass_enablement_issue(
    evidence: dict[str, object] | None = None,
) -> str | None:
    """First concrete missing prerequisite for the GRX-010 tonemap real-pass
    enablement gate, or None.

    The gate only accepts a REAL measured strict success: a status=success
    historical artifact where the opt-in real dispatch actually executed and
    completed (``RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS`` marker line pinned and
    the patch 0013 writeback scaffold marker observed on the candidate leg),
    the raw RGB8 frame artifacts exist on disk with matching SHA-256 digests, a
    fresh recompute of |reference - candidate| matches the recorded diff and
    sits within the pinned LDR thresholds, the forced-failure red leg fell back
    with ``unsupported_device``, the 0001..0013 patch-stack identity and scratch
    source provenance audits hold, the runtime log audit is clean, and no
    performance/FPS/GPU-timestamp claim exists. Even such a success keeps
    default_enable_state=disabled and performance_claim=none."""
    if not GRX010_REAL_PASS_ENABLEMENT_SCHEMA.exists():
        return "tonemap real_pass_enablement_evidence.schema.json is missing"
    if evidence is None:
        evidence = grx010_real_pass_enablement_success_evidence()
    if not isinstance(evidence, dict):
        return (
            "historical measured success artifact "
            "real_pass_enablement_success_evidence.json is missing; the tonemap "
            "opt-in real-pass gate has not measured a strict success"
        )
    status = normalize_string(evidence.get("status"))
    if status != "success":
        return (
            "tonemap real-pass enablement success evidence status is "
            f"{status or 'missing'}, not success (a SKIP/FAIL run is never ready)"
        )
    if evidence.get("subject") != GRX010_REAL_PASS_SUBJECT:
        return (
            "success evidence subject is not "
            "grx010_tonemap_real_pass_enablement_smoke"
        )
    if evidence.get("pass_id") != "tonemap":
        return "success evidence pass_id is not tonemap"
    if normalize_string(evidence.get("segment")) != GRX010_REAL_PASS_SEGMENT:
        return "success evidence segment is not grx010_real_pass_enablement"
    if evidence.get("evidence_kind") != GRX010_SUCCESS_EVIDENCE_KIND:
        return "success evidence evidence_kind is not historical_measured_success"
    if (
        normalize_string(evidence.get("latest_evidence_path"))
        != GRX010_LATEST_EVIDENCE_REL_PATH
    ):
        return "success evidence latest_evidence_path is wrong"
    if evidence.get("runtime_state") != "fallback_only":
        return "success evidence does not keep runtime_state=fallback_only"
    if evidence.get("godot_runtime_tonemap_path_enabled") is not False:
        return (
            "success evidence does not keep "
            "godot_runtime_tonemap_path_enabled=false"
        )
    if evidence.get("default_enable_state") != "disabled":
        return "success evidence does not keep default_enable_state=disabled"
    if evidence.get("gpu_timestamp_status") != "not_yet":
        return "success evidence does not keep gpu_timestamp_status=not_yet"
    if normalize_string(evidence.get("performance_claim")) != "none":
        return (
            "success evidence performance_claim is not 'none'; no performance or "
            "FPS improvement claim is allowed at this gate"
        )
    if (
        normalize_string(evidence.get("expected_first_missing_prerequisite"))
        != GRX010_EXPECTED_FIRST_MISSING_PREREQUISITE
    ):
        return "success evidence does not pin the expected first missing prerequisite"
    if evidence.get("real_gpu_pass") is not True:
        return "success evidence does not record real_gpu_pass=true"
    if evidence.get("real_d3d12_dispatch_recorded") is not True:
        return "success evidence does not record real_d3d12_dispatch_recorded=true"
    marker_line = normalize_string(evidence.get("real_pass_marker_line"))
    if not marker_line or GRX010_REAL_PASS_MARKER not in marker_line:
        return (
            "success evidence does not pin the observed "
            f"{GRX010_REAL_PASS_MARKER} marker line"
        )

    checks = evidence.get("checks")
    if not isinstance(checks, dict) or any(
        checks.get(name) is not True for name in GRX010_REQUIRED_TRUE_CHECKS
    ):
        return "success evidence checks are not all green"
    if any(checks.get(name) is not False for name in GRX010_REQUIRED_FALSE_CHECKS):
        return (
            "success evidence records a candidate fallback/blocked marker check; "
            "the success outcome is contradictory"
        )

    matrix = evidence.get("pass_enable_matrix")
    if not isinstance(matrix, dict):
        return "success evidence is missing the pass_enable_matrix"
    reference_leg = matrix.get("disabled_default")
    candidate_leg = matrix.get("enabled_real_pass_optin")
    forced_leg = matrix.get("forced_capability_downgrade")
    if not all(
        isinstance(leg, dict) for leg in (reference_leg, candidate_leg, forced_leg)
    ):
        return (
            "pass_enable_matrix must record the disabled_default, "
            "enabled_real_pass_optin, and forced_capability_downgrade legs"
        )
    for name, leg in (
        ("disabled_default", reference_leg),
        ("enabled_real_pass_optin", candidate_leg),
        ("forced_capability_downgrade", forced_leg),
    ):
        if leg.get("exit_code") != 0:
            return f"pass_enable_matrix {name} leg did not exit 0"
        if leg.get("session_ready") is not True:
            return f"pass_enable_matrix {name} leg did not observe a ready session"
        if leg.get("record_marker_observed") is not False:
            return f"pass_enable_matrix {name} leg observed the recording marker"
    for marker_key in (
        "bridge_fallback_marker_observed",
        "real_pass_blocked_marker_observed",
        "real_pass_marker_observed",
        "writeback_marker_observed",
    ):
        if reference_leg.get(marker_key) is not False:
            return (
                "disabled_default leg unexpectedly observed a bridge marker; "
                "the disabled pass must never invoke the bridge"
            )
    if candidate_leg.get("real_pass_marker_observed") is not True:
        return "enabled_real_pass_optin leg did not observe the real-pass marker"
    if candidate_leg.get("writeback_marker_observed") is not True:
        return (
            "enabled_real_pass_optin leg did not observe the patch 0013 "
            "writeback scaffold marker"
        )
    if candidate_leg.get("real_pass_blocked_marker_observed") is not False:
        return (
            "enabled_real_pass_optin leg observed the blocked diagnostic; the "
            "success outcome is contradictory"
        )
    if candidate_leg.get("bridge_fallback_marker_observed") is not False:
        return (
            "enabled_real_pass_optin leg observed the fallback marker; the "
            "success outcome is contradictory"
        )
    if forced_leg.get("bridge_fallback_marker_observed") is not True:
        return (
            "forced_capability_downgrade leg did not observe the fallback "
            "marker; the forced-failure red leg was not measured"
        )
    if forced_leg.get("real_pass_blocked_marker_observed") is not True:
        return (
            "forced_capability_downgrade leg did not observe the blocked "
            "diagnostic"
        )
    if forced_leg.get("real_pass_marker_observed") is not False:
        return "forced_capability_downgrade leg observed the real-pass marker"
    if forced_leg.get("writeback_marker_observed") is not False:
        return "forced_capability_downgrade leg observed the writeback marker"

    visual = evidence.get("visual")
    if not isinstance(visual, dict):
        return "success evidence is missing the visual section"
    if visual.get("measured_local") is not True:
        return "visual evidence is not measured_local=true"
    if visual.get("metric_kind") != GRX009_SEGMENT4G_METRIC_KIND:
        return f"visual metric_kind is not {GRX009_SEGMENT4G_METRIC_KIND}"
    if visual.get("format") != GRX009_SEGMENT4G_FRAME_FORMAT:
        return f"visual frame format is not {GRX009_SEGMENT4G_FRAME_FORMAT}"
    width = visual.get("width")
    height = visual.get("height")
    if not _grx009_is_frame_dimension(width) or not _grx009_is_frame_dimension(height):
        return (
            "visual width/height are malformed or below the "
            f"{GRX009_SEGMENT4G_MIN_FRAME_DIMENSION}px minimum"
        )
    capture_frame_index = visual.get("capture_frame_index")
    if (
        not isinstance(capture_frame_index, int)
        or isinstance(capture_frame_index, bool)
        or capture_frame_index < 1
    ):
        return "visual capture_frame_index is missing or malformed"
    if visual.get("max_abs_diff_threshold") != GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD:
        return "recorded max_abs_diff_threshold does not equal the pinned gate threshold"
    if (
        visual.get("mean_abs_diff_threshold")
        != GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD
    ):
        return "recorded mean_abs_diff_threshold does not equal the pinned gate threshold"
    expected_size = width * height * 3

    frame_bytes: dict[str, bytes] = {}
    for key, path in (
        ("reference_frame", GRX010_VISUAL_REAL_PASS_REFERENCE_FRAME),
        ("candidate_frame", GRX010_VISUAL_REAL_PASS_CANDIDATE_FRAME),
        ("diff_artifact", GRX010_VISUAL_REAL_PASS_DIFF_ARTIFACT),
    ):
        entry = visual.get(key)
        if not isinstance(entry, dict):
            return f"visual {key} entry is missing"
        if not path.is_file():
            return f"visual {key} artifact is missing on disk ({path.name})"
        actual_sha = sha256_of_file(path)
        if actual_sha is None or normalize_string(entry.get("sha256")) != actual_sha:
            return f"visual {key} sha256 does not match the on-disk artifact"
        try:
            raw = path.read_bytes()
        except OSError:
            return f"visual {key} artifact is unreadable ({path.name})"
        if len(raw) != expected_size:
            return (
                f"visual {key} size {len(raw)} does not equal width*height*3="
                f"{expected_size} (malformed dimensions)"
            )
        frame_bytes[key] = raw

    computed_diff = bytes(
        abs(a - b)
        for a, b in zip(frame_bytes["reference_frame"], frame_bytes["candidate_frame"])
    )
    if computed_diff != frame_bytes["diff_artifact"]:
        return (
            "diff artifact bytes do not equal the recomputed "
            "|reference - candidate| LDR diff"
        )
    computed_max = max(computed_diff) if computed_diff else 0
    computed_mean = (sum(computed_diff) / len(computed_diff)) if computed_diff else 0.0
    diffs = visual.get("diffs")
    if not isinstance(diffs, dict):
        return "visual diffs section is missing"
    candidate_diff = diffs.get("candidate")
    forced_diff = diffs.get("forced_fallback")
    if not isinstance(candidate_diff, dict) or not isinstance(forced_diff, dict):
        return "visual diffs must record candidate and forced_fallback entries"
    recorded_max = candidate_diff.get("max_abs_diff")
    if (
        not isinstance(recorded_max, (int, float))
        or isinstance(recorded_max, bool)
        or float(recorded_max) != float(computed_max)
    ):
        return "recorded candidate max_abs_diff does not match the recomputed diff"
    recorded_mean = candidate_diff.get("mean_abs_diff")
    if (
        not isinstance(recorded_mean, (int, float))
        or isinstance(recorded_mean, bool)
        or abs(float(recorded_mean) - computed_mean) > GRX009_SEGMENT4G_MEAN_ABS_EPSILON
    ):
        return "recorded candidate mean_abs_diff does not match the recomputed diff"
    if (
        computed_max > GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD
        or computed_mean > GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD
    ):
        return (
            "measured LDR absolute diff exceeds the pinned visual gate threshold "
            f"(max_abs={computed_max}, mean_abs={computed_mean:.6f})"
        )
    for name, entry in (("candidate", candidate_diff), ("forced_fallback", forced_diff)):
        if entry.get("within_threshold") is not True:
            return f"visual diffs {name} within_threshold is not true"

    telemetry = evidence.get("fallback_telemetry")
    if not isinstance(telemetry, dict):
        return "success evidence is missing the fallback_telemetry section"
    if telemetry.get("no_fps_claim") is not True:
        return "fallback telemetry does not record no_fps_claim=true"
    telemetry_entry = telemetry.get("telemetry_document")
    if not isinstance(telemetry_entry, dict):
        return "fallback telemetry is missing the telemetry_document fingerprint"
    if not GRX010_REAL_PASS_ENABLEMENT_TELEMETRY.is_file():
        return "tonemap real_pass_enablement_telemetry.json is missing on disk"
    telemetry_sha = sha256_of_file(GRX010_REAL_PASS_ENABLEMENT_TELEMETRY)
    if (
        telemetry_sha is None
        or normalize_string(telemetry_entry.get("sha256")) != telemetry_sha
    ):
        return (
            "real_pass_enablement_telemetry.json sha256 does not match the "
            "recorded fingerprint"
        )
    telemetry_doc = load_json_file(GRX010_REAL_PASS_ENABLEMENT_TELEMETRY)
    if telemetry_doc is None:
        return "real_pass_enablement_telemetry.json is unreadable"
    if telemetry_doc.get("evidence_level") != "measured_local":
        return "real-pass enablement telemetry is not evidence_level=measured_local"
    passes = telemetry_doc.get("passes")
    forced_entry = None
    if isinstance(passes, list):
        for entry in passes:
            if not isinstance(entry, dict):
                continue
            if entry.get("leg") == "enabled_real_pass_optin":
                return (
                    "telemetry document carries a candidate fallback entry "
                    "although the success evidence claims a real pass; the "
                    "outcome is contradictory"
                )
            if entry.get("leg") == "forced_capability_downgrade":
                forced_entry = entry
    if forced_entry is None:
        return "telemetry document has no forced_capability_downgrade entry"
    if forced_entry.get("pass_id") != "tonemap":
        return "forced telemetry entry pass_id is not tonemap"
    if normalize_string(forced_entry.get("enable_state")) != "enabled":
        return "forced telemetry entry does not record enable_state=enabled"
    if normalize_string(forced_entry.get("fallback_reason")) != "unsupported_device":
        return "forced telemetry entry fallback_reason is not unsupported_device"
    if forced_entry.get("godot_fallback_active") is not True:
        return "forced telemetry entry does not record godot_fallback_active=true"
    telemetry_frame = forced_entry.get("telemetry_frame")
    if (
        not isinstance(telemetry_frame, int)
        or isinstance(telemetry_frame, bool)
        or telemetry_frame != capture_frame_index
    ):
        return (
            "forced telemetry entry telemetry_frame is stale: it does not "
            "equal the visual capture_frame_index"
        )
    if (
        _bench_script_exit_code(
            GRX008_FALLBACK_TELEMETRY_SCRIPT,
            ["--validate-only", str(GRX010_REAL_PASS_ENABLEMENT_TELEMETRY)],
        )
        != 0
    ):
        return (
            "real_pass_enablement_telemetry.json does not pass "
            "fallback_telemetry.py --validate-only"
        )

    if evidence.get("artifact_hashes_match_offline_evidence") is not True:
        return (
            "success evidence does not record "
            "artifact_hashes_match_offline_evidence=true"
        )
    artifact_issue = _grx010_enablement_artifact_digests_issue(evidence)
    if artifact_issue is not None:
        return artifact_issue

    exe_fp = evidence.get("godot_exe_fingerprint")
    if not isinstance(exe_fp, dict):
        return "success evidence godot_exe_fingerprint is missing"
    if not _is_sha256_hex(exe_fp.get("exe_sha256")):
        return "success evidence godot_exe_fingerprint sha256 is malformed"
    if not _is_positive_int(exe_fp.get("exe_size_bytes")):
        return "success evidence godot_exe_fingerprint size is malformed"
    if exe_fp.get("committed") is not False:
        return "success evidence godot_exe_fingerprint must record committed=false"

    dll_fp = evidence.get("dll_fingerprint")
    if not isinstance(dll_fp, dict):
        return "success evidence dll_fingerprint is missing"
    if not _is_sha256_hex(dll_fp.get("dll_sha256")):
        return "success evidence dll_fingerprint sha256 is malformed"
    dll_features = dll_fp.get("features")
    # The tonemap real-pass arm's linked real dispatch path is compiled ONLY
    # under the d3d12-recording-shim feature (the shipping feature-off bridge
    # stays fail-closed), so the strict success must record exactly that build.
    if not isinstance(dll_features, list) or dll_features != [
        GRX009_SEGMENT4F_RECORDING_SHIM_FEATURE
    ]:
        return (
            "success evidence dll_fingerprint must record the "
            "d3d12-recording-shim bridge build (the only build with the linked "
            "real dispatch path)"
        )

    if not grx009_segment4f_patch_stack_identity_ok(
        evidence.get("patch_stack_identity"),
        GRX010_PATCH_STACK_FILES,
        GRX010_PATCH_STACK_ID,
    ):
        return (
            "success evidence patch_stack_identity does not match the tracked "
            f"{GRX010_PATCH_STACK_ID} patch stack"
        )
    if not grx009_segment4f_scratch_source_provenance_ok(
        evidence,
        GRX010_PATCH_STACK_FILES,
        GRX010_PATCH_STACK_ID,
    ):
        return (
            "success evidence scratch source provenance does not pass the "
            f"{GRX010_PATCH_STACK_ID} tracked-patch-stack-only audit"
        )

    log_issue = grx009_segment4h_runtime_log_audit_issue(evidence)
    if log_issue is not None:
        return log_issue
    return None


def grx010_real_pass_measured_success_active() -> bool:
    """Stage-A5 fail-closed switch: True only when the tonemap real-pass
    ``real_pass_enablement_success_evidence.json`` exists AND passes the full
    strict audit. A missing artifact, a SKIP/FAIL document, or a hand-edited
    placeholder never activates the relaxed manifest acceptance
    (implemented=true, real_gpu_pass=true,
    runtime_state=fallback_only_by_default_real_pass_optin_measured)."""
    path = GRX010_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE
    if not path.exists():
        return False
    key = str(path)
    cached = _GRX010_REAL_PASS_SUCCESS_AUDIT_CACHE.get(key)
    if cached is None:
        cached = grx010_real_pass_enablement_issue() is None
        _GRX010_REAL_PASS_SUCCESS_AUDIT_CACHE[key] = cached
    return cached


def grx010_real_pass_success_evidence_conflict() -> bool:
    """True when a tonemap real-pass success artifact exists but FAILS the
    strict audit (placeholder/tampered document); fail-closed rejection."""
    return (
        GRX010_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE.exists()
        and not grx010_real_pass_measured_success_active()
    )


def grx010_manifest_implemented_ok(manifest: dict[str, object]) -> bool:
    """implemented=false is always accepted; implemented=true only under the
    audited tonemap real-pass measured success (fail-closed otherwise)."""
    value = manifest.get("implemented")
    if value is False:
        return True
    return value is True and grx010_real_pass_measured_success_active()


def grx010_manifest_runtime_state_ok(
    implementation_status: dict[str, object],
) -> bool:
    """runtime_state=fallback_only is always accepted; the stage-A5
    fallback_only_by_default_real_pass_optin_measured value only under the
    audited measured success (fail-closed otherwise)."""
    runtime_state = implementation_status.get("runtime_state")
    if runtime_state == "fallback_only":
        return True
    return (
        runtime_state == GRX010_MANIFEST_OPTIN_MEASURED_RUNTIME_STATE
        and grx010_real_pass_measured_success_active()
    )


def grx010_manifest_real_gpu_pass_ok(
    implementation_status: dict[str, object],
) -> bool:
    """real_gpu_pass=false is always accepted; true only under the audited
    stage-A5 measured success (fail-closed otherwise)."""
    value = implementation_status.get("real_gpu_pass")
    if value is False:
        return True
    return value is True and grx010_real_pass_measured_success_active()


def grx010_manifest_dispatch_recorded_ok(
    implementation_status: dict[str, object],
) -> bool:
    """real_d3d12_dispatch_recorded=false is always accepted; true only under
    the audited stage-A5 measured success (fail-closed otherwise)."""
    value = implementation_status.get("real_d3d12_dispatch_recorded")
    if value is False:
        return True
    return value is True and grx010_real_pass_measured_success_active()


GRX010_DEFAULT_ENABLE_DECISION_DOC_SECTIONS = [
    "## Owner Decision",
    "## Rationale",
    "## Re-evaluation Conditions",
    "## Fail-Closed Invariants",
]


def grx010_real_pass_default_enable_decision_evidence() -> dict[str, object] | None:
    return load_json_file(GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE)


def grx010_real_pass_default_enable_decision_status(
    evidence: dict[str, object] | None = None,
) -> str:
    candidate = (
        evidence
        if evidence is not None
        else grx010_real_pass_default_enable_decision_evidence()
    )
    if not isinstance(candidate, dict):
        return "missing"
    return normalize_string(candidate.get("status")) or "malformed"


def grx010_real_pass_default_enable_decision_issue(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
) -> str | None:
    """First missing prerequisite of the tonemap owner default-enable decision
    gate, or None. Mirror of the GRX-009 segment 4m gate: ready only when the
    tonemap real-pass strict measured success is active AND the owner decision
    evidence records keep_default_disabled with a signed owner block, the
    rationale/re-evaluation lists, the decision document with its required
    sections, performance_claim=none, and the manifest keeps
    default_enable_state=disabled while referencing the decision files from the
    real_pass_measured_success block."""
    if not grx010_real_pass_measured_success_active():
        return "real_pass_measured_success_not_active"
    candidate = (
        evidence
        if evidence is not None
        else grx010_real_pass_default_enable_decision_evidence()
    )
    if not isinstance(candidate, dict):
        return "default_enable_decision_evidence_missing"
    if candidate.get("pass_id") != "tonemap":
        return "default_enable_decision_pass_id_mismatch"
    if normalize_string(candidate.get("segment")) != GRX010_DECISION_SEGMENT:
        return "default_enable_decision_segment_mismatch"
    if normalize_string(candidate.get("status")) != "success":
        return "default_enable_decision_status_must_be_success"
    if candidate.get("decision_ready") is not True:
        return "default_enable_decision_ready_must_be_true"
    if normalize_string(candidate.get("default_enable_state")) != "disabled":
        return "default_enable_decision_state_must_be_disabled"
    if normalize_string(candidate.get("performance_claim")) != "none":
        return "default_enable_decision_performance_claim_must_be_none"
    owner_decision = candidate.get("owner_decision")
    if not isinstance(owner_decision, dict):
        return "default_enable_decision_owner_block_missing"
    if normalize_string(owner_decision.get("decision")) != (
        GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION
    ):
        return "default_enable_decision_owner_decision_mismatch"
    if not normalize_string(owner_decision.get("approved_by")):
        return "default_enable_decision_owner_approved_by_missing"
    if not normalize_string(owner_decision.get("machine_role")):
        return "default_enable_decision_owner_machine_role_missing"
    rationale = candidate.get("decision_rationale")
    if not isinstance(rationale, list) or not rationale:
        return "default_enable_decision_rationale_missing"
    reevaluate = candidate.get("reevaluate_when")
    if not isinstance(reevaluate, list) or not reevaluate:
        return "default_enable_decision_reevaluate_conditions_missing"
    prerequisite = candidate.get("prerequisite_evidence")
    if not isinstance(prerequisite, dict):
        return "default_enable_decision_prerequisite_evidence_missing"
    success_path = grx009_repo_path(
        normalize_string(prerequisite.get("real_pass_enablement_success"))
    )
    if success_path != GRX010_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE:
        return "default_enable_decision_prerequisite_success_path_mismatch"
    decision_doc_path = grx009_repo_path(
        normalize_string(candidate.get("decision_document"))
    )
    if decision_doc_path != GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION_DOC:
        return "default_enable_decision_document_path_mismatch"
    if not GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION_DOC.is_file():
        return "default_enable_decision_document_missing"
    if not file_contains_all(
        GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION_DOC,
        GRX010_DEFAULT_ENABLE_DECISION_DOC_SECTIONS,
    ):
        return "default_enable_decision_document_required_sections_missing"
    if normalize_string(candidate.get("next_action_if_ready")) != GRX011_NEXT_ACTION:
        return "default_enable_decision_next_action_mismatch"
    manifest_doc = manifest if manifest is not None else grx010_manifest()
    if not isinstance(manifest_doc, dict):
        return "manifest_missing"
    if manifest_doc.get("default_enable_state") != "disabled":
        return "manifest_default_enable_state_must_remain_disabled"
    implementation_status = manifest_doc.get("implementation_status")
    if not isinstance(implementation_status, dict):
        return "manifest_implementation_status_missing"
    measured_success = implementation_status.get("real_pass_measured_success")
    if not isinstance(measured_success, dict):
        return "manifest_measured_success_block_missing"
    if normalize_string(measured_success.get("status")) != "success":
        return "manifest_measured_success_status_mismatch"
    if normalize_string(measured_success.get("decision")) != (
        GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION
    ):
        return "manifest_measured_success_decision_mismatch"
    if grx009_repo_path(
        normalize_string(measured_success.get("default_enable_decision"))
    ) != GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE:
        return "manifest_measured_success_decision_path_mismatch"
    if grx009_repo_path(
        normalize_string(measured_success.get("evidence"))
    ) != GRX010_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE:
        return "manifest_measured_success_evidence_path_mismatch"
    if normalize_string(measured_success.get("next_action_when_ready")) != (
        GRX011_NEXT_ACTION
    ):
        return "manifest_measured_success_next_action_mismatch"
    if normalize_string(measured_success.get("performance_claim")) != "none":
        return "manifest_measured_success_performance_claim_must_be_none"
    return None


def grx010_real_pass_default_enable_decision_ready(
    evidence: dict[str, object] | None = None,
    manifest: dict[str, object] | None = None,
) -> bool:
    return (
        grx010_real_pass_default_enable_decision_issue(evidence, manifest) is None
    )


def grx010_patch_0012_applyability_result() -> dict[str, object]:
    """0012 stacks on 0004..0011 (0004..0010 forward-applicable + tonemap
    0011), checked in a temporary scratch copy so the snapshot working tree is
    never modified."""
    return evaluate_stacked_patch_applyability(
        ROOT,
        EXTERNAL_GODOT,
        [
            GRX009_PATCH_0004,
            GRX009_PATCH_0005,
            GRX009_PATCH_0006,
            GRX009_PATCH_0007,
            GRX009_PATCH_0008,
            GRX009_PATCH_0009,
            GRX009_PATCH_0010,
            GRX010_PATCH_0011,
        ],
        GRX010_PATCH_0012,
        "0012",
    )


def grx010_patch_0012_applyable(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx010_patch_0012_applyability_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx010_patch_0013_applyability_result() -> dict[str, object]:
    """0013 stacks on 0004..0012."""
    return evaluate_stacked_patch_applyability(
        ROOT,
        EXTERNAL_GODOT,
        [
            GRX009_PATCH_0004,
            GRX009_PATCH_0005,
            GRX009_PATCH_0006,
            GRX009_PATCH_0007,
            GRX009_PATCH_0008,
            GRX009_PATCH_0009,
            GRX009_PATCH_0010,
            GRX010_PATCH_0011,
            GRX010_PATCH_0012,
        ],
        GRX010_PATCH_0013,
        "0013",
    )


def grx010_patch_0013_applyable(result: dict[str, object] | None = None) -> bool:
    candidate = result or grx010_patch_0013_applyability_result()
    return candidate.get("ok") is True and candidate.get("ready") is True


def grx009_manifest() -> dict[str, object] | None:
    return load_json_file(GRX009_PASS_MANIFEST)


def grx009_manifest_implementation_status(
    manifest: dict[str, object],
) -> dict[str, object] | None:
    implementation_status = manifest.get("implementation_status")
    return implementation_status if isinstance(implementation_status, dict) else None


def command_has_required_scons_args(command: object) -> bool:
    candidate = normalize_string(command)
    if not candidate:
        return False
    return all(arg in candidate for arg in REQUIRED_SCONS_ARGS)


def build_summary_primary_command(build_summary: dict[str, object] | None) -> str | None:
    if not isinstance(build_summary, dict):
        return None
    direct_command = normalize_string(build_summary.get("command"))
    if direct_command:
        return direct_command
    attempts = build_summary.get("attempts")
    if not isinstance(attempts, list):
        return None
    successful_command = None
    fallback_command = None
    for attempt in attempts:
        if not isinstance(attempt, dict):
            continue
        command = normalize_string(attempt.get("command"))
        if not command:
            continue
        fallback_command = command
        if attempt.get("exit_code") == 0:
            successful_command = command
            break
    return successful_command or fallback_command


def build_summary_required_scons_args_satisfied(
    build_summary: dict[str, object] | None,
) -> bool:
    if not isinstance(build_summary, dict):
        return False
    explicit_ready = build_summary.get("required_scons_args_satisfied")
    if isinstance(explicit_ready, bool):
        return explicit_ready
    explicit_path_ready = build_summary.get("path_overrides_ready")
    if isinstance(explicit_path_ready, bool):
        return explicit_path_ready
    primary_command = build_summary_primary_command(build_summary)
    ice_workaround_command = normalize_string(build_summary.get("ice_workaround_command"))
    return command_has_required_scons_args(
        primary_command
    ) and command_has_required_scons_args(ice_workaround_command)


def build_summary_has_required_artifacts(build_summary: dict[str, object] | None) -> bool:
    if not isinstance(build_summary, dict):
        return False
    if build_summary.get("artifacts_complete") is not True:
        return False
    artifacts = build_summary.get("artifacts")
    if not isinstance(artifacts, dict):
        return False
    for key in REQUIRED_BUILD_ARTIFACT_KEYS:
        artifact = artifacts.get(key)
        if (
            not isinstance(artifact, dict)
            or artifact.get("exists") is not True
            or normalize_string(artifact.get("path")) is None
            or artifact.get("size_bytes") is None
            or normalize_string(artifact.get("mtime_utc")) is None
            or normalize_string(artifact.get("sha256")) is None
        ):
            return False
    return True


def load_smoke_summary_is_success(load_smoke_summary: dict[str, object] | None) -> bool:
    if not isinstance(load_smoke_summary, dict):
        return False
    return load_smoke_summary.get("status") == "success"


def bench_scenes_ready(bench_smoke_summary: dict[str, object] | None) -> bool:
    if not isinstance(bench_smoke_summary, dict):
        return False
    return (
        bench_smoke_summary.get("status") == "success"
        and bench_smoke_summary.get("scene_count") == 7
        and bench_smoke_summary.get("failure_count") == 0
    )


def bench_runner_ready(bench_runner_summary: dict[str, object] | None) -> bool:
    if not isinstance(bench_runner_summary, dict):
        return False
    return (
        bench_runner_summary.get("status") == "success"
        and bench_runner_summary.get("scene_count") == 7
        and bench_runner_summary.get("failure_count") == 0
    )


def _perf_gate_exit_code(args: list[str]) -> int | None:
    """Run perf_gate.py with args (cwd=ROOT). Returns exit code, or None on error."""
    try:
        proc = subprocess.run(
            [sys.executable, str(GRX006_PERF_GATE_SCRIPT), *args],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
            timeout=PROBE_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    return proc.returncode


def grx006_schema_ready() -> bool:
    """GRX-006 schema/sample validation is available and the gate scripts work.

    Evidence-based: every tracked GRX-006 schema and base sample file must exist
    and parse as JSON, AND the GRX-006 red/green sample commands must behave as
    expected (green sample validates, red samples fail). This is not
    "file exists = done"; unparseable files or a broken gate script keep the
    readiness false so the probe does not advance to GRX-007 prematurely.
    """
    for path in GRX006_SCHEMA_SAMPLE_FILES:
        if not path.exists():
            return False
        try:
            json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return False

    if not GRX006_PERF_GATE_SCRIPT.exists():
        return False
    for sample in (
        GRX006_BASELINE_SMOKE_SAMPLE,
        GRX006_FORBIDDEN_SKIP_SAMPLE,
        GRX006_MISSING_SAMPLE_COUNT_SAMPLE,
    ):
        if not sample.exists():
            return False

    # Green: baseline smoke sample validates (exit 0).
    green = _perf_gate_exit_code(
        ["--kind", "baseline", "--validate-only", str(GRX006_BASELINE_SMOKE_SAMPLE)]
    )
    if green != 0:
        return False
    # Red: forbidden SKIP marker under --strict must fail (non-zero).
    strict_red = _perf_gate_exit_code(["--strict", str(GRX006_FORBIDDEN_SKIP_SAMPLE)])
    if strict_red is None or strict_red == 0:
        return False
    # Red: missing sample_count under baseline validation must fail (non-zero).
    baseline_red = _perf_gate_exit_code(
        [
            "--kind",
            "baseline",
            "--validate-only",
            str(GRX006_MISSING_SAMPLE_COUNT_SAMPLE),
        ]
    )
    if baseline_red is None or baseline_red == 0:
        return False
    return True


def _bench_script_exit_code(script: pathlib.Path, args: list[str]) -> int | None:
    """Run a bench script with args (cwd=ROOT). Returns exit code, or None on error."""
    try:
        proc = subprocess.run(
            [sys.executable, str(script), *args],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
            timeout=PROBE_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    return proc.returncode


def grx007_visual_ready() -> bool:
    """GRX-007 visual diff scaffold/hardening is available and red/green behave.

    Evidence-based: the visual_diff.py script, schema, and tracked samples must
    exist and parse as JSON, AND the visual diff red/green sample commands must
    behave as expected (placeholder + matching LDR pass validate; missing ldr,
    mismatch, and skip-with-fake-ldr fail). This is not "file exists = done".
    """
    for path in (
        GRX007_VISUAL_DIFF_SCRIPT,
        GRX007_VISUAL_SCHEMA,
        GRX007_VISUAL_PLACEHOLDER_SAMPLE,
        GRX007_VISUAL_LDR_PASS_SAMPLE,
        GRX007_VISUAL_MISSING_LDR_SAMPLE,
        GRX007_VISUAL_MISMATCH_SAMPLE,
        GRX007_VISUAL_SKIP_FAKE_LDR_SAMPLE,
        GRX007_VISUAL_MISSING_FRAME_ARTIFACT_SAMPLE,
    ):
        if not path.exists():
            return False
    for json_path in (
        GRX007_VISUAL_SCHEMA,
        GRX007_VISUAL_PLACEHOLDER_SAMPLE,
        GRX007_VISUAL_LDR_PASS_SAMPLE,
        GRX007_VISUAL_MISSING_LDR_SAMPLE,
        GRX007_VISUAL_MISMATCH_SAMPLE,
        GRX007_VISUAL_SKIP_FAKE_LDR_SAMPLE,
        GRX007_VISUAL_MISSING_FRAME_ARTIFACT_SAMPLE,
    ):
        try:
            json.loads(json_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return False

    # Green: placeholder validates (exit 0).
    if _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT,
        ["--validate-only", str(GRX007_VISUAL_PLACEHOLDER_SAMPLE)],
    ) != 0:
        return False
    # Green: recorded ldr_diff matches the computed diff (exit 0).
    if _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT, [str(GRX007_VISUAL_LDR_PASS_SAMPLE)]
    ) != 0:
        return False
    # Red: status=pass missing ldr_diff must FORMAT FAIL (non-zero).
    missing_red = _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT,
        ["--validate-only", str(GRX007_VISUAL_MISSING_LDR_SAMPLE)],
    )
    if missing_red is None or missing_red == 0:
        return False
    # Red: recorded ldr_diff mismatched computed diff must DIFF FAIL (non-zero).
    mismatch_red = _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT, [str(GRX007_VISUAL_MISMATCH_SAMPLE)]
    )
    if mismatch_red is None or mismatch_red == 0:
        return False
    # Red: skip frame carrying a fabricated ldr_diff must FORMAT FAIL (non-zero).
    skip_fake_red = _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT,
        ["--validate-only", str(GRX007_VISUAL_SKIP_FAKE_LDR_SAMPLE)],
    )
    if skip_fake_red is None or skip_fake_red == 0:
        return False
    # Red: status=pass frame whose reference/candidate artifacts are missing on
    # disk must DIFF FAIL (non-zero); it must not be downgraded to SKIP.
    missing_frame_red = _bench_script_exit_code(
        GRX007_VISUAL_DIFF_SCRIPT,
        [str(GRX007_VISUAL_MISSING_FRAME_ARTIFACT_SAMPLE)],
    )
    if missing_frame_red is None or missing_frame_red == 0:
        return False
    return True


def grx008_telemetry_ready() -> bool:
    """GRX-008 fallback telemetry scaffold/hardening is available and red/green behave.

    Evidence-based: the fallback_telemetry.py script, schema, and tracked samples
    must exist and parse as JSON, AND the red/green sample commands must behave as
    expected (scaffold placeholder validates; full/measured_local with null
    timestamp and scaffold with inactive fallback both fail). This is not
    "file exists = done".
    """
    for path in (
        GRX008_FALLBACK_TELEMETRY_SCRIPT,
        GRX008_FALLBACK_SCHEMA,
        GRX008_FALLBACK_PLACEHOLDER_SAMPLE,
        GRX008_FALLBACK_FULL_NULL_TIMESTAMP_SAMPLE,
        GRX008_FALLBACK_SCAFFOLD_INACTIVE_SAMPLE,
    ):
        if not path.exists():
            return False
    for json_path in (
        GRX008_FALLBACK_SCHEMA,
        GRX008_FALLBACK_PLACEHOLDER_SAMPLE,
        GRX008_FALLBACK_FULL_NULL_TIMESTAMP_SAMPLE,
        GRX008_FALLBACK_SCAFFOLD_INACTIVE_SAMPLE,
    ):
        try:
            json.loads(json_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return False

    # Green: scaffold placeholder validates (exit 0).
    if _bench_script_exit_code(
        GRX008_FALLBACK_TELEMETRY_SCRIPT,
        ["--validate-only", str(GRX008_FALLBACK_PLACEHOLDER_SAMPLE)],
    ) != 0:
        return False
    # Red: full/measured_local with null timestamp/frame must FORMAT FAIL.
    full_null_red = _bench_script_exit_code(
        GRX008_FALLBACK_TELEMETRY_SCRIPT,
        ["--validate-only", str(GRX008_FALLBACK_FULL_NULL_TIMESTAMP_SAMPLE)],
    )
    if full_null_red is None or full_null_red == 0:
        return False
    # Red: scaffold with godot_fallback_active=false must FORMAT FAIL.
    scaffold_inactive_red = _bench_script_exit_code(
        GRX008_FALLBACK_TELEMETRY_SCRIPT,
        ["--validate-only", str(GRX008_FALLBACK_SCAFFOLD_INACTIVE_SAMPLE)],
    )
    if scaffold_inactive_red is None or scaffold_inactive_red == 0:
        return False
    return True


def grx009_manifest_godot_files(manifest: dict[str, object]) -> list[str] | None:
    """Collect the Godot-relative files recorded by the GRX-009 manifest.

    Returns the header/source/shader/call-site file paths recorded under
    godot_hook_investigation, or None when any of them is missing, empty, or
    not a string. The paths are investigation records only; nothing here
    mutates external/godot-master.
    """
    investigation = manifest.get("godot_hook_investigation")
    if not isinstance(investigation, dict):
        return None
    files: list[str] = []
    effect_class = investigation.get("effect_class")
    if not isinstance(effect_class, dict):
        return None
    for key in ("header", "source"):
        value = normalize_string(effect_class.get(key))
        if not value:
            return None
        files.append(value)
    shaders = investigation.get("shaders")
    if not isinstance(shaders, list) or not shaders:
        return None
    for shader in shaders:
        value = normalize_string(shader)
        if not value:
            return None
        files.append(value)
    call_sites = investigation.get("call_sites")
    if not isinstance(call_sites, list) or not call_sites:
        return None
    for call_site in call_sites:
        if not isinstance(call_site, dict):
            return None
        value = normalize_string(call_site.get("file"))
        if not value:
            return None
        files.append(value)
    return files


def grx009_manifest_godot_files_exist(manifest: dict[str, object]) -> bool:
    """Every manifest-recorded Godot file must exist under external/godot-master.

    Read-only check: each recorded header/source/shader/call-site path must be
    a relative path that resolves to an existing file inside the ignored Godot
    snapshot. Absolute paths or paths escaping the snapshot root fail.
    """
    files = grx009_manifest_godot_files(manifest)
    if files is None:
        return False
    try:
        external_root = EXTERNAL_GODOT.resolve()
    except OSError:
        return False
    for rel in files:
        if pathlib.PurePath(rel).is_absolute():
            return False
        candidate = EXTERNAL_GODOT / rel
        try:
            resolved = candidate.resolve()
        except OSError:
            return False
        if not resolved.is_relative_to(external_root):
            return False
        if not candidate.is_file():
            return False
    return True


def grx009_prep_ready() -> bool:
    """GRX-009 luminance reduction pass preparation artifacts are present.

    Evidence-based: the PASS_CONTRACT.md and pass_manifest.json preparation
    artifacts must exist, the manifest must parse as JSON, it must declare the
    default-disabled luminance_reduction pass (pass_id, implemented=false,
    default disabled, target scenes post_fx_chain + mixed_forward_plus), and
    every Godot source/header/shader/call-site file recorded in
    godot_hook_investigation must exist under external/godot-master (checked
    read-only; the snapshot is never modified). Readiness here means the
    preparation record is real; it does NOT mean any acceleration pass is
    implemented.
    """
    if not GRX009_PASS_CONTRACT.exists() or not GRX009_PASS_MANIFEST.exists():
        return False
    manifest = grx009_manifest()
    if manifest is None:
        return False
    if manifest.get("pass_id") != "luminance_reduction":
        return False
    if not grx009_manifest_implemented_ok(manifest):
        return False
    if manifest.get("default_enable_state") != "disabled":
        return False
    target_scenes = manifest.get("target_scenes")
    if not isinstance(target_scenes, list):
        return False
    if "post_fx_chain" not in target_scenes or "mixed_forward_plus" not in target_scenes:
        return False
    return grx009_manifest_godot_files_exist(manifest)


def grx009_segment1_ready() -> bool:
    """GRX-009 segment 1 gated scaffold artifacts are present and coherent.

    Evidence-based: the prep artifacts must already be valid, the manifest must
    still declare implementation_status.segment == 1 with
    godot_core_call_site_wired == false and real_gpu_pass == false, the 0002
    module patch must carry the expected luminance gate markers, the disabled
    telemetry sample must exist and parse, and the Rust bridge must still carry
    the LuminanceReductionGate marker. This remains a historical gate once the
    manifest advances to segment 2.
    """
    if not grx009_prep_ready():
        return False
    manifest = grx009_manifest()
    if manifest is None:
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    if implementation_status.get("segment") != 1:
        return False
    if not grx009_manifest_real_gpu_pass_ok(implementation_status):
        return False
    if implementation_status.get("godot_core_call_site_wired") is not False:
        return False
    if not file_contains_all(
        GRX009_PATCH_0002,
        [
            "rendering/rurix_accel/passes/luminance_reduction/enabled",
            "RXGD_PASS_LUMINANCE_REDUCTION",
            "rxgd_record_pass",
            "try_record_luminance_reduction",
        ],
    ):
        return False
    if not validate_fallback_telemetry_sample(GRX009_DISABLED_TELEMETRY_SAMPLE):
        return False
    if not file_contains_all(GRX009_BRIDGE_LIB, ["LuminanceReductionGate"]):
        return False
    return True


def grx009_segment2_ready(patch_stack_result: dict[str, object] | None = None) -> bool:
    """GRX-009 segment 2 core call-site fallback wiring is present and coherent.

    Evidence-based: the prep artifacts must already be valid, the manifest must
    declare implementation_status.segment == 2 with
    godot_core_call_site_wired == true and real_gpu_pass == false, the segment 1
    bridge/module markers must still exist, the new 0003 core call-site patch
    must carry the expected D3D12Hooks + renderer_scene_render_rd wiring
    markers, and the segment-2 scaffold telemetry sample must exist and parse.
    """
    if not grx009_prep_ready():
        return False
    manifest = grx009_manifest()
    if manifest is None:
        return False
    implementation_status = grx009_manifest_implementation_status(manifest)
    if implementation_status is None:
        return False
    if implementation_status.get("segment") != 2:
        return False
    if not grx009_manifest_real_gpu_pass_ok(implementation_status):
        return False
    if implementation_status.get("godot_core_call_site_wired") is not True:
        return False
    if not file_contains_all(
        GRX009_PATCH_0002,
        [
            "rendering/rurix_accel/passes/luminance_reduction/enabled",
            "RXGD_PASS_LUMINANCE_REDUCTION",
            "rxgd_record_pass",
            "try_record_luminance_reduction",
        ],
    ):
        return False
    if not file_contains_all(GRX009_BRIDGE_LIB, ["LuminanceReductionGate"]):
        return False
    if not grx009_patch_stack_ready(patch_stack_result):
        return False
    if not file_contains_all(
        GRX009_PATCH_0003,
        [
            "drivers/d3d12/d3d12_hooks.h",
            "renderer_scene_render_rd.cpp",
            "D3D12Hooks::get_singleton",
            "try_record_luminance_reduction",
            "override",
        ],
    ):
        return False
    if not validate_fallback_telemetry_sample(GRX009_CALLSITE_WIRED_TELEMETRY_SAMPLE):
        return False
    return True


def run_probe(name: str, cmd: list[str], *, ok_status: str = "PASS") -> ProbeResult:
    details: dict[str, object] = {
        "command": " ".join(cmd),
        "timeout_seconds": PROBE_TIMEOUT_SECONDS,
    }
    try:
        proc = subprocess.run(
            cmd,
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
            timeout=PROBE_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as exc:
        details["timed_out"] = True
        details["output"] = timeout_output(exc)
        return ProbeResult(name, "SKIP", timeout_output(exc), details)
    except FileNotFoundError as exc:
        return ProbeResult(name, "SKIP", f"command not found: {exc.filename or cmd[0]}", {})

    output = completed_output(proc)
    if proc.returncode == 0:
        reason = output or "command succeeded"
        return ProbeResult(name, ok_status, reason, details)
    details["exit_code"] = proc.returncode
    return ProbeResult(
        name,
        "SKIP",
        output or f"command failed with exit code {proc.returncode}",
        details,
    )


def find_paths_from_env(paths: str | None) -> list[pathlib.Path]:
    if not paths:
        return []
    found: list[pathlib.Path] = []
    for item in paths.split(os.pathsep):
        item = item.strip().strip('"')
        if item:
            found.append(pathlib.Path(item))
    return found


def newest_subdir(root: pathlib.Path) -> pathlib.Path | None:
    if not root.exists():
        return None
    subdirs = sorted(path for path in root.iterdir() if path.is_dir())
    return subdirs[-1] if subdirs else None


def find_windows_sdk_versions(root: pathlib.Path) -> list[pathlib.Path]:
    if not root.exists():
        return []
    return sorted((path for path in root.iterdir() if path.is_dir()), reverse=True)


def probe_godot_tree() -> list[ProbeResult]:
    results: list[ProbeResult] = []
    if EXTERNAL_GODOT.exists():
        results.append(
            ProbeResult(
                "godot_snapshot",
                "PASS",
                "external/godot-master exists",
                {"path": str(EXTERNAL_GODOT)},
            )
        )
    else:
        results.append(
            ProbeResult(
                "godot_snapshot",
                "FAIL",
                "external/godot-master is missing",
                {"path": str(EXTERNAL_GODOT)},
            )
        )

    if SCONSTRUCT.exists():
        results.append(
            ProbeResult(
                "godot_sconstruct",
                "PASS",
                "SConstruct exists",
                {"path": str(SCONSTRUCT)},
            )
        )
    else:
        results.append(
            ProbeResult(
                "godot_sconstruct",
                "FAIL",
                "SConstruct is missing",
                {"path": str(SCONSTRUCT)},
            )
        )
    return results


def probe_vs_build_tools() -> ProbeResult:
    details: dict[str, object] = {}
    env_install = os.environ.get("VSINSTALLDIR")
    if env_install:
        details["VSINSTALLDIR"] = env_install

    if VSWHERE.exists():
        details["vswhere"] = str(VSWHERE)
        details["timeout_seconds"] = PROBE_TIMEOUT_SECONDS
        try:
            proc = subprocess.run(
                [
                    str(VSWHERE),
                    "-products",
                    "*",
                    "-requires",
                    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                    "-format",
                    "json",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
                timeout=PROBE_TIMEOUT_SECONDS,
            )
        except subprocess.TimeoutExpired as exc:
            details["timed_out"] = True
            details["output"] = timeout_output(exc)
            return ProbeResult("vs_build_tools", "SKIP", timeout_output(exc), details)
        output = completed_output(proc)
        if proc.returncode != 0:
            return ProbeResult(
                "vs_build_tools",
                "SKIP",
                output or f"vswhere failed with exit code {proc.returncode}",
                details,
            )
        try:
            installs = json.loads(proc.stdout or "[]")
        except json.JSONDecodeError:
            installs = []
        if not installs:
            return ProbeResult(
                "vs_build_tools",
                "SKIP",
                "vswhere did not find a Visual Studio installation with VC tools",
                details,
            )

        install_records: list[dict[str, object]] = []
        for install_record in installs[:8]:
            if not isinstance(install_record, dict):
                continue
            record: dict[str, object] = {}
            for key in (
                "displayName",
                "installationName",
                "installationPath",
                "installationVersion",
                "productId",
                "isPrerelease",
            ):
                value = install_record.get(key)
                if value not in (None, ""):
                    record[key] = value
            if record:
                install_records.append(record)
        if install_records:
            details["installations"] = install_records

        selected_install = next(
            (
                install_record
                for install_record in installs
                if isinstance(install_record, dict)
                and normalize_string(install_record.get("installationPath"))
            ),
            None,
        )
        if not isinstance(selected_install, dict):
            return ProbeResult(
                "vs_build_tools",
                "SKIP",
                "vswhere did not return a usable installationPath",
                details,
            )

        install_path = pathlib.Path(str(selected_install["installationPath"]))
        details["selected_installation_path"] = str(install_path)
        details["installation_path"] = str(install_path)
        for key in ("displayName", "installationName", "installationVersion", "productId"):
            value = selected_install.get(key)
            if value not in (None, ""):
                details[f"selected_{key}"] = value
        vcvarsall = install_path / VCVARSALL_REL
        if vcvarsall.exists():
            details["vcvarsall_bat"] = str(vcvarsall)
        msvc_toolset = newest_subdir(install_path / "VC" / "Tools" / "MSVC")
        if msvc_toolset:
            details["msvc_toolset"] = str(msvc_toolset)
            candidate_cl = msvc_toolset / "bin" / "Hostx64" / "x64" / "cl.exe"
            if candidate_cl.exists():
                details["selected_candidate_cl"] = str(candidate_cl)
                details["candidate_cl"] = str(candidate_cl)
        return ProbeResult(
            "vs_build_tools",
            "PASS",
            "Visual Studio/Build Tools with VC tools were found via vswhere",
            details,
        )

    if env_install:
        install_path = pathlib.Path(env_install)
        vcvarsall = install_path / VCVARSALL_REL
        if vcvarsall.exists():
            details["vcvarsall_bat"] = str(vcvarsall)
        details["selected_installation_path"] = str(install_path)
        details["installation_path"] = str(install_path)
        msvc_toolset = newest_subdir(install_path / "VC" / "Tools" / "MSVC")
        if msvc_toolset:
            details["msvc_toolset"] = str(msvc_toolset)
            candidate_cl = msvc_toolset / "bin" / "Hostx64" / "x64" / "cl.exe"
            if candidate_cl.exists():
                details["selected_candidate_cl"] = str(candidate_cl)
                details["candidate_cl"] = str(candidate_cl)
        return ProbeResult(
            "vs_build_tools",
            "PASS",
            "VSINSTALLDIR is set; assuming this shell comes from a Visual Studio installation",
            details,
        )

    return ProbeResult(
        "vs_build_tools",
        "SKIP",
        "vswhere.exe was not found and VSINSTALLDIR is not set",
        details,
    )


def probe_msvc(vs_probe: ProbeResult) -> ProbeResult:
    details = collect_msvc_shell_evidence()
    for key in ("vswhere", "installation_path", "vcvarsall_bat", "msvc_toolset", "candidate_cl"):
        value = vs_probe.details.get(key)
        if value:
            details[key] = value
    compiler_path = normalize_string(details.get("compiler_path"))
    if compiler_path:
        details["path"] = compiler_path
        output = normalize_string(details.get("cl_bv"))
        return ProbeResult("msvc_cl", "PASS", output or "cl is available", details)

    if details.get("candidate_cl"):
        return ProbeResult(
            "msvc_cl",
            "SKIP",
            "cl is not available on PATH; launch a Developer PowerShell or call vcvarsall.bat first",
            details,
        )

    return ProbeResult(
        "msvc_cl",
        "SKIP",
        "cl is not available on PATH and no usable VC toolset was discovered",
        details,
    )


def probe_msvc_via_vcvarsall(vs_probe: ProbeResult) -> ProbeResult:
    vcvarsall = vs_probe.details.get("vcvarsall_bat")
    if isinstance(vcvarsall, str) and vcvarsall:
        details = collect_msvc_shell_evidence(vcvarsall)
        command = f'cmd.exe /d /s /c "call "{vcvarsall}" x64 >nul && where cl && cl /Bv"'
        details["command"] = command
        compiler_path = normalize_string(details.get("compiler_path"))
        if compiler_path:
            details["path"] = compiler_path
            return ProbeResult(
                "msvc_cl_via_vcvarsall",
                "PASS",
                "cl is available when wrapped with vcvarsall.bat",
                details | {"activation_output": details.get("cl_bv", "cl invocation succeeded")},
            )
        return ProbeResult(
            "msvc_cl_via_vcvarsall",
            "SKIP",
            normalize_string(details.get("where_cl_error"))
            or normalize_string(details.get("cl_bv"))
            or "vcvarsall activation did not expose cl.exe",
            details,
        )

    return ProbeResult(
        "msvc_cl_via_vcvarsall",
        "SKIP",
        "vcvarsall.bat was not discovered",
        details,
    )


def probe_headers() -> ProbeResult:
    include_roots = find_paths_from_env(os.environ.get("INCLUDE"))
    sdk_dir = os.environ.get("WindowsSdkDir")
    sdk_version = os.environ.get("WindowsSdkVersion", "").strip("\\/")
    details: dict[str, object] = {}

    if sdk_dir:
        details["WindowsSdkDir"] = sdk_dir
    if sdk_version:
        details["WindowsSdkVersion"] = sdk_version

    header_hits: dict[str, str] = {}
    search_roots: list[pathlib.Path] = []
    search_roots.extend(include_roots)

    if sdk_dir and sdk_version:
        search_roots.append(pathlib.Path(sdk_dir) / "Include" / sdk_version / "um")
        search_roots.append(pathlib.Path(sdk_dir) / "Include" / sdk_version / "shared")

    for root in DEFAULT_SDK_INCLUDE_ROOTS:
        for version_dir in find_windows_sdk_versions(root):
            search_roots.append(version_dir / "um")
            search_roots.append(version_dir / "shared")

    deduped_roots: list[pathlib.Path] = []
    seen: set[str] = set()
    for root in search_roots:
        key = str(root)
        if key not in seen:
            seen.add(key)
            deduped_roots.append(root)

    for header in HEADER_CANDIDATES:
        for root in deduped_roots:
            candidate = root / header
            if candidate.exists():
                header_hits[header] = str(candidate)
                break

    details.update(header_hits)
    if all(header in header_hits for header in HEADER_CANDIDATES):
        return ProbeResult(
            "windows_sdk_d3d12_headers",
            "PASS",
            "required Windows SDK D3D12 headers were found",
            details,
        )

    missing = [header for header in HEADER_CANDIDATES if header not in header_hits]
    details["searched_roots"] = [str(path) for path in deduped_roots[:12]]
    return ProbeResult(
        "windows_sdk_d3d12_headers",
        "SKIP",
        f"missing headers: {', '.join(missing)}",
        details,
    )


def probe_tool_path(tool_name: str) -> ProbeResult:
    on_path = shutil.which(tool_name)
    details: dict[str, object] = {}
    if on_path:
        return ProbeResult(
            tool_name.lower().removesuffix(".exe"),
            "PASS",
            f"{tool_name} found on PATH",
            {"path": on_path},
        )

    for root in DEFAULT_SDK_BIN_ROOTS:
        if not root.exists():
            continue
        for version_dir in find_windows_sdk_versions(root):
            for arch in ("x64", "x86"):
                candidate = version_dir / arch / tool_name
                if candidate.exists():
                    details["path"] = str(candidate)
                    return ProbeResult(
                        tool_name.lower().removesuffix(".exe"),
                        "PASS",
                        f"{tool_name} found in Windows SDK bin",
                        details,
                    )

    return ProbeResult(
        tool_name.lower().removesuffix(".exe"),
        "SKIP",
        f"{tool_name} was not found on PATH or common Windows SDK bin paths",
        details,
    )


def probe_rurix_godot_dll() -> ProbeResult:
    if RURIX_GODOT_DLL.exists():
        return ProbeResult(
            "rurix_godot_dll",
            "PASS",
            "target/debug/rurix_godot.dll exists",
            {"path": str(RURIX_GODOT_DLL)},
        )
    return ProbeResult(
        "rurix_godot_dll",
        "SKIP",
        "target/debug/rurix_godot.dll is missing; actual buildability is verified by cargo build later",
        {"path": str(RURIX_GODOT_DLL)},
    )


def render_command(parts: list[str]) -> str:
    return subprocess.list2cmdline(parts)


def render_godot_local_command(parts: list[str]) -> str:
    base = render_command(parts)
    return f"set LOCALAPPDATA={LOCAL_GODOT_LOCALAPPDATA} && {base}"


def wrap_with_localappdata(cmd: str) -> str:
    return f"$env:LOCALAPPDATA='{LOCAL_GODOT_LOCALAPPDATA}'; {cmd}"


def first_match(root: pathlib.Path, pattern: str) -> pathlib.Path | None:
    if not root.exists():
        return None
    return next(root.glob(pattern), None)


def first_recursive_match(root: pathlib.Path, pattern: str) -> pathlib.Path | None:
    if not root.exists():
        return None
    return next(root.rglob(pattern), None)


def probe_godot_accesskit_deps() -> ProbeResult:
    accesskit_root = LOCAL_GODOT_BUILD_DEPS / "accesskit"
    include_dir = accesskit_root / "include"
    lib_dir = (
        accesskit_root
        / "lib"
        / "windows"
        / ACCESSKIT_WINDOWS_ARCH
        / "msvc"
        / "static"
    )
    accesskit_lib = first_match(lib_dir, "accesskit*.lib")
    details: dict[str, object] = {
        "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
        "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "accesskit_sdk_path": str(accesskit_root),
        "include_dir": str(include_dir),
        "lib_dir": str(lib_dir),
        "arch": GODOT_WINDOWS_ARCH,
        "recommended_install_command": render_godot_local_command(
            ["py", "-3", str(GODOT_INSTALL_ACCESSKIT)]
        ),
    }
    if include_dir.exists() and accesskit_lib is not None:
        details["library"] = str(accesskit_lib)
        return ProbeResult(
            "godot_accesskit_deps",
            "PASS",
            "workspace-local AccessKit SDK was found",
            details,
        )
    return ProbeResult(
        "godot_accesskit_deps",
        "SKIP",
        "workspace-local AccessKit SDK is missing",
        details,
    )


def probe_godot_d3d12_deps() -> ProbeResult:
    mesa_arch_root = LOCAL_GODOT_BUILD_DEPS / f"mesa-{GODOT_WINDOWS_ARCH}-msvc"
    mesa_fallback_root = LOCAL_GODOT_BUILD_DEPS / "mesa"
    details: dict[str, object] = {
        "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
        "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "mesa_arch_path": str(mesa_arch_root),
        "mesa_fallback_path": str(mesa_fallback_root),
        "arch": GODOT_WINDOWS_ARCH,
        "recommended_install_command": render_godot_local_command(
            ["py", "-3", str(GODOT_INSTALL_D3D12_DEPS)]
        ),
    }

    candidates = [
        mesa_arch_root / "bin",
        mesa_fallback_root / "bin",
    ]
    for bin_dir in candidates:
        if not bin_dir.exists():
            continue
        libnir = first_match(bin_dir, f"libNIR.windows.{GODOT_WINDOWS_ARCH}*")
        if libnir is None:
            libnir = first_match(bin_dir, "libNIR.windows.*")
        if libnir is not None:
            details["bin_dir"] = str(bin_dir)
            details["libnir"] = str(libnir)
            return ProbeResult(
                "godot_d3d12_deps",
                "PASS",
                "workspace-local Mesa/NIR D3D12 build deps were found",
                details,
            )

    return ProbeResult(
        "godot_d3d12_deps",
        "SKIP",
        "workspace-local Mesa/NIR D3D12 build deps are missing",
        details,
    )


def probe_godot_agility_sdk() -> ProbeResult:
    agility_root = LOCAL_GODOT_BUILD_DEPS / "agility_sdk"
    expected_dir = agility_root / "build" / "native" / "bin" / AGILITY_WINDOWS_ARCH
    d3d12core = expected_dir / "D3D12Core.dll"
    sdk_layers = expected_dir / "d3d12SDKLayers.dll"
    details: dict[str, object] = {
        "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
        "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "agility_sdk_path": str(agility_root),
        "arch": AGILITY_WINDOWS_ARCH,
        "expected_dir": str(expected_dir),
    }
    mismatched_hits: dict[str, dict[str, str]] = {}
    for arch_name in ("x64", "arm64", "win32"):
        if arch_name == AGILITY_WINDOWS_ARCH:
            continue
        arch_dir = agility_root / "build" / "native" / "bin" / arch_name
        arch_hits: dict[str, str] = {}
        core_candidate = arch_dir / "D3D12Core.dll"
        layers_candidate = arch_dir / "d3d12SDKLayers.dll"
        if core_candidate.exists():
            arch_hits["D3D12Core.dll"] = str(core_candidate)
        if layers_candidate.exists():
            arch_hits["D3D12SDKLayers.dll"] = str(layers_candidate)
        if arch_hits:
            mismatched_hits[str(arch_dir)] = arch_hits
    if mismatched_hits:
        details["mismatched_arch_hits"] = mismatched_hits
    if d3d12core.exists():
        details["D3D12Core.dll"] = str(d3d12core)
    if sdk_layers.exists():
        details["D3D12SDKLayers.dll"] = str(sdk_layers)
    if d3d12core.exists():
        return ProbeResult(
            "godot_agility_sdk",
            "PASS",
            "workspace-local Agility SDK was found for the current architecture",
            details,
        )
    return ProbeResult(
        "godot_agility_sdk",
        "SKIP",
        "workspace-local Agility SDK was not found for the current architecture",
        details,
    )


def probe_godot_pix_runtime() -> ProbeResult:
    pix_root = LOCAL_GODOT_BUILD_DEPS / "pix"
    expected_dir = pix_root / "bin" / PIX_WINDOWS_ARCH
    pix_runtime = expected_dir / "WinPixEventRuntime.dll"
    details: dict[str, object] = {
        "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
        "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "pix_path": str(pix_root),
        "arch": PIX_WINDOWS_ARCH,
        "expected_dir": str(expected_dir),
    }
    mismatched_hits: dict[str, dict[str, str]] = {}
    for arch_name in ("x64", "ARM64"):
        if arch_name == PIX_WINDOWS_ARCH:
            continue
        arch_dir = pix_root / "bin" / arch_name
        runtime_candidate = arch_dir / "WinPixEventRuntime.dll"
        if runtime_candidate.exists():
            mismatched_hits[str(arch_dir)] = {"WinPixEventRuntime.dll": str(runtime_candidate)}
    if mismatched_hits:
        details["mismatched_arch_hits"] = mismatched_hits
    if pix_runtime.exists():
        details["WinPixEventRuntime.dll"] = str(pix_runtime)
        return ProbeResult(
            "godot_pix_runtime",
            "PASS",
            "workspace-local PIX runtime was found for the current architecture",
            details,
        )
    return ProbeResult(
        "godot_pix_runtime",
        "SKIP",
        "workspace-local PIX runtime was not found for the current architecture",
        details,
    )


def preferred_scons_info(by_name: dict[str, dict[str, object]]) -> tuple[str | None, str]:
    if by_name.get("scons_cli", {}).get("status") == "PASS":
        return "scons", "existing"
    if by_name.get("python_scons", {}).get("status") == "PASS":
        return render_command(["py", "-3", "-m", "SCons"]), "existing"
    if by_name.get("local_python_scons", {}).get("status") == "PASS":
        return render_command([str(LOCAL_SCONS_PYTHON), "-m", "SCons"]), "workspace-local venv"
    return None, "unavailable"


def shell_wrap_with_vcvars(cmd: str, by_name: dict[str, dict[str, object]]) -> str:
    vcvarsall = by_name.get("vs_build_tools", {}).get("details", {}).get("vcvarsall_bat")
    if not isinstance(vcvarsall, str) or not vcvarsall:
        return cmd
    if by_name.get("msvc_cl", {}).get("status") == "PASS":
        return cmd
    return f'& $env:ComSpec /c \'call "{vcvarsall}" x64 && {cmd}\''


def summarize(results: list[ProbeResult]) -> dict[str, object]:
    by_name = {result.name: asdict(result) for result in results}
    dxil_toolchain_preflight = build_dxil_toolchain_preflight()
    dxil_llc = dxil_toolchain_preflight["rurix_llc"]
    dxil_validator_suite = dxil_toolchain_preflight["signed_dxc_validator_suite"]
    build_summary = load_json_report(BUILD_SUMMARY_REPORT)
    load_smoke_summary = load_json_report(LOAD_SMOKE_SUMMARY_REPORT)
    bench_smoke_summary = load_json_report(BENCH_SMOKE_SUMMARY_REPORT)
    bench_runner_summary = load_json_report(BENCH_RUNNER_SUMMARY_REPORT)
    build_summary_required_args_satisfied = build_summary_required_scons_args_satisfied(
        build_summary
    )
    build_summary_status = (
        normalize_string(build_summary.get("status")) if isinstance(build_summary, dict) else None
    )
    build_summary_primary_cmd = build_summary_primary_command(build_summary)
    build_summary_ice_cmd = normalize_string(
        build_summary.get("ice_workaround_command") if isinstance(build_summary, dict) else None
    )
    build_artifacts_ready = build_summary_has_required_artifacts(
        build_summary
    ) and build_summary_required_args_satisfied
    load_smoke_ready = load_smoke_summary_is_success(load_smoke_summary)
    scenes_ready = bench_scenes_ready(bench_smoke_summary)
    runner_ready = bench_runner_ready(bench_runner_summary)
    grx006_ready = grx006_schema_ready()
    grx007_ready = grx007_visual_ready()
    grx008_ready = grx008_telemetry_ready()
    grx009_ready = grx009_prep_ready()
    grx009_segment1 = grx009_segment1_ready()
    grx009_patch_stack = grx009_patch_stack_result()
    grx009_patch_stack_state = normalize_string(grx009_patch_stack.get("state"))
    grx009_patch_stack_reason = normalize_string(grx009_patch_stack.get("reason"))
    grx009_patch_stack_is_ready = grx009_patch_stack_ready(grx009_patch_stack)
    grx009_segment2 = grx009_segment2_ready(grx009_patch_stack)
    grx009_compile = grx009_compile_evidence()
    grx009_compile_status = (
        normalize_string(grx009_compile.get("status"))
        if isinstance(grx009_compile, dict)
        else None
    )
    grx009_compile_blocker_category = (
        normalize_string(grx009_compile.get("blocker_category"))
        if isinstance(grx009_compile, dict)
        else None
    )
    grx009_compile_blocker_summary = (
        normalize_string(grx009_compile.get("blocker_summary"))
        if isinstance(grx009_compile, dict)
        else None
    )
    # GRX-009 stage A3: the tracked canonical package is the owner-approved
    # texture-capable hlsl_bridge workaround artifact, so the bridge tracked
    # kernel now declares per-slot texture binding kinds
    # (LUMINANCE_KERNEL_RESOURCE_BINDING_KINDS = ["texture2d", "rwtexture2d"];
    # the scalar constant reports slot 0), matching the Godot runtime
    # Texture2D ID3D12Resource* handles. attempted_binding_kinds from the
    # canonical evidence surfaces the binding kinds the canonical package
    # targets; math_parity_status records the level-0 CPU-proven /
    # pending-GPU parity state that the bridge's check_real_pass_math_parity
    # gates on. The historical raw-buffer kernel lives only at
    # artifacts/raw_buffer_historical/.
    grx009_luminance_kernel_binding_kind = "texture2d"
    grx009_luminance_math_parity_status = (
        normalize_string(grx009_compile.get("math_parity_status"))
        if isinstance(grx009_compile, dict)
        else None
    )
    grx009_luminance_offline_binding_kinds = (
        grx009_compile.get("attempted_binding_kinds")
        if isinstance(grx009_compile, dict)
        and isinstance(grx009_compile.get("attempted_binding_kinds"), list)
        else None
    )
    grx009_manifest_data = grx009_manifest()
    grx009_compile_manifest_consistency_warning = (
        grx009_compile_manifest_consistency_issue(grx009_manifest_data, grx009_compile)
        if isinstance(grx009_manifest_data, dict) and isinstance(grx009_compile, dict)
        else None
    )
    grx009_segment3a = grx009_segment3a_compile_ready()
    grx009_patch_0004_applyability = grx009_patch_0004_applyability_result()
    grx009_patch_0004_applyable_ready = grx009_patch_0004_applyable(
        grx009_patch_0004_applyability
    )
    grx009_segment3b_inputs = grx009_segment3b_resource_mapping_inputs_ready()
    grx009_segment3b = grx009_segment3b_inputs and grx009_patch_0004_applyable_ready
    grx009_patch_0005_applyability = grx009_patch_0005_applyability_result()
    grx009_patch_0005_applyable_ready = grx009_patch_0005_applyable(
        grx009_patch_0005_applyability
    )
    grx009_segment4a_inputs = grx009_segment4a_runtime_binding_preflight_inputs_ready()
    grx009_segment4a = grx009_segment4a_inputs and grx009_patch_0005_applyable_ready
    grx009_patch_0006_applyability = grx009_patch_0006_applyability_result()
    grx009_patch_0006_applyable_ready = grx009_patch_0006_applyable(
        grx009_patch_0006_applyability
    )
    grx009_segment4b_inputs = grx009_segment4b_gated_dispatch_bringup_inputs_ready()
    grx009_segment4b = grx009_segment4b_inputs and grx009_patch_0006_applyable_ready
    grx009_dispatch_smoke_evidence = grx009_real_d3d12_dispatch_smoke_evidence()
    grx009_dispatch_smoke_status = (
        normalize_string(grx009_dispatch_smoke_evidence.get("status"))
        if isinstance(grx009_dispatch_smoke_evidence, dict)
        else None
    )
    grx009_real_d3d12_dispatch_smoke = grx009_real_d3d12_dispatch_smoke_ready(
        grx009_dispatch_smoke_evidence, grx009_segment4b
    )
    grx009_bridge_recording_evidence_doc = grx009_bridge_recording_evidence()
    grx009_bridge_recording_status = (
        normalize_string(grx009_bridge_recording_evidence_doc.get("status"))
        if isinstance(grx009_bridge_recording_evidence_doc, dict)
        else None
    )
    grx009_bridge_real_d3d12_dispatch_recording = (
        grx009_bridge_real_d3d12_dispatch_recording_ready(
            grx009_bridge_recording_evidence_doc, grx009_real_d3d12_dispatch_smoke
        )
    )
    grx009_patch_0007_applyability = grx009_patch_0007_applyability_result()
    grx009_patch_0007_applyable_ready = grx009_patch_0007_applyable(
        grx009_patch_0007_applyability
    )
    grx009_segment4e_inputs = (
        grx009_segment4e_native_resource_handle_mapping_inputs_ready()
    )
    grx009_segment4e = grx009_segment4e_native_resource_handle_mapping_ready(
        grx009_segment4e_inputs,
        grx009_patch_0007_applyability,
        grx009_bridge_real_d3d12_dispatch_recording,
    )
    grx009_patch_0008_applyability = grx009_patch_0008_applyability_result()
    grx009_patch_0008_applyable_ready = grx009_patch_0008_applyable(
        grx009_patch_0008_applyability
    )
    grx009_segment4f_inputs = grx009_segment4f_inputs_ready()
    # Latest runtime smoke evidence: reproducible-default SKIP when the scratch
    # Godot exe env var is absent. Reported for visibility only; it does not
    # advance the readiness gate on its own.
    grx009_godot_runtime_recording_evidence_doc = (
        grx009_godot_runtime_recording_evidence()
    )
    grx009_godot_runtime_recording_status = (
        normalize_string(grx009_godot_runtime_recording_evidence_doc.get("status"))
        if isinstance(grx009_godot_runtime_recording_evidence_doc, dict)
        else None
    )
    # Historical measured success artifact: only present after a strict
    # status=success run, never overwritten by a later SKIP/FAIL. The segment 4f
    # readiness gate advances off THIS file.
    grx009_godot_runtime_recording_success_evidence_doc = (
        grx009_godot_runtime_recording_success_evidence()
    )
    grx009_godot_runtime_recording_success_status = (
        normalize_string(
            grx009_godot_runtime_recording_success_evidence_doc.get("status")
        )
        if isinstance(grx009_godot_runtime_recording_success_evidence_doc, dict)
        else None
    )
    grx009_segment4f = grx009_segment4f_godot_runtime_bridge_recording_ready(
        grx009_godot_runtime_recording_success_evidence_doc,
        grx009_segment4e,
    )
    grx009_segment4f_issue = (
        grx009_segment4f_godot_runtime_bridge_recording_issue(
            grx009_godot_runtime_recording_success_evidence_doc,
            grx009_segment4e,
        )
        if not grx009_segment4f
        else None
    )
    if not grx009_segment4f and not grx009_segment4f_issue:
        grx009_segment4f_issue = (
            "segment 4f readiness is false but no specific issue was reported; "
            "update grx009_segment4f_godot_runtime_bridge_recording_issue coverage"
        )
    # Segment 4g: latest evidence is a reproducible-default SKIP without the
    # tracked Godot exe; the readiness gate advances off the historical
    # measured success artifact only.
    grx009_visual_fallback_latest_doc = grx009_segment4g_visual_fallback_evidence()
    grx009_visual_fallback_latest_status = (
        normalize_string(grx009_visual_fallback_latest_doc.get("status"))
        if isinstance(grx009_visual_fallback_latest_doc, dict)
        else None
    )
    grx009_visual_fallback_success_doc = (
        grx009_segment4g_visual_fallback_success_evidence()
    )
    grx009_visual_fallback_success_status = (
        normalize_string(grx009_visual_fallback_success_doc.get("status"))
        if isinstance(grx009_visual_fallback_success_doc, dict)
        else None
    )
    grx009_segment4g_issue = grx009_segment4g_visual_fallback_issue(
        grx009_visual_fallback_success_doc,
        grx009_segment4f,
    )
    grx009_segment4g = grx009_segment4g_visual_fallback_ready(
        grx009_visual_fallback_success_doc,
        grx009_segment4f,
    )
    # Segment 4h: latest evidence is a reproducible-default SKIP without the
    # 0001..0009 scratch exe; a completed measured run records the first
    # missing prerequisite (skip_kind=measured_prerequisite_blocked). The
    # readiness gate advances only off the historical measured success
    # artifact, which is unreachable with the tracked segment 3a artifact.
    grx009_patch_0009_applyability = grx009_patch_0009_applyability_result()
    grx009_patch_0009_applyable_ready = grx009_patch_0009_applyable(
        grx009_patch_0009_applyability
    )
    grx009_real_pass_latest_doc = grx009_segment4h_real_pass_enablement_evidence()
    grx009_real_pass_latest_status = (
        normalize_string(grx009_real_pass_latest_doc.get("status"))
        if isinstance(grx009_real_pass_latest_doc, dict)
        else None
    )
    grx009_real_pass_latest_skip_kind = (
        normalize_string(grx009_real_pass_latest_doc.get("skip_kind"))
        if isinstance(grx009_real_pass_latest_doc, dict)
        else None
    )
    grx009_real_pass_first_missing_prerequisite = (
        normalize_string(
            grx009_real_pass_latest_doc.get("first_missing_prerequisite")
        )
        if isinstance(grx009_real_pass_latest_doc, dict)
        else None
    )
    grx009_real_pass_latest_issue = grx009_segment4h_latest_evidence_hash_chain_issue(
        grx009_real_pass_latest_doc
    )
    grx009_real_pass_success_doc = (
        grx009_segment4h_real_pass_enablement_success_evidence()
    )
    grx009_real_pass_success_status = (
        normalize_string(grx009_real_pass_success_doc.get("status"))
        if isinstance(grx009_real_pass_success_doc, dict)
        else None
    )
    grx009_segment4h_issue = grx009_segment4h_real_pass_enablement_issue(
        grx009_real_pass_success_doc
    )
    grx009_segment4h = grx009_segment4h_real_pass_enablement_ready(
        grx009_real_pass_success_doc,
        grx009_segment4g,
    )
    grx009_default_enable_decision_doc = (
        grx009_real_pass_default_enable_decision_evidence()
    )
    grx009_default_enable_decision_status_value = (
        grx009_real_pass_default_enable_decision_status(
            grx009_default_enable_decision_doc
        )
    )
    grx009_default_enable_decision_issue_value = (
        grx009_real_pass_default_enable_decision_issue(
            grx009_default_enable_decision_doc,
            grx009_manifest_data,
        )
    )
    grx009_default_enable_decision_ready_value = (
        grx009_default_enable_decision_issue_value is None
    )
    # GRX-010 tonemap gates (segment A): only consulted once the GRX-009
    # chain (incl. the owner default-enable decision) is fully ready.
    grx010_patch_0011_applyability = grx010_patch_0011_applyability_result()
    grx010_patch_0011_applyable_ready = grx010_patch_0011_applyable(
        grx010_patch_0011_applyability
    )
    grx010_tonemap_contract_issue_value = grx010_tonemap_contract_issue()
    grx010_tonemap_contract_ready_value = grx010_tonemap_contract_issue_value is None
    grx010_dispatch_smoke_status_value = grx010_tonemap_d3d12_dispatch_smoke_status()
    grx010_dispatch_smoke_issue_value = grx010_tonemap_d3d12_dispatch_smoke_issue()
    grx010_dispatch_smoke_ready_value = grx010_dispatch_smoke_issue_value is None
    # GRX-010 stage-A5 close-out gates: the tonemap runtime-binding (0012) and
    # recording-smoke/real-pass-optin (0013) patches, the opt-in real-pass
    # measured success, and the owner default-enable decision.
    grx010_manifest_data = grx010_manifest()
    grx010_patch_0012_applyability = grx010_patch_0012_applyability_result()
    grx010_patch_0012_applyable_ready = grx010_patch_0012_applyable(
        grx010_patch_0012_applyability
    )
    grx010_patch_0013_applyability = grx010_patch_0013_applyability_result()
    grx010_patch_0013_applyable_ready = grx010_patch_0013_applyable(
        grx010_patch_0013_applyability
    )
    grx010_real_pass_success_doc = grx010_real_pass_enablement_success_evidence()
    grx010_real_pass_success_status = (
        normalize_string(grx010_real_pass_success_doc.get("status"))
        if isinstance(grx010_real_pass_success_doc, dict)
        else None
    )
    grx010_real_pass_enablement_issue_value = grx010_real_pass_enablement_issue(
        grx010_real_pass_success_doc
    )
    grx010_real_pass_enablement_ready_value = (
        grx010_real_pass_enablement_issue_value is None
    )
    grx010_default_enable_decision_doc = (
        grx010_real_pass_default_enable_decision_evidence()
    )
    grx010_default_enable_decision_status_value = (
        grx010_real_pass_default_enable_decision_status(
            grx010_default_enable_decision_doc
        )
    )
    grx010_default_enable_decision_issue_value = (
        grx010_real_pass_default_enable_decision_issue(
            grx010_default_enable_decision_doc,
            grx010_manifest_data,
        )
    )
    grx010_default_enable_decision_ready_value = (
        grx010_default_enable_decision_issue_value is None
    )
    grx009_texture_dxc_feasibility_doc = grx009_texture_dxc_feasibility_evidence()
    grx009_texture_dxc_feasibility_status_value = (
        grx009_texture_dxc_feasibility_status(grx009_texture_dxc_feasibility_doc)
    )
    grx009_texture_dxc_feasibility_ready_value = (
        grx009_texture_dxc_feasibility_ready(grx009_texture_dxc_feasibility_doc)
    )
    grx009_texture_dxc_feasibility_issue_value = (
        grx009_texture_dxc_feasibility_issue(grx009_texture_dxc_feasibility_doc)
    )
    grx009_dxc_texture_artifact_bridge_design_doc = (
        grx009_dxc_texture_artifact_bridge_design_evidence()
    )
    grx009_dxc_texture_artifact_bridge_design_issue_value = (
        grx009_dxc_texture_artifact_bridge_design_issue(
            grx009_dxc_texture_artifact_bridge_design_doc,
            grx009_manifest_data,
            grx009_compile,
            grx009_texture_dxc_feasibility_doc,
        )
    )
    grx009_dxc_texture_artifact_bridge_design_ready_value = (
        grx009_dxc_texture_artifact_bridge_design_issue_value is None
    )
    grx009_dxc_texture_artifact_bridge_scaffold_doc = (
        grx009_dxc_texture_artifact_bridge_scaffold_evidence()
    )
    grx009_dxc_texture_artifact_bridge_scaffold_status_value = (
        grx009_dxc_texture_artifact_bridge_scaffold_status(
            grx009_dxc_texture_artifact_bridge_scaffold_doc
        )
    )
    grx009_dxc_texture_artifact_bridge_scaffold_issue_value = (
        grx009_dxc_texture_artifact_bridge_scaffold_issue(
            grx009_dxc_texture_artifact_bridge_scaffold_doc,
            grx009_manifest_data,
            grx009_compile,
            grx009_texture_dxc_feasibility_doc,
            grx009_dxc_texture_artifact_bridge_design_doc,
        )
    )
    grx009_dxc_texture_artifact_bridge_scaffold_ready_value = (
        grx009_dxc_texture_artifact_bridge_scaffold_issue_value is None
    )
    grx009_dxc_texture_rts0_integration_issue_value = (
        grx009_dxc_texture_rts0_integration_issue(
            grx009_dxc_texture_artifact_bridge_scaffold_doc,
            grx009_manifest_data,
            grx009_compile,
            grx009_texture_dxc_feasibility_doc,
            grx009_dxc_texture_artifact_bridge_design_doc,
        )
    )
    grx009_dxc_texture_rts0_integration_ready_value = (
        grx009_dxc_texture_rts0_integration_issue_value is None
    )
    grx009_dxc_texture_descriptor_rts0_crosscheck_doc = (
        grx009_dxc_texture_descriptor_rts0_crosscheck_evidence()
    )
    grx009_dxc_texture_descriptor_rts0_crosscheck_status_value = (
        grx009_dxc_texture_descriptor_rts0_crosscheck_status(
            grx009_dxc_texture_descriptor_rts0_crosscheck_doc
        )
    )
    grx009_dxc_texture_descriptor_rts0_crosscheck_issue_value = (
        grx009_dxc_texture_descriptor_rts0_crosscheck_issue(
            grx009_dxc_texture_descriptor_rts0_crosscheck_doc,
            grx009_dxc_texture_artifact_bridge_scaffold_doc,
            grx009_manifest_data,
            grx009_compile,
            grx009_texture_dxc_feasibility_doc,
            grx009_dxc_texture_artifact_bridge_design_doc,
        )
    )
    grx009_dxc_texture_descriptor_rts0_crosscheck_ready_value = (
        grx009_dxc_texture_descriptor_rts0_crosscheck_issue_value is None
    )
    grx009_texture_artifact_provenance_policy_doc = (
        grx009_texture_artifact_provenance_policy_evidence()
    )
    grx009_texture_artifact_provenance_policy_status_value = (
        grx009_texture_artifact_provenance_policy_status(
            grx009_texture_artifact_provenance_policy_doc
        )
    )
    grx009_texture_artifact_provenance_policy_issue_value = (
        grx009_texture_artifact_provenance_policy_issue(
            grx009_texture_artifact_provenance_policy_doc,
            grx009_dxc_texture_descriptor_rts0_crosscheck_doc,
            grx009_dxc_texture_artifact_bridge_scaffold_doc,
            grx009_manifest_data,
            grx009_compile,
            grx009_texture_dxc_feasibility_doc,
            grx009_dxc_texture_artifact_bridge_design_doc,
        )
    )
    grx009_texture_artifact_provenance_policy_ready_value = (
        grx009_texture_artifact_provenance_policy_issue_value is None
    )
    grx009_dxc_texture_rts0_integration_status_value = None
    grx009_dxc_texture_bridge_rts0_sha256_value = None
    grx009_dxc_texture_bridge_descriptor_sha256_value = None
    grx009_dxc_texture_reserialized_rts0_sha256_value = None
    grx009_dxc_texture_rts0_byte_for_byte_match_value = None
    grx009_dxc_texture_bridge_artifact_dir_value = None
    grx009_dxc_texture_bridge_container_sha256_value = None
    if isinstance(grx009_dxc_texture_descriptor_rts0_crosscheck_doc, dict):
        descriptor = grx009_dxc_texture_descriptor_rts0_crosscheck_doc.get(
            "descriptor_layout_artifact"
        )
        if isinstance(descriptor, dict):
            grx009_dxc_texture_bridge_descriptor_sha256_value = normalize_string(
                descriptor.get("sha256")
            )
        rts0 = grx009_dxc_texture_descriptor_rts0_crosscheck_doc.get("rts0_artifact")
        if isinstance(rts0, dict):
            grx009_dxc_texture_bridge_rts0_sha256_value = normalize_string(
                rts0.get("sha256")
            )
        reserialized = grx009_dxc_texture_descriptor_rts0_crosscheck_doc.get(
            "reserialized_rts0_artifact"
        )
        if isinstance(reserialized, dict):
            grx009_dxc_texture_reserialized_rts0_sha256_value = normalize_string(
                reserialized.get("sha256")
            )
        grx009_dxc_texture_rts0_byte_for_byte_match_value = (
            grx009_dxc_texture_descriptor_rts0_crosscheck_doc.get("byte_for_byte_match")
        )
    if isinstance(grx009_dxc_texture_artifact_bridge_scaffold_doc, dict):
        grx009_dxc_texture_rts0_integration_status_value = normalize_string(
            grx009_dxc_texture_artifact_bridge_scaffold_doc.get("rts0_integration_status")
        )
        grx009_dxc_texture_bridge_artifact_dir_value = normalize_string(
            grx009_dxc_texture_artifact_bridge_scaffold_doc.get("artifact_dir")
        )
        descriptor = grx009_dxc_texture_artifact_bridge_scaffold_doc.get(
            "descriptor_layout_artifact"
        )
        if isinstance(descriptor, dict):
            grx009_dxc_texture_bridge_descriptor_sha256_value = normalize_string(
                descriptor.get("sha256")
            )
        root_signature = grx009_dxc_texture_artifact_bridge_scaffold_doc.get(
            "root_signature_scaffold"
        )
        if isinstance(root_signature, dict):
            rts0 = root_signature.get("rts0_artifact")
            if isinstance(rts0, dict):
                grx009_dxc_texture_bridge_rts0_sha256_value = normalize_string(
                    rts0.get("sha256")
                )
        dxil_metadata = grx009_dxc_texture_artifact_bridge_scaffold_doc.get(
            "dxil_container_metadata"
        )
        if isinstance(dxil_metadata, dict):
            container = dxil_metadata.get("container")
            if isinstance(container, dict):
                grx009_dxc_texture_bridge_container_sha256_value = normalize_string(
                    container.get("sha256")
                )
    # Segment 4d evidence artifact hygiene: compare the DLL fingerprint recorded
    # by the historical smoke run against the current on-disk target/debug DLL.
    # A mismatch does NOT fail 4d (the evidence is a historical measured run);
    # it only warns that a feature-off build likely overwrote target/debug and
    # the smoke should be rerun to refresh the current artifact fingerprint.
    grx009_bridge_recording_evidence_dll_sha256_value = (
        grx009_bridge_recording_evidence_dll_sha256(
            grx009_bridge_recording_evidence_doc
        )
    )
    grx009_bridge_recording_current_dll_sha256_value = sha256_of_file(
        RURIX_GODOT_DLL
    )
    grx009_bridge_recording_current_dll_matches_evidence = (
        grx009_bridge_recording_evidence_dll_sha256_value is not None
        and grx009_bridge_recording_current_dll_sha256_value is not None
        and grx009_bridge_recording_evidence_dll_sha256_value
        == grx009_bridge_recording_current_dll_sha256_value
    )
    launcher, scons_source = preferred_scons_info(by_name)
    msvc_ready = (
        by_name["msvc_cl"]["status"] == "PASS"
        or by_name.get("msvc_cl_via_vcvarsall", {}).get("status") == "PASS"
    )
    accesskit_ready = by_name["godot_accesskit_deps"]["status"] == "PASS"
    d3d12_deps_ready = by_name["godot_d3d12_deps"]["status"] == "PASS"
    build_ready = (
        launcher is not None
        and by_name["godot_snapshot"]["status"] == "PASS"
        and by_name["godot_sconstruct"]["status"] == "PASS"
        and msvc_ready
        and by_name["windows_sdk_d3d12_headers"]["status"] == "PASS"
        and by_name["dxc"]["status"] == "PASS"
        and accesskit_ready
        and d3d12_deps_ready
        and by_name["rurix_godot_dll"]["status"] == "PASS"
    )

    blockers: list[str] = []
    warnings: list[str] = []
    optional_tools_missing: list[str] = []
    if by_name["godot_snapshot"]["status"] != "PASS":
        blockers.append("missing external/godot-master snapshot")
    if by_name["godot_sconstruct"]["status"] != "PASS":
        blockers.append("missing external/godot-master/SConstruct")
    if launcher is None:
        blockers.append(
            "missing SCons launcher (`scons`, `py -3 -m SCons`, and workspace-local "
            "`target/grx/scons-venv/Scripts/python.exe -m SCons` are all unavailable)"
        )
    if not msvc_ready:
        if by_name.get("msvc_cl_via_vcvarsall", {}).get("status") == "SKIP":
            blockers.append(
                "MSVC toolset was discovered, but `cl` could not be activated even with vcvarsall.bat"
            )
        elif by_name["vs_build_tools"]["status"] == "PASS":
            blockers.append("MSVC toolset was discovered, but `cl` is not active in the current shell")
        else:
            blockers.append("MSVC `cl` was not discovered")
    elif by_name["msvc_cl"]["status"] != "PASS":
        warnings.append(
            "`cl` is not active in the current shell; wrap commands with vcvarsall.bat"
        )
    if by_name["windows_sdk_d3d12_headers"]["status"] != "PASS":
        blockers.append("required Windows SDK D3D12 headers are missing")
    if by_name["dxc"]["status"] != "PASS":
        blockers.append("`dxc.exe` is missing")
    if not accesskit_ready:
        blockers.append(
            "workspace-local AccessKit SDK is missing; install it under "
            "`target/grx/localappdata/Godot/build_deps/accesskit` before running the default Godot SCons build"
        )
    if not d3d12_deps_ready:
        blockers.append(
            "workspace-local Godot Mesa/NIR D3D12 deps are missing; `d3d12=yes` cannot proceed without them"
        )
    if by_name["godot_agility_sdk"]["status"] != "PASS":
        warnings.append(
            "workspace-local Agility SDK was not found; Godot can still configure/build, but runtime packaging may need it later"
        )
    if by_name["godot_pix_runtime"]["status"] != "PASS":
        warnings.append(
            "workspace-local PIX runtime was not found; this is optional unless `use_pix=yes` is requested"
        )
    if by_name["dxv"]["status"] != "PASS":
        optional_tools_missing.append("`dxv.exe` is missing")
        warnings.append(
            "`dxv.exe` is unavailable; this is a later DXIL/device validation warning, not a Godot SCons build blocker"
        )
    if by_name["rurix_godot_dll"]["status"] != "PASS":
        blockers.append("`target/debug/rurix_godot.dll` is missing")
    if build_artifacts_ready and build_summary_status and build_summary_status != "success":
        warnings.append(
            "Latest Godot wrapper build exited nonzero, but required GRX artifacts with "
            "`disable_path_overrides=no` evidence are present; see godot_scons_build_summary.json for failure_targets"
        )
    if grx009_compile_manifest_consistency_warning:
        warnings.append(grx009_compile_manifest_consistency_warning)
    if (
        grx009_bridge_real_d3d12_dispatch_recording
        and grx009_bridge_recording_evidence_dll_sha256_value is not None
        and not grx009_bridge_recording_current_dll_matches_evidence
    ):
        warnings.append(
            "GRX-009 segment 4d bridge recording evidence stays ready (it is a "
            "historical measured run), but the current target/debug/rurix_godot.dll "
            f"(sha256={grx009_bridge_recording_current_dll_sha256_value or 'missing'}) "
            "no longer matches the feature-built DLL fingerprint recorded in "
            f"bridge_dispatch_recording_evidence.json "
            f"(sha256={grx009_bridge_recording_evidence_dll_sha256_value}). "
            "target/debug/rurix_godot.dll is a mutable artifact that a feature-off "
            "`cargo build -p rurix-godot` overwrites; rerun "
            r"ci\grx009_luminance_bridge_recording_smoke.py to refresh the current "
            "artifact fingerprint."
        )

    recommended_probe = wrap_with_localappdata(
        shell_wrap_with_vcvars(
            render_command(["py", "-3", str(ROOT / "ci" / "godot_rurix_toolchain_probe.py")]),
            by_name,
        )
    )
    recommended_toolchain_cl = normalize_string(
        by_name.get("vs_build_tools", {}).get("details", {}).get("selected_candidate_cl")
    ) or normalize_string(by_name.get("vs_build_tools", {}).get("details", {}).get("candidate_cl"))
    recommended_toolchain_install = normalize_string(
        by_name.get("vs_build_tools", {}).get("details", {}).get("selected_installation_path")
    ) or normalize_string(by_name.get("vs_build_tools", {}).get("details", {}).get("installation_path"))
    raw_scons_command = None
    if build_ready:
        raw_scons_command = f"{launcher} {SCONS_BUILD_ARGS}"
    recommended_scons = (
        wrap_with_localappdata(shell_wrap_with_vcvars(raw_scons_command, by_name))
        if raw_scons_command
        else None
    )
    raw_ice_workaround_command = f"{launcher} {SCONS_ICE_ARGS}" if launcher else None
    ice_workaround_command = (
        wrap_with_localappdata(shell_wrap_with_vcvars(raw_ice_workaround_command, by_name))
        if raw_ice_workaround_command
        else None
    )
    recommended_accesskit_install = wrap_with_localappdata(
        shell_wrap_with_vcvars(
            render_command(["py", "-3", str(GODOT_INSTALL_ACCESSKIT)]),
            by_name,
        )
    )
    recommended_d3d12_install = wrap_with_localappdata(
        shell_wrap_with_vcvars(
            render_command(["py", "-3", str(GODOT_INSTALL_D3D12_DEPS)]),
            by_name,
        )
    )
    recommended_all_build_deps_install = wrap_with_localappdata(
        shell_wrap_with_vcvars(
            f"py -3 {subprocess.list2cmdline([str(GODOT_INSTALL_D3D12_DEPS)])} && "
            f"py -3 {subprocess.list2cmdline([str(GODOT_INSTALL_ACCESSKIT)])}",
            by_name,
        )
    )
    recommended_dev_shell = None
    vcvarsall = by_name.get("vs_build_tools", {}).get("details", {}).get("vcvarsall_bat")
    if isinstance(vcvarsall, str) and vcvarsall:
        recommended_dev_shell = f'cmd /k ""{vcvarsall}" x64"'

    scons_actual_compiler_path = None
    scons_actual_compiler_source = None
    scons_actual_compiler_install = None
    if isinstance(build_summary, dict):
        scons_actual_compiler_path = normalize_string(build_summary.get("actual_compiler_path"))
        scons_actual_compiler_source = normalize_string(build_summary.get("actual_compiler_source"))
        scons_actual_compiler_install = normalize_string(build_summary.get("actual_compiler_install"))
    if not scons_actual_compiler_path:
        vcvars_details = by_name.get("msvc_cl_via_vcvarsall", {}).get("details", {})
        scons_actual_compiler_path = normalize_string(vcvars_details.get("compiler_path"))
        if scons_actual_compiler_path:
            scons_actual_compiler_source = "env_only"
            scons_actual_compiler_install = normalize_string(
                vcvars_details.get("compiler_installation_root")
            )
    scons_compiler_matches_probe = None
    if recommended_toolchain_cl and scons_actual_compiler_path:
        scons_compiler_matches_probe = (
            pathlib.Path(recommended_toolchain_cl) == pathlib.Path(scons_actual_compiler_path)
        )

    next_action = None
    next_action_reason = None
    next_command = None
    if launcher is None:
        next_action = "install_or_enable_scons"
        next_action_reason = (
            "SCons unavailable; run the Godot SCons build only after SCons is installed or enabled."
        )
        next_command = recommended_probe
    elif not msvc_ready:
        next_action = "activate_msvc_toolchain"
        next_action_reason = "MSVC `cl` must be available directly or through vcvarsall.bat."
        next_command = recommended_dev_shell or recommended_probe
    elif not d3d12_deps_ready and not accesskit_ready:
        next_action = "install_workspace_local_godot_build_deps"
        next_action_reason = (
            "The default Godot SCons build requires both workspace-local D3D12 Mesa/NIR deps and AccessKit "
            "under `target/grx/localappdata/Godot/build_deps`."
        )
        next_command = recommended_all_build_deps_install
    elif not d3d12_deps_ready:
        next_action = "install_workspace_local_d3d12_deps"
        next_action_reason = (
            "Godot `d3d12=yes` requires workspace-local Mesa/NIR deps under "
            "`target/grx/localappdata/Godot/build_deps`."
        )
        next_command = recommended_d3d12_install
    elif not accesskit_ready:
        next_action = "install_workspace_local_accesskit_deps"
        next_action_reason = (
            "The default Godot SCons build requires a workspace-local AccessKit SDK under "
            "`target/grx/localappdata/Godot/build_deps/accesskit`."
        )
        next_command = recommended_accesskit_install
    elif not build_ready and blockers:
        next_action = "resolve_remaining_build_blockers"
        next_action_reason = blockers[0]
        next_command = recommended_probe
    elif isinstance(build_summary, dict) and not build_summary_required_args_satisfied:
        next_action = "rebuild_godot_with_path_overrides"
        next_action_reason = (
            "Existing Godot build summary does not prove `disable_path_overrides=no`; "
            "rebuild before running fresh load smoke."
        )
        next_command = r"py -3 ci\godot_rurix_scons_build.py"
    elif build_artifacts_ready and load_smoke_ready:
        if scenes_ready and runner_ready:
            if grx006_ready:
                if not grx007_ready:
                    next_action = "start_grx007_visual_diff_scaffold"
                    next_action_reason = (
                        "GRX-006 baseline/perf schema and strict perf gate format "
                        "infrastructure is available and parseable; proceed to GRX-007 "
                        "visual capture / diff scaffold."
                    )
                    next_command = (
                        r"py -3 spike\godot-rurix\bench\visual_diff.py --validate-only "
                        r"spike\godot-rurix\bench\samples\visual_diff_placeholder.json"
                    )
                elif not grx008_ready:
                    next_action = "start_grx008_fallback_telemetry_scaffold"
                    next_action_reason = (
                        "GRX-007 visual diff scaffold/hardening red/green samples pass; "
                        "proceed to GRX-008 fallback telemetry scaffold."
                    )
                    next_command = (
                        r"py -3 spike\godot-rurix\bench\fallback_telemetry.py --validate-only "
                        r"spike\godot-rurix\bench\samples\fallback_telemetry_placeholder.json"
                    )
                else:
                    if not grx009_ready:
                        next_action = "start_grx009_luminance_reduction_pass_contract"
                        next_action_reason = (
                            "GRX-007 visual diff and GRX-008 fallback telemetry "
                            "scaffold/hardening red/green samples all pass; produce the "
                            "GRX-009 luminance reduction pass contract and manifest under "
                            "spike/godot-rurix/passes/luminance_reduction. This is "
                            "preparation only: no actual Rurix acceleration pass, visual "
                            "verification, fallback wiring, or performance improvement is "
                            "implemented or claimed."
                        )
                        if next_action not in {
                            "provide_or_locate_patched_dxil_llc",
                            "provide_signed_dxc_validator_suite",
                        }:
                            next_command = None
                    else:
                        if (
                            grx009_compile_status == "compile_failed"
                            and grx009_offline_evidence_records_texture_binding_kinds()
                        ):
                            if grx009_texture_dxc_feasibility_ready_value:
                                if grx009_dxc_texture_artifact_bridge_design_ready_value:
                                    if grx009_dxc_texture_artifact_bridge_scaffold_ready_value:
                                        if grx009_dxc_texture_rts0_integration_ready_value:
                                            if grx009_dxc_texture_descriptor_rts0_crosscheck_ready_value:
                                                if grx009_texture_artifact_provenance_policy_ready_value:
                                                    next_action = (
                                                        "provide_grx009_runtime_mappable_luminance_kernel_artifact"
                                                    )
                                                    next_action_reason = (
                                                        "GRX-009 segment 4l texture artifact provenance policy is "
                                                        "ready: the owner-approved policy records the "
                                                        "hlsl_bridge_workaround canonical-switch exception "
                                                        "(rurix_owned=false, Rurix-synthesized RTS0), the policy "
                                                        "document carries the owner decision plus revert/re-cut "
                                                        "conditions, and the bridge contract documents the owner "
                                                        "exception. Runtime remains closed: runtime_mappable=false, "
                                                        "real_gpu_pass=false, canonical_artifact_replaced=false, "
                                                        "offline_compile_evidence remains compile_failed, and the "
                                                        "canonical descriptor still records raw_buffer_view. Proceed "
                                                        "only to a runtime-mappable math-parity luminance kernel "
                                                        "artifact package; do not advance to real pass enablement, "
                                                        "visual success, GPU timestamp, FPS, or performance claims."
                                                    )
                                                else:
                                                    next_action = (
                                                        "define_grx009_texture_artifact_provenance_policy"
                                                    )
                                                    next_action_reason = (
                                                        "GRX-009 segment 4k descriptor/RTS0 cross-check is ready: "
                                                        "descriptor_layout.json and root_signature.rts0.bin are tied "
                                                        "to the same Rurix binding layout semantics, the descriptor "
                                                        "resource bindings match src_luminance SRV t0 space0 and "
                                                        "dst_luminance UAV u0 space0, root_constants=none, and a "
                                                        "fresh Rurix re-serialization matches the tracked RTS0 bytes "
                                                        "byte-for-byte, but the texture artifact provenance policy "
                                                        "gate is not ready yet "
                                                        f"(issue={grx009_texture_artifact_provenance_policy_issue_value or 'unknown'}). "
                                                        "Runtime remains closed: provenance stays "
                                                        "hlsl_bridge_workaround, rurix_owned=false, runtime_mappable=false, "
                                                        "real_gpu_pass=false, canonical_artifact_replaced=false, "
                                                        "offline_compile_evidence remains compile_failed, and the canonical "
                                                        "descriptor still records raw_buffer_view. Proceed only to texture "
                                                        "artifact provenance policy; do not advance to real pass enablement, "
                                                        "visual success, GPU timestamp, FPS, or performance claims."
                                                    )
                                            else:
                                                next_action = (
                                                    "prepare_grx009_texture_artifact_descriptor_rts0_crosscheck_or_provenance_policy"
                                                )
                                                next_action_reason = (
                                                    "GRX-009 segment 4k dxc texture artifact bridge has a "
                                                    "Rurix-synthesized RTS0 integration scaffold, but the "
                                                    "descriptor/RTS0 cross-check gate is not ready yet "
                                                    f"(issue={grx009_dxc_texture_descriptor_rts0_crosscheck_issue_value or 'unknown'}). "
                                                    "Runtime remains closed: provenance stays hlsl_bridge_workaround, "
                                                    "rurix_owned=false, runtime_mappable=false, real_gpu_pass=false, "
                                                    "canonical_artifact_replaced=false, offline_compile_evidence remains "
                                                    "compile_failed, and the canonical descriptor still records "
                                                    "raw_buffer_view. Proceed only to descriptor/RTS0 cross-check or "
                                                    "provenance policy; do not advance to real pass enablement, visual "
                                                    "success, GPU timestamp, FPS, or performance claims."
                                                )
                                        else:
                                            next_action = (
                                                "prepare_grx009_texture_artifact_rurix_provenance_or_rts0_integration"
                                            )
                                            next_action_reason = (
                                                "GRX-009 segment 4k dxc texture artifact bridge "
                                                "scaffold evidence is complete and fail-closed, but "
                                                "Rurix-owned RTS0 integration is not ready yet "
                                                f"(issue={grx009_dxc_texture_rts0_integration_issue_value or 'unknown'}). "
                                                "Runtime remains closed: canonical artifacts stay "
                                                "raw-buffer fallback, offline_compile_evidence remains "
                                                "compile_failed, runtime_mappable=false, real_gpu_pass=false, "
                                                "and canonical_artifact_replaced=false. Proceed only to "
                                                "Rurix provenance or RTS0/root-signature integration; do "
                                                "not advance to real pass enablement, visual success, GPU "
                                                "timestamp, FPS, or performance claims."
                                            )
                                    else:
                                        next_action = (
                                            "implement_grx009_dxc_texture_artifact_bridge_scaffold"
                                        )
                                        next_action_reason = (
                                            "GRX-009 segment 4k dxc texture feasibility evidence "
                                            "is ready and the Dxc texture artifact bridge design "
                                            "contract is design-ready, but the scaffold gate is not "
                                            "ready yet "
                                            f"(issue={grx009_dxc_texture_artifact_bridge_scaffold_issue_value or 'unknown'}). "
                                            "Generate an independent non-canonical dxc_texture_bridge "
                                            "package scaffold with DXIL container metadata, descriptor "
                                            "layout artifact, root signature scaffold/cross-check status, "
                                            "binding_kind mapping, DXIL validation metadata, and HLSL "
                                            "workaround provenance. Keep runtime_mappable=false, "
                                            "real_gpu_pass=false, offline compile status compile_failed, "
                                            "and the canonical descriptor raw-buffer."
                                        )
                                else:
                                    next_action = "design_grx009_dxc_texture_artifact_bridge"
                                    next_action_reason = (
                                        "GRX-009 segment 4k dxc texture feasibility evidence "
                                        "proves a minimal Texture2D<float>/RWTexture2D<float> "
                                        "HLSL compute shader can produce a dxv-validated DXIL "
                                        "container, but the Dxc texture artifact bridge design "
                                        "gate is not ready yet "
                                        f"(issue={grx009_dxc_texture_artifact_bridge_design_issue_value or 'unknown'}). "
                                        "Keep canonical offline evidence compile_failed/"
                                        "runtime_mappable=false and the bridge tracked package "
                                        "raw-buffer; finish root signature strategy, descriptor "
                                        "layout synthesis, binding_kind mapping, DXIL validation "
                                        "integration, Rurix provenance, canonical switch "
                                        "conditions, and fail-closed regression. Do not enable "
                                        "the default pass, flip real_gpu_pass, or claim visual/"
                                        "GPU-timestamp/performance success."
                                    )
                            else:
                                next_action = (
                                    "provide_grx009_runtime_mappable_luminance_kernel_artifact"
                                )
                                next_action_reason = (
                                    "GRX-009 segment 4i fail-closed path is active: "
                                    "the canonical offline_compile_evidence.json records "
                                    "status=compile_failed with blocker_category="
                                    "dxil_container_missing, attempted_binding_kinds "
                                    "includes both texture2d and rwtexture2d (signaling a "
                                    "texture-capable compile attempt), and a non-empty "
                                    "math_parity_status. Attempted texture-capable source "
                                    "exists (src/lib_texture.rx declares "
                                    "Texture2D<f32>/RWTexture2D<f32>; attempted_binding_kinds "
                                    "includes texture2d/rwtexture2d), but no runtime-mappable "
                                    "artifact exists yet because the patched llc at "
                                    "H:\\llvm-dxil\\build\\bin\\llc.exe does not support the "
                                    "llvm.dx.resource.load.texture.2d intrinsic; "
                                    "runtime_mappable=false; the bridge tracked package "
                                    "stays raw-buffer. The dxc texture feasibility status is "
                                    f"{grx009_texture_dxc_feasibility_status_value}"
                                    f" (issue={grx009_texture_dxc_feasibility_issue_value or 'none'}), "
                                    "so it has not yet proven a validated texture DXIL "
                                    "container bridge path. The compiler supports the "
                                    "RWTexture2D<F> lang item, MirResourceType::RWTexture2D, "
                                    "texture_target_ty, and the "
                                    "@llvm.dx.resource.load.texture.* / "
                                    "@llvm.dx.resource.store.texture.* emit, but the "
                                    "patched llc rejects the texture-capable kernel with "
                                    "`unknown intrinsic 'llvm.dx.resource.load.texture.2d'`. "
                                    "The offline compile records status=compile_failed with "
                                    "blocker dxil_container_missing, so the texture-capable "
                                    "DXIL container cannot be produced yet and the bridge "
                                    "tracked package stays raw-buffer. The probe stays at "
                                    "kernel_binding_kind_mismatch until a newer patched "
                                    "llc supports texture intrinsics or a designed dxc bridge "
                                    "lands; the forward-looking compiler/kernel changes are "
                                    "retained so they activate when such a path lands. Do not "
                                    "enable the default pass, flip real_gpu_pass, or claim any "
                                    "visual/GPU-timestamp/performance success."
                                )
                            next_command = None
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                            and grx009_segment4e
                            and grx009_segment4f
                            and grx009_segment4g
                            and grx009_segment4h
                        ):
                            next_command = None
                            if grx009_default_enable_decision_ready_value:
                                grx009_closed_out_preamble = (
                                    "GRX-009 stage A5 is closed out: segment 4h gated "
                                    "real-pass enablement is strict and measured "
                                    "(real_pass_enablement_success_evidence.json records "
                                    "a completed opt-in real dispatch with the LDR "
                                    "visual gate within thresholds, the forced-failure "
                                    "fallback red leg measured, and the full 0001..0010 "
                                    "provenance/log audits green), and the owner "
                                    "default-enable decision "
                                    "(real_pass_default_enable_decision.json) records "
                                    "keep_default_disabled; the luminance pass stays "
                                    "default disabled with NO performance/FPS/"
                                    "GPU-timestamp claim. "
                                )
                                if not grx010_tonemap_contract_ready_value:
                                    next_action = GRX010_NEXT_ACTION
                                    next_action_reason = grx009_closed_out_preamble + (
                                        "The Tier 1 GRX-010 tonemap pass contract slice "
                                        "is not ready yet (issue="
                                        f"{grx010_tonemap_contract_issue_value or 'unknown'}): "
                                        "deliver/repair the tonemap contract trio, the "
                                        "hlsl_bridge offline kernel package (DXC + DXV + "
                                        "Rurix-owned RTS0, hlsl_bridge_workaround "
                                        "provenance), the fail-closed bridge TonemapGate, "
                                        "and patch 0011. The pass stays default disabled "
                                        "and no performance claim is made."
                                    )
                                elif not grx010_patch_0011_applyable_ready:
                                    next_action = GRX010_FIX_PATCH_0011_ACTION
                                    next_action_reason = grx009_closed_out_preamble + (
                                        "The GRX-010 tonemap contract slice is ready, but "
                                        "0011-rurix-accel-tonemap-pass-gate-and-callsite."
                                        "patch does not pass git apply --check on a "
                                        "scratch copy of the 0001+0002+0003 snapshot with "
                                        "0004..0010 applied. Fix the 0011 patch artifact; "
                                        "the native Godot tonemap path stays active and "
                                        "no real pass is claimed."
                                    )
                                elif not grx010_dispatch_smoke_ready_value:
                                    next_action = GRX010_PROVIDE_DISPATCH_SMOKE_ACTION
                                    next_action_reason = grx009_closed_out_preamble + (
                                        "The GRX-010 tonemap contract slice and patch "
                                        "0011 are ready, but the standalone real D3D12 "
                                        "dispatch smoke evidence is not a verified "
                                        "measured success (issue="
                                        f"{grx010_dispatch_smoke_issue_value or 'unknown'}). "
                                        "Run ci/grx010_tonemap_d3d12_dispatch_smoke.py on "
                                        "a real D3D12 adapter; SKIP never advances the "
                                        "gate."
                                    )
                                elif not grx010_patch_0012_applyable_ready:
                                    next_action = GRX010_FIX_PATCH_0012_ACTION
                                    next_action_reason = grx009_closed_out_preamble + (
                                        "The GRX-010 tonemap segment A slice and the "
                                        "standalone real D3D12 dispatch smoke are ready, "
                                        "but 0012-rurix-accel-tonemap-runtime-resource-"
                                        "binding.patch does not pass git apply --check on "
                                        "a scratch copy of the 0001+0002+0003 snapshot "
                                        "with 0004..0011 applied. Fix the 0012 patch "
                                        "artifact; the native Godot tonemap path stays "
                                        "active and no real pass is claimed."
                                    )
                                    next_command = r"py -3 ci\godot_rurix_patch_stack.py"
                                elif not grx010_patch_0013_applyable_ready:
                                    next_action = GRX010_FIX_PATCH_0013_ACTION
                                    next_action_reason = grx009_closed_out_preamble + (
                                        "The GRX-010 tonemap runtime resource binding "
                                        "patch 0012 is stacked-applyable, but "
                                        "0013-rurix-accel-tonemap-recording-smoke-and-"
                                        "real-pass-optin.patch does not pass git apply "
                                        "--check on a scratch copy with 0004..0012 "
                                        "applied. Fix the 0013 patch artifact; the native "
                                        "Godot tonemap path stays active and no real pass "
                                        "is claimed."
                                    )
                                    next_command = r"py -3 ci\godot_rurix_patch_stack.py"
                                elif not grx010_real_pass_enablement_ready_value:
                                    next_action = GRX010_PROVIDE_ENABLEMENT_ACTION
                                    next_action_reason = grx009_closed_out_preamble + (
                                        "The GRX-010 tonemap 0001..0013 patch stack is "
                                        "stacked-applyable, but the opt-in real-pass "
                                        "enablement gate has not measured a strict success "
                                        "(issue="
                                        f"{grx010_real_pass_enablement_issue_value or 'unknown'}). "
                                        "Run ci/grx010_tonemap_real_pass_enablement_smoke.py "
                                        "on the 0001..0013 scratch Godot build with the "
                                        "d3d12-recording-shim bridge; SKIP never advances "
                                        "the gate. The pass stays default disabled and no "
                                        "performance claim is made."
                                    )
                                elif not grx010_default_enable_decision_ready_value:
                                    next_action = GRX010_DESIGN_DECISION_ACTION
                                    next_action_reason = grx009_closed_out_preamble + (
                                        "GRX-010 tonemap gated real-pass enablement is "
                                        "strict and measured "
                                        "(real_pass_enablement_success_evidence.json "
                                        "records a completed opt-in real dispatch with the "
                                        "LDR visual gate within thresholds, the forced-"
                                        "failure fallback red leg measured, and the full "
                                        "0001..0013 provenance/log audits green). The "
                                        "runtime default STILL stays disabled and no "
                                        "performance/FPS/GPU-timestamp claim exists; the "
                                        "owner default-enable decision gate is not ready "
                                        "yet (issue="
                                        f"{grx010_default_enable_decision_issue_value or 'unknown'}). "
                                        "Any default-enable decision is a separate owner-"
                                        "decided slice."
                                    )
                                    next_command = None
                                else:
                                    next_action = GRX011_NEXT_ACTION
                                    next_action_reason = grx009_closed_out_preamble + (
                                        "GRX-010 tonemap is closed out: the opt-in real-"
                                        "pass enablement is strict and measured "
                                        "(real_pass_enablement_success_evidence.json, "
                                        "22 checks green incl. the forced_capability_"
                                        "downgrade red leg), and the owner default-enable "
                                        "decision (real_pass_default_enable_decision.json) "
                                        "records keep_default_disabled. The tonemap pass "
                                        "stays default disabled with NO performance/FPS/"
                                        "GPU-timestamp claim; the native Godot tonemapper "
                                        "remains the continuation/backstop. Proceed to "
                                        "GRX-011 (SSAO/blur, Godot patch 0014)."
                                    )
                                    next_command = None
                            else:
                                next_action = (
                                    "design_grx009_luminance_real_pass_default_enable_decision"
                                )
                                next_action_reason = (
                                    "GRX-009 segment 4h gated real-pass enablement is strict "
                                    "and measured: real_pass_enablement_success_evidence.json "
                                    "records a completed opt-in real dispatch with the LDR "
                                    "visual gate within thresholds, the forced-failure "
                                    "fallback red leg measured, and the full 0001..0009 "
                                    "provenance/log audits green. The runtime default STILL "
                                    "stays disabled and no performance/FPS/GPU-timestamp "
                                    "claim exists; the owner default-enable decision gate "
                                    "is not ready yet "
                                    f"(issue={grx009_default_enable_decision_issue_value or 'unknown'}). "
                                    "Any default-enable decision, visual hardening, or "
                                    "performance gate is a separate owner-decided slice."
                                )
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                            and grx009_segment4e
                            and grx009_segment4f
                            and grx009_segment4g
                            and not grx009_patch_0009_applyable_ready
                        ):
                            next_action = (
                                "fix_grx009_luminance_segment4h_patch_0009_applyability"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4g is ready, but "
                                "0009-rurix-accel-luminance-real-pass-optin.patch does "
                                "not pass git apply --check on a scratch copy of the "
                                "0001+0002+0003 snapshot with 0004..0008 applied. Fix "
                                "the 0009 patch artifact; the native Godot luminance "
                                "path stays active and no real pass is claimed."
                            )
                            next_command = r"py -3 ci\godot_rurix_patch_stack.py"
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                            and grx009_segment4e
                            and grx009_segment4f
                            and grx009_segment4g
                            and grx009_real_pass_latest_status == "skip"
                            and grx009_real_pass_latest_skip_kind
                            == "measured_prerequisite_blocked"
                            and (
                                grx009_real_pass_first_missing_prerequisite
                                == GRX009_SEGMENT4H_EXPECTED_FIRST_MISSING_PREREQUISITE
                                or grx009_real_pass_first_missing_prerequisite is None
                            )
                        ):
                            next_action = (
                                "provide_grx009_runtime_mappable_luminance_kernel_artifact"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4h gated real-pass enablement measured "
                                "the predicted fail-closed shape on real hardware: with "
                                "the default-false dispatch_real_pass opt-in explicitly "
                                "armed, the bridge's kernel-binding-kind conformance "
                                "check returned validation_failed because the tracked "
                                "package is raw-buffer (the Godot runtime provides "
                                "Texture2D ID3D12Resource* handles which mismatch the "
                                "tracked kernel's declared raw_buffer_view binding "
                                "kind). The texture-capable kernel source "
                                "src/lib_texture.rx is in place (declaring "
                                "Texture2D<f32>/RWTexture2D<f32>), and the compiler "
                                "supports the RWTexture2D<F> lang item, "
                                "MirResourceType::RWTexture2D, texture_target_ty, and "
                                "the @llvm.dx.resource.load.texture.* / "
                                "@llvm.dx.resource.store.texture.* emit, but the "
                                "patched llc at H:\\llvm-dxil\\build\\bin\\llc.exe does "
                                "NOT support the llvm.dx.resource.load.texture.2d "
                                "intrinsic; the offline compile records "
                                "status=compile_failed with blocker "
                                "dxil_container_missing, so the texture-capable DXIL "
                                "container cannot be produced yet and the bridge "
                                "tracked package stays raw-buffer. The native Godot "
                                "luminance path rendered every leg (visual diff within "
                                "thresholds, forced-failure red leg recorded "
                                "unsupported_device telemetry). The first missing "
                                "prerequisite is "
                                f"{grx009_real_pass_first_missing_prerequisite or GRX009_SEGMENT4H_EXPECTED_FIRST_MISSING_PREREQUISITE}: "
                                "the slice's main goal (a runtime-mappable "
                                "texture-capable kernel artifact) is blocked by the "
                                "patched llc not supporting texture intrinsics. The "
                                "probe stays at kernel_binding_kind_mismatch until a "
                                "newer patched llc supports texture intrinsics; the "
                                "forward-looking compiler/kernel changes are retained "
                                "so they activate when such an llc lands. Do not "
                                "enable the default pass, flip real_gpu_pass, or claim "
                                "any visual/GPU-timestamp/performance success."
                            )
                            next_command = None
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                            and grx009_segment4e
                            and grx009_segment4f
                            and grx009_segment4g
                            and grx009_real_pass_latest_status == "fail"
                        ):
                            next_action = (
                                "fix_grx009_segment4h_real_pass_enablement_failure"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4h latest evidence records status=fail: "
                                "an integrity violation (unexpected markers, "
                                "over-threshold visual diff, invalid telemetry, or "
                                "unexpected ERROR lines) occurred in the enablement "
                                "matrix. Fix the violation; the pass stays default "
                                "disabled and fail-closed."
                            )
                            next_command = (
                                r"py -3 ci\grx009_segment4h_real_pass_enablement_smoke.py"
                            )
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                            and grx009_segment4e
                            and grx009_segment4f
                            and grx009_segment4g
                        ):
                            next_action = (
                                "start_grx009_segment4h_real_pass_enablement_smoke"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4g real visual diff + measured fallback "
                                "telemetry gate is strict and measured, and the segment "
                                "4h gated real-pass enablement gate is wired (patch 0009 "
                                "stacks cleanly; the bridge real-pass arm is fail-closed "
                                "with a kernel-binding-kind conformance check). The "
                                "latest real_pass_enablement_evidence.json status is "
                                f"{grx009_real_pass_latest_status or 'missing'} — run "
                                "the segment 4h smoke against a scratch Godot console "
                                "build carrying the FULL 0001..0009 patch stack "
                                f"({GODOT_EXE_ENV_SEGMENT4H} plus the source/provenance "
                                "env vars) to measure the three-leg enablement matrix. "
                                "Expected honest outcome with the tracked segment 4i "
                                "artifact: a measured_prerequisite_blocked SKIP naming "
                                "kernel_binding_kind_mismatch (the patched llc does not "
                                "support texture intrinsics yet), never a real pass. The "
                                "pass stays default disabled and no performance claim is "
                                "made."
                            )
                            next_command = (
                                r"py -3 ci\grx009_segment4h_real_pass_enablement_smoke.py"
                            )
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                            and grx009_segment4e
                            and grx009_segment4f
                        ):
                            next_action = (
                                "start_grx009_luminance_real_visual_diff_and_measured_fallback_telemetry"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4f Godot-runtime bridge dispatch recording "
                                "smoke is ready: the historical measured success artifact "
                                "godot_runtime_bridge_recording_success_evidence.json "
                                "records status=success (the latest "
                                "godot_runtime_bridge_recording_evidence.json may be a "
                                "reproducible-default SKIP when the scratch Godot exe env var "
                                "is absent, and that does not regress readiness) — the patched "
                                "Godot runtime luminance call site (via the harness-only "
                                "dispatch_recording_smoke "
                                "opt-in and a d3d12-recording-shim rurix_godot.dll) drove at "
                                "least one real bridge-recorded RXGD_PASS_LUMINANCE_REDUCTION "
                                "dispatch through the real ID3D12Resource* native handles it "
                                "resolved via RenderingDevice::get_driver_resource, and the "
                                "recorded artifact digests still match the segment 3a offline "
                                "compile evidence. This is measured harness-only Godot-runtime "
                                "smoke evidence: it keeps runtime_state=fallback_only, "
                                "real_gpu_pass=false, real_d3d12_dispatch_recorded=false, "
                                "godot_runtime_luminance_path_enabled=false, and "
                                "default_enable_state=disabled — the record path is only "
                                "linked under the test-only d3d12-recording-shim feature and "
                                "armed by the default-false dispatch_recording_smoke opt-in, "
                                "so the shipping/feature-off bridge and default Godot config "
                                "still return RXGD_STATUS_FALLBACK. Next, land the real visual "
                                "diff and measured fallback telemetry gate (with real "
                                "reference/candidate frames and measured_local telemetry) "
                                "before enabling the default pass or claiming any pass "
                                "completion, GPU timestamp, or performance improvement. "
                                "Current segment 4g gate blocker: "
                                f"{grx009_segment4g_issue or 'unknown'}."
                            )
                            next_command = (
                                r"py -3 ci\grx009_segment4g_visual_fallback_smoke.py"
                            )
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                            and grx009_segment4e
                            and grx009_segment4f_inputs
                            and not grx009_patch_0008_applyable_ready
                        ):
                            next_action = (
                                "fix_grx009_luminance_segment4f_patch_0008_applyability"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4f Godot-runtime bridge dispatch recording "
                                "inputs are present, but "
                                "0008-rurix-accel-luminance-godot-runtime-bridge-recording-"
                                "smoke.patch does not pass git apply --check on a scratch copy "
                                "of the 0001+0002+0003 snapshot with 0004+0005+0006+0007 "
                                "applied. Fix the 0008 patch artifact; the native Godot "
                                "luminance path stays active and no real dispatch is claimed."
                            )
                            next_command = r"py -3 ci\godot_rurix_patch_stack.py"
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                            and grx009_segment4e
                        ):
                            next_action = (
                                "start_grx009_godot_runtime_bridge_dispatch_recording_smoke"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4f Godot-runtime bridge dispatch recording "
                                "inputs are wired (segment 4e native handle mapping is ready "
                                "and the 0008 patch stacks cleanly on 0004+0005+0006+0007), "
                                "but the historical measured success artifact does not pass "
                                "the strict segment 4f provenance/runtime-log audit: "
                                "godot_runtime_bridge_recording_success_evidence.json status "
                                f"is {grx009_godot_runtime_recording_success_status or 'missing'} "
                                "and the latest godot_runtime_bridge_recording_evidence.json "
                                f"status is {grx009_godot_runtime_recording_status or 'missing'} "
                                "(the latest evidence is a reproducible-default SKIP when the "
                                "scratch Godot exe env var is absent, and a missing / SKIP / "
                                "FAIL run does not advance readiness — only the historical "
                                "success artifact with scratch_source_provenance and "
                                "runtime_log_audit does). The task now is to make that runtime "
                                "smoke a strict audited success at least once: "
                                "produce an ignored scratch Godot console build with the FULL "
                                "0001..0008 patch stack applied on top of the ignored "
                                "external/godot-master snapshot, rebuilt with "
                                "module_rurix_accel_enabled=yes d3d12=yes (the tracked "
                                "external/godot-master build carries only 0001+0002+0003 and "
                                "must NOT be reused), keep that source tree clean with no "
                                "untracked RXGD_DIAG/source deltas, generate a strict "
                                "source provenance sidecar proving the scratch source tree "
                                "has the same final tree as external/godot-master plus "
                                "exactly tracked patches 0001..0008 applied in order, point "
                                "RURIX_GRX009_SEGMENT4F_GODOT_EXE at that console exe, set "
                                "RURIX_GRX009_SEGMENT4F_GODOT_SOURCE if the source root cannot "
                                "be inferred from the exe path, set "
                                "RURIX_GRX009_SEGMENT4F_GODOT_SOURCE_PROVENANCE to the sidecar, "
                                "and run "
                                "ci/grx009_godot_runtime_bridge_recording_smoke.py so the "
                                "patched Godot runtime luminance call site (via the "
                                "default-false dispatch_recording_smoke opt-in and a "
                                "d3d12-recording-shim rurix_godot.dll) drives one real "
                                "bridge-recorded RXGD_PASS_LUMINANCE_REDUCTION dispatch "
                                "through the real ID3D12Resource* native handles it resolves "
                                "via RenderingDevice::get_driver_resource, prints the "
                                "RXGD_GODOT_RUNTIME_LUMINANCE_RECORD marker with recorded=1, "
                                "and exits 0 (checks.godot_exit_code_zero=true), while the "
                                "success JSON records source_clean=true, sidecar-backed "
                                "tracked_patch_stack_only=true, final_tree/actual_tree, ordered "
                                "patch commit/tree audit, and runtime_log_audit with no "
                                "unexpected diagnostics/errors. A success here is measured "
                                "harness-only Godot-runtime smoke evidence only: it keeps "
                                "runtime_state=fallback_only, "
                                "real_gpu_pass=false, real_d3d12_dispatch_recorded=false, "
                                "godot_runtime_luminance_path_enabled=false, and "
                                "default_enable_state=disabled — the record path is linked "
                                "only under the test-only d3d12-recording-shim feature and "
                                "armed only by the default-false dispatch_recording_smoke "
                                "opt-in, so the shipping/feature-off bridge and the default "
                                "Godot config still return RXGD_STATUS_FALLBACK. Do not "
                                "enable the default pass or claim any real GPU pass, visual "
                                "diff, measured telemetry, GPU timestamp, or performance "
                                "improvement."
                            )
                            next_command = (
                                r"$env:RURIX_DXC_DIR='H:\dxc-round7\extracted\bin\x64'; "
                                r"py -3 ci\grx009_godot_runtime_bridge_recording_smoke.py"
                            )
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                            and grx009_segment4e_inputs
                            and not grx009_patch_0007_applyable_ready
                        ):
                            next_action = (
                                "fix_grx009_luminance_segment4e_patch_0007_applyability"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4e native resource handle mapping inputs "
                                "are present, but "
                                "0007-rurix-accel-luminance-native-resource-handle-mapping"
                                ".patch does not pass git apply --check on a scratch copy "
                                "of the 0001+0002+0003 snapshot with 0004+0005+0006 "
                                "applied. Fix the 0007 patch artifact; the native Godot "
                                "luminance path stays active and no real dispatch is "
                                "claimed."
                            )
                            next_command = r"py -3 ci\godot_rurix_patch_stack.py"
                        elif (
                            grx009_segment4b
                            and grx009_real_d3d12_dispatch_smoke
                            and grx009_bridge_real_d3d12_dispatch_recording
                        ):
                            next_action = (
                                "start_grx009_godot_native_resource_handle_mapping"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4d bridge real D3D12 dispatch recording "
                                "smoke is ready: bridge_dispatch_recording_evidence.json "
                                "records status=success — rurix_godot.dll built with the "
                                "d3d12-recording-shim feature recorded ONE real luminance "
                                "compute dispatch on a real D3D12 device/queue via its C "
                                "ABI (rxgd_record_pass returned RXGD_STATUS_OK, "
                                "recorded_passes=1, fallback_passes=0, fence completed, dst "
                                "UAV read back), and the recorded artifact digests still "
                                "match both the on-disk artifacts and the segment 3a "
                                "offline compile evidence. This is measured BRIDGE smoke "
                                "evidence only: it keeps runtime_state=fallback_only, "
                                "real_gpu_pass=false, godot_runtime_luminance_path_enabled="
                                "false, default_enable_state=disabled, and "
                                "gpu_timestamp_status=not_yet. The recording path is "
                                "compiled only under the test-only feature and armed only "
                                "by the harness; the default Godot bridge path is "
                                "unchanged and still returns RXGD_STATUS_FALLBACK. It is "
                                "NOT a Godot runtime pass and makes no visual, perf, or "
                                "GPU-timestamp claim. Next, map real Godot native D3D12 "
                                "resource handles (not logical RID ids) into the bridge so "
                                "the recording path can be driven from the Godot runtime; "
                                "do not enable the Godot luminance Rurix path by default or "
                                "claim any pass completion until that lands with measured "
                                "telemetry and visual/perf gates."
                            )
                            next_command = None
                        elif grx009_segment4b and grx009_real_d3d12_dispatch_smoke:
                            next_action = (
                                "start_grx009_bridge_real_d3d12_dispatch_recording"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4c real D3D12 dispatch smoke is ready: "
                                "real_d3d12_dispatch_smoke.json records status=success on "
                                "a real D3D12 device/queue (the Rurix RTS0 root signature "
                                "was accepted, a compute PSO was created from the tracked "
                                "DXIL container, the SRV t0 / UAV u0 / b0 root constants "
                                "were bound from the descriptor layout, a minimal dispatch "
                                "executed, the fence completed, and the dst UAV was read "
                                "back), and the recorded artifact digests still match both "
                                "the on-disk artifacts and the segment 3a offline compile "
                                "evidence. This is measured smoke evidence only: it keeps "
                                "runtime_state=fallback_only and real_gpu_pass=false and is "
                                "NOT a Godot runtime pass, visual, perf, or measured "
                                "telemetry claim. Next, run the bridge real D3D12 dispatch "
                                "recording smoke so rxgd_record_pass records a real "
                                "dispatch with measured bridge telemetry; the current "
                                f"bridge recording smoke status is "
                                f"{grx009_bridge_recording_status or 'missing'} (a missing "
                                "/ SKIP / FAIL smoke does not advance readiness). Do not "
                                "enable the Godot luminance Rurix path by default or claim "
                                "any pass completion until that slice lands with measured "
                                "evidence."
                            )
                            next_command = (
                                r"$env:RURIX_DXC_DIR='H:\dxc-round7\extracted\bin\x64'; "
                                r"py -3 ci\grx009_luminance_bridge_recording_smoke.py"
                            )
                        elif grx009_segment4b:
                            next_action = (
                                "provide_grx009_luminance_real_d3d12_dispatch_smoke"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4b gated dispatch bring-up is ready: "
                                "the Godot side has an explicit, default-false "
                                "dispatch_bringup opt-in that advertises the reserved "
                                "RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP flag, and the bridge "
                                "runs the runtime binding preflight plus a dispatch "
                                "eligibility gate (opt-in flag, 64-bit integer "
                                "capability, non-null native D3D12 device/queue and "
                                "resource handles, and a compiled package whose "
                                "layout/digests match the offline evidence). Even when "
                                "every precondition passes the explicit dispatch gate "
                                "stays closed and rxgd_record_pass still returns "
                                "RXGD_STATUS_FALLBACK: no real D3D12 dispatch executes "
                                "and no measured GPU/CPU time is attributed. Run the "
                                "segment 4c real D3D12 dispatch smoke "
                                "(ci/grx009_luminance_d3d12_dispatch_smoke.py) on a "
                                "machine with a real D3D12 adapter and the signed DXC "
                                "suite to produce measured evidence "
                                "(real_d3d12_dispatch_smoke.json). The current smoke "
                                f"status is {grx009_dispatch_smoke_status or 'missing'}; "
                                "a missing / SKIP / FAIL smoke does not advance readiness "
                                "and no dispatch may return OK before status=success. Do "
                                "not treat this as a real GPU pass, visual evidence, "
                                "measured telemetry, or performance evidence."
                            )
                            next_command = (
                                r"$env:RURIX_DXC_DIR='H:\dxc-round7\extracted\bin\x64'; "
                                r"py -3 ci\grx009_luminance_d3d12_dispatch_smoke.py"
                            )
                        elif (
                            grx009_segment4b_inputs
                            and not grx009_patch_0006_applyable_ready
                        ):
                            next_action = (
                                "fix_grx009_luminance_segment4b_patch_0006_applyability"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4b gated dispatch bring-up inputs are "
                                "present, but "
                                "0006-rurix-accel-luminance-gated-dispatch-bringup.patch "
                                "does not pass git apply --check on a scratch copy of the "
                                "0001+0002+0003 snapshot with 0004+0005 applied. Fix the "
                                "0006 patch artifact; the native Godot luminance path "
                                "stays active and no real dispatch is claimed."
                            )
                            next_command = r"py -3 ci\godot_rurix_patch_stack.py"
                        elif grx009_segment4a:
                            next_action = (
                                "start_grx009_luminance_segment4b_gated_dispatch_bringup"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4a runtime binding preflight is ready: "
                                "the Godot side passes the real luminance binding, the "
                                "bridge validates descriptor shape, 28-byte push "
                                "constants, source dimensions, and the 64-bit integer "
                                "shader capability, and even a valid preflight still "
                                "returns RXGD_STATUS_FALLBACK. Runtime remains "
                                "fallback-only with no recorded D3D12 dispatch. Proceed "
                                "only to a future gated dispatch bring-up slice; do not "
                                "treat this as a real GPU pass, visual evidence, measured "
                                "telemetry, or performance evidence."
                            )
                        elif (
                            grx009_segment4a_inputs
                            and not grx009_patch_0005_applyable_ready
                        ):
                            next_action = (
                                "fix_grx009_luminance_segment4a_patch_0005_applyability"
                            )
                            next_action_reason = (
                                "GRX-009 segment 4a runtime binding preflight inputs are "
                                "present, but "
                                "0005-rurix-accel-luminance-runtime-binding-preflight.patch "
                                "does not pass git apply --check on a scratch copy of the "
                                "0001+0002+0003 snapshot with 0004 applied. Fix the 0005 "
                                "patch artifact before entering any gated dispatch "
                                "bring-up slice."
                            )
                            next_command = r"py -3 ci\godot_rurix_patch_stack.py"
                        elif grx009_segment3b:
                            next_action = (
                                "start_grx009_luminance_segment4_runtime_binding"
                            )
                            next_action_reason = (
                                "GRX-009 segment 3b resource mapping scaffold is ready: "
                                "the Godot luminance resources and descriptor layout are "
                                "tracked, the 64-bit integer shader capability is gated, "
                                "and runtime still remains fallback-only. Proceed only to "
                                "a future gated runtime binding slice; do not treat this as "
                                "a real GPU pass, visual evidence, measured telemetry, or "
                                "performance evidence."
                            )
                        elif (
                            grx009_segment3b_inputs
                            and not grx009_patch_0004_applyable_ready
                        ):
                            next_action = (
                                "fix_grx009_luminance_segment3b_patch_0004_applyability"
                            )
                            next_action_reason = (
                                "GRX-009 segment 3b resource mapping scaffold inputs are present, "
                                "but 0004-rurix-accel-luminance-resource-mapping-scaffold.patch "
                                "does not pass git apply --check on the current 0001+0002+0003 "
                                "Godot snapshot. Fix the 0004 patch artifact before entering any "
                                "future runtime binding slice."
                            )
                            next_command = (
                                r"git -C external\godot-master apply --check "
                                r"H:\rurix\spike\godot-rurix\patches\0004-rurix-accel-luminance-resource-mapping-scaffold.patch"
                            )
                        elif grx009_segment3a:
                            next_action = (
                                "start_grx009_luminance_segment3_resource_mapping"
                            )
                            next_action_reason = (
                                "GRX-009 segment 3a offline compile evidence is ready: "
                                "the manifest and compile evidence agree on success, "
                                "the DXIL/root signature/descriptor layout artifacts all "
                                "exist, and runtime still remains fallback-only. Proceed "
                                "to segment 3 resource mapping and gated runtime wiring."
                            )
                        elif grx009_segment2:
                            if grx009_compile_status in GRX009_SEGMENT3A_BLOCKED_COMPILE_STATUSES:
                                if (
                                    grx009_compile_status == "toolchain_missing"
                                    and dxil_llc.get("status") != "PASS"
                                ):
                                    next_action = "provide_or_locate_patched_dxil_llc"
                                    next_action_reason = (
                                        "GRX-009 segment 2 core call-site fallback wiring is in place, "
                                        "but segment 3a is blocked because the patched DXIL `llc` path "
                                        f"from RURIX_LLC is missing or not runnable ({dxil_llc.get('missing_reason') or 'see dxil_toolchain_probe.json'}). "
                                        "Keep runtime fallback active and keep the manifest at segment 2."
                                    )
                                    next_command = dxil_toolchain_preflight.get("next_command")
                                elif (
                                    grx009_compile_status == "toolchain_missing"
                                    and dxil_validator_suite.get("status") != "PASS"
                                ):
                                    missing_files = dxil_validator_suite.get("missing_files")
                                    next_action = "provide_signed_dxc_validator_suite"
                                    next_action_reason = (
                                        "GRX-009 segment 2 core call-site fallback wiring is in place, "
                                        "but segment 3a needs RURIX_DXC_DIR or RURIX_DXC_NEW_DIR to point "
                                        "at a signed DXC validator suite containing dxc.exe, dxv.exe, and dxil.dll. "
                                        f"Missing files: {missing_files or 'see dxil_toolchain_probe.json'}. PATH dxc.exe/dxv.exe is not accepted as this suite. "
                                        "Keep runtime fallback active and keep the manifest at segment 2."
                                    )
                                    next_command = dxil_toolchain_preflight.get("next_command")
                                else:
                                    next_action = "fix_grx009_luminance_segment3a_dxil_container_body_lowering_blocker"
                                    next_action_reason = (
                                        "GRX-009 segment 2 core call-site fallback wiring is in "
                                        "place, but the latest segment 3a offline compile attempt "
                                        f"recorded {grx009_compile_status} evidence. Keep runtime fallback "
                                        "active, keep the manifest at segment 2, and resolve the "
                                        f"DXIL validator rejection/container/body lowering compile blocker first: {grx009_compile_blocker_summary or 'see offline_compile_evidence.json'}."
                                    )
                            elif grx009_compile_status == "success":
                                next_action = (
                                    "fix_grx009_luminance_compile_artifact_gaps"
                                )
                                next_action_reason = (
                                    "A GRX-009 segment 3a compile attempt reported success, "
                                    "but the manifest/evidence gate is still not ready. Do "
                                    "not advance past segment 2 until DXIL, root signature, "
                                    "and descriptor layout artifacts are all present and "
                                    "traceable."
                                )
                            elif (
                                grx009_patch_stack_state is not None
                                and grx009_patch_stack_state != "0001+0002+0003"
                            ):
                                next_action = "restore_grx009_patch_stack_segment2"
                                next_action_reason = (
                                    "GRX-009 segment 2 artifacts exist, but the ignored "
                                    "Godot snapshot patch stack is not at the required "
                                    "`0001+0002+0003` state. Re-establish the legal patch "
                                    "stack before advancing to segment 3a compile work."
                                )
                            elif not grx009_patch_stack_is_ready:
                                next_action = "restore_grx009_patch_stack_segment2"
                                next_action_reason = (
                                    "GRX-009 segment 2 requires the shared patch stack check "
                                    "to pass before offline compile evidence work can be "
                                    "trusted. Fix the ignored Godot snapshot drift first."
                                )
                            else:
                                next_action = "start_grx009_luminance_reduction_real_gpu_pass"
                                next_action_reason = (
                                    "GRX-009 segment 2 core call-site fallback wiring is in "
                                    "place (patch 0003), but the actual Rurix GPU luminance "
                                    "pass is still NOT implemented. The per-pass setting "
                                    "still defaults to disabled, the bridge still returns "
                                    "fallback for luminance_reduction, and no performance "
                                    "improvement or visual verification is claimed yet."
                                )
                        elif grx009_compile_status in GRX009_SEGMENT3A_BLOCKED_COMPILE_STATUSES:
                            next_action = "restore_grx009_segment2_then_fix_compile_blocker"
                            next_action_reason = (
                                f"A GRX-009 {grx009_compile_status} evidence document exists, but the "
                                "segment 2 gate is not currently coherent. Re-establish the "
                                "segment 2 wiring gate first, then continue fixing the "
                                "offline compile blocker."
                            )
                        elif grx009_segment1:
                            next_action = (
                                "start_grx009_luminance_core_callsite_fallback_wiring"
                            )
                            next_action_reason = (
                                "GRX-009 segment 1 gated scaffold is delivered: the "
                                "bridge gate, 0002 module patch, and disabled/fallback "
                                "sample are all present, but the Godot core Auto Exposure "
                                "call site is not wired yet. Proceed to segment 2 core "
                                "call-site fallback wiring."
                            )
                        else:
                            next_action = (
                                "start_grx009_luminance_reduction_pass_gated_scaffold_segment1"
                            )
                            next_action_reason = (
                                "GRX-009 preparation is in place, but the segment 1 gated "
                                "scaffold evidence is incomplete. Re-establish the bridge "
                                "gate, 0002 module patch markers, and disabled/fallback "
                                "sample before advancing."
                            )
                        if next_action not in {
                            "provide_or_locate_patched_dxil_llc",
                            "provide_signed_dxc_validator_suite",
                            "fix_grx009_luminance_segment3b_patch_0004_applyability",
                            "fix_grx009_luminance_segment4a_patch_0005_applyability",
                            "start_grx009_luminance_real_visual_diff_and_measured_fallback_telemetry",
                        }:
                            next_command = None
            else:
                next_action = "start_grx006_baseline_schema_perf_gate"
                next_action_reason = (
                    "GRX-001~005 build/load/scenes/runner evidence is complete; proceed to "
                    "GRX-006 baseline schema / perf gate input format."
                )
                next_command = (
                    r"py -3 spike\godot-rurix\bench\perf_gate.py --kind baseline "
                    r"--validate-only spike\godot-rurix\bench\samples\baseline_smoke_example.json"
                )
        elif scenes_ready:
            next_action = "run_grx005_benchmark_runner"
            next_action_reason = (
                "GRX-004 per-scene smoke is complete, but no GRX-005 runner evidence is "
                "present; run the benchmark runner."
            )
            next_command = (
                r"py -3 spike\godot-rurix\bench\run_benchmark_scenes.py --quick-smoke"
            )
        else:
            next_action = "start_grx2_tier0_benchmark_skeleton"
            next_action_reason = (
                "GRX-001/002/003 build/load/fallback evidence is complete; proceed to GRX-004."
            )
    elif build_artifacts_ready:
        next_action = "run_grx003_load_smoke"
        next_action_reason = (
            "Godot build summary is success and required artifacts are present; proceed to GRX-003 load smoke."
        )
        next_command = LOAD_SMOKE_COMMAND
    elif build_ready:
        next_action = "run_godot_scons_build"
        next_action_reason = "All required blockers are clear for the default `d3d12=yes` build."
        next_command = recommended_scons

    # --- GRX gate sequence (table-driven per-pass registration) -------------
    # Engages ONLY once the legacy grx010 chain has closed out and handed off to
    # grx011 (next_action == GRX011_NEXT_ACTION), then walks the registered
    # downstream gate modules fail closed. With an EMPTY table this is a no-op
    # and next_action is unchanged (hard regression requirement).
    grx_gate_sequence_ids = [
        normalize_string(entry.get("gate_id")) if isinstance(entry, dict) else None
        for entry in GRX_GATE_SEQUENCE
    ]
    grx_gate_evaluations: list[dict[str, object]] = []
    grx_gate_module_errors: list[dict[str, object]] = []
    if next_action == GRX011_NEXT_ACTION:
        gate_walk = walk_grx_gate_sequence(
            GRX_GATE_SEQUENCE, next_action, next_action_reason, next_command
        )
        next_action = gate_walk["next_action"]
        next_action_reason = gate_walk["next_action_reason"]
        next_command = gate_walk["next_command"]
        grx_gate_evaluations = gate_walk["evaluations"]
        grx_gate_module_errors = gate_walk["module_errors"]
        for error in grx_gate_module_errors:
            warnings.append(
                "grx_gate_module_error gate_id="
                + str(error.get("gate_id"))
                + " reason="
                + str(error.get("reason"))
            )

    return {
        "build_ready": build_ready,
        "build_artifacts_ready": build_artifacts_ready,
        "load_smoke_ready": load_smoke_ready,
        "bench_scenes_ready": scenes_ready,
        "bench_runner_ready": runner_ready,
        "grx006_schema_ready": grx006_ready,
        "grx007_visual_ready": grx007_ready,
        "grx008_telemetry_ready": grx008_ready,
        "grx009_prep_ready": grx009_ready,
        "grx009_segment1_ready": grx009_segment1,
        "grx009_segment2_ready": grx009_segment2,
        "grx009_patch_stack_state": grx009_patch_stack_state,
        "grx009_patch_stack_ready": grx009_patch_stack_is_ready,
        "grx009_patch_stack_reason": grx009_patch_stack_reason,
        "grx009_patch_0004_applyable": grx009_patch_0004_applyable_ready,
        "grx009_patch_0004_applyability_reason": normalize_string(
            grx009_patch_0004_applyability.get("reason")
        ),
        "grx009_patch_0004_applyability_details": grx009_patch_0004_applyability.get(
            "details"
        ),
        "grx009_segment3b_resource_mapping_inputs_ready": grx009_segment3b_inputs,
        "grx009_segment3a_compile_ready": grx009_segment3a,
        "grx009_segment3b_resource_mapping_ready": grx009_segment3b,
        "grx009_patch_0005_applyable": grx009_patch_0005_applyable_ready,
        "grx009_patch_0005_applyability_reason": normalize_string(
            grx009_patch_0005_applyability.get("reason")
        ),
        "grx009_patch_0005_applyability_details": grx009_patch_0005_applyability.get(
            "details"
        ),
        "grx009_segment4a_runtime_binding_preflight_inputs_ready": grx009_segment4a_inputs,
        "grx009_segment4a_runtime_binding_preflight_ready": grx009_segment4a,
        "grx009_patch_0006_applyable": grx009_patch_0006_applyable_ready,
        "grx009_patch_0006_applyability_reason": normalize_string(
            grx009_patch_0006_applyability.get("reason")
        ),
        "grx009_patch_0006_applyability_details": grx009_patch_0006_applyability.get(
            "details"
        ),
        "grx009_segment4b_gated_dispatch_bringup_inputs_ready": grx009_segment4b_inputs,
        "grx009_segment4b_gated_dispatch_bringup_ready": grx009_segment4b,
        "grx009_real_d3d12_dispatch_smoke_status": grx009_dispatch_smoke_status,
        "grx009_real_d3d12_dispatch_smoke_ready": grx009_real_d3d12_dispatch_smoke,
        "grx009_real_d3d12_dispatch_smoke_evidence_path": (
            str(GRX009_REAL_D3D12_DISPATCH_SMOKE)
            if GRX009_REAL_D3D12_DISPATCH_SMOKE.exists()
            else None
        ),
        "grx009_bridge_real_d3d12_dispatch_recording_status": grx009_bridge_recording_status,
        "grx009_bridge_real_d3d12_dispatch_recording_ready": (
            grx009_bridge_real_d3d12_dispatch_recording
        ),
        "grx009_bridge_real_d3d12_dispatch_recording_evidence_path": (
            str(GRX009_BRIDGE_RECORDING_EVIDENCE)
            if GRX009_BRIDGE_RECORDING_EVIDENCE.exists()
            else None
        ),
        "grx009_bridge_recording_evidence_dll_sha256": (
            grx009_bridge_recording_evidence_dll_sha256_value
        ),
        "grx009_bridge_recording_current_dll_sha256": (
            grx009_bridge_recording_current_dll_sha256_value
        ),
        "grx009_bridge_recording_current_dll_matches_evidence": (
            grx009_bridge_recording_current_dll_matches_evidence
        ),
        "grx009_patch_0007_applyable": grx009_patch_0007_applyable_ready,
        "grx009_patch_0007_applyability_reason": normalize_string(
            grx009_patch_0007_applyability.get("reason")
        ),
        "grx009_patch_0007_applyability_details": grx009_patch_0007_applyability.get(
            "details"
        ),
        "grx009_segment4e_native_resource_handle_mapping_inputs_ready": (
            grx009_segment4e_inputs
        ),
        "grx009_segment4e_native_resource_handle_mapping_ready": grx009_segment4e,
        "grx009_patch_0008_applyable": grx009_patch_0008_applyable_ready,
        "grx009_patch_0008_applyability_reason": normalize_string(
            grx009_patch_0008_applyability.get("reason")
        ),
        "grx009_patch_0008_applyability_details": grx009_patch_0008_applyability.get(
            "details"
        ),
        "grx009_segment4f_godot_runtime_bridge_recording_inputs_ready": (
            grx009_segment4f_inputs
        ),
        "grx009_segment4f_godot_runtime_bridge_recording_status": (
            grx009_godot_runtime_recording_status
        ),
        "grx009_segment4f_godot_runtime_bridge_recording_latest_status": (
            grx009_godot_runtime_recording_status
        ),
        "grx009_segment4f_godot_runtime_bridge_recording_success_status": (
            grx009_godot_runtime_recording_success_status
        ),
        "grx009_segment4f_godot_runtime_bridge_recording_ready": grx009_segment4f,
        "grx009_segment4f_godot_runtime_bridge_recording_issue": (
            grx009_segment4f_issue
        ),
        "grx009_segment4f_godot_runtime_bridge_recording_evidence_path": (
            str(GRX009_GODOT_RUNTIME_RECORDING_EVIDENCE)
            if GRX009_GODOT_RUNTIME_RECORDING_EVIDENCE.exists()
            else None
        ),
        "grx009_segment4f_godot_runtime_bridge_recording_success_evidence_path": (
            str(GRX009_GODOT_RUNTIME_RECORDING_SUCCESS_EVIDENCE)
            if GRX009_GODOT_RUNTIME_RECORDING_SUCCESS_EVIDENCE.exists()
            else None
        ),
        "grx009_segment4g_visual_fallback_latest_status": (
            grx009_visual_fallback_latest_status
        ),
        "grx009_segment4g_visual_fallback_success_status": (
            grx009_visual_fallback_success_status
        ),
        "grx009_segment4g_visual_fallback_issue": grx009_segment4g_issue,
        "grx009_segment4g_visual_fallback_ready": grx009_segment4g,
        "grx009_segment4g_visual_fallback_evidence_path": (
            str(GRX009_VISUAL_FALLBACK_EVIDENCE)
            if GRX009_VISUAL_FALLBACK_EVIDENCE.exists()
            else None
        ),
        "grx009_segment4g_visual_fallback_success_evidence_path": (
            str(GRX009_VISUAL_FALLBACK_SUCCESS_EVIDENCE)
            if GRX009_VISUAL_FALLBACK_SUCCESS_EVIDENCE.exists()
            else None
        ),
        "grx009_segment4g_measured_fallback_telemetry_path": (
            str(GRX009_MEASURED_FALLBACK_TELEMETRY)
            if GRX009_MEASURED_FALLBACK_TELEMETRY.exists()
            else None
        ),
        "grx009_patch_0009_applyable": grx009_patch_0009_applyable_ready,
        "grx009_patch_0009_applyability_reason": normalize_string(
            grx009_patch_0009_applyability.get("reason")
        ),
        "grx010_tonemap_contract_ready": grx010_tonemap_contract_ready_value,
        "grx010_tonemap_contract_issue": grx010_tonemap_contract_issue_value,
        "grx010_patch_0011_applyable": grx010_patch_0011_applyable_ready,
        "grx010_patch_0011_applyability_reason": normalize_string(
            grx010_patch_0011_applyability.get("reason")
        ),
        "grx010_tonemap_d3d12_dispatch_smoke_ready": grx010_dispatch_smoke_ready_value,
        "grx010_tonemap_d3d12_dispatch_smoke_status": grx010_dispatch_smoke_status_value,
        "grx010_tonemap_d3d12_dispatch_smoke_issue": grx010_dispatch_smoke_issue_value,
        "grx010_patch_0012_applyable": grx010_patch_0012_applyable_ready,
        "grx010_patch_0012_applyability_reason": normalize_string(
            grx010_patch_0012_applyability.get("reason")
        ),
        "grx010_patch_0013_applyable": grx010_patch_0013_applyable_ready,
        "grx010_patch_0013_applyability_reason": normalize_string(
            grx010_patch_0013_applyability.get("reason")
        ),
        "grx010_real_pass_enablement_success_status": grx010_real_pass_success_status,
        "grx010_real_pass_enablement_ready": grx010_real_pass_enablement_ready_value,
        "grx010_real_pass_enablement_issue": grx010_real_pass_enablement_issue_value,
        "grx010_real_pass_default_enable_decision_status": (
            grx010_default_enable_decision_status_value
        ),
        "grx010_real_pass_default_enable_decision_ready": (
            grx010_default_enable_decision_ready_value
        ),
        "grx010_real_pass_default_enable_decision_issue": (
            grx010_default_enable_decision_issue_value
        ),
        "grx010_real_pass_default_enable_decision_evidence_path": (
            str(GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE)
            if GRX010_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE.exists()
            else None
        ),
        "grx009_patch_0009_applyability_details": grx009_patch_0009_applyability.get(
            "details"
        ),
        "grx009_segment4h_real_pass_enablement_latest_status": (
            grx009_real_pass_latest_status
        ),
        "grx009_segment4h_real_pass_enablement_latest_skip_kind": (
            grx009_real_pass_latest_skip_kind
        ),
        "grx009_segment4h_first_missing_prerequisite": (
            grx009_real_pass_first_missing_prerequisite
        ),
        "grx009_segment4h_real_pass_enablement_latest_issue": (
            grx009_real_pass_latest_issue
        ),
        "grx009_luminance_kernel_binding_kind": (
            grx009_luminance_kernel_binding_kind
        ),
        "grx009_luminance_math_parity_status": (
            grx009_luminance_math_parity_status
        ),
        "grx009_luminance_offline_binding_kinds": (
            grx009_luminance_offline_binding_kinds
        ),
        "grx009_segment4h_real_pass_enablement_success_status": (
            grx009_real_pass_success_status
        ),
        "grx009_segment4h_real_pass_enablement_issue": grx009_segment4h_issue,
        "grx009_segment4h_real_pass_enablement_ready": grx009_segment4h,
        "grx009_real_pass_default_enable_decision_status": (
            grx009_default_enable_decision_status_value
        ),
        "grx009_real_pass_default_enable_decision_issue": (
            grx009_default_enable_decision_issue_value
        ),
        "grx009_real_pass_default_enable_decision_ready": (
            grx009_default_enable_decision_ready_value
        ),
        "grx009_real_pass_default_enable_decision_evidence_path": (
            str(GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE)
            if GRX009_REAL_PASS_DEFAULT_ENABLE_DECISION_EVIDENCE.exists()
            else None
        ),
        "grx009_texture_dxc_feasibility_status": (
            grx009_texture_dxc_feasibility_status_value
        ),
        "grx009_texture_dxc_feasibility_ready": (
            grx009_texture_dxc_feasibility_ready_value
        ),
        "grx009_texture_dxc_feasibility_issue": (
            grx009_texture_dxc_feasibility_issue_value
        ),
        "grx009_texture_dxc_feasibility_evidence_path": (
            str(GRX009_TEXTURE_DXC_FEASIBILITY_EVIDENCE)
            if GRX009_TEXTURE_DXC_FEASIBILITY_EVIDENCE.exists()
            else None
        ),
        "grx009_dxc_texture_artifact_bridge_design_ready": (
            grx009_dxc_texture_artifact_bridge_design_ready_value
        ),
        "grx009_dxc_texture_artifact_bridge_design_issue": (
            grx009_dxc_texture_artifact_bridge_design_issue_value
        ),
        "grx009_dxc_texture_artifact_bridge_design_evidence_path": (
            str(GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DESIGN)
            if GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DESIGN.exists()
            else None
        ),
        "grx009_dxc_texture_artifact_bridge_scaffold_status": (
            grx009_dxc_texture_artifact_bridge_scaffold_status_value
        ),
        "grx009_dxc_texture_artifact_bridge_scaffold_ready": (
            grx009_dxc_texture_artifact_bridge_scaffold_ready_value
        ),
        "grx009_dxc_texture_artifact_bridge_scaffold_issue": (
            grx009_dxc_texture_artifact_bridge_scaffold_issue_value
        ),
        "grx009_dxc_texture_artifact_bridge_scaffold_evidence_path": (
            str(GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_SCAFFOLD_EVIDENCE)
            if GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_SCAFFOLD_EVIDENCE.exists()
            else None
        ),
        "grx009_dxc_texture_rts0_integration_status": (
            grx009_dxc_texture_rts0_integration_status_value
        ),
        "grx009_dxc_texture_rts0_integration_ready": (
            grx009_dxc_texture_rts0_integration_ready_value
        ),
        "grx009_dxc_texture_rts0_integration_issue": (
            grx009_dxc_texture_rts0_integration_issue_value
        ),
        "grx009_dxc_texture_descriptor_rts0_crosscheck_status": (
            grx009_dxc_texture_descriptor_rts0_crosscheck_status_value
        ),
        "grx009_dxc_texture_descriptor_rts0_crosscheck_ready": (
            grx009_dxc_texture_descriptor_rts0_crosscheck_ready_value
        ),
        "grx009_dxc_texture_descriptor_rts0_crosscheck_issue": (
            grx009_dxc_texture_descriptor_rts0_crosscheck_issue_value
        ),
        "grx009_dxc_texture_descriptor_rts0_crosscheck_evidence_path": (
            str(GRX009_DXC_TEXTURE_DESCRIPTOR_RTS0_CROSSCHECK_EVIDENCE)
            if GRX009_DXC_TEXTURE_DESCRIPTOR_RTS0_CROSSCHECK_EVIDENCE.exists()
            else None
        ),
        "grx009_texture_artifact_provenance_policy_status": (
            grx009_texture_artifact_provenance_policy_status_value
        ),
        "grx009_texture_artifact_provenance_policy_ready": (
            grx009_texture_artifact_provenance_policy_ready_value
        ),
        "grx009_texture_artifact_provenance_policy_issue": (
            grx009_texture_artifact_provenance_policy_issue_value
        ),
        "grx009_texture_artifact_provenance_policy_evidence_path": (
            str(GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_EVIDENCE)
            if GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_EVIDENCE.exists()
            else None
        ),
        "grx009_texture_artifact_provenance_policy_document_path": (
            str(GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC)
            if GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC.exists()
            else None
        ),
        "grx009_dxc_texture_bridge_rts0_sha256": (
            grx009_dxc_texture_bridge_rts0_sha256_value
        ),
        "grx009_dxc_texture_bridge_descriptor_sha256": (
            grx009_dxc_texture_bridge_descriptor_sha256_value
        ),
        "grx009_dxc_texture_reserialized_rts0_sha256": (
            grx009_dxc_texture_reserialized_rts0_sha256_value
        ),
        "grx009_dxc_texture_rts0_byte_for_byte_match": (
            grx009_dxc_texture_rts0_byte_for_byte_match_value
        ),
        "grx009_dxc_texture_bridge_artifact_dir": (
            grx009_dxc_texture_bridge_artifact_dir_value
        ),
        "grx009_dxc_texture_bridge_container_sha256": (
            grx009_dxc_texture_bridge_container_sha256_value
        ),
        "grx009_dxc_texture_artifact_bridge_document_path": (
            str(GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC)
            if GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC.exists()
            else None
        ),
        "grx009_canonical_descriptor_binding_kinds": (
            grx009_canonical_descriptor_binding_kinds()
        ),
        "grx009_segment4h_real_pass_enablement_evidence_path": (
            str(GRX009_REAL_PASS_ENABLEMENT_EVIDENCE)
            if GRX009_REAL_PASS_ENABLEMENT_EVIDENCE.exists()
            else None
        ),
        "grx009_segment4h_real_pass_enablement_success_evidence_path": (
            str(GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE)
            if GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE.exists()
            else None
        ),
        "grx009_compile_evidence_status": grx009_compile_status,
        "grx009_compile_evidence_path": (
            str(GRX009_COMPILE_EVIDENCE) if GRX009_COMPILE_EVIDENCE.exists() else None
        ),
        "grx009_compile_blocker_category": grx009_compile_blocker_category,
        "grx009_compile_blocker_summary": grx009_compile_blocker_summary,
        "dxil_toolchain_preflight": dxil_toolchain_preflight,
        "dxil_toolchain_preflight_ready": dxil_toolchain_preflight["ready"],
        "dxil_toolchain_preflight_path": str(DXIL_TOOLCHAIN_REPORT),
        "dxil_toolchain_missing_reasons": dxil_toolchain_preflight[
            "missing_reasons"
        ],
        "rurix_llc_status": dxil_llc.get("status"),
        "rurix_llc_path": dxil_llc.get("path"),
        "signed_dxc_validator_suite_status": dxil_validator_suite.get("status"),
        "signed_dxc_validator_suite_dir": dxil_validator_suite.get("path"),
        "signed_dxc_validator_suite_missing_files": dxil_validator_suite.get(
            "missing_files"
        ),
        "workspace_localappdata": str(LOCAL_GODOT_LOCALAPPDATA),
        "godot_build_deps_root": str(LOCAL_GODOT_BUILD_DEPS),
        "godot_windows_arch": GODOT_WINDOWS_ARCH,
        "scons_source": scons_source,
        "preferred_scons_launcher": launcher,
        "recommended_toolchain_cl": recommended_toolchain_cl,
        "recommended_toolchain_install": recommended_toolchain_install,
        "scons_actual_compiler_path": scons_actual_compiler_path,
        "scons_actual_compiler_source": scons_actual_compiler_source,
        "scons_actual_compiler_install": scons_actual_compiler_install,
        "scons_compiler_matches_probe": scons_compiler_matches_probe,
        "build_summary_command": build_summary_primary_cmd,
        "build_summary_ice_workaround_command": build_summary_ice_cmd,
        "build_summary_status": build_summary_status,
        "build_summary_required_scons_args": list(REQUIRED_SCONS_ARGS),
        "build_summary_required_scons_args_satisfied": build_summary_required_args_satisfied,
        "build_summary_path_overrides_ready": build_summary_required_args_satisfied,
        "last_build_summary_path": str(BUILD_SUMMARY_REPORT) if BUILD_SUMMARY_REPORT.exists() else None,
        "last_load_smoke_summary_path": (
            str(LOAD_SMOKE_SUMMARY_REPORT) if LOAD_SMOKE_SUMMARY_REPORT.exists() else None
        ),
        "recommended_probe_command": recommended_probe,
        "recommended_scons_command": recommended_scons,
        "ice_workaround_command": ice_workaround_command,
        "recommended_accesskit_install_command": recommended_accesskit_install,
        "recommended_d3d12_install_command": recommended_d3d12_install,
        "recommended_dev_shell_command": recommended_dev_shell,
        "next_action": next_action,
        "next_action_reason": next_action_reason,
        "next_command": next_command,
        "grx_gate_sequence": grx_gate_sequence_ids,
        "grx_gate_evaluations": grx_gate_evaluations,
        "grx_gate_module_errors": grx_gate_module_errors,
        "blockers": blockers,
        "warnings": warnings,
        "optional_tools_missing": optional_tools_missing,
        "results": by_name,
    }


def write_report(summary: dict[str, object]) -> None:
    LOCAL_LOG_DIR.mkdir(parents=True, exist_ok=True)
    dxil_toolchain_preflight = summary.get("dxil_toolchain_preflight")
    if isinstance(dxil_toolchain_preflight, dict):
        DXIL_TOOLCHAIN_REPORT.write_text(
            json.dumps(dxil_toolchain_preflight, indent=2, ensure_ascii=True) + "\n",
            encoding="utf-8",
        )
        print(f"[godot-toolchain] dxil_toolchain_report_path: {DXIL_TOOLCHAIN_REPORT}")
    JSON_REPORT.write_text(
        json.dumps(summary, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
    )
    print(f"[godot-toolchain] report_path: {JSON_REPORT}")


def main() -> int:
    results: list[ProbeResult] = []
    results.extend(probe_godot_tree())
    results.append(
        ProbeResult(
            "godot_workspace_localappdata",
            "PASS",
            "workspace-local LOCALAPPDATA root is configured",
            {
                "LOCALAPPDATA": str(LOCAL_GODOT_LOCALAPPDATA),
                "deps_root": str(LOCAL_GODOT_BUILD_DEPS),
                "arch": GODOT_WINDOWS_ARCH,
            },
        )
    )
    results.append(run_probe("scons_cli", ["scons", "--version"]))
    results.append(run_probe("python_scons", ["py", "-3", "-m", "SCons", "--version"]))
    results.append(
        run_probe(
            "local_python_scons",
            [str(LOCAL_SCONS_PYTHON), "-m", "SCons", "--version"],
        )
    )
    vs_probe = probe_vs_build_tools()
    results.append(vs_probe)
    results.append(probe_msvc(vs_probe))
    results.append(probe_msvc_via_vcvarsall(vs_probe))
    results.append(probe_headers())
    results.append(probe_godot_accesskit_deps())
    results.append(probe_godot_d3d12_deps())
    results.append(probe_godot_agility_sdk())
    results.append(probe_godot_pix_runtime())
    for tool_name in TOOL_CANDIDATES:
        results.append(probe_tool_path(tool_name))
    results.append(probe_rurix_godot_dll())

    summary = summarize(results)
    for result in results:
        print_result(result)
    print(
        "[godot-toolchain] build_ready: "
        + ("PASS" if summary["build_ready"] else "SKIP")
        + f" - preferred launcher: {summary['preferred_scons_launcher'] or 'none'}"
    )
    print(
        "[godot-toolchain] load_smoke_ready: "
        + ("true" if summary["load_smoke_ready"] else "false")
    )
    print(
        "[godot-toolchain] bench_scenes_ready: "
        + ("true" if summary["bench_scenes_ready"] else "false")
    )
    print(
        "[godot-toolchain] bench_runner_ready: "
        + ("true" if summary["bench_runner_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx006_schema_ready: "
        + ("true" if summary["grx006_schema_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx007_visual_ready: "
        + ("true" if summary["grx007_visual_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx008_telemetry_ready: "
        + ("true" if summary["grx008_telemetry_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_prep_ready: "
        + ("true" if summary["grx009_prep_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_segment1_ready: "
        + ("true" if summary["grx009_segment1_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_segment2_ready: "
        + ("true" if summary["grx009_segment2_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_patch_stack_state: "
        + str(summary["grx009_patch_stack_state"] or "unknown")
    )
    print(
        "[godot-toolchain] grx009_patch_stack_ready: "
        + ("true" if summary["grx009_patch_stack_ready"] else "false")
    )
    if summary["grx009_patch_stack_reason"]:
        print(
            "[godot-toolchain] grx009_patch_stack_reason: "
            + str(summary["grx009_patch_stack_reason"])
        )
    print(
        "[godot-toolchain] grx009_patch_0004_applyable: "
        + ("true" if summary["grx009_patch_0004_applyable"] else "false")
    )
    if summary["grx009_patch_0004_applyability_reason"]:
        print(
            "[godot-toolchain] grx009_patch_0004_applyability_reason: "
            + str(summary["grx009_patch_0004_applyability_reason"])
        )
    print(
        "[godot-toolchain] grx009_segment3b_resource_mapping_inputs_ready: "
        + (
            "true"
            if summary["grx009_segment3b_resource_mapping_inputs_ready"]
            else "false"
        )
    )
    print(
        "[godot-toolchain] grx009_segment3a_compile_ready: "
        + ("true" if summary["grx009_segment3a_compile_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_segment3b_resource_mapping_ready: "
        + ("true" if summary["grx009_segment3b_resource_mapping_ready"] else "false")
    )
    print(
        "[godot-toolchain] grx009_patch_0005_applyable: "
        + ("true" if summary["grx009_patch_0005_applyable"] else "false")
    )
    if summary["grx009_patch_0005_applyability_reason"]:
        print(
            "[godot-toolchain] grx009_patch_0005_applyability_reason: "
            + str(summary["grx009_patch_0005_applyability_reason"])
        )
    print(
        "[godot-toolchain] grx009_segment4a_runtime_binding_preflight_inputs_ready: "
        + (
            "true"
            if summary["grx009_segment4a_runtime_binding_preflight_inputs_ready"]
            else "false"
        )
    )
    print(
        "[godot-toolchain] grx009_segment4a_runtime_binding_preflight_ready: "
        + (
            "true"
            if summary["grx009_segment4a_runtime_binding_preflight_ready"]
            else "false"
        )
    )
    print(
        "[godot-toolchain] grx009_patch_0006_applyable: "
        + ("true" if summary["grx009_patch_0006_applyable"] else "false")
    )
    if summary["grx009_patch_0006_applyability_reason"]:
        print(
            "[godot-toolchain] grx009_patch_0006_applyability_reason: "
            + str(summary["grx009_patch_0006_applyability_reason"])
        )
    print(
        "[godot-toolchain] grx009_segment4b_gated_dispatch_bringup_inputs_ready: "
        + (
            "true"
            if summary["grx009_segment4b_gated_dispatch_bringup_inputs_ready"]
            else "false"
        )
    )
    print(
        "[godot-toolchain] grx009_segment4b_gated_dispatch_bringup_ready: "
        + (
            "true"
            if summary["grx009_segment4b_gated_dispatch_bringup_ready"]
            else "false"
        )
    )
    print(
        "[godot-toolchain] grx009_real_d3d12_dispatch_smoke_ready: "
        + ("true" if summary["grx009_real_d3d12_dispatch_smoke_ready"] else "false")
    )
    if summary["grx009_real_d3d12_dispatch_smoke_status"]:
        print(
            "[godot-toolchain] grx009_real_d3d12_dispatch_smoke_status: "
            + str(summary["grx009_real_d3d12_dispatch_smoke_status"])
        )
    print(
        "[godot-toolchain] grx009_bridge_real_d3d12_dispatch_recording_ready: "
        + (
            "true"
            if summary["grx009_bridge_real_d3d12_dispatch_recording_ready"]
            else "false"
        )
    )
    print(
        "[godot-toolchain] grx009_patch_0007_applyable: "
        + ("true" if summary["grx009_patch_0007_applyable"] else "false")
    )
    if summary["grx009_patch_0007_applyability_reason"]:
        print(
            "[godot-toolchain] grx009_patch_0007_applyability_reason: "
            + str(summary["grx009_patch_0007_applyability_reason"])
        )
    print(
        "[godot-toolchain] grx009_segment4e_native_resource_handle_mapping_inputs_ready: "
        + (
            "true"
            if summary["grx009_segment4e_native_resource_handle_mapping_inputs_ready"]
            else "false"
        )
    )
    print(
        "[godot-toolchain] grx009_segment4e_native_resource_handle_mapping_ready: "
        + (
            "true"
            if summary["grx009_segment4e_native_resource_handle_mapping_ready"]
            else "false"
        )
    )
    print(
        "[godot-toolchain] grx009_patch_0008_applyable: "
        + ("true" if summary["grx009_patch_0008_applyable"] else "false")
    )
    if summary["grx009_patch_0008_applyability_reason"]:
        print(
            "[godot-toolchain] grx009_patch_0008_applyability_reason: "
            + str(summary["grx009_patch_0008_applyability_reason"])
        )
    print(
        "[godot-toolchain] grx009_segment4f_godot_runtime_bridge_recording_inputs_ready: "
        + (
            "true"
            if summary["grx009_segment4f_godot_runtime_bridge_recording_inputs_ready"]
            else "false"
        )
    )
    if summary["grx009_segment4f_godot_runtime_bridge_recording_latest_status"]:
        print(
            "[godot-toolchain] grx009_segment4f_godot_runtime_bridge_recording_latest_status: "
            + str(
                summary[
                    "grx009_segment4f_godot_runtime_bridge_recording_latest_status"
                ]
            )
            + " (reproducible-default SKIP when the scratch Godot exe env var is absent)"
        )
    print(
        "[godot-toolchain] grx009_segment4f_godot_runtime_bridge_recording_success_status: "
        + str(
            summary[
                "grx009_segment4f_godot_runtime_bridge_recording_success_status"
            ]
            or "missing"
        )
        + " (historical measured success artifact; readiness advances off this file)"
    )
    print(
        "[godot-toolchain] grx009_segment4f_godot_runtime_bridge_recording_ready: "
        + (
            "true"
            if summary["grx009_segment4f_godot_runtime_bridge_recording_ready"]
            else "false"
        )
    )
    if summary["grx009_segment4f_godot_runtime_bridge_recording_issue"]:
        print(
            "[godot-toolchain] grx009_segment4f_godot_runtime_bridge_recording_issue: "
            + str(
                summary["grx009_segment4f_godot_runtime_bridge_recording_issue"]
            )
        )
    if summary["grx009_segment4g_visual_fallback_latest_status"]:
        print(
            "[godot-toolchain] grx009_segment4g_visual_fallback_latest_status: "
            + str(summary["grx009_segment4g_visual_fallback_latest_status"])
            + " (reproducible-default SKIP when the tracked Godot exe is absent)"
        )
    print(
        "[godot-toolchain] grx009_segment4g_visual_fallback_success_status: "
        + str(
            summary["grx009_segment4g_visual_fallback_success_status"] or "missing"
        )
        + " (historical measured success artifact; readiness advances off this file)"
    )
    print(
        "[godot-toolchain] grx009_segment4g_visual_fallback_ready: "
        + ("true" if summary["grx009_segment4g_visual_fallback_ready"] else "false")
    )
    if summary["grx009_segment4g_visual_fallback_issue"]:
        print(
            "[godot-toolchain] grx009_segment4g_visual_fallback_issue: "
            + str(summary["grx009_segment4g_visual_fallback_issue"])
        )
    print(
        "[godot-toolchain] grx010_tonemap_contract_ready: "
        + ("true" if summary["grx010_tonemap_contract_ready"] else "false")
    )
    if summary["grx010_tonemap_contract_issue"]:
        print(
            "[godot-toolchain] grx010_tonemap_contract_issue: "
            + str(summary["grx010_tonemap_contract_issue"])
        )
    print(
        "[godot-toolchain] grx010_patch_0011_applyable: "
        + ("true" if summary["grx010_patch_0011_applyable"] else "false")
    )
    if summary["grx010_patch_0011_applyability_reason"]:
        print(
            "[godot-toolchain] grx010_patch_0011_applyability_reason: "
            + str(summary["grx010_patch_0011_applyability_reason"])
        )
    print(
        "[godot-toolchain] grx010_tonemap_d3d12_dispatch_smoke_ready: "
        + ("true" if summary["grx010_tonemap_d3d12_dispatch_smoke_ready"] else "false")
    )
    if summary["grx010_tonemap_d3d12_dispatch_smoke_status"]:
        print(
            "[godot-toolchain] grx010_tonemap_d3d12_dispatch_smoke_status: "
            + str(summary["grx010_tonemap_d3d12_dispatch_smoke_status"])
        )
    if summary["grx010_tonemap_d3d12_dispatch_smoke_issue"]:
        print(
            "[godot-toolchain] grx010_tonemap_d3d12_dispatch_smoke_issue: "
            + str(summary["grx010_tonemap_d3d12_dispatch_smoke_issue"])
        )
    print(
        "[godot-toolchain] grx010_patch_0012_applyable: "
        + ("true" if summary["grx010_patch_0012_applyable"] else "false")
    )
    if summary["grx010_patch_0012_applyability_reason"]:
        print(
            "[godot-toolchain] grx010_patch_0012_applyability_reason: "
            + str(summary["grx010_patch_0012_applyability_reason"])
        )
    print(
        "[godot-toolchain] grx010_patch_0013_applyable: "
        + ("true" if summary["grx010_patch_0013_applyable"] else "false")
    )
    if summary["grx010_patch_0013_applyability_reason"]:
        print(
            "[godot-toolchain] grx010_patch_0013_applyability_reason: "
            + str(summary["grx010_patch_0013_applyability_reason"])
        )
    print(
        "[godot-toolchain] grx010_real_pass_enablement_success_status: "
        + str(summary["grx010_real_pass_enablement_success_status"] or "missing")
        + " (historical measured success artifact; readiness advances off this "
        + "file; the opt-in real-pass arm ran a real dispatch on the 0001..0013 "
        + "scratch build; default stays disabled and no performance claim exists)"
    )
    print(
        "[godot-toolchain] grx010_real_pass_enablement_ready: "
        + ("true" if summary["grx010_real_pass_enablement_ready"] else "false")
    )
    if summary["grx010_real_pass_enablement_issue"]:
        print(
            "[godot-toolchain] grx010_real_pass_enablement_issue: "
            + str(summary["grx010_real_pass_enablement_issue"])
        )
    print(
        "[godot-toolchain] grx010_real_pass_default_enable_decision_status: "
        + str(summary["grx010_real_pass_default_enable_decision_status"])
        + " (owner decision; keep_default_disabled keeps the tonemap pass "
        + "default disabled with no performance claim)"
    )
    print(
        "[godot-toolchain] grx010_real_pass_default_enable_decision_ready: "
        + (
            "true"
            if summary["grx010_real_pass_default_enable_decision_ready"]
            else "false"
        )
    )
    if summary["grx010_real_pass_default_enable_decision_issue"]:
        print(
            "[godot-toolchain] grx010_real_pass_default_enable_decision_issue: "
            + str(summary["grx010_real_pass_default_enable_decision_issue"])
        )
    print(
        "[godot-toolchain] grx009_patch_0009_applyable: "
        + ("true" if summary["grx009_patch_0009_applyable"] else "false")
    )
    if summary["grx009_patch_0009_applyability_reason"]:
        print(
            "[godot-toolchain] grx009_patch_0009_applyability_reason: "
            + str(summary["grx009_patch_0009_applyability_reason"])
        )
    if summary["grx009_segment4h_real_pass_enablement_latest_status"]:
        print(
            "[godot-toolchain] grx009_segment4h_real_pass_enablement_latest_status: "
            + str(summary["grx009_segment4h_real_pass_enablement_latest_status"])
            + (
                f" (skip_kind={summary['grx009_segment4h_real_pass_enablement_latest_skip_kind']})"
                if summary["grx009_segment4h_real_pass_enablement_latest_skip_kind"]
                else ""
            )
            + " (reproducible-default SKIP when the 0001..0009 scratch Godot exe is absent)"
        )
    if summary["grx009_segment4h_first_missing_prerequisite"]:
        print(
            "[godot-toolchain] grx009_segment4h_first_missing_prerequisite: "
            + str(summary["grx009_segment4h_first_missing_prerequisite"])
        )
    if summary["grx009_segment4h_real_pass_enablement_latest_issue"]:
        print(
            "[godot-toolchain] grx009_segment4h_real_pass_enablement_latest_issue: "
            + str(summary["grx009_segment4h_real_pass_enablement_latest_issue"])
        )
    print(
        "[godot-toolchain] grx009_luminance_kernel_binding_kind: "
        + str(summary["grx009_luminance_kernel_binding_kind"])
    )
    if summary["grx009_luminance_math_parity_status"]:
        print(
            "[godot-toolchain] grx009_luminance_math_parity_status: "
            + str(summary["grx009_luminance_math_parity_status"])
        )
    if summary["grx009_luminance_offline_binding_kinds"]:
        print(
            "[godot-toolchain] grx009_luminance_offline_binding_kinds: "
            + str(summary["grx009_luminance_offline_binding_kinds"])
        )
    print(
        "[godot-toolchain] grx009_segment4h_real_pass_enablement_success_status: "
        + str(
            summary["grx009_segment4h_real_pass_enablement_success_status"]
            or "missing"
        )
        + " (historical measured success artifact; readiness advances off this file; "
        + "reached at stage A5 via the opt-in real-pass arm on the 0001..0010 scratch "
        + "build; default stays disabled and no performance claim exists)"
    )
    print(
        "[godot-toolchain] grx009_segment4h_real_pass_enablement_ready: "
        + (
            "true"
            if summary["grx009_segment4h_real_pass_enablement_ready"]
            else "false"
        )
    )
    if summary["grx009_segment4h_real_pass_enablement_issue"]:
        print(
            "[godot-toolchain] grx009_segment4h_real_pass_enablement_issue: "
            + str(summary["grx009_segment4h_real_pass_enablement_issue"])
        )
    print(
        "[godot-toolchain] grx009_real_pass_default_enable_decision_status: "
        + str(summary["grx009_real_pass_default_enable_decision_status"])
        + " (stage A5 owner decision; keep_default_disabled keeps the pass "
        + "default disabled with no performance claim)"
    )
    print(
        "[godot-toolchain] grx009_real_pass_default_enable_decision_ready: "
        + (
            "true"
            if summary["grx009_real_pass_default_enable_decision_ready"]
            else "false"
        )
    )
    if summary["grx009_real_pass_default_enable_decision_issue"]:
        print(
            "[godot-toolchain] grx009_real_pass_default_enable_decision_issue: "
            + str(summary["grx009_real_pass_default_enable_decision_issue"])
        )
    print(
        "[godot-toolchain] grx009_texture_dxc_feasibility_status: "
        + str(summary["grx009_texture_dxc_feasibility_status"])
    )
    print(
        "[godot-toolchain] grx009_texture_dxc_feasibility_ready: "
        + ("true" if summary["grx009_texture_dxc_feasibility_ready"] else "false")
    )
    if summary["grx009_texture_dxc_feasibility_issue"]:
        print(
            "[godot-toolchain] grx009_texture_dxc_feasibility_issue: "
            + str(summary["grx009_texture_dxc_feasibility_issue"])
        )
    print(
        "[godot-toolchain] grx009_dxc_texture_artifact_bridge_design_ready: "
        + (
            "true"
            if summary["grx009_dxc_texture_artifact_bridge_design_ready"]
            else "false"
        )
    )
    if summary["grx009_dxc_texture_artifact_bridge_design_issue"]:
        print(
            "[godot-toolchain] grx009_dxc_texture_artifact_bridge_design_issue: "
            + str(summary["grx009_dxc_texture_artifact_bridge_design_issue"])
        )
    print(
        "[godot-toolchain] grx009_dxc_texture_artifact_bridge_scaffold_status: "
        + str(summary["grx009_dxc_texture_artifact_bridge_scaffold_status"])
    )
    print(
        "[godot-toolchain] grx009_dxc_texture_artifact_bridge_scaffold_ready: "
        + (
            "true"
            if summary["grx009_dxc_texture_artifact_bridge_scaffold_ready"]
            else "false"
        )
    )
    if summary["grx009_dxc_texture_artifact_bridge_scaffold_issue"]:
        print(
            "[godot-toolchain] grx009_dxc_texture_artifact_bridge_scaffold_issue: "
            + str(summary["grx009_dxc_texture_artifact_bridge_scaffold_issue"])
        )
    print(
        "[godot-toolchain] grx009_dxc_texture_rts0_integration_status: "
        + str(summary["grx009_dxc_texture_rts0_integration_status"])
    )
    print(
        "[godot-toolchain] grx009_dxc_texture_rts0_integration_ready: "
        + (
            "true"
            if summary["grx009_dxc_texture_rts0_integration_ready"]
            else "false"
        )
    )
    if summary["grx009_dxc_texture_rts0_integration_issue"]:
        print(
            "[godot-toolchain] grx009_dxc_texture_rts0_integration_issue: "
            + str(summary["grx009_dxc_texture_rts0_integration_issue"])
        )
    print(
        "[godot-toolchain] grx009_dxc_texture_descriptor_rts0_crosscheck_status: "
        + str(summary["grx009_dxc_texture_descriptor_rts0_crosscheck_status"])
    )
    print(
        "[godot-toolchain] grx009_dxc_texture_descriptor_rts0_crosscheck_ready: "
        + (
            "true"
            if summary["grx009_dxc_texture_descriptor_rts0_crosscheck_ready"]
            else "false"
        )
    )
    if summary["grx009_dxc_texture_descriptor_rts0_crosscheck_issue"]:
        print(
            "[godot-toolchain] grx009_dxc_texture_descriptor_rts0_crosscheck_issue: "
            + str(summary["grx009_dxc_texture_descriptor_rts0_crosscheck_issue"])
        )
    print(
        "[godot-toolchain] grx009_texture_artifact_provenance_policy_status: "
        + str(summary["grx009_texture_artifact_provenance_policy_status"])
    )
    print(
        "[godot-toolchain] grx009_texture_artifact_provenance_policy_ready: "
        + (
            "true"
            if summary["grx009_texture_artifact_provenance_policy_ready"]
            else "false"
        )
    )
    if summary["grx009_texture_artifact_provenance_policy_issue"]:
        print(
            "[godot-toolchain] grx009_texture_artifact_provenance_policy_issue: "
            + str(summary["grx009_texture_artifact_provenance_policy_issue"])
        )
    if summary["grx009_dxc_texture_bridge_artifact_dir"]:
        print(
            "[godot-toolchain] grx009_dxc_texture_bridge_artifact_dir: "
            + str(summary["grx009_dxc_texture_bridge_artifact_dir"])
        )
    if summary["grx009_dxc_texture_bridge_descriptor_sha256"]:
        print(
            "[godot-toolchain] grx009_dxc_texture_bridge_descriptor_sha256: "
            + str(summary["grx009_dxc_texture_bridge_descriptor_sha256"])
        )
    if summary["grx009_dxc_texture_bridge_rts0_sha256"]:
        print(
            "[godot-toolchain] grx009_dxc_texture_bridge_rts0_sha256: "
            + str(summary["grx009_dxc_texture_bridge_rts0_sha256"])
        )
    if summary["grx009_dxc_texture_reserialized_rts0_sha256"]:
        print(
            "[godot-toolchain] grx009_dxc_texture_reserialized_rts0_sha256: "
            + str(summary["grx009_dxc_texture_reserialized_rts0_sha256"])
        )
    if summary["grx009_dxc_texture_rts0_byte_for_byte_match"] is not None:
        print(
            "[godot-toolchain] grx009_dxc_texture_rts0_byte_for_byte_match: "
            + (
                "true"
                if summary["grx009_dxc_texture_rts0_byte_for_byte_match"]
                else "false"
            )
        )
    if summary["grx009_dxc_texture_bridge_container_sha256"]:
        print(
            "[godot-toolchain] grx009_dxc_texture_bridge_container_sha256: "
            + str(summary["grx009_dxc_texture_bridge_container_sha256"])
        )
    if summary["grx009_compile_evidence_status"]:
        print(
            "[godot-toolchain] grx009_compile_evidence_status: "
            + str(summary["grx009_compile_evidence_status"])
        )
    if summary["grx009_compile_evidence_path"]:
        print(
            "[godot-toolchain] grx009_compile_evidence_path: "
            + str(summary["grx009_compile_evidence_path"])
        )
    if summary["grx009_compile_blocker_category"]:
        print(
            "[godot-toolchain] grx009_compile_blocker_category: "
            + str(summary["grx009_compile_blocker_category"])
        )
    if summary["grx009_compile_blocker_summary"]:
        print(
            "[godot-toolchain] grx009_compile_blocker_summary: "
            + str(summary["grx009_compile_blocker_summary"])
        )
    print(
        "[godot-toolchain] dxil_toolchain_preflight_ready: "
        + ("true" if summary["dxil_toolchain_preflight_ready"] else "false")
    )
    print(
        "[godot-toolchain] dxil_toolchain_preflight_path: "
        + str(summary["dxil_toolchain_preflight_path"])
    )
    print(
        "[godot-toolchain] rurix_llc_status: "
        + str(summary["rurix_llc_status"])
    )
    if summary["rurix_llc_path"]:
        print("[godot-toolchain] rurix_llc_path: " + str(summary["rurix_llc_path"]))
    print(
        "[godot-toolchain] signed_dxc_validator_suite_status: "
        + str(summary["signed_dxc_validator_suite_status"])
    )
    if summary["signed_dxc_validator_suite_dir"]:
        print(
            "[godot-toolchain] signed_dxc_validator_suite_dir: "
            + str(summary["signed_dxc_validator_suite_dir"])
        )
    for missing_reason in summary["dxil_toolchain_missing_reasons"]:
        print(f"[godot-toolchain] dxil_toolchain_missing_reason: {missing_reason}")
    print("[godot-toolchain] scons_source: " + str(summary["scons_source"]))
    if summary["recommended_toolchain_cl"]:
        print(
            "[godot-toolchain] recommended_toolchain_cl: "
            + str(summary["recommended_toolchain_cl"])
        )
    if summary["recommended_toolchain_install"]:
        print(
            "[godot-toolchain] recommended_toolchain_install: "
            + str(summary["recommended_toolchain_install"])
        )
    if summary["scons_actual_compiler_path"]:
        print(
            "[godot-toolchain] scons_actual_compiler_path: "
            + str(summary["scons_actual_compiler_path"])
        )
    if summary["scons_actual_compiler_source"]:
        print(
            "[godot-toolchain] scons_actual_compiler_source: "
            + str(summary["scons_actual_compiler_source"])
        )
    if summary["scons_compiler_matches_probe"] is not None:
        print(
            "[godot-toolchain] scons_compiler_matches_probe: "
            + ("PASS" if summary["scons_compiler_matches_probe"] else "MISMATCH")
        )
    print(
        "[godot-toolchain] build_summary_required_scons_args_satisfied: "
        + ("true" if summary["build_summary_required_scons_args_satisfied"] else "false")
    )
    if summary["build_summary_command"]:
        print(
            "[godot-toolchain] build_summary_command: "
            + str(summary["build_summary_command"])
        )
    if summary["build_summary_ice_workaround_command"]:
        print(
            "[godot-toolchain] build_summary_ice_workaround_command: "
            + str(summary["build_summary_ice_workaround_command"])
        )
    if summary["last_load_smoke_summary_path"]:
        print(
            "[godot-toolchain] last_load_smoke_summary_path: "
            + str(summary["last_load_smoke_summary_path"])
        )
    for blocker in summary["blockers"]:
        print(f"[godot-toolchain] blocker: {blocker}")
    for warning in summary["warnings"]:
        print(f"[godot-toolchain] warning: {warning}")
    for optional_tool in summary["optional_tools_missing"]:
        print(f"[godot-toolchain] optional_tool_missing: {optional_tool}")
    print(
        "[godot-toolchain] recommended_probe_command: "
        + summary["recommended_probe_command"]
    )
    print(
        "[godot-toolchain] recommended_scons_command: "
        + str(summary["recommended_scons_command"])
    )
    print(
        "[godot-toolchain] ice_workaround_command: "
        + str(summary["ice_workaround_command"])
    )
    print(
        "[godot-toolchain] recommended_accesskit_install_command: "
        + str(summary["recommended_accesskit_install_command"])
    )
    print(
        "[godot-toolchain] recommended_d3d12_install_command: "
        + str(summary["recommended_d3d12_install_command"])
    )
    if summary["recommended_dev_shell_command"]:
        print(
            "[godot-toolchain] recommended_dev_shell_command: "
            + summary["recommended_dev_shell_command"]
        )
    if summary["next_action"]:
        print("[godot-toolchain] next_action: " + summary["next_action"])
    if summary["next_action_reason"]:
        print("[godot-toolchain] next_action_reason: " + summary["next_action_reason"])
    if summary["next_command"]:
        print("[godot-toolchain] next_command: " + str(summary["next_command"]))
    grx_gate_module_errors = summary.get("grx_gate_module_errors")
    if grx_gate_module_errors:
        for error in grx_gate_module_errors:
            print(
                "[godot-toolchain] grx_gate_module_error: gate_id="
                + str(error.get("gate_id"))
                + " reason="
                + str(error.get("reason"))
            )
    write_report(summary)

    if any(result.status == "FAIL" for result in results):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
