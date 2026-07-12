#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-010: gated real tonemap pass enablement smoke.

This harness drives the opt-in real-pass enablement gate for the GRX-010
tonemap path. It is a strict, fail-closed bring-up gate, NOT a default
enablement: ``rendering/rurix_accel/passes/tonemap/enabled`` stays ``false``
by default, the ``.../tonemap/dispatch_real_pass`` opt-in also defaults to
``false``, and NO FPS, GPU-timestamp, or performance claim is made anywhere
in this gate. Template copy of
``ci/grx009_segment4h_real_pass_enablement_smoke.py`` pointed at the tonemap
pass and the 0001..0013 patch stack.

What it measures, honestly, against a scratch Godot console exe rebuilt with
the full 0001..0013 patch stack (``RURIX_GRX010_TONEMAP_GODOT_EXE``) and a
``rurix_godot.dll`` built WITH the ``d3d12-recording-shim`` feature (the
real-pass arm routes through the linked recording shim, so a real dispatch
can only be attempted when the shim is compiled in; the shipping feature-off
bridge still fails closed with ``real_dispatch_path_not_linked``):

  * **Pass enable matrix (three legs)**: a *reference* leg (all tonemap
    per-pass settings at their ``false`` defaults; native Godot tonemap
    path), an *enabled_real_pass_optin* candidate leg (``enabled=true`` +
    ``dispatch_real_pass=true``), and a *forced_capability_downgrade* red
    leg (candidate settings plus the harness-only
    ``real_pass_force_capability_downgrade=true`` knob, which clears the
    shader-int64 capability so the bridge tonemap preflight must fail
    closed with ``unsupported_device``).
  * **Gated real-pass attempt**: the canonical tonemap artifact paths carry
    the texture-capable hlsl_bridge workaround package (DXC ``cs_6_0``
    container validated by ``dxv``, per-slot ``texture2d``/``rwtexture2d``
    binding kinds, owner-approved ``hlsl_bridge_workaround`` provenance),
    and the LINEAR + linear_to_srgb math subset is CPU-proven
    (``math_parity_evidence.json``), so every software gate can pass and
    the candidate leg may print the ``RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS``
    marker (plus the patch 0013 result writeback scaffold marker) after a
    real recorded dispatch. If the real dispatch cannot complete in this
    environment the candidate leg must instead print the tracked fallback
    marker AND the bridge's machine-readable
    ``RXGD_TONEMAP_REAL_PASS_BLOCKED`` diagnostic naming the FIRST missing
    prerequisite (``real_dispatch_recording_failed``).
  * **Fallback red/green + visual stability**: all three legs must render
    via the native Godot tonemapper and exit 0 (the patch 0013 writeback
    scaffold deliberately keeps the native raster pass as the
    continuation/backstop, so the rendered image can never change); the
    candidate and forced-failure frames must match the reference frame
    within the pinned LDR absolute-diff thresholds, and a GRX-008-format
    ``evidence_level=measured_local`` telemetry document must record the
    fallback entries with ``godot_fallback_active=true`` and
    ``telemetry_frame`` equal to the measured capture frame index.
  * **Runtime log audit**: the FULL merged stdout+stderr of every leg is
    audited (GRX-009 segment 4f/4g policy): only the known ``Could not load
    global script cache`` warning is tolerated; any other ``ERROR:`` line
    is an integrity FAIL.

Outcome semantics (``real_pass_enablement_evidence.json`` under the tonemap
pass dir, the *latest* run evidence rewritten on EVERY run):

  * ``status=skip`` with ``skip_kind=environment``: a precondition (scratch
    exe, auditable source provenance sidecar, ready bridge session) is
    unavailable. ``RURIX_REQUIRE_REAL=1`` upgrades THIS kind of skip to a
    hard FAIL.
  * ``status=skip`` with ``skip_kind=measured_prerequisite_blocked``: every
    leg ran and measured EXACTLY the predicted fail-closed shape; the gate
    honestly reports ``first_missing_prerequisite`` instead of claiming
    success. Not upgraded to FAIL by RURIX_REQUIRE_REAL; never advances the
    readiness gate.
  * ``status=fail``: any integrity violation (unexpected markers, marker in
    the wrong leg, over-threshold visual diff, invalid telemetry, unexpected
    ERROR lines, non-zero exits, tampered artifacts).
  * ``status=success`` (strict): the opt-in real dispatch actually executed
    and completed (``RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS`` observed, no
    blocked marker), the visual diff stayed within thresholds, and every
    audit passed. ONLY then is ``real_gpu_pass=true`` recorded and the
    *historical measured success* artifact
    ``real_pass_enablement_success_evidence.json`` written (never
    overwritten by a later SKIP/FAIL run); the readiness gate advances off
    THAT file alone. Even a success keeps
    ``default_enable_state=disabled`` and ``performance_claim=none``.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import shutil
import struct
import subprocess
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
# Reuse the tracked GRX-009 segment 4f scratch-provenance and log-audit
# helpers with the GRX-010 0001..0013 stack.
from ci.grx009_godot_runtime_bridge_recording_smoke import (
    PATCH_STACK as PATCH_STACK_4F,
    find_git_root,
    patch_stack_identity,
    runtime_log_audit,
    source_status_clean,
    verify_source_provenance_sidecar,
)

PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "tonemap"
ARTIFACTS = PASS_DIR / "artifacts"
VISUAL_DIR = ARTIFACTS / "visual"
DXIL = ARTIFACTS / "tonemap.dxil"
RTS0 = ARTIFACTS / "tonemap.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "tonemap_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
SCHEMA = PASS_DIR / "real_pass_enablement_evidence.schema.json"
# The *latest* run evidence: rewritten on every run; reproducible-default SKIP
# without the scratch exe. Never advances the gate on its own.
EVIDENCE_OUT = PASS_DIR / "real_pass_enablement_evidence.json"
# The *historical measured success* artifact: written ONLY on a strict
# status=success run (real dispatch executed AND completed AND visual gate
# green) and never overwritten by a later SKIP/FAIL run. The GRX-010
# readiness gate advances off THIS file.
SUCCESS_EVIDENCE_OUT = PASS_DIR / "real_pass_enablement_success_evidence.json"
# GRX-008-format measured_local telemetry for this gate (candidate + forced
# fallback entries); written on every completed measured run.
TELEMETRY_OUT = PASS_DIR / "real_pass_enablement_telemetry.json"
FALLBACK_TELEMETRY_SCRIPT = (
    ROOT / "spike" / "godot-rurix" / "bench" / "fallback_telemetry.py"
)

# Tracked frame artifacts, committed ONLY on a strict status=success run.
REFERENCE_FRAME = VISUAL_DIR / "tonemap_real_pass_reference.rgb8"
CANDIDATE_FRAME = VISUAL_DIR / "tonemap_real_pass_candidate.rgb8"
DIFF_ARTIFACT = VISUAL_DIR / "tonemap_real_pass_diff.rgb8"

RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"
WORK = ROOT / "target" / "grx010_tonemap_real_pass_enablement_smoke"
LOG_DIR = WORK / "logs"

SUBJECT = "grx010_tonemap_real_pass_enablement_smoke"

# Visual gate pins. These MUST stay in sync with the probe's pinned values
# (ci/godot_rurix_toolchain_probe.py); the regression test asserts equality.
METRIC_KIND = "ldr_absolute_diff"
FRAME_FORMAT = "R8G8B8_raw"
LDR_MAX_ABS_DIFF_THRESHOLD = 2
LDR_MEAN_ABS_DIFF_THRESHOLD = 0.25
MIN_FRAME_DIMENSION = 64
CAPTURE_FRAME_INDEX = 24
VIEWPORT_WIDTH = 256
VIEWPORT_HEIGHT = 144
ALLOWED_GODOT_ERROR = "Could not load global script cache"

# Markers in the full 0001..0013 build. The fallback marker carries the
# patch 0012 native-resource-handle-mapping wording (NOT the 0011 0002-level
# wording).
FALLBACK_MARKER = (
    "RurixAccel: tonemap native resource handle mapping fallback rc="
)
SESSION_READY_MARKER = "RurixAccel: D3D12 Forward+ bridge session ready."
# Bridge-side machine-readable first-missing-prerequisite diagnostic
# (printed once per session by the fail-closed TonemapGate in
# src/rurix-godot/src/lib.rs).
REAL_PASS_BLOCKED_MARKER = "RXGD_TONEMAP_REAL_PASS_BLOCKED"
# Real-pass markers: the bridge prints "RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS
# recorded=1" and the patch 0013 module gate prints its own
# "RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS: pass=..." line ONLY after
# rxgd_record_pass actually returned RXGD_STATUS_OK on the real-pass arm.
REAL_PASS_MARKER = "RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS"
# Patch 0013 result writeback scaffold marker: the native tonemapper stays
# the continuation/backstop after a real dispatch, so the image never
# changes.
WRITEBACK_MARKER = "RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS_WRITEBACK"
# Recording-smoke marker: must never appear here (the tonemap
# dispatch_recording_smoke opt-in is not enabled by this harness).
RECORD_MARKER = "RXGD_GODOT_RUNTIME_TONEMAP_RECORD"

# The predicted first missing prerequisite when the opt-in real dispatch
# cannot complete in this environment. The canonical tonemap package is
# texture-capable (per-slot texture2d/rwtexture2d binding kinds matching the
# Texture2D ID3D12Resource* handles the Godot runtime provides), the LINEAR
# + linear_to_srgb math subset is CPU-proven (math_parity_evidence.json),
# and the bridge DLL is built with the d3d12-recording-shim feature, so
# every software gate can pass; the only remaining blocker is the linked
# real dispatch itself failing (e.g. no signed DXC dxil.dll,
# PSO/root-signature/D3D12 failure). Pinned so a drift in the gate chain is
# a loud FAIL, not a silent re-labelling.
EXPECTED_FIRST_MISSING_PREREQUISITE = "real_dispatch_recording_failed"
EXPECTED_BLOCKED_FALLBACK_REASON = "validation_failed"
EXPECTED_FORCED_PREREQUISITE = "runtime_binding_preflight_failed"
EXPECTED_FORCED_FALLBACK_REASON = "unsupported_device"

KNOWN_GAPS = [
    (
        "math parity is CPU-proven for the LINEAR + linear_to_srgb subset "
        "only and still pending GPU observation (math_parity_evidence.json "
        "status=pending_gpu_dispatch); Reinhard/Filmic/ACES/AgX, auto "
        "exposure, glow, FXAA, BCS, color correction, debanding, multiview, "
        "and HDR output are recorded gaps"
    ),
    (
        "canonical artifact provenance is hlsl_bridge_workaround "
        "(owner-approved GRX-009 texture artifact provenance policy): the "
        "DXIL container is DXC-compiled from "
        "artifacts/hlsl_bridge/tonemap_apply.hlsl, not rurixc-owned; a "
        "rurixc-owned texture-capable compile still requires a patched llc "
        "that supports texture intrinsics plus float4 texture element "
        "support"
    ),
    (
        "the real dispatch path is linked only under the d3d12-recording-"
        "shim feature; the shipping feature-off bridge still fails closed "
        "with real_dispatch_path_not_linked, and the patch 0013 result "
        "writeback is a SCAFFOLD: the native Godot tonemapper re-renders "
        "every frame as the continuation/backstop (raster-vs-compute output "
        "seam and full-mode parity are later rounds)"
    ),
]

GODOT_TIMEOUT_SECONDS = 180
REQUESTED_RENDERER = "d3d12"
REQUESTED_RENDERING_METHOD = "forward_plus"

GODOT_EXE_ENV = "RURIX_GRX010_TONEMAP_GODOT_EXE"
SCRATCH_SOURCE_ENV = "RURIX_GRX010_TONEMAP_GODOT_SOURCE"
SCRATCH_SOURCE_PROVENANCE_ENV = "RURIX_GRX010_TONEMAP_GODOT_SOURCE_PROVENANCE"
SCRATCH_BUILD_COMMAND_ENV = "RURIX_GRX010_TONEMAP_GODOT_BUILD_COMMAND"
SCRATCH_BUILD_LOG_ENV = "RURIX_GRX010_TONEMAP_GODOT_BUILD_LOG"
CAPTURE_PREFIX_ENV = "RURIX_GRX010_TONEMAP_CAPTURE_PREFIX"

TARGET_BACKEND = "Godot 4.7-dev Windows D3D12 Forward+"
PASS_SETTING_PREFIX = "rendering/rurix_accel/passes/tonemap"

PATCH_STACK_ID = "0001..0013"
PATCH_STACK_GRX010 = (
    *PATCH_STACK_4F,
    "0009-rurix-accel-luminance-real-pass-optin.patch",
    "0010-rurix-accel-luminance-real-pass-result-writeback.patch",
    "0011-rurix-accel-tonemap-pass-gate-and-callsite.patch",
    "0012-rurix-accel-tonemap-runtime-resource-binding.patch",
    "0013-rurix-accel-tonemap-recording-smoke-and-real-pass-optin.patch",
)


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def now_iso() -> str:
    return _dt.datetime.now().astimezone().replace(microsecond=0).isoformat()


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def load_json(path: Path) -> dict | None:
    if not path.is_file():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def rel(path: Path) -> str:
    return str(path.relative_to(ROOT)).replace("\\", "/")


def offline_artifact_digests(evidence: dict) -> dict[str, str | None]:
    artifacts = evidence.get("artifacts")
    out: dict[str, str | None] = {
        "dxil": None,
        "root_signature": None,
        "descriptor_layout": None,
    }
    if isinstance(artifacts, dict):
        for key in out:
            entry = artifacts.get(key)
            if isinstance(entry, dict):
                sha = entry.get("sha256")
                if isinstance(sha, str):
                    out[key] = sha
    return out


def file_fingerprint(path: Path) -> dict:
    fp: dict = {
        "path": rel(path) if path.is_relative_to(ROOT) else str(path),
        "sha256": None,
        "size_bytes": None,
    }
    if path.is_file():
        fp["sha256"] = sha256_file(path)
        fp["size_bytes"] = path.stat().st_size
    return fp


def godot_exe_fingerprint(path: Path) -> dict:
    fp: dict = {
        "exe_path_at_run": str(path),
        "exe_sha256": None,
        "exe_size_bytes": None,
        "exe_mtime_utc": None,
        "committed": False,
        "scratch_build_note": (
            "Scratch Godot build binaries are NOT committed to the repo. This "
            "console exe is a local, gitignored artifact rebuilt from the "
            f"ignored external/godot-master snapshot with the full {PATCH_STACK_ID} "
            "GRX-010 patch stack applied (module_rurix_accel_enabled=yes "
            "d3d12=yes). Only its fingerprint is recorded here so the measured "
            f"evidence stays auditable; re-point {GODOT_EXE_ENV} at an "
            "equivalent rebuild to reproduce it."
        ),
    }
    if path.is_file():
        stat = path.stat()
        fp["exe_sha256"] = sha256_file(path)
        fp["exe_size_bytes"] = stat.st_size
        fp["exe_mtime_utc"] = (
            _dt.datetime.fromtimestamp(stat.st_mtime, tz=_dt.timezone.utc)
            .replace(microsecond=0)
            .isoformat()
        )
    return fp


def dll_fingerprint(path: Path) -> dict:
    fp: dict = {
        "dll_path_at_run": rel(path),
        "dll_sha256": None,
        "dll_size_bytes": None,
        "dll_mtime_utc": None,
        "build_profile": "debug",
        "features": ["d3d12-recording-shim"],
        "feature_note": (
            "Bridge built WITH the d3d12-recording-shim feature: the tonemap "
            "real-pass arm routes through the linked recording shim, so "
            "arming RXGD_CAP_TONEMAP_REAL_PASS can make rxgd_record_pass "
            "return RXGD_STATUS_OK only after a real recorded dispatch. The "
            "shipping feature-off bridge still fails closed with "
            "real_dispatch_path_not_linked. target/debug/rurix_godot.dll is "
            "a mutable build artifact."
        ),
    }
    if not path.is_file():
        return fp
    stat = path.stat()
    fp["dll_sha256"] = sha256_file(path)
    fp["dll_size_bytes"] = stat.st_size
    fp["dll_mtime_utc"] = (
        _dt.datetime.fromtimestamp(stat.st_mtime, tz=_dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
    )
    return fp


def _write_json(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    # Byte-level LF only (repo .gitattributes pins `* -text`); never emit CRLF.
    path.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def write_png_rgb8(path: Path, width: int, height: int, data: bytes) -> None:
    """Minimal stdlib PNG writer (8-bit RGB, filter 0) for human-viewable
    siblings of the canonical .rgb8 raw frame artifacts."""
    stride = width * 3
    raw = b"".join(
        b"\x00" + data[y * stride : (y + 1) * stride] for y in range(height)
    )

    def chunk(tag: bytes, payload: bytes) -> bytes:
        return (
            struct.pack(">I", len(payload))
            + tag
            + payload
            + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 6))
        + chunk(b"IEND", b"")
    )


def compute_ldr_abs_diff(reference: bytes, candidate: bytes) -> tuple[int, float, bytes]:
    diff = bytes(abs(a - b) for a, b in zip(reference, candidate))
    if not diff:
        return 0, 0.0, diff
    return max(diff), sum(diff) / len(diff), diff


def parse_blocked_marker(line: str) -> dict[str, str]:
    """Parse the key=value tokens of an RXGD_TONEMAP_REAL_PASS_BLOCKED line."""
    tokens: dict[str, str] = {}
    for part in line.split():
        if "=" in part:
            key, _, value = part.partition("=")
            tokens[key] = value
    return tokens


# Assembled at runtime so the evidence always records the exact digests/paths.
_EVIDENCE_BASE: dict = {}


def write_evidence(status: str, *, reason: str | None = None, extra: dict | None = None) -> None:
    doc = dict(_EVIDENCE_BASE)
    doc["status"] = status
    doc["timestamp"] = now_iso()
    doc["run_url"] = github_run_url()
    if reason is not None:
        doc["reason"] = reason
    if extra:
        doc.update(extra)

    _write_json(EVIDENCE_OUT, doc)
    print(
        f"[grx010-tonemap-real-pass-smoke] wrote {rel(EVIDENCE_OUT)} status={status}"
    )

    if status == "success":
        success_doc = dict(doc)
        success_doc["evidence_kind"] = "historical_measured_success"
        success_doc["latest_evidence_path"] = rel(EVIDENCE_OUT)
        success_doc["success_evidence_note"] = (
            "Historical measured success artifact for the GRX-010 tonemap "
            "real-pass enablement gate. It is written ONLY on a strict "
            "status=success run (opt-in real dispatch executed AND completed "
            "AND the LDR visual gate stayed within thresholds AND every "
            "audit passed) and is never deleted or overwritten by a later "
            "SKIP/FAIL run. Even this success keeps "
            "default_enable_state=disabled and performance_claim=none."
        )
        _write_json(SUCCESS_EVIDENCE_OUT, success_doc)
        print(
            "[grx010-tonemap-real-pass-smoke] wrote "
            f"{rel(SUCCESS_EVIDENCE_OUT)} status=success (historical measured success)"
        )


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx010-tonemap-real-pass-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip_environment(msg: str, extra: dict | None = None) -> int:
    """Environment-level SKIP: a precondition is unavailable. Upgraded to a
    hard FAIL under RURIX_REQUIRE_REAL=1."""
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx010-tonemap-real-pass-smoke] SKIP {msg}(降级 SKIP,退出 0)")
    payload = dict(extra or {})
    payload["skip_kind"] = "environment"
    write_evidence("skip", reason=msg, extra=payload)
    return 0


def skip_measured_prerequisite(prerequisite: str, msg: str, extra: dict) -> int:
    """Measured prerequisite-blocked SKIP: every leg ran on real hardware and
    the fail-closed gate reported exactly the predicted first missing
    prerequisite. This is a real measured run, so RURIX_REQUIRE_REAL does NOT
    upgrade it to FAIL; it still never advances the readiness gate."""
    print(
        "[grx010-tonemap-real-pass-smoke] SKIP (measured) first missing "
        f"prerequisite: {prerequisite} — {msg}"
    )
    payload = dict(extra)
    payload["skip_kind"] = "measured_prerequisite_blocked"
    payload["first_missing_prerequisite"] = prerequisite
    write_evidence("skip", reason=msg, extra=payload)
    return 0


def locate_godot_exe() -> tuple[Path | None, str | None]:
    override = os.environ.get(GODOT_EXE_ENV)
    if not override:
        return None, (
            f"{GODOT_EXE_ENV} is not set; the GRX-010 tonemap enablement "
            "smoke needs a scratch Godot console exe rebuilt from the "
            "ignored external/godot-master snapshot with the full "
            f"{PATCH_STACK_ID} patch stack applied "
            "(module_rurix_accel_enabled=yes d3d12=yes). The tracked "
            "external/godot-master build only has 0001+0002+0003 and must "
            "NOT be reused here"
        )
    p = Path(override)
    if not p.is_file():
        return None, f"{GODOT_EXE_ENV}={override} does not point at an existing file"
    return p, None


def build_bridge_dll() -> tuple[bool, str]:
    """Build rurix_godot.dll WITH the d3d12-recording-shim feature (the
    tonemap real-pass arm can only attempt a real dispatch when the
    recording shim is linked)."""
    p = subprocess.run(
        ["cargo", "build", "-p", "rurix-godot", "--features", "d3d12-recording-shim"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    log = (p.stdout + p.stderr).strip()
    ok = p.returncode == 0 and RURIX_GODOT_DLL.is_file()
    return ok, log[-3000:]


def load_sidecar(path: Path | None) -> tuple[dict | None, str | None]:
    if path is None:
        return None, f"{SCRATCH_SOURCE_PROVENANCE_ENV} is not set"
    if not path.is_file():
        return None, (
            f"{SCRATCH_SOURCE_PROVENANCE_ENV}={path} does not point at an existing file"
        )
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return None, f"could not load source provenance sidecar: {type(exc).__name__}: {exc}"
    if not isinstance(payload, dict):
        return None, "source provenance sidecar is not a JSON object"
    return payload, None


def scratch_source_provenance(godot_exe: Path) -> dict:
    """Audit the scratch Godot source worktree the exe was built from against
    the tracked 0001..0013 patch stack (GRX-009 segment 4f machinery,
    GRX-010 stack)."""
    override = os.environ.get(SCRATCH_SOURCE_ENV)
    source_root = None
    source_error = None
    if override:
        source = Path(override)
        if not source.is_dir():
            source_error = f"{SCRATCH_SOURCE_ENV}={override} does not point at an existing directory"
        else:
            source_root = find_git_root(source)
            if source_root is None:
                source_error = f"{SCRATCH_SOURCE_ENV}={override} is not inside a git worktree"
    else:
        source_root = find_git_root(godot_exe)
        if source_root is None:
            source_error = (
                f"cannot locate scratch Godot source root from {godot_exe}; set "
                f"{SCRATCH_SOURCE_ENV} to the full-stack Godot source worktree"
            )

    exe_fp = godot_exe_fingerprint(godot_exe)
    build_command = os.environ.get(SCRATCH_BUILD_COMMAND_ENV)
    build_log = os.environ.get(SCRATCH_BUILD_LOG_ENV)
    provenance: dict = {
        "base_snapshot": "external/godot-master",
        "source_root_at_run": str(source_root) if source_root is not None else None,
        "source_clean": False,
        "source_status": [],
        "tracked_patch_stack_only": False,
        "source_audit_supported": False,
        "source_audit_errors": [],
        "source_provenance_sidecar_path": None,
        "applied_patch_stack": patch_stack_identity(PATCH_STACK_GRX010, PATCH_STACK_ID),
        "godot_exe": {
            "path_at_run": exe_fp.get("exe_path_at_run"),
            "sha256": exe_fp.get("exe_sha256"),
            "size_bytes": exe_fp.get("exe_size_bytes"),
            "mtime_utc": exe_fp.get("exe_mtime_utc"),
        },
        "build": {
            "available": bool(build_command or build_log),
            "command": build_command,
            "log_path": build_log,
        },
    }
    if source_error is not None or source_root is None:
        provenance["source_status"] = [source_error or "scratch source root unavailable"]
        provenance["source_audit_errors"] = provenance["source_status"]
        return provenance
    clean, status_lines = source_status_clean(source_root)
    provenance["source_clean"] = clean
    provenance["source_status"] = status_lines
    sidecar_env = os.environ.get(SCRATCH_SOURCE_PROVENANCE_ENV)
    sidecar_path = Path(sidecar_env) if sidecar_env else None
    sidecar, sidecar_error = load_sidecar(sidecar_path)
    ok, errors, audit = verify_source_provenance_sidecar(
        sidecar,
        source_root,
        stack_names=PATCH_STACK_GRX010,
        stack_id=PATCH_STACK_ID,
        sidecar_path=sidecar_path,
    )
    if sidecar_error is not None:
        errors.insert(0, sidecar_error)
    provenance.update(audit)
    provenance["source_audit_errors"] = errors
    provenance["tracked_patch_stack_only"] = clean and ok
    provenance["source_audit_supported"] = clean and ok
    return provenance


def write_smoke_project(
    project_dir: Path,
    *,
    dll_path: Path,
    pass_enabled: bool,
    dispatch_real_pass: bool,
    force_capability_downgrade: bool,
) -> None:
    """Generate a minimal deterministic Godot project. Only the tracked
    tonemap per-pass opt-in settings differ between legs; everything else is
    byte-identical so the opt-in matrix is the only delta. The tonemap
    dispatch_recording_smoke opt-in stays false in EVERY leg (the RECORD
    marker must never appear)."""
    project_dir.mkdir(parents=True, exist_ok=True)

    def flag(value: bool) -> str:
        return "true" if value else "false"

    project_text = f"""\
; Engine configuration file.
; Auto-generated by ci/grx010_tonemap_real_pass_enablement_smoke.py

config_version=5

[application]

config/name="GRX-010 tonemap real-pass enablement smoke"
run/main_scene="res://main.tscn"

[display]

window/size/viewport_width={VIEWPORT_WIDTH}
window/size/viewport_height={VIEWPORT_HEIGHT}

[rendering]

rurix_accel/enabled=true
rurix_accel/require_forward_plus=true
rurix_accel/dll_path="{dll_path.as_posix()}"
rurix_accel/passes/tonemap/enabled={flag(pass_enabled)}
rurix_accel/passes/tonemap/dispatch_recording_smoke=false
rurix_accel/passes/tonemap/dispatch_real_pass={flag(dispatch_real_pass)}
rurix_accel/passes/tonemap/real_pass_force_capability_downgrade={flag(force_capability_downgrade)}
"""
    scene_text = """\
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://main.gd" id="1"]

[node name="GRX010TonemapRoot" type="Node3D"]
script = ExtResource("1")

[node name="Camera3D" type="Camera3D" parent="."]

[node name="WorldEnvironment" type="WorldEnvironment" parent="."]
"""
    # Deterministic flat-color scene with an explicit tonemapper so the
    # Tonemap call site runs every frame, then a frame capture at a fixed
    # frame index (with --fixed-fps the state at that frame is deterministic
    # across runs).
    script_text = f"""\
extends Node3D

var _frames := 0
var _captured := false

func _ready() -> void:
    var cam: Camera3D = $Camera3D
    cam.make_current()

    var env := Environment.new()
    env.background_mode = Environment.BG_COLOR
    env.background_color = Color(0.6, 0.45, 0.3)
    env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
    $WorldEnvironment.environment = env
    print("GRX010Tonemap: scene ready")

func _process(_delta: float) -> void:
    _frames += 1
    if _frames >= {CAPTURE_FRAME_INDEX} and not _captured:
        _captured = true
        _capture()

func _capture() -> void:
    await RenderingServer.frame_post_draw
    var img: Image = get_viewport().get_texture().get_image()
    img.convert(Image.FORMAT_RGB8)
    var prefix := OS.get_environment("{CAPTURE_PREFIX_ENV}")
    if prefix.is_empty():
        printerr("GRX010Tonemap: capture prefix env var missing")
        get_tree().quit(3)
        return
    var raw := FileAccess.open(prefix + ".rgb8", FileAccess.WRITE)
    raw.store_buffer(img.get_data())
    raw.close()
    img.save_png(prefix + ".png")
    var meta := FileAccess.open(prefix + ".json", FileAccess.WRITE)
    meta.store_string(JSON.stringify({{
        "width": img.get_width(),
        "height": img.get_height(),
        "format": "{FRAME_FORMAT}",
        "capture_frame_index": _frames,
    }}))
    meta.close()
    print("GRX010Tonemap: captured frame=%d width=%d height=%d" % [_frames, img.get_width(), img.get_height()])
    get_tree().quit()
"""
    (project_dir / "project.godot").write_text(project_text, encoding="utf-8", newline="\n")
    (project_dir / "main.tscn").write_text(scene_text, encoding="utf-8", newline="\n")
    (project_dir / "main.gd").write_text(script_text, encoding="utf-8", newline="\n")


def run_godot(godot_exe: Path, project_dir: Path, capture_prefix: Path, log_name: str) -> tuple[int, str]:
    command = [
        str(godot_exe),
        "--path",
        str(project_dir),
        "--rendering-driver",
        REQUESTED_RENDERER,
        "--rendering-method",
        REQUESTED_RENDERING_METHOD,
        "--fixed-fps",
        "60",
        "--verbose",
    ]
    env = dict(os.environ)
    env[CAPTURE_PREFIX_ENV] = str(capture_prefix)
    # GRX Wave 4: keep this enablement gate on the INSTRUMENTED real-pass path
    # (per-dispatch readback + RXGD_GODOT_RUNTIME_*_REAL_PASS stdout marker) so
    # the strict-success marker/evidence semantics are preserved. The bench
    # runner leaves this unset, so its measured real-pass path stays production
    # (zero per-dispatch readback/stdout).
    env["RXGD_DISPATCH_INSTRUMENTED"] = "1"
    try:
        proc = subprocess.run(
            command,
            cwd=project_dir,
            text=True,
            capture_output=True,
            check=False,
            timeout=GODOT_TIMEOUT_SECONDS,
            env=env,
        )
    except subprocess.TimeoutExpired as exc:
        out = ""
        if isinstance(exc.stdout, str):
            out += exc.stdout
        if isinstance(exc.stderr, str):
            out += exc.stderr
        return -1, out.strip()
    output = "\n".join(part for part in (proc.stdout, proc.stderr) if part).strip()
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    (LOG_DIR / log_name).write_text(output + "\n", encoding="utf-8", newline="\n")
    return proc.returncode, output


def load_capture(capture_prefix: Path) -> tuple[dict | None, bytes | None, str | None]:
    meta = load_json(Path(str(capture_prefix) + ".json"))
    raw_path = Path(str(capture_prefix) + ".rgb8")
    if meta is None:
        return None, None, f"capture metadata missing/unreadable at {capture_prefix}.json"
    if not raw_path.is_file():
        return meta, None, f"raw frame missing at {raw_path}"
    width = meta.get("width")
    height = meta.get("height")
    if (
        not isinstance(width, int)
        or not isinstance(height, int)
        or width < MIN_FRAME_DIMENSION
        or height < MIN_FRAME_DIMENSION
    ):
        return meta, None, (
            f"captured frame dimensions {width}x{height} are malformed or below "
            f"the {MIN_FRAME_DIMENSION}px minimum"
        )
    data = raw_path.read_bytes()
    if len(data) != width * height * 3:
        return meta, None, (
            f"raw frame size {len(data)} does not equal width*height*3 "
            f"({width}x{height}x3={width * height * 3})"
        )
    return meta, data, None


LEG_SETTINGS = {
    "reference": {
        "pass_enabled": False,
        "dispatch_real_pass": False,
        "force_capability_downgrade": False,
    },
    "candidate": {
        "pass_enabled": True,
        "dispatch_real_pass": True,
        "force_capability_downgrade": False,
    },
    "forced_fallback": {
        "pass_enabled": True,
        "dispatch_real_pass": True,
        "force_capability_downgrade": True,
    },
}

LEG_ROLES = {
    "reference": "reference",
    "candidate": "enabled_real_pass_optin",
    "forced_fallback": "forced_capability_downgrade",
}


def run_matrix_leg(godot_exe: Path, *, leg: str, dll_path: Path) -> dict:
    settings = LEG_SETTINGS[leg]
    project_dir = WORK / f"project_{leg}"
    capture_prefix = WORK / f"capture_{leg}"
    for suffix in (".rgb8", ".png", ".json"):
        Path(str(capture_prefix) + suffix).unlink(missing_ok=True)
    write_smoke_project(project_dir, dll_path=dll_path, **settings)
    exit_code, output = run_godot(
        godot_exe, project_dir, capture_prefix, f"godot_{leg}.log"
    )
    fallback_lines = [
        line.strip() for line in output.splitlines() if FALLBACK_MARKER in line
    ]
    blocked_lines = [
        line.strip()
        for line in output.splitlines()
        if REAL_PASS_BLOCKED_MARKER in line
    ]
    # The real-pass marker prints once per dispatched frame; pin the first
    # line from the FULL output (the writeback scaffold marker contains the
    # real-pass marker as a prefix, so it is excluded here).
    real_pass_lines = [
        line.strip()
        for line in output.splitlines()
        if REAL_PASS_MARKER in line and WRITEBACK_MARKER not in line
    ]
    meta, data, capture_error = load_capture(capture_prefix)
    return {
        "leg": leg,
        "role": LEG_ROLES[leg],
        "project_settings": {
            f"{PASS_SETTING_PREFIX}/enabled": settings["pass_enabled"],
            f"{PASS_SETTING_PREFIX}/dispatch_recording_smoke": False,
            f"{PASS_SETTING_PREFIX}/dispatch_real_pass": settings["dispatch_real_pass"],
            f"{PASS_SETTING_PREFIX}/real_pass_force_capability_downgrade": settings[
                "force_capability_downgrade"
            ],
        },
        "exit_code": exit_code,
        "session_ready": SESSION_READY_MARKER in output,
        "bridge_fallback_marker_observed": bool(fallback_lines),
        "bridge_fallback_marker_line": fallback_lines[0] if fallback_lines else None,
        "real_pass_blocked_marker_observed": bool(blocked_lines),
        "real_pass_blocked_marker_line": blocked_lines[0] if blocked_lines else None,
        "real_pass_marker_observed": REAL_PASS_MARKER in output,
        "real_pass_marker_line": real_pass_lines[0] if real_pass_lines else None,
        "writeback_marker_observed": WRITEBACK_MARKER in output,
        "record_marker_observed": RECORD_MARKER in output,
        "capture_meta": meta,
        "capture_error": capture_error,
        "capture_prefix": capture_prefix,
        "frame_bytes": data,
        "runtime_log_audit": runtime_log_audit(output, PATCH_STACK_GRX010),
        "stdout_tail": output[-4000:],
    }


def leg_public(leg: dict) -> dict:
    return {
        "role": leg["role"],
        "project_settings": leg["project_settings"],
        "exit_code": leg["exit_code"],
        "session_ready": leg["session_ready"],
        "bridge_fallback_marker_observed": leg["bridge_fallback_marker_observed"],
        "bridge_fallback_marker_line": leg["bridge_fallback_marker_line"],
        "real_pass_blocked_marker_observed": leg["real_pass_blocked_marker_observed"],
        "real_pass_blocked_marker_line": leg["real_pass_blocked_marker_line"],
        "real_pass_marker_observed": leg["real_pass_marker_observed"],
        "writeback_marker_observed": leg["writeback_marker_observed"],
        "record_marker_observed": leg["record_marker_observed"],
        "capture_meta": leg["capture_meta"],
        "capture_error": leg["capture_error"],
    }


def telemetry_entries_issue(
    doc: dict, capture_frame_index: int, *, expect_candidate_fallback: bool = True
) -> str | None:
    """First incoherence in the generated GRX-010 telemetry entries, or None.

    The GRX-008 format records FALLBACK telemetry: on the measured
    prerequisite-blocked outcome both armed legs fell back (candidate entry
    ``validation_failed`` + forced entry ``unsupported_device``); on a strict
    real-pass success only the forced leg falls back, so the document must
    NOT carry a candidate fallback entry."""
    passes = doc.get("passes")
    expected_count = 2 if expect_candidate_fallback else 1
    if not isinstance(passes, list) or len(passes) != expected_count:
        return (
            f"telemetry document must carry exactly {expected_count} "
            "tonemap fallback entries"
        )
    by_leg = {
        entry.get("leg"): entry for entry in passes if isinstance(entry, dict)
    }
    if not expect_candidate_fallback and "enabled_real_pass_optin" in by_leg:
        return (
            "telemetry document carries a candidate fallback entry although "
            "the real pass succeeded; the outcome is contradictory"
        )
    expectations = {
        "forced_capability_downgrade": EXPECTED_FORCED_FALLBACK_REASON,
    }
    if expect_candidate_fallback:
        expectations["enabled_real_pass_optin"] = EXPECTED_BLOCKED_FALLBACK_REASON
    for leg_name, expected_reason in expectations.items():
        entry = by_leg.get(leg_name)
        if entry is None:
            return f"telemetry document has no {leg_name} entry"
        if entry.get("pass_id") != "tonemap":
            return f"{leg_name} entry pass_id is not tonemap"
        if entry.get("enable_state") != "enabled":
            return f"{leg_name} entry enable_state is not 'enabled'"
        if entry.get("fallback_reason") != expected_reason:
            return (
                f"{leg_name} entry fallback_reason is "
                f"{entry.get('fallback_reason')!r}, not {expected_reason!r}"
            )
        if entry.get("godot_fallback_active") is not True:
            return f"{leg_name} entry godot_fallback_active is not true"
        telemetry_frame = entry.get("telemetry_frame")
        if (
            not isinstance(telemetry_frame, int)
            or isinstance(telemetry_frame, bool)
            or telemetry_frame != capture_frame_index
        ):
            return (
                f"{leg_name} entry telemetry_frame {telemetry_frame!r} is stale: "
                f"it does not equal the measured capture_frame_index {capture_frame_index}"
            )
    return None


def main() -> int:
    global _EVIDENCE_BASE

    for path in (DXIL, RTS0, DESCRIPTOR_LAYOUT, OFFLINE_EVIDENCE):
        if not path.is_file():
            _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
            return fail(f"required artifact missing: {rel(path)}")

    dxil_sha = sha256_file(DXIL)
    rts0_sha = sha256_file(RTS0)
    layout_sha = sha256_file(DESCRIPTOR_LAYOUT)
    offline = load_json(OFFLINE_EVIDENCE)
    if offline is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read offline_compile_evidence.json")

    offline_digests = offline_artifact_digests(offline)
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "tonemap",
        "segment": "grx010_real_pass_enablement",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "godot_runtime_tonemap_path_enabled": False,
        "default_enable_state": "disabled",
        "gpu_timestamp_status": "not_yet",
        "performance_claim": "none",
        "target_backend": TARGET_BACKEND,
        "expected_first_missing_prerequisite": EXPECTED_FIRST_MISSING_PREREQUISITE,
        "known_gaps": KNOWN_GAPS,
        "artifacts": {
            "dxil": {"path": rel(DXIL), "sha256": dxil_sha},
            "root_signature": {"path": rel(RTS0), "sha256": rts0_sha},
            "descriptor_layout": {"path": rel(DESCRIPTOR_LAYOUT), "sha256": layout_sha},
        },
        "offline_evidence": {
            "path": rel(OFFLINE_EVIDENCE),
            "dxil_sha256": offline_digests["dxil"],
            "root_signature_sha256": offline_digests["root_signature"],
            "descriptor_layout_sha256": offline_digests["descriptor_layout"],
        },
        "artifact_hashes_match_offline_evidence": (
            dxil_sha == offline_digests["dxil"]
            and rts0_sha == offline_digests["root_signature"]
            and layout_sha == offline_digests["descriptor_layout"]
        ),
        "note": (
            "GRX-010 tonemap gated real-pass enablement gate evidence. The "
            "opt-in real-pass arm (dispatch_real_pass, default false) runs "
            "against the texture-capable hlsl_bridge workaround canonical "
            "package (per-slot texture2d/rwtexture2d binding kinds, "
            "owner-approved hlsl_bridge_workaround provenance, LINEAR + "
            "linear_to_srgb math parity CPU-proven pending GPU) and a "
            "d3d12-recording-shim bridge DLL, so every software gate can "
            "pass and a real recorded dispatch may return RXGD_STATUS_OK; "
            "when the dispatch cannot complete the gate reports "
            "first_missing_prerequisite=real_dispatch_recording_failed "
            "instead of claiming success. The patch 0013 result writeback is "
            "a SCAFFOLD (native Godot tonemapper stays the continuation/"
            "backstop), default_enable_state stays disabled, and no "
            "performance, FPS, or GPU-timestamp claim is made."
        ),
    }

    if not _EVIDENCE_BASE["artifact_hashes_match_offline_evidence"]:
        return fail(
            "artifact SHA-256 does not match tracked offline compile evidence "
            f"(dxil={dxil_sha} vs {offline_digests['dxil']}, "
            f"rts0={rts0_sha} vs {offline_digests['root_signature']}, "
            f"layout={layout_sha} vs {offline_digests['descriptor_layout']})"
        )

    godot_exe, godot_reason = locate_godot_exe()
    if godot_exe is None:
        return skip_environment(godot_reason or "GRX-010 tonemap Godot exe unavailable")

    built_dll, dll_log = build_bridge_dll()
    if not built_dll:
        print(dll_log, file=sys.stderr)
        return fail(
            "cargo build -p rurix-godot --features d3d12-recording-shim failed",
            extra={"build_log_tail": dll_log},
        )
    _EVIDENCE_BASE["dll_fingerprint"] = dll_fingerprint(RURIX_GODOT_DLL)
    _EVIDENCE_BASE["godot_exe_fingerprint"] = godot_exe_fingerprint(godot_exe)
    _EVIDENCE_BASE["patch_stack_identity"] = patch_stack_identity(
        PATCH_STACK_GRX010, PATCH_STACK_ID
    )

    provenance = scratch_source_provenance(godot_exe)
    _EVIDENCE_BASE["scratch_source_provenance"] = provenance
    if provenance.get("tracked_patch_stack_only") is not True:
        return skip_environment(
            "scratch Godot source provenance is not auditable as tracked-patch-"
            f"stack-only ({PATCH_STACK_ID}); errors: "
            + "; ".join(str(e) for e in provenance.get("source_audit_errors", []))[:1200]
        )

    WORK.mkdir(parents=True, exist_ok=True)
    reference = run_matrix_leg(godot_exe, leg="reference", dll_path=RURIX_GODOT_DLL)
    candidate = run_matrix_leg(godot_exe, leg="candidate", dll_path=RURIX_GODOT_DLL)
    forced = run_matrix_leg(godot_exe, leg="forced_fallback", dll_path=RURIX_GODOT_DLL)
    legs = {"reference": reference, "candidate": candidate, "forced_fallback": forced}
    matrix = {
        "disabled_default": leg_public(reference),
        "enabled_real_pass_optin": leg_public(candidate),
        "forced_capability_downgrade": leg_public(forced),
    }
    runs_extra = {
        "pass_enable_matrix": matrix,
        "stdout_reference": reference["stdout_tail"],
        "stdout_candidate": candidate["stdout_tail"],
        "stdout_forced_fallback": forced["stdout_tail"],
        "runtime_log_audit": {
            name: leg["runtime_log_audit"] for name, leg in legs.items()
        },
    }

    # Environment-level outcomes are SKIP (they do not advance the gate);
    # integrity/coherence violations after a working session are FAIL.
    for name, leg in legs.items():
        if leg["exit_code"] == -1:
            return skip_environment(
                f"Godot {name} run timed out after {GODOT_TIMEOUT_SECONDS}s",
                extra=runs_extra,
            )
    if not all(leg["session_ready"] for leg in legs.values()):
        return skip_environment(
            "Rurix bridge session was not ready in this environment (no "
            f"'{SESSION_READY_MARKER}' marker); no D3D12 Forward+ session, so "
            "the real-pass enablement matrix cannot be measured",
            extra=runs_extra,
        )

    for name, leg in legs.items():
        if leg["exit_code"] != 0:
            return fail(
                f"Godot {name} run exited with non-zero exit code "
                f"{leg['exit_code']}; a clean enablement smoke requires exit 0",
                extra=runs_extra,
            )
        audit = leg["runtime_log_audit"]
        if (
            audit.get("unexpected_godot_error_count") != 0
            or audit.get("unexpected_rxgd_diag_count") != 0
        ):
            return fail(
                f"{name} run output contained unexpected Godot ERROR / "
                f"RXGD_DIAG lines (only the known '{ALLOWED_GODOT_ERROR}' "
                "warning is tolerated): "
                f"{audit.get('unexpected_lines_tail')}",
                extra=runs_extra,
            )
        if leg["record_marker_observed"]:
            return fail(
                f"{name} run printed the tonemap recording-smoke marker "
                f"'{RECORD_MARKER}'; the dispatch_recording_smoke opt-in must "
                "stay off in the GRX-010 enablement matrix",
                extra=runs_extra,
            )
        if leg["capture_error"] is not None or leg["frame_bytes"] is None:
            return fail(
                f"{name} frame capture failed: {leg['capture_error']}",
                extra=runs_extra,
            )

    # Reference leg: no bridge invocation, no markers of any kind.
    for marker_key, marker_name in (
        ("bridge_fallback_marker_observed", FALLBACK_MARKER),
        ("real_pass_blocked_marker_observed", REAL_PASS_BLOCKED_MARKER),
        ("real_pass_marker_observed", REAL_PASS_MARKER),
        ("writeback_marker_observed", WRITEBACK_MARKER),
    ):
        if reference[marker_key]:
            return fail(
                "reference run (all tonemap per-pass settings at their false "
                f"defaults) unexpectedly printed '{marker_name}'; the "
                "disabled pass must never invoke the bridge",
                extra=runs_extra,
            )

    # Forced-failure red leg: fallback + blocked marker with the forced
    # capability-downgrade shape; never a real pass or a writeback.
    if forced["real_pass_marker_observed"] or forced["writeback_marker_observed"]:
        return fail(
            "forced_capability_downgrade run printed a real-pass/writeback "
            "marker; the downgraded device capability must fail closed",
            extra=runs_extra,
        )
    if not forced["bridge_fallback_marker_observed"]:
        return fail(
            "forced_capability_downgrade run did not print the tracked "
            f"fallback marker '{FALLBACK_MARKER}'; the fallback path was not "
            "measured",
            extra=runs_extra,
        )
    if not forced["real_pass_blocked_marker_observed"]:
        return fail(
            "forced_capability_downgrade run did not print the "
            f"'{REAL_PASS_BLOCKED_MARKER}' diagnostic",
            extra=runs_extra,
        )
    forced_tokens = parse_blocked_marker(forced["real_pass_blocked_marker_line"] or "")
    if (
        forced_tokens.get("first_missing_prerequisite") != EXPECTED_FORCED_PREREQUISITE
        or forced_tokens.get("fallback_reason") != EXPECTED_FORCED_FALLBACK_REASON
    ):
        return fail(
            "forced_capability_downgrade blocked diagnostic did not record the "
            f"forced shape (expected {EXPECTED_FORCED_PREREQUISITE}/"
            f"{EXPECTED_FORCED_FALLBACK_REASON}, got "
            f"{forced['real_pass_blocked_marker_line']!r})",
            extra=runs_extra,
        )

    # Candidate leg: either the strict real-pass success shape, or the
    # predicted fail-closed blocked shape. Anything else is a FAIL.
    real_pass_success = candidate["real_pass_marker_observed"]
    candidate_tokens = parse_blocked_marker(
        candidate["real_pass_blocked_marker_line"] or ""
    )
    if real_pass_success:
        if candidate["real_pass_blocked_marker_observed"]:
            return fail(
                "candidate run printed BOTH the real-pass marker and the "
                "blocked diagnostic; the gate outcome is ambiguous",
                extra=runs_extra,
            )
        if candidate["bridge_fallback_marker_observed"]:
            return fail(
                "candidate run printed both the real-pass marker and the "
                "fallback marker; the gate outcome is ambiguous",
                extra=runs_extra,
            )
        if not candidate["writeback_marker_observed"]:
            return fail(
                "candidate run printed the real-pass marker but not the "
                f"'{WRITEBACK_MARKER}' scaffold marker; the native "
                "continuation/backstop was not recorded",
                extra=runs_extra,
            )
    else:
        if candidate["writeback_marker_observed"]:
            return fail(
                "candidate run printed the writeback scaffold marker without "
                "a real pass; the gate outcome is contradictory",
                extra=runs_extra,
            )
        if not candidate["bridge_fallback_marker_observed"]:
            return fail(
                "candidate run (real-pass opt-in armed) did not print the "
                f"tracked fallback marker '{FALLBACK_MARKER}'; the fallback "
                "path was not measured",
                extra=runs_extra,
            )
        if not candidate["real_pass_blocked_marker_observed"]:
            return fail(
                "candidate run (real-pass opt-in armed) did not print the "
                f"'{REAL_PASS_BLOCKED_MARKER}' diagnostic naming the first "
                "missing prerequisite",
                extra=runs_extra,
            )
        if (
            candidate_tokens.get("first_missing_prerequisite")
            != EXPECTED_FIRST_MISSING_PREREQUISITE
            or candidate_tokens.get("fallback_reason")
            != EXPECTED_BLOCKED_FALLBACK_REASON
        ):
            return fail(
                "candidate blocked diagnostic did not record the predicted "
                f"first missing prerequisite (expected "
                f"{EXPECTED_FIRST_MISSING_PREREQUISITE}/"
                f"{EXPECTED_BLOCKED_FALLBACK_REASON}, got "
                f"{candidate['real_pass_blocked_marker_line']!r}); the "
                "fail-closed chain is not in the tracked state",
                extra=runs_extra,
            )

    # Frame coherence across all three legs.
    ref_meta = reference["capture_meta"]
    for name in ("candidate", "forced_fallback"):
        meta = legs[name]["capture_meta"]
        if (
            ref_meta.get("width") != meta.get("width")
            or ref_meta.get("height") != meta.get("height")
        ):
            return fail(
                f"reference/{name} frame dimensions mismatch "
                f"({ref_meta.get('width')}x{ref_meta.get('height')} vs "
                f"{meta.get('width')}x{meta.get('height')})",
                extra=runs_extra,
            )
    width = int(ref_meta["width"])
    height = int(ref_meta["height"])
    frame_indices = {
        name: leg["capture_meta"].get("capture_frame_index")
        for name, leg in legs.items()
    }
    unique_indices = set(frame_indices.values())
    if len(unique_indices) != 1 or not all(
        isinstance(v, int) and not isinstance(v, bool) and v >= 1
        for v in frame_indices.values()
    ):
        return fail(
            "measured capture frame indices are malformed or do not match "
            f"across legs ({frame_indices!r})",
            extra=runs_extra,
        )
    capture_frame_index = int(next(iter(unique_indices)))

    # LDR absolute diff of each armed leg against the native reference.
    diffs: dict[str, dict] = {}
    diff_bytes_by_leg: dict[str, bytes] = {}
    for name in ("candidate", "forced_fallback"):
        max_abs, mean_abs, diff_bytes = compute_ldr_abs_diff(
            reference["frame_bytes"], legs[name]["frame_bytes"]
        )
        within = (
            max_abs <= LDR_MAX_ABS_DIFF_THRESHOLD
            and mean_abs <= LDR_MEAN_ABS_DIFF_THRESHOLD
        )
        diffs[name] = {
            "max_abs_diff": max_abs,
            "mean_abs_diff": mean_abs,
            "within_threshold": within,
        }
        diff_bytes_by_leg[name] = diff_bytes
        print(
            f"[grx010-tonemap-real-pass-smoke] LDR absolute diff ({name} vs "
            f"reference) max_abs={max_abs} mean_abs={mean_abs:.6f} "
            f"(thresholds max<={LDR_MAX_ABS_DIFF_THRESHOLD} "
            f"mean<={LDR_MEAN_ABS_DIFF_THRESHOLD})"
        )
    visual = {
        "measured_local": True,
        "metric_kind": METRIC_KIND,
        "width": width,
        "height": height,
        "format": FRAME_FORMAT,
        "capture_frame_index": capture_frame_index,
        "max_abs_diff_threshold": LDR_MAX_ABS_DIFF_THRESHOLD,
        "mean_abs_diff_threshold": LDR_MEAN_ABS_DIFF_THRESHOLD,
        "reference_frame": file_fingerprint(Path(str(reference["capture_prefix"]) + ".rgb8")),
        "candidate_frame": file_fingerprint(Path(str(candidate["capture_prefix"]) + ".rgb8")),
        "forced_fallback_frame": file_fingerprint(
            Path(str(forced["capture_prefix"]) + ".rgb8")
        ),
        "diffs": diffs,
        "frame_artifact_note": (
            "Frame artifacts live in the local work dir and are hash-pinned "
            "here; they are committed under artifacts/visual/ ONLY on a "
            "strict status=success run."
        ),
    }
    runs_extra["visual"] = visual
    if not all(entry["within_threshold"] for entry in diffs.values()):
        return fail(
            "LDR absolute diff exceeded the visual gate threshold: arming the "
            "fail-closed real-pass opt-in (or the forced downgrade knob) "
            f"changed the rendered image ({diffs!r})",
            extra=runs_extra,
        )

    # GRX-008-format measured telemetry: candidate + forced fallback entries.
    telemetry_doc = {
        "run_mode": "full",
        "evidence_level": "measured_local",
        "target_backend": TARGET_BACKEND,
        "note": (
            "GRX-010 measured tonemap real-pass enablement telemetry: with "
            "the default-false dispatch_real_pass opt-in explicitly enabled, "
            "the tracked Godot Tonemap call site invoked the Rurix bridge "
            "through the patch 0012 native resource handle binding; every "
            "fallback entry records the fail-closed gate outcome "
            "(validation_failed when the linked real dispatch cannot "
            "complete; unsupported_device under the forced capability "
            "downgrade) while the native Godot tonemapper rendered every "
            "frame. The patch 0013 writeback is a scaffold (native "
            "continuation active), runtime_state stays fallback_only for the "
            "default path, and no performance or FPS claim is made."
        ),
        "passes": (
            []
            if real_pass_success
            else [
                {
                    "pass_id": "tonemap",
                    "leg": "enabled_real_pass_optin",
                    "enable_state": "enabled",
                    "fallback_reason": EXPECTED_BLOCKED_FALLBACK_REASON,
                    "godot_fallback_active": True,
                    "telemetry_timestamp": now_iso(),
                    "telemetry_frame": capture_frame_index,
                }
            ]
        )
        + [
            {
                "pass_id": "tonemap",
                "leg": "forced_capability_downgrade",
                "enable_state": "enabled",
                "fallback_reason": EXPECTED_FORCED_FALLBACK_REASON,
                "godot_fallback_active": True,
                "telemetry_timestamp": now_iso(),
                "telemetry_frame": capture_frame_index,
            }
        ],
    }
    entries_issue = telemetry_entries_issue(
        telemetry_doc,
        capture_frame_index,
        expect_candidate_fallback=not real_pass_success,
    )
    if entries_issue is not None:
        return fail(
            f"generated measured telemetry entries are incoherent: {entries_issue}",
            extra=runs_extra,
        )
    work_telemetry = WORK / "real_pass_enablement_telemetry.json"
    _write_json(work_telemetry, telemetry_doc)
    telemetry_check = subprocess.run(
        [sys.executable, str(FALLBACK_TELEMETRY_SCRIPT), "--validate-only", str(work_telemetry)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if telemetry_check.returncode != 0:
        return fail(
            "generated measured telemetry failed fallback_telemetry.py --validate-only",
            extra={
                **runs_extra,
                "telemetry_validation_output": (
                    (telemetry_check.stdout + telemetry_check.stderr).strip()[-2000:]
                ),
            },
        )
    shutil.copy2(work_telemetry, TELEMETRY_OUT)
    telemetry_section = {
        "fallback_path_observed": True,
        "bridge_fallback_marker": FALLBACK_MARKER,
        "real_pass_blocked_marker": REAL_PASS_BLOCKED_MARKER,
        "candidate_blocked_marker_line": candidate["real_pass_blocked_marker_line"],
        "forced_blocked_marker_line": forced["real_pass_blocked_marker_line"],
        "telemetry_document": file_fingerprint(TELEMETRY_OUT),
        "no_fps_claim": True,
    }

    checks = {
        "artifact_hashes_match_offline_evidence": True,
        "reference_run_exit_zero": reference["exit_code"] == 0,
        "candidate_run_exit_zero": candidate["exit_code"] == 0,
        "forced_fallback_run_exit_zero": forced["exit_code"] == 0,
        "session_ready_all_runs": True,
        "markers_absent_reference": True,
        "fallback_marker_observed_candidate": candidate["bridge_fallback_marker_observed"],
        "fallback_marker_observed_forced_fallback": True,
        "real_pass_blocked_marker_observed_candidate": candidate[
            "real_pass_blocked_marker_observed"
        ],
        "real_pass_blocked_marker_observed_forced_fallback": True,
        "record_marker_absent_all_runs": True,
        "frames_captured": True,
        "dimensions_match": True,
        "capture_frame_indices_match": True,
        "runtime_log_audit_clean": True,
        "diff_within_threshold_candidate": diffs["candidate"]["within_threshold"],
        "diff_within_threshold_forced_fallback": diffs["forced_fallback"][
            "within_threshold"
        ],
        "telemetry_document_valid": True,
        "telemetry_entries_coherent": True,
        "scratch_source_provenance_ok": True,
        "native_continuation_writeback_scaffold": candidate["writeback_marker_observed"],
        "real_pass_dispatched_and_completed": real_pass_success,
    }
    measured_extra = {
        **runs_extra,
        "fallback_telemetry": telemetry_section,
        "checks": checks,
    }

    if real_pass_success:
        # Strict success: the opt-in real dispatch executed and completed AND
        # the visual gate stayed green. Publish the tracked frame artifacts
        # and flip real_gpu_pass=true in THIS evidence only;
        # default_enable_state stays disabled and no performance claim exists.
        VISUAL_DIR.mkdir(parents=True, exist_ok=True)
        REFERENCE_FRAME.write_bytes(reference["frame_bytes"])
        CANDIDATE_FRAME.write_bytes(candidate["frame_bytes"])
        DIFF_ARTIFACT.write_bytes(diff_bytes_by_leg["candidate"])
        write_png_rgb8(
            Path(str(DIFF_ARTIFACT) + ".png"), width, height, diff_bytes_by_leg["candidate"]
        )
        visual["reference_frame"] = file_fingerprint(REFERENCE_FRAME)
        visual["candidate_frame"] = file_fingerprint(CANDIDATE_FRAME)
        visual["diff_artifact"] = file_fingerprint(DIFF_ARTIFACT)
        success_extra = dict(measured_extra)
        success_extra["visual"] = visual
        success_extra["real_gpu_pass"] = True
        success_extra["real_d3d12_dispatch_recorded"] = True
        success_extra["real_pass_marker_line"] = candidate["real_pass_marker_line"]
        write_evidence("success", extra=success_extra)
        print(
            "[grx010-tonemap-real-pass-smoke] PASS measured opt-in real pass "
            "+ LDR visual gate within threshold (default enablement "
            "unchanged; no performance claim)"
        )
        return 0

    return skip_measured_prerequisite(
        candidate_tokens.get(
            "first_missing_prerequisite", EXPECTED_FIRST_MISSING_PREREQUISITE
        ),
        "the opt-in real-pass gate measured the predicted fail-closed shape on "
        "real hardware: every software gate passed against the texture-capable "
        "hlsl_bridge canonical tonemap package, but the linked real dispatch "
        "recording did not complete in this environment (e.g. no signed DXC "
        "dxil.dll or a D3D12 recording failure), so the gate honestly reports "
        "real_dispatch_recording_failed instead of claiming success",
        measured_extra,
    )


if __name__ == "__main__":
    sys.exit(main())
