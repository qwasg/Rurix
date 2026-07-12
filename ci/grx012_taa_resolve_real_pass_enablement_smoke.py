#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-012: gated real taa_resolve pass enablement smoke (temporal).

This harness drives the opt-in real-pass enablement gate for the GRX-012
taa_resolve path. It is a strict, fail-closed bring-up gate, NOT a default
enablement: ``rendering/rurix_accel/passes/taa_resolve/enabled`` stays
``false`` by default, the ``.../taa_resolve/dispatch_real_pass`` opt-in also
defaults to ``false``, and NO FPS, GPU-timestamp, or performance claim is
made anywhere in this gate. Template copy of
``ci/grx011_ssao_blur_real_pass_enablement_smoke.py`` pointed at the
taa_resolve pass and the 0001..0022 patch stack, extended with the GRX_PLAN
temporal DoD hard constraint (a real TAA resolve is a temporal accumulation,
so a single-frame screenshot may NOT stand in for the temporal evidence).

What it measures, honestly, against the scratch Godot console exe rebuilt with
the full 0001..0022 patch stack (``RURIX_GRX012_TAA_RESOLVE_GODOT_EXE``) and a
``rurix_godot.dll`` built WITH the ``d3d12-recording-shim`` feature (the
real-pass arm routes through the linked recording shim, so a real dispatch
can only be attempted when the shim is compiled in; the shipping feature-off
bridge still fails closed with ``real_dispatch_path_not_linked``):

  * **Pass enable matrix (three legs)**: a *reference* leg (all taa_resolve
    per-pass settings at their ``false`` defaults; native Godot TAA resolve
    path), an *enabled_real_pass_optin* candidate leg (``enabled=true`` +
    ``dispatch_real_pass=true``), and a *forced_capability_downgrade* red
    leg (candidate settings plus the harness-only
    ``real_pass_force_capability_downgrade=true`` knob, which clears the
    shader-int64 capability so the bridge taa_resolve preflight must fail
    closed with ``unsupported_device``). Every leg runs against the same
    deterministic viewport-TAA scene (``use_taa=true`` + fixed-seed motion
    animation driven by an integer frame counter under ``--fixed-fps``).
  * **Temporal capture (GRX_PLAN DoD)**: because a real TAA resolve is a
    temporal accumulation, this gate captures a CONTIGUOUS sequence of
    ``CAPTURE_COUNT`` frames per leg (never a single screenshot), diffs the
    candidate/forced sequences frame-for-frame against the reference
    sequence, AND records the reference/candidate frame-to-frame (temporal)
    stability so the evidence proves the sequence carries real inter-frame
    motion (velocity is non-trivial; a static TAA scene would make the
    resolve meaningless). With the patch 0019 writeback scaffold the native
    Godot TAA resolve stays the continuation/backstop, so the candidate
    sequence is expected to be frame-for-frame bit-exact to the reference
    sequence — recorded honestly, not asserted as a superiority claim.
  * **Gated real-pass attempt**: the canonical taa_resolve artifact paths
    carry the texture-capable hlsl_bridge workaround package (DXC ``cs_6_0``
    container validated by ``dxv``, per-slot ``texture2d``/``rwtexture2d``
    binding kinds for the six 1:1 full-res resolve textures, owner-approved
    ``hlsl_bridge_workaround`` provenance), and the single-resolve math
    subset is CPU-proven (``math_parity_evidence.json``), so every software
    gate can pass and the candidate leg may print the
    ``RXGD_GODOT_RUNTIME_TAA_RESOLVE_REAL_PASS`` marker (plus the patch 0019
    result writeback scaffold marker) after a real recorded dispatch. If the
    real dispatch cannot complete in this environment the candidate leg must
    instead print the tracked fallback marker AND the bridge's
    machine-readable ``RXGD_TAA_REAL_PASS_BLOCKED`` diagnostic naming the
    FIRST missing prerequisite (``real_dispatch_recording_failed``).
  * **Fallback red/green + visual stability**: all three legs must render via
    the native Godot TAA resolve and exit 0 (the patch 0019 writeback
    scaffold deliberately keeps the native taa->process as the continuation/
    backstop, so the rendered image can never change); the candidate and
    forced-failure sequences must match the reference sequence within the
    pinned LDR absolute-diff thresholds at EVERY captured frame, and a
    GRX-008-format ``evidence_level=measured_local`` telemetry document must
    record the fallback entries with ``godot_fallback_active=true`` and
    ``telemetry_frame`` equal to the last captured frame index.
  * **Runtime log audit**: the FULL merged stdout+stderr of every leg is
    audited (GRX-009 policy): only the known ``Could not load global script
    cache`` warning is tolerated; any other ``ERROR:`` line is an integrity
    FAIL.

Outcome semantics mirror GRX-011 (see that harness for the SKIP/FAIL/success
contract). ``status=success`` (strict) is written ONLY when the opt-in real
dispatch actually executed and completed, every captured frame stayed within
the LDR thresholds, and every audit passed; only then is ``real_gpu_pass=true``
recorded and the historical measured success artifact written. Even a success
keeps ``default_enable_state=disabled`` and ``performance_claim=none``.
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
# Reuse the tracked GRX-009 scratch-provenance and log-audit helpers, plus the
# GRX-011 0001..0016 stack (the taa_resolve stack extends it with 0017..0019
# and shares the particles_copy 0020..0022 tail through the scratch-0022 build).
from ci.grx009_godot_runtime_bridge_recording_smoke import (
    find_git_root,
    patch_stack_identity,
    runtime_log_audit,
    source_status_clean,
    verify_source_provenance_sidecar,
)
from ci.grx011_ssao_blur_real_pass_enablement_smoke import PATCH_STACK_GRX011

PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "taa_resolve"
ARTIFACTS = PASS_DIR / "artifacts"
VISUAL_DIR = ARTIFACTS / "visual"
DXIL = ARTIFACTS / "taa_resolve.dxil"
RTS0 = ARTIFACTS / "taa_resolve.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "taa_resolve_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
# The *latest* run evidence: rewritten on every run; reproducible-default SKIP
# without the scratch exe. Never advances the gate on its own.
EVIDENCE_OUT = PASS_DIR / "real_pass_enablement_evidence.json"
# The *historical measured success* artifact: written ONLY on a strict
# status=success run and never overwritten by a later SKIP/FAIL run. The
# GRX-012 readiness gate advances off THIS file (it reads strict_success=true).
SUCCESS_EVIDENCE_OUT = PASS_DIR / "real_pass_enablement_success_evidence.json"
# GRX-008-format measured_local telemetry for this gate.
TELEMETRY_OUT = PASS_DIR / "real_pass_enablement_telemetry.json"
FALLBACK_TELEMETRY_SCRIPT = (
    ROOT / "spike" / "godot-rurix" / "bench" / "fallback_telemetry.py"
)

# Tracked frame artifacts, committed ONLY on a strict status=success run (first
# + last captured frame of the sequence, plus the last-frame candidate diff).
REFERENCE_FRAME = VISUAL_DIR / "taa_resolve_real_pass_reference.rgb8"
CANDIDATE_FRAME = VISUAL_DIR / "taa_resolve_real_pass_candidate.rgb8"
DIFF_ARTIFACT = VISUAL_DIR / "taa_resolve_real_pass_diff.rgb8"

RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"
WORK = ROOT / "target" / "grx012_taa_resolve_real_pass_enablement_smoke"
LOG_DIR = WORK / "logs"

SUBJECT = "grx012_taa_resolve_real_pass_enablement_smoke"

# Visual gate pins (same caliber as GRX-009 4h / GRX-010 / GRX-011).
METRIC_KIND = "ldr_absolute_diff"
FRAME_FORMAT = "R8G8B8_raw"
LDR_MAX_ABS_DIFF_THRESHOLD = 2
LDR_MEAN_ABS_DIFF_THRESHOLD = 0.25
MIN_FRAME_DIMENSION = 64
# Temporal capture: a CONTIGUOUS sequence, never a single frame. CAPTURE_START
# leaves enough warmup for the TAA history slice to be allocated (needs >=2
# frames) and for the temporal accumulation to spin up; CAPTURE_COUNT >= 8 is
# the GRX_PLAN temporal-evidence floor.
CAPTURE_START_FRAME = 16
CAPTURE_COUNT = 8
MIN_TEMPORAL_FRAMES = 8
VIEWPORT_WIDTH = 256
VIEWPORT_HEIGHT = 144
ALLOWED_GODOT_ERROR = "Could not load global script cache"

# Markers in the full 0001..0022 build. The fallback marker carries the patch
# 0018 native-resource-handle-mapping wording (NOT the 0017 gate-level wording).
FALLBACK_MARKER = (
    "RurixAccel: taa_resolve native resource handle mapping fallback rc="
)
SESSION_READY_MARKER = "RurixAccel: D3D12 Forward+ bridge session ready."
# Bridge-side machine-readable first-missing-prerequisite diagnostic (printed
# once per session by the fail-closed TaaResolveGate in src/rurix-godot/src/lib.rs).
REAL_PASS_BLOCKED_MARKER = "RXGD_TAA_REAL_PASS_BLOCKED"
# Real-pass markers: the bridge prints
# "RXGD_GODOT_RUNTIME_TAA_RESOLVE_REAL_PASS recorded=1" and the patch 0019
# module gate prints its own dispatched/writeback lines ONLY after
# rxgd_record_pass actually returned RXGD_STATUS_OK on the real-pass arm.
REAL_PASS_MARKER = "RXGD_GODOT_RUNTIME_TAA_RESOLVE_REAL_PASS"
# Patch 0019 result writeback scaffold marker: the native TAA resolve stays the
# continuation/backstop after a real dispatch, so the image never changes.
WRITEBACK_MARKER = "RXGD_GODOT_RUNTIME_TAA_RESOLVE_REAL_PASS_WRITEBACK"
# Recording-smoke marker. GRX Wave 4: the candidate/forced legs arm the
# taa_resolve dispatch_recording_smoke opt-in (it now gates the per-dispatch
# REAL_PASS/WRITEBACK instrumentation markers), and RECORD prints only on
# the real-pass OK path — so it must appear in the candidate leg IFF the
# real pass succeeded, and never in the reference/forced legs.
RECORD_MARKER = "RXGD_GODOT_RUNTIME_TAA_RESOLVE_RECORD"

# The predicted first missing prerequisite when the opt-in real dispatch cannot
# complete in this environment. Pinned so a drift in the gate chain is a loud
# FAIL, not a silent re-labelling.
EXPECTED_FIRST_MISSING_PREREQUISITE = "real_dispatch_recording_failed"
EXPECTED_BLOCKED_FALLBACK_REASON = "validation_failed"
# taa_resolve's runtime binding preflight checks the int64 capability FIRST, so
# the forced capability downgrade fails closed at the preflight level.
EXPECTED_FORCED_PREREQUISITE = "runtime_binding_preflight_failed"
EXPECTED_FORCED_FALLBACK_REASON = "unsupported_device"

KNOWN_GAPS = [
    (
        "math parity is CPU-proven for the single full-resolution TAA resolve "
        "subset only and still pending GPU observation "
        "(math_parity_evidence.json status=pending_gpu_dispatch); hardware "
        "bilinear sub-texel rounding, rgba16f/rg16f half storage "
        "quantization, the resolve->temp->internal->history physical copy "
        "chain, one-frame latency (draw_graph true replacement), and "
        "multiview are recorded gaps"
    ),
    (
        "canonical artifact provenance is hlsl_bridge_workaround "
        "(owner-approved GRX-009 texture artifact provenance policy): the "
        "DXIL container is DXC-compiled from "
        "artifacts/hlsl_bridge/taa_resolve.hlsl, not rurixc-owned; a "
        "rurixc-owned texture-capable compile still requires a patched llc "
        "that supports texture intrinsics plus multi-channel texture element "
        "support"
    ),
    (
        "the real dispatch path is linked only under the d3d12-recording-"
        "shim feature; the shipping feature-off bridge still fails closed "
        "with real_dispatch_path_not_linked, and the patch 0019 result "
        "writeback is a SCAFFOLD: the native Godot TAA resolve re-resolves "
        "every view every frame as the continuation/backstop (one-frame "
        "latency draw_graph true replacement and the history physical "
        "maintenance chain are later rounds)"
    ),
]

GODOT_TIMEOUT_SECONDS = 240
REQUESTED_RENDERER = "d3d12"
REQUESTED_RENDERING_METHOD = "forward_plus"

GODOT_EXE_ENV = "RURIX_GRX012_TAA_RESOLVE_GODOT_EXE"
SCRATCH_SOURCE_ENV = "RURIX_GRX012_TAA_RESOLVE_GODOT_SOURCE"
SCRATCH_SOURCE_PROVENANCE_ENV = "RURIX_GRX012_TAA_RESOLVE_GODOT_SOURCE_PROVENANCE"
SCRATCH_BUILD_COMMAND_ENV = "RURIX_GRX012_TAA_RESOLVE_GODOT_BUILD_COMMAND"
SCRATCH_BUILD_LOG_ENV = "RURIX_GRX012_TAA_RESOLVE_GODOT_BUILD_LOG"
CAPTURE_PREFIX_ENV = "RURIX_GRX012_TAA_RESOLVE_CAPTURE_PREFIX"
CAPTURE_START_ENV = "RURIX_GRX012_TAA_RESOLVE_CAPTURE_START"
CAPTURE_COUNT_ENV = "RURIX_GRX012_TAA_RESOLVE_CAPTURE_COUNT"

TARGET_BACKEND = "Godot 4.7-dev Windows D3D12 Forward+"
PASS_SETTING_PREFIX = "rendering/rurix_accel/passes/taa_resolve"

PATCH_STACK_ID = "0001..0022"
# The scratch-0022 exe carries the FULL 0001..0022 stack (taa_resolve owns
# 0017..0019; the shared build also carries the particles_copy 0020..0022
# tail), so the source-provenance audit is against the whole stack.
PATCH_STACK_GRX012 = (
    *PATCH_STACK_GRX011,
    "0017-rurix-accel-taa-resolve-pass-gate-and-callsite.patch",
    "0018-rurix-accel-taa-resolve-runtime-resource-binding.patch",
    "0019-rurix-accel-taa-resolve-recording-smoke-and-real-pass-optin.patch",
    "0020-rurix-accel-particles-copy-pass-gate-and-callsite.patch",
    "0021-rurix-accel-particles-copy-runtime-resource-binding.patch",
    "0022-rurix-accel-particles-copy-recording-smoke-and-real-pass-optin.patch",
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
            "patch stack applied (module_rurix_accel_enabled=yes d3d12=yes). "
            "Only its fingerprint is recorded here so the measured evidence "
            f"stays auditable; re-point {GODOT_EXE_ENV} at an equivalent "
            "rebuild to reproduce it."
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
            "Bridge built WITH the d3d12-recording-shim feature: the "
            "taa_resolve real-pass arm routes through the linked recording "
            "shim, so arming RXGD_CAP_TAA_RESOLVE_REAL_PASS can make "
            "rxgd_record_pass return RXGD_STATUS_OK only after a real "
            "recorded dispatch. The shipping feature-off bridge still fails "
            "closed with real_dispatch_path_not_linked. "
            "target/debug/rurix_godot.dll is a mutable build artifact."
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
    """Parse the key=value tokens of an RXGD_TAA_REAL_PASS_BLOCKED line."""
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
        f"[grx012-taa-resolve-real-pass-smoke] wrote {rel(EVIDENCE_OUT)} status={status}"
    )

    if status == "success":
        success_doc = dict(doc)
        success_doc["evidence_kind"] = "historical_measured_success"
        # The field the GRX-012 gate (ci/grx_gates/grx012_taa_resolve.py
        # _enablement_ready) reads. Only ever written on a strict success.
        success_doc["strict_success"] = True
        success_doc["latest_evidence_path"] = rel(EVIDENCE_OUT)
        success_doc["success_evidence_note"] = (
            "Historical measured success artifact for the GRX-012 taa_resolve "
            "real-pass enablement gate. It is written ONLY on a strict "
            "status=success run (opt-in real dispatch executed AND completed "
            "AND the temporal LDR visual gate stayed within thresholds at "
            "every captured frame AND every audit passed) and is never deleted "
            "or overwritten by a later SKIP/FAIL run. Even this success keeps "
            "default_enable_state=disabled and performance_claim=none."
        )
        _write_json(SUCCESS_EVIDENCE_OUT, success_doc)
        print(
            "[grx012-taa-resolve-real-pass-smoke] wrote "
            f"{rel(SUCCESS_EVIDENCE_OUT)} status=success (historical measured success)"
        )


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx012-taa-resolve-real-pass-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip_environment(msg: str, extra: dict | None = None) -> int:
    """Environment-level SKIP: a precondition is unavailable. Upgraded to a
    hard FAIL under RURIX_REQUIRE_REAL=1. Never writes/overwrites the success
    evidence artifact."""
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx012-taa-resolve-real-pass-smoke] SKIP {msg}(降级 SKIP,退出 0)")
    payload = dict(extra or {})
    payload["skip_kind"] = "environment"
    write_evidence("skip", reason=msg, extra=payload)
    return 0


def skip_measured_prerequisite(prerequisite: str, msg: str, extra: dict) -> int:
    """Measured prerequisite-blocked SKIP: every leg ran on real hardware and
    the fail-closed gate reported exactly the predicted first missing
    prerequisite. This is a real measured run, so RURIX_REQUIRE_REAL does NOT
    upgrade it to FAIL; it still never advances the readiness gate and never
    writes/overwrites the success evidence artifact."""
    print(
        "[grx012-taa-resolve-real-pass-smoke] SKIP (measured) first missing "
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
            f"{GODOT_EXE_ENV} is not set; the GRX-012 taa_resolve enablement "
            "smoke needs a scratch Godot console exe rebuilt from the ignored "
            "external/godot-master snapshot with the full "
            f"{PATCH_STACK_ID} patch stack applied "
            "(module_rurix_accel_enabled=yes d3d12=yes). The tracked "
            "external/godot-master build only has 0001+0002+0003 and must NOT "
            "be reused here"
        )
    p = Path(override)
    if not p.is_file():
        return None, f"{GODOT_EXE_ENV}={override} does not point at an existing file"
    return p, None


def build_bridge_dll() -> tuple[bool, str]:
    """Build rurix_godot.dll WITH the d3d12-recording-shim feature (the
    taa_resolve real-pass arm can only attempt a real dispatch when the
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
    the tracked 0001..0022 patch stack (GRX-009 machinery, GRX-012 stack)."""
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
        "applied_patch_stack": patch_stack_identity(PATCH_STACK_GRX012, PATCH_STACK_ID),
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
        stack_names=PATCH_STACK_GRX012,
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
    dispatch_recording_smoke: bool,
    dispatch_real_pass: bool,
    force_capability_downgrade: bool,
) -> None:
    """Generate a minimal deterministic Godot project. Only the tracked
    taa_resolve per-pass opt-in settings differ between legs; everything else
    (including viewport use_taa=true and the fixed-seed motion animation that
    keeps the TAA velocity buffer non-trivial) is byte-identical so the opt-in
    matrix is the only delta. GRX Wave 4 print gating:
    the candidate/forced legs arm the taa_resolve
    dispatch_recording_smoke opt-in because it now gates the per-dispatch
    REAL_PASS/WRITEBACK instrumentation markers this harness asserts on
    (the production dispatch_real_pass path emits zero per-dispatch
    stdout); the RECORD marker only prints on the real-pass OK path, so
    it must appear in the candidate leg IFF the real pass succeeded and
    never elsewhere."""
    project_dir.mkdir(parents=True, exist_ok=True)

    def flag(value: bool) -> str:
        return "true" if value else "false"

    project_text = f"""\
; Engine configuration file.
; Auto-generated by ci/grx012_taa_resolve_real_pass_enablement_smoke.py

config_version=5

[application]

config/name="GRX-012 taa_resolve real-pass enablement smoke"
run/main_scene="res://main.tscn"

[display]

window/size/viewport_width={VIEWPORT_WIDTH}
window/size/viewport_height={VIEWPORT_HEIGHT}

[rendering]

anti_aliasing/quality/use_taa=true
anti_aliasing/quality/msaa_3d=0
rurix_accel/enabled=true
rurix_accel/require_forward_plus=true
rurix_accel/dll_path="{dll_path.as_posix()}"
rurix_accel/passes/taa_resolve/enabled={flag(pass_enabled)}
rurix_accel/passes/taa_resolve/dispatch_recording_smoke={flag(dispatch_recording_smoke)}
rurix_accel/passes/taa_resolve/dispatch_real_pass={flag(dispatch_real_pass)}
rurix_accel/passes/taa_resolve/real_pass_force_capability_downgrade={flag(force_capability_downgrade)}
"""
    scene_text = """\
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://main.gd" id="1"]

[node name="GRX012TaaResolveRoot" type="Node3D"]
script = ExtResource("1")

[node name="Camera3D" type="Camera3D" parent="."]

[node name="DirectionalLight3D" type="DirectionalLight3D" parent="."]

[node name="WorldEnvironment" type="WorldEnvironment" parent="."]
"""
    # Deterministic scene with viewport TAA enabled and fixed-seed motion so the
    # velocity buffer stays non-trivial and the native TAA resolve compute
    # dispatch (the taa_resolve call site) runs every frame, then a CONTIGUOUS
    # frame-sequence capture. All motion is driven by an INTEGER frame counter
    # (no float delta accumulation) so, under --fixed-fps, the state at each
    # captured frame is deterministic and identical across legs.
    script_text = f"""\
extends Node3D

var _frames := 0
var _boxes: Array = []
var _cam: Camera3D

func _ready() -> void:
    _cam = $Camera3D
    _cam.position = Vector3(0.0, 2.5, 7.0)
    _cam.rotation_degrees = Vector3(-18.0, 0.0, 0.0)
    _cam.make_current()

    var light: DirectionalLight3D = $DirectionalLight3D
    light.rotation_degrees = Vector3(-55.0, -35.0, 0.0)

    var env := Environment.new()
    env.background_mode = Environment.BG_COLOR
    env.background_color = Color(0.30, 0.36, 0.46)
    env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
    env.ambient_light_color = Color(0.5, 0.5, 0.5)
    env.ambient_light_energy = 1.0
    env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
    $WorldEnvironment.environment = env

    var ground := MeshInstance3D.new()
    var plane := PlaneMesh.new()
    plane.size = Vector2(30.0, 30.0)
    ground.mesh = plane
    ground.position = Vector3(0.0, 0.0, 0.0)
    add_child(ground)

    # A fixed grid of boxes that ORBIT deterministically so the TAA velocity
    # buffer is non-trivial (a static scene would make the temporal resolve
    # meaningless). Positions are recomputed each frame from the integer frame
    # counter, so every run/leg produces byte-identical motion.
    var box := BoxMesh.new()
    box.size = Vector3(0.9, 0.9, 0.9)
    for i in range(6):
        var mi := MeshInstance3D.new()
        mi.mesh = box
        add_child(mi)
        _boxes.append(mi)
    _apply_motion(0)

    print("GRX012TaaResolve: scene ready use_taa=%s" % str(get_viewport().use_taa))
    _capture_sequence()

func _apply_motion(fi: int) -> void:
    # Deterministic per-frame orbit driven purely by the integer frame index.
    var t := float(fi) * 0.13
    for i in range(_boxes.size()):
        var mi: MeshInstance3D = _boxes[i]
        var ang := t + float(i) * (TAU / 6.0)
        var radius := 2.6
        mi.position = Vector3(cos(ang) * radius, 0.6 + 0.35 * sin(t + float(i)), sin(ang) * radius - 1.0)
        mi.rotation = Vector3(0.0, ang, 0.0)
    # Gentle deterministic camera pan keeps a global velocity component.
    _cam.position = Vector3(0.6 * sin(t * 0.5), 2.5, 7.0)

func _process(_delta: float) -> void:
    _frames += 1
    _apply_motion(_frames)

func _capture_sequence() -> void:
    var start := {CAPTURE_START_FRAME}
    var count := {CAPTURE_COUNT}
    var start_env := OS.get_environment("{CAPTURE_START_ENV}")
    if not start_env.is_empty():
        start = int(start_env)
    var count_env := OS.get_environment("{CAPTURE_COUNT_ENV}")
    if not count_env.is_empty():
        count = int(count_env)
    var prefix := OS.get_environment("{CAPTURE_PREFIX_ENV}")
    if prefix.is_empty():
        printerr("GRX012TaaResolve: capture prefix env var missing")
        get_tree().quit(3)
        return
    var saved := 0
    while saved < count:
        await RenderingServer.frame_post_draw
        if _frames < start:
            continue
        _save_frame(prefix, _frames)
        saved += 1
    print("GRX012TaaResolve: captured %d frames start=%d" % [saved, start])
    get_tree().quit()

func _save_frame(prefix: String, fi: int) -> void:
    var img: Image = get_viewport().get_texture().get_image()
    img.convert(Image.FORMAT_RGB8)
    var frame_prefix := "%s.%03d" % [prefix, fi]
    var raw := FileAccess.open(frame_prefix + ".rgb8", FileAccess.WRITE)
    raw.store_buffer(img.get_data())
    raw.close()
    var meta := FileAccess.open(frame_prefix + ".json", FileAccess.WRITE)
    meta.store_string(JSON.stringify({{
        "width": img.get_width(),
        "height": img.get_height(),
        "format": "{FRAME_FORMAT}",
        "capture_frame_index": fi,
    }}))
    meta.close()
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
    env[CAPTURE_START_ENV] = str(CAPTURE_START_FRAME)
    env[CAPTURE_COUNT_ENV] = str(CAPTURE_COUNT)
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


def load_frame(capture_prefix: Path, frame_index: int) -> tuple[dict | None, bytes | None, str | None]:
    frame_prefix = f"{capture_prefix}.{frame_index:03d}"
    meta = load_json(Path(frame_prefix + ".json"))
    raw_path = Path(frame_prefix + ".rgb8")
    if meta is None:
        return None, None, f"capture metadata missing/unreadable at {frame_prefix}.json"
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
        "dispatch_recording_smoke": False,
        "dispatch_real_pass": False,
        "force_capability_downgrade": False,
    },
    "candidate": {
        "pass_enabled": True,
        "dispatch_recording_smoke": True,
        "dispatch_real_pass": True,
        "force_capability_downgrade": False,
    },
    "forced_fallback": {
        "pass_enabled": True,
        "dispatch_recording_smoke": True,
        "dispatch_real_pass": True,
        "force_capability_downgrade": True,
    },
}

LEG_ROLES = {
    "reference": "reference",
    "candidate": "enabled_real_pass_optin",
    "forced_fallback": "forced_capability_downgrade",
}

EXPECTED_FRAME_INDICES = tuple(
    range(CAPTURE_START_FRAME, CAPTURE_START_FRAME + CAPTURE_COUNT)
)


def run_matrix_leg(godot_exe: Path, *, leg: str, dll_path: Path) -> dict:
    settings = LEG_SETTINGS[leg]
    project_dir = WORK / f"project_{leg}"
    capture_prefix = WORK / f"capture_{leg}"
    for fi in EXPECTED_FRAME_INDICES:
        for suffix in (".rgb8", ".json"):
            Path(f"{capture_prefix}.{fi:03d}{suffix}").unlink(missing_ok=True)
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
    real_pass_lines = [
        line.strip()
        for line in output.splitlines()
        if REAL_PASS_MARKER in line and WRITEBACK_MARKER not in line
    ]
    # Load the full captured sequence.
    frames: dict[int, bytes] = {}
    metas: dict[int, dict] = {}
    capture_error = None
    for fi in EXPECTED_FRAME_INDICES:
        meta, data, err = load_frame(capture_prefix, fi)
        if err is not None:
            capture_error = err
            break
        frames[fi] = data
        metas[fi] = meta
    return {
        "leg": leg,
        "role": LEG_ROLES[leg],
        "project_settings": {
            f"{PASS_SETTING_PREFIX}/enabled": settings["pass_enabled"],
            f"{PASS_SETTING_PREFIX}/dispatch_recording_smoke": settings[
                "dispatch_recording_smoke"
            ],
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
        "capture_metas": metas,
        "capture_error": capture_error,
        "capture_prefix": capture_prefix,
        "frame_sequence": frames,
        "runtime_log_audit": runtime_log_audit(output, PATCH_STACK_GRX012),
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
        "captured_frame_indices": sorted(leg["capture_metas"].keys()),
        "capture_error": leg["capture_error"],
    }


def telemetry_entries_issue(
    doc: dict, capture_frame_index: int, *, expect_candidate_fallback: bool = True
) -> str | None:
    """First incoherence in the generated GRX-012 telemetry entries, or None."""
    passes = doc.get("passes")
    expected_count = 2 if expect_candidate_fallback else 1
    if not isinstance(passes, list) or len(passes) != expected_count:
        return (
            f"telemetry document must carry exactly {expected_count} "
            "taa_resolve fallback entries"
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
        if entry.get("pass_id") != "taa_resolve":
            return f"{leg_name} entry pass_id is not taa_resolve"
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
        "pass_id": "taa_resolve",
        "segment": "grx012_real_pass_enablement",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "godot_runtime_taa_resolve_path_enabled": False,
        "default_enable_state": "disabled",
        "gpu_timestamp_status": "not_yet",
        "performance_claim": "none",
        "temporal_evidence": {
            "kind": "contiguous_frame_sequence",
            "capture_start_frame": CAPTURE_START_FRAME,
            "capture_count": CAPTURE_COUNT,
            "min_temporal_frames": MIN_TEMPORAL_FRAMES,
            "note": (
                "A real TAA resolve is a temporal accumulation, so this gate "
                "captures a CONTIGUOUS frame sequence (never a single "
                "screenshot) and diffs the candidate/forced sequences "
                "frame-for-frame against the reference sequence; the "
                "reference/candidate frame-to-frame deltas are recorded to "
                "prove the sequence carries real inter-frame motion."
            ),
        },
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
            "GRX-012 taa_resolve gated real-pass enablement gate evidence. The "
            "opt-in real-pass arm (dispatch_real_pass, default false) runs "
            "against the texture-capable hlsl_bridge workaround canonical "
            "package (six 1:1 full-res resolve textures, per-slot "
            "texture2d/rwtexture2d binding kinds, owner-approved "
            "hlsl_bridge_workaround provenance, single-resolve math parity "
            "CPU-proven pending GPU) and a d3d12-recording-shim bridge DLL, so "
            "every software gate can pass and a real recorded dispatch may "
            "return RXGD_STATUS_OK; when the dispatch cannot complete the gate "
            "reports first_missing_prerequisite=real_dispatch_recording_failed "
            "instead of claiming success. The patch 0019 result writeback is a "
            "SCAFFOLD (native Godot TAA resolve stays the continuation/"
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
        return skip_environment(godot_reason or "GRX-012 taa_resolve Godot exe unavailable")

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
        PATCH_STACK_GRX012, PATCH_STACK_ID
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
        if leg["record_marker_observed"] and name != "candidate":
            # GRX Wave 4: the candidate/forced legs arm
            # dispatch_recording_smoke (it gates the per-dispatch
            # instrumentation markers), but RECORD only prints on the
            # real-pass OK path — the reference leg (all defaults) and the
            # fail-closed forced leg must never print it; the candidate
            # leg's RECORD/real-pass coupling is asserted below.
            return fail(
                f"{name} run printed the taa_resolve recording-smoke marker "
                f"'{RECORD_MARKER}'; the {name} leg must never reach the "
                "real-pass OK path",
                extra=runs_extra,
            )
        if leg["capture_error"] is not None:
            return fail(
                f"{name} frame-sequence capture failed: {leg['capture_error']}",
                extra=runs_extra,
            )
        got = sorted(leg["capture_metas"].keys())
        if got != list(EXPECTED_FRAME_INDICES):
            return fail(
                f"{name} captured frame indices {got} do not match the expected "
                f"contiguous temporal sequence {list(EXPECTED_FRAME_INDICES)}",
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
                "reference run (all taa_resolve per-pass settings at their false "
                f"defaults) unexpectedly printed '{marker_name}'; the disabled "
                "pass must never invoke the bridge",
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
    if candidate["record_marker_observed"] != real_pass_success:
        return fail(
            "candidate run RECORD marker presence "
            f"({candidate['record_marker_observed']}) does not match the "
            "real-pass outcome; with the recording-smoke opt-in armed the "
            "RECORD marker must print IFF the real-pass arm returned OK",
            extra=runs_extra,
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
                "candidate run printed the writeback scaffold marker without a "
                "real pass; the gate outcome is contradictory",
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

    # Frame coherence across all three legs at every captured frame.
    ref_meta0 = reference["capture_metas"][CAPTURE_START_FRAME]
    width = int(ref_meta0["width"])
    height = int(ref_meta0["height"])
    for name in ("candidate", "forced_fallback"):
        for fi in EXPECTED_FRAME_INDICES:
            meta = legs[name]["capture_metas"][fi]
            if (
                ref_meta0.get("width") != meta.get("width")
                or ref_meta0.get("height") != meta.get("height")
            ):
                return fail(
                    f"reference/{name} frame dimensions mismatch at frame {fi} "
                    f"({ref_meta0.get('width')}x{ref_meta0.get('height')} vs "
                    f"{meta.get('width')}x{meta.get('height')})",
                    extra=runs_extra,
                )

    capture_frame_index = CAPTURE_START_FRAME + CAPTURE_COUNT - 1

    # Per-frame LDR absolute diff of each armed leg against the native reference
    # sequence (the temporal visual gate), plus the reference/candidate
    # frame-to-frame (temporal) stability so the evidence proves the sequence
    # carries real inter-frame motion.
    per_frame: dict[str, list[dict]] = {"candidate": [], "forced_fallback": []}
    diff_bytes_last: dict[str, bytes] = {}
    for name in ("candidate", "forced_fallback"):
        for fi in EXPECTED_FRAME_INDICES:
            max_abs, mean_abs, diff_bytes = compute_ldr_abs_diff(
                reference["frame_sequence"][fi], legs[name]["frame_sequence"][fi]
            )
            within = (
                max_abs <= LDR_MAX_ABS_DIFF_THRESHOLD
                and mean_abs <= LDR_MEAN_ABS_DIFF_THRESHOLD
            )
            per_frame[name].append(
                {
                    "frame_index": fi,
                    "max_abs_diff": max_abs,
                    "mean_abs_diff": mean_abs,
                    "within_threshold": within,
                }
            )
            if fi == capture_frame_index:
                diff_bytes_last[name] = diff_bytes
        worst_max = max(entry["max_abs_diff"] for entry in per_frame[name])
        worst_mean = max(entry["mean_abs_diff"] for entry in per_frame[name])
        print(
            f"[grx012-taa-resolve-real-pass-smoke] temporal LDR diff ({name} vs "
            f"reference) over {CAPTURE_COUNT} frames worst_max_abs={worst_max} "
            f"worst_mean_abs={worst_mean:.6f} (thresholds "
            f"max<={LDR_MAX_ABS_DIFF_THRESHOLD} mean<={LDR_MEAN_ABS_DIFF_THRESHOLD})"
        )

    def temporal_stability(leg: dict) -> dict:
        deltas = []
        prev = None
        for fi in EXPECTED_FRAME_INDICES:
            cur = leg["frame_sequence"][fi]
            if prev is not None:
                max_abs, mean_abs, _ = compute_ldr_abs_diff(prev, cur)
                deltas.append(
                    {"from": fi - 1, "to": fi, "max_abs_diff": max_abs, "mean_abs_diff": mean_abs}
                )
            prev = cur
        nonzero = sum(1 for d in deltas if d["max_abs_diff"] > 0)
        return {
            "adjacent_pairs": len(deltas),
            "nonzero_delta_pairs": nonzero,
            "max_interframe_abs_diff": max((d["max_abs_diff"] for d in deltas), default=0),
            "mean_interframe_abs_diff": (
                sum(d["mean_abs_diff"] for d in deltas) / len(deltas) if deltas else 0.0
            ),
            "per_adjacent": deltas,
        }

    reference_stability = temporal_stability(reference)
    candidate_stability = temporal_stability(candidate)

    diffs_within = all(
        entry["within_threshold"]
        for entries in per_frame.values()
        for entry in entries
    )
    # The candidate is expected frame-for-frame bit-exact (scaffold); the
    # reference sequence must carry real motion (non-zero inter-frame deltas)
    # or the temporal evidence is meaningless.
    candidate_bit_exact = all(
        entry["max_abs_diff"] == 0 for entry in per_frame["candidate"]
    )
    temporal_motion_present = reference_stability["nonzero_delta_pairs"] > 0

    visual = {
        "measured_local": True,
        "metric_kind": METRIC_KIND,
        "width": width,
        "height": height,
        "format": FRAME_FORMAT,
        "temporal_sequence": {
            "capture_start_frame": CAPTURE_START_FRAME,
            "capture_count": CAPTURE_COUNT,
            "captured_frame_indices": list(EXPECTED_FRAME_INDICES),
        },
        "max_abs_diff_threshold": LDR_MAX_ABS_DIFF_THRESHOLD,
        "mean_abs_diff_threshold": LDR_MEAN_ABS_DIFF_THRESHOLD,
        "per_frame_diffs": per_frame,
        "candidate_frame_for_frame_bit_exact": candidate_bit_exact,
        "reference_temporal_stability": reference_stability,
        "candidate_temporal_stability": candidate_stability,
        "temporal_motion_present": temporal_motion_present,
        "last_frame_index": capture_frame_index,
        "reference_frame_last": file_fingerprint(
            Path(f"{reference['capture_prefix']}.{capture_frame_index:03d}.rgb8")
        ),
        "candidate_frame_last": file_fingerprint(
            Path(f"{candidate['capture_prefix']}.{capture_frame_index:03d}.rgb8")
        ),
        "forced_fallback_frame_last": file_fingerprint(
            Path(f"{forced['capture_prefix']}.{capture_frame_index:03d}.rgb8")
        ),
        "frame_artifact_note": (
            "Frame artifacts live in the local work dir and are hash-pinned "
            "here; the first + last frames of the sequence are committed under "
            "artifacts/visual/ ONLY on a strict status=success run."
        ),
    }
    runs_extra["visual"] = visual
    if not diffs_within:
        return fail(
            "temporal LDR absolute diff exceeded the visual gate threshold at "
            "one or more captured frames: arming the fail-closed real-pass "
            "opt-in (or the forced downgrade knob) changed the rendered "
            f"sequence ({per_frame!r})",
            extra=runs_extra,
        )
    if not temporal_motion_present:
        return fail(
            "the reference TAA sequence carried no inter-frame motion "
            "(all adjacent frames identical); the temporal evidence would be "
            "meaningless — the deterministic motion animation did not drive "
            "the velocity buffer",
            extra=runs_extra,
        )

    # GRX-008-format measured telemetry: candidate + forced fallback entries.
    telemetry_doc = {
        "run_mode": "full",
        "evidence_level": "measured_local",
        "target_backend": TARGET_BACKEND,
        "note": (
            "GRX-012 measured taa_resolve real-pass enablement telemetry: with "
            "the default-false dispatch_real_pass opt-in explicitly enabled, "
            "the tracked Godot TAA resolve call site invoked the Rurix bridge "
            "through the patch 0018 native resource handle binding; every "
            "fallback entry records the fail-closed gate outcome "
            "(validation_failed when the linked real dispatch cannot complete; "
            "unsupported_device under the forced capability downgrade) while "
            "the native Godot TAA resolve rendered every frame. The patch 0019 "
            "writeback is a scaffold (native continuation active), "
            "runtime_state stays fallback_only for the default path, and no "
            "performance or FPS claim is made."
        ),
        "passes": (
            []
            if real_pass_success
            else [
                {
                    "pass_id": "taa_resolve",
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
                "pass_id": "taa_resolve",
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
        # GRX Wave 4: candidate/forced arm dispatch_recording_smoke (it
        # gates the per-dispatch instrumentation markers); RECORD stays
        # forbidden in the reference/forced legs and the candidate RECORD
        # marker must match the real-pass outcome.
        "record_marker_absent_reference_and_forced": True,
        "record_marker_matches_real_pass_candidate": True,
        "temporal_sequence_captured": True,
        "temporal_min_frames_met": CAPTURE_COUNT >= MIN_TEMPORAL_FRAMES,
        "temporal_motion_present_reference": temporal_motion_present,
        "candidate_frame_for_frame_bit_exact": candidate_bit_exact,
        "dimensions_match": True,
        "runtime_log_audit_clean": True,
        "diff_within_threshold_all_frames": diffs_within,
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
        # the temporal visual gate stayed green at every captured frame.
        # Publish the first + last frame artifacts and flip real_gpu_pass=true
        # in THIS evidence only; default_enable_state stays disabled and no
        # performance claim exists.
        VISUAL_DIR.mkdir(parents=True, exist_ok=True)
        REFERENCE_FRAME.write_bytes(reference["frame_sequence"][capture_frame_index])
        CANDIDATE_FRAME.write_bytes(candidate["frame_sequence"][capture_frame_index])
        DIFF_ARTIFACT.write_bytes(diff_bytes_last["candidate"])
        write_png_rgb8(
            Path(str(DIFF_ARTIFACT) + ".png"), width, height, diff_bytes_last["candidate"]
        )
        first_fi = CAPTURE_START_FRAME
        (VISUAL_DIR / "taa_resolve_real_pass_reference_first.rgb8").write_bytes(
            reference["frame_sequence"][first_fi]
        )
        (VISUAL_DIR / "taa_resolve_real_pass_candidate_first.rgb8").write_bytes(
            candidate["frame_sequence"][first_fi]
        )
        visual["reference_frame_last"] = file_fingerprint(REFERENCE_FRAME)
        visual["candidate_frame_last"] = file_fingerprint(CANDIDATE_FRAME)
        visual["diff_artifact"] = file_fingerprint(DIFF_ARTIFACT)
        success_extra = dict(measured_extra)
        success_extra["visual"] = visual
        success_extra["real_gpu_pass"] = True
        success_extra["real_d3d12_dispatch_recorded"] = True
        success_extra["real_pass_marker_line"] = candidate["real_pass_marker_line"]
        write_evidence("success", extra=success_extra)
        print(
            "[grx012-taa-resolve-real-pass-smoke] PASS measured opt-in real pass "
            "+ temporal LDR visual gate within threshold at every captured "
            "frame (default enablement unchanged; no performance claim)"
        )
        return 0

    return skip_measured_prerequisite(
        candidate_tokens.get(
            "first_missing_prerequisite", EXPECTED_FIRST_MISSING_PREREQUISITE
        ),
        "the opt-in real-pass gate measured the predicted fail-closed shape on "
        "real hardware: every software gate passed against the texture-capable "
        "hlsl_bridge canonical taa_resolve package, but the linked real "
        "dispatch recording did not complete in this environment (e.g. no "
        "signed DXC dxil.dll or a D3D12 recording failure), so the gate "
        "honestly reports real_dispatch_recording_failed instead of claiming "
        "success",
        measured_extra,
    )


if __name__ == "__main__":
    sys.exit(main())
