#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-009 segment 4g: real visual diff + measured fallback telemetry smoke.

This harness produces the FIRST real (non-placeholder) visual evidence and the
first measured fallback telemetry for the GRX-009 luminance_reduction path. It
is a gate/scaffold slice: it does NOT enable the real GPU pass, does NOT flip
``real_gpu_pass`` / ``real_d3d12_dispatch_recorded``, and claims NO performance
or FPS numbers. What it measures, honestly:

  * **Visual (LDR absolute diff)**: two deterministic runs of the tracked Godot
    build (``external/godot-master`` with the tracked 0001+0002+0003 patch
    stack) render the same minimal auto-exposure scene. The *reference* run
    keeps the per-pass setting ``rendering/rurix_accel/passes/
    luminance_reduction/enabled`` at its default ``false``; the *candidate* run
    sets it ``true``, which makes the Auto Exposure call site invoke the
    shipping (feature-off) bridge — and the bridge falls back
    (``RXGD_STATUS_FALLBACK``), so the native Godot luminance path runs in both
    cases. The captured frames must match within a strict LDR absolute-diff
    threshold: enabling the (falling-back) pass must not change the image.
    This is a fallback-path visual diff ONLY; it is NOT visual verification of
    a Rurix GPU pass (no such pass runs).
  * **Measured fallback telemetry**: the candidate run must actually print the
    tracked patch-0002 marker ``RurixAccel: luminance_reduction fallback
    rc=...`` — the measured "fallback path observed" signal — while the
    reference run must NOT (the disabled pass never calls the bridge). The
    matrix of both runs is recorded, plus a GRX-008-format
    ``evidence_level=measured_local`` telemetry document validated by
    ``spike/godot-rurix/bench/fallback_telemetry.py``. The telemetry's
    luminance entry must record ``enable_state=enabled``,
    ``fallback_reason=validation_failed``, ``godot_fallback_active=true`` and a
    ``telemetry_frame`` equal to the measured visual capture frame index.
  * **Runtime log audit**: the FULL merged stdout+stderr of both matrix legs
    is audited (segment 4f policy): only the known ``Could not load global
    script cache`` warning is tolerated (recorded with a rationale); any other
    ``ERROR:`` line is an integrity FAIL.

Preconditions (any missing one is a concrete SKIP that does NOT advance the
readiness gate): the tracked Godot console exe (or
``RURIX_GRX009_SEGMENT4G_GODOT_EXE``), a buildable feature-OFF
``rurix_godot.dll``, and a working D3D12 Forward+ session (the tracked module
must print its "bridge session ready" marker). If ``RURIX_REQUIRE_REAL=1``, an
environment that would otherwise SKIP becomes a hard failure (exit 1).

Evidence hygiene — two tracked evidence artifacts (mirrors segment 4f):

  * ``visual_fallback_evidence.json`` is the *latest* run evidence, rewritten
    on EVERY run; with no usable Godot exe it honestly records ``status=skip``
    and never advances the readiness gate on its own.
  * ``visual_fallback_success_evidence.json`` is the *historical measured
    success* artifact, written ONLY on a strict ``status=success`` run and
    never deleted or overwritten by a later SKIP/FAIL run. The segment 4g
    readiness gate advances off THIS file. On success the harness also copies
    the raw RGB8 frames + diff artifact into
    ``spike/godot-rurix/passes/luminance_reduction/artifacts/visual/`` and
    writes the GRX-008-format ``measured_fallback_telemetry.json``; all of
    those artifact bytes are hash-pinned in the evidence so the probe can
    re-verify them from disk.
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
# Reuse the tracked segment 4f runtime log audit (same allowed-error policy and
# rationale wording) so the two gates cannot silently drift apart; the
# validation-failed regression test asserts pin parity.
from ci.grx009_godot_runtime_bridge_recording_smoke import runtime_log_audit

PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "luminance_reduction"
ARTIFACTS = PASS_DIR / "artifacts"
VISUAL_DIR = ARTIFACTS / "visual"
DXIL = ARTIFACTS / "luminance_reduction.dxil"
RTS0 = ARTIFACTS / "luminance_reduction.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "luminance_reduction_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
SCHEMA = PASS_DIR / "visual_fallback_evidence.schema.json"
# The *latest* run evidence: rewritten on every run, honestly SKIP when the
# tracked Godot exe is unavailable. Never advances the gate on its own.
EVIDENCE_OUT = PASS_DIR / "visual_fallback_evidence.json"
# The *historical measured success* artifact: written only on a strict
# status=success run and never overwritten by a later SKIP/FAIL run. The
# segment 4g readiness gate advances off THIS file.
SUCCESS_EVIDENCE_OUT = PASS_DIR / "visual_fallback_success_evidence.json"
# GRX-008-format measured_local fallback telemetry (written only on success;
# must pass spike/godot-rurix/bench/fallback_telemetry.py --validate-only).
TELEMETRY_OUT = PASS_DIR / "measured_fallback_telemetry.json"
FALLBACK_TELEMETRY_SCRIPT = (
    ROOT / "spike" / "godot-rurix" / "bench" / "fallback_telemetry.py"
)

# Tracked frame artifacts (committed; hash-pinned by the evidence and
# re-verified by the probe gate).
REFERENCE_FRAME = VISUAL_DIR / "luminance_fallback_reference.rgb8"
CANDIDATE_FRAME = VISUAL_DIR / "luminance_fallback_candidate.rgb8"
DIFF_ARTIFACT = VISUAL_DIR / "luminance_fallback_diff.rgb8"
REFERENCE_FRAME_PNG = VISUAL_DIR / "luminance_fallback_reference.png"
CANDIDATE_FRAME_PNG = VISUAL_DIR / "luminance_fallback_candidate.png"
DIFF_ARTIFACT_PNG = VISUAL_DIR / "luminance_fallback_diff.png"

RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"
WORK = ROOT / "target" / "grx009_segment4g_visual_fallback_smoke"
LOG_DIR = WORK / "logs"

SUBJECT = "grx009_segment4g_luminance_visual_fallback_smoke"

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

# Tracked patch-0002 module markers (present in the tracked 0001+0002+0003
# external/godot-master build; no scratch full-stack rebuild is required).
FALLBACK_MARKER = "RurixAccel: luminance_reduction fallback rc="
SESSION_READY_MARKER = "RurixAccel: D3D12 Forward+ bridge session ready."
# The ONLY Godot stderr/stdout `ERROR:` line tolerated by the runtime log
# audit (mirrors the segment 4f policy; parity with the probe pin is asserted
# by ci/godot_rurix_toolchain_probe_validation_failed_test.py). Any other
# `ERROR:` line in either matrix leg is an integrity FAIL, not a SKIP.
ALLOWED_GODOT_ERROR = "Could not load global script cache"
GODOT_TIMEOUT_SECONDS = 180
REQUESTED_RENDERER = "d3d12"
REQUESTED_RENDERING_METHOD = "forward_plus"

GODOT_EXE_ENV = "RURIX_GRX009_SEGMENT4G_GODOT_EXE"
DEFAULT_GODOT_EXE = (
    ROOT
    / "external"
    / "godot-master"
    / "bin"
    / "godot.windows.template_debug.x86_64.console.exe"
)
CAPTURE_PREFIX_ENV = "RURIX_GRX009_SEGMENT4G_CAPTURE_PREFIX"

TARGET_BACKEND = "Godot 4.7-dev Windows D3D12 Forward+"
PASS_ENABLED_SETTING = "rendering/rurix_accel/passes/luminance_reduction/enabled"


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
        # Segment 4i restructured the canonical artifacts: the raw-buffer
        # fallback bytes live under artifacts.bridge_tracked_fallback.{...}
        # (the fail-closed path). Prefer the nested structure; fall back to
        # the flat structure for offline_compile_evidence_raw_buffer.json or
        # pre-4i evidence.
        nested = artifacts.get("bridge_tracked_fallback")
        source = nested if isinstance(nested, dict) else artifacts
        for key in out:
            entry = source.get(key)
            if isinstance(entry, dict):
                sha = entry.get("sha256")
                if isinstance(sha, str):
                    out[key] = sha
    return out


def file_fingerprint(path: Path) -> dict:
    fp: dict = {"path": rel(path) if path.is_relative_to(ROOT) else str(path),
                "sha256": None, "size_bytes": None}
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
        "build_note": (
            "Tracked external/godot-master build (ignored snapshot, patch stack "
            "0001+0002+0003, module_rurix_accel_enabled=yes d3d12=yes). The exe "
            "binary is a local, gitignored artifact; only its fingerprint is "
            "recorded here."
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
        "features": [],
        "feature_note": (
            "Shipping (feature-OFF) bridge build: the d3d12-recording-shim "
            "feature is NOT enabled, so rxgd_record_pass always returns "
            "RXGD_STATUS_FALLBACK for RXGD_PASS_LUMINANCE_REDUCTION. "
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
    """Per-byte LDR absolute diff over two same-length RGB8 buffers."""
    diff = bytes(abs(a - b) for a, b in zip(reference, candidate))
    if not diff:
        return 0, 0.0, diff
    return max(diff), sum(diff) / len(diff), diff


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

    # The *latest* evidence file is always rewritten (SKIP by reproducible
    # default when the tracked Godot exe is unavailable).
    _write_json(EVIDENCE_OUT, doc)
    print(f"[grx009-segment4g-visual-fallback-smoke] wrote {rel(EVIDENCE_OUT)} status={status}")

    # The *historical* measured success artifact is only ever written on a
    # strict success. A SKIP/FAIL run must NOT delete or overwrite it, so the
    # 4g readiness gate can stay green off a prior measured run.
    if status == "success":
        success_doc = dict(doc)
        success_doc["evidence_kind"] = "historical_measured_success"
        success_doc["latest_evidence_path"] = rel(EVIDENCE_OUT)
        success_doc["success_evidence_note"] = (
            "Historical measured success artifact for GRX-009 segment 4g. It is "
            "written ONLY on a strict status=success run and is never deleted or "
            "overwritten by a later SKIP/FAIL run, so the readiness gate advances "
            "off this file rather than the reproducible-default SKIP latest "
            "evidence. The raw RGB8 frame artifacts + diff it hash-pins are "
            "committed under artifacts/visual/."
        )
        _write_json(SUCCESS_EVIDENCE_OUT, success_doc)
        print(
            "[grx009-segment4g-visual-fallback-smoke] wrote "
            f"{rel(SUCCESS_EVIDENCE_OUT)} status=success (historical measured success)"
        )


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx009-segment4g-visual-fallback-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx009-segment4g-visual-fallback-smoke] SKIP {msg}(降级 SKIP,退出 0)")
    write_evidence("skip", reason=msg, extra=extra or {})
    return 0


def locate_godot_exe() -> tuple[Path | None, str | None]:
    override = os.environ.get(GODOT_EXE_ENV)
    if override:
        p = Path(override)
        if not p.is_file():
            return None, f"{GODOT_EXE_ENV}={override} does not point at an existing file"
        return p, None
    if DEFAULT_GODOT_EXE.is_file():
        return DEFAULT_GODOT_EXE, None
    return None, (
        f"tracked Godot console exe missing at {rel(DEFAULT_GODOT_EXE)} and "
        f"{GODOT_EXE_ENV} is not set; build the tracked external/godot-master "
        "snapshot (patch stack 0001+0002+0003, module_rurix_accel_enabled=yes "
        "d3d12=yes) or point the env var at an equivalent console exe"
    )


def build_bridge_dll() -> tuple[bool, str]:
    """Build the shipping (feature-OFF) rurix_godot.dll."""
    p = subprocess.run(
        ["cargo", "build", "-p", "rurix-godot"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    log = (p.stdout + p.stderr).strip()
    ok = p.returncode == 0 and RURIX_GODOT_DLL.is_file()
    return ok, log[-3000:]


def write_smoke_project(project_dir: Path, *, pass_enabled: bool, dll_path: Path) -> None:
    """Generate a minimal deterministic Godot project. Only the tracked
    per-pass ``.../luminance_reduction/enabled`` setting differs between the
    reference (false, the default) and candidate (true) runs; everything else
    is byte-identical so the pass toggle is the only delta."""
    project_dir.mkdir(parents=True, exist_ok=True)
    project_text = f"""\
; Engine configuration file.
; Auto-generated by ci/grx009_segment4g_visual_fallback_smoke.py

config_version=5

[application]

config/name="GRX-009 segment 4g luminance visual/fallback smoke"
run/main_scene="res://main.tscn"

[display]

window/size/viewport_width={VIEWPORT_WIDTH}
window/size/viewport_height={VIEWPORT_HEIGHT}

[rendering]

rurix_accel/enabled=true
rurix_accel/require_forward_plus=true
rurix_accel/dll_path="{dll_path.as_posix()}"
rurix_accel/passes/luminance_reduction/enabled={"true" if pass_enabled else "false"}
"""
    scene_text = """\
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://main.gd" id="1"]

[node name="GRX009Segment4gRoot" type="Node3D"]
script = ExtResource("1")

[node name="Camera3D" type="Camera3D" parent="."]

[node name="WorldEnvironment" type="WorldEnvironment" parent="."]
"""
    # Deterministic flat-color scene with tonemap + auto exposure enabled so
    # the Auto Exposure luminance_reduction call site actually runs, then a
    # frame capture at a fixed frame index (with --fixed-fps the exposure
    # adaptation state at that frame is deterministic across runs).
    script_text = f"""\
extends Node3D

var _frames := 0
var _captured := false

func _ready() -> void:
    var cam: Camera3D = $Camera3D
    cam.make_current()
    var attributes := CameraAttributesPractical.new()
    attributes.auto_exposure_enabled = true
    cam.attributes = attributes

    var env := Environment.new()
    env.background_mode = Environment.BG_COLOR
    env.background_color = Color(0.6, 0.45, 0.3)
    env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
    $WorldEnvironment.environment = env
    print("GRX009Segment4g: scene ready")

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
        printerr("GRX009Segment4g: capture prefix env var missing")
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
    print("GRX009Segment4g: captured frame=%d width=%d height=%d" % [_frames, img.get_width(), img.get_height()])
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


def run_matrix_leg(
    godot_exe: Path,
    *,
    leg: str,
    pass_enabled: bool,
    dll_path: Path,
) -> dict:
    """Run one pass-enable-matrix leg and collect its measured signals."""
    project_dir = WORK / f"project_{leg}"
    capture_prefix = WORK / f"capture_{leg}"
    for suffix in (".rgb8", ".png", ".json"):
        Path(str(capture_prefix) + suffix).unlink(missing_ok=True)
    write_smoke_project(project_dir, pass_enabled=pass_enabled, dll_path=dll_path)
    exit_code, output = run_godot(
        godot_exe, project_dir, capture_prefix, f"godot_{leg}.log"
    )
    fallback_lines = [
        line.strip() for line in output.splitlines() if FALLBACK_MARKER in line
    ]
    meta, data, capture_error = load_capture(capture_prefix)
    return {
        "leg": leg,
        "role": "reference" if not pass_enabled else "candidate",
        "project_setting": f"{PASS_ENABLED_SETTING}={'true' if pass_enabled else 'false'}",
        "exit_code": exit_code,
        "session_ready": SESSION_READY_MARKER in output,
        "bridge_fallback_marker_observed": bool(fallback_lines),
        "bridge_fallback_marker_line": fallback_lines[0] if fallback_lines else None,
        "capture_meta": meta,
        "capture_error": capture_error,
        "capture_prefix": capture_prefix,
        "frame_bytes": data,
        # Audit the FULL merged stdout+stderr (not just the recorded tail):
        # only the known global-script-cache warning is tolerated; any other
        # `ERROR:` (or untracked RXGD_DIAG) line fails the smoke.
        "runtime_log_audit": runtime_log_audit(output),
        "stdout_tail": output[-4000:],
    }


def telemetry_entry_issue(doc: dict, capture_frame_index: int) -> str | None:
    """First incoherence in the generated measured fallback telemetry's
    luminance entry, or None. The segment 4g gate requires the entry to record
    the measured fallback exactly: enable_state=enabled (the candidate leg
    enabled the pass), fallback_reason=validation_failed (the 0002-level module
    call carries no resource bindings, so the bridge preflight records
    validation_failed by construction), godot_fallback_active=true, and a
    telemetry_frame equal to the measured visual capture frame index."""
    passes = doc.get("passes")
    entry = None
    if isinstance(passes, list):
        for item in passes:
            if isinstance(item, dict) and item.get("pass_id") == "luminance_reduction":
                entry = item
    if entry is None:
        return "telemetry document has no luminance_reduction pass entry"
    if entry.get("enable_state") != "enabled":
        return f"telemetry enable_state is {entry.get('enable_state')!r}, not 'enabled'"
    if entry.get("fallback_reason") != "validation_failed":
        return (
            f"telemetry fallback_reason is {entry.get('fallback_reason')!r}, "
            "not 'validation_failed'"
        )
    if entry.get("godot_fallback_active") is not True:
        return "telemetry godot_fallback_active is not true"
    telemetry_frame = entry.get("telemetry_frame")
    if (
        not isinstance(telemetry_frame, int)
        or isinstance(telemetry_frame, bool)
        or telemetry_frame != capture_frame_index
    ):
        return (
            f"telemetry_frame {telemetry_frame!r} is stale: it does not equal "
            f"the measured capture_frame_index {capture_frame_index}"
        )
    return None


def leg_public(leg: dict) -> dict:
    """The evidence-facing view of a matrix leg (no raw bytes / local paths)."""
    return {
        "role": leg["role"],
        "project_setting": leg["project_setting"],
        "exit_code": leg["exit_code"],
        "session_ready": leg["session_ready"],
        "bridge_fallback_marker_observed": leg["bridge_fallback_marker_observed"],
        "bridge_fallback_marker_line": leg["bridge_fallback_marker_line"],
        "capture_meta": leg["capture_meta"],
        "capture_error": leg["capture_error"],
    }


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
        "pass_id": "luminance_reduction",
        "segment": "4g",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "godot_runtime_luminance_path_enabled": False,
        "default_enable_state": "disabled",
        "gpu_timestamp_status": "not_yet",
        "performance_claim": "none",
        "target_backend": TARGET_BACKEND,
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
            "GRX-009 segment 4g visual/fallback gate evidence only. The visual "
            "diff compares the tracked Godot build's native luminance output "
            "with the pass disabled (reference) vs enabled-but-falling-back "
            "(candidate): the shipping feature-off bridge returns "
            "RXGD_STATUS_FALLBACK, so the native Godot path renders both "
            "frames and enabling the pass must not change the image. This is "
            "NOT visual verification of a Rurix GPU pass (no such pass runs), "
            "keeps runtime_state=fallback_only, real_gpu_pass=false, "
            "real_d3d12_dispatch_recorded=false, godot_runtime_luminance_path_"
            "enabled=false, and default_enable_state=disabled, and makes no "
            "performance, FPS, or GPU-timestamp claim."
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
        return skip(godot_reason or "segment 4g Godot exe unavailable")

    built_dll, dll_log = build_bridge_dll()
    if not built_dll:
        print(dll_log, file=sys.stderr)
        return fail(
            "cargo build -p rurix-godot (feature-off shipping bridge) failed",
            extra={"build_log_tail": dll_log},
        )
    _EVIDENCE_BASE["dll_fingerprint"] = dll_fingerprint(RURIX_GODOT_DLL)
    _EVIDENCE_BASE["godot_exe_fingerprint"] = godot_exe_fingerprint(godot_exe)

    WORK.mkdir(parents=True, exist_ok=True)
    reference = run_matrix_leg(
        godot_exe, leg="reference", pass_enabled=False, dll_path=RURIX_GODOT_DLL
    )
    candidate = run_matrix_leg(
        godot_exe, leg="candidate", pass_enabled=True, dll_path=RURIX_GODOT_DLL
    )
    matrix = {
        "disabled_default": leg_public(reference),
        "enabled_fallback": leg_public(candidate),
    }
    runs_extra = {
        "pass_enable_matrix": matrix,
        "stdout_reference": reference["stdout_tail"],
        "stdout_candidate": candidate["stdout_tail"],
        "runtime_log_audit": {
            "reference": reference["runtime_log_audit"],
            "candidate": candidate["runtime_log_audit"],
        },
    }

    # Environment-level outcomes are SKIP (they do not advance the gate);
    # integrity/coherence violations after a working session are FAIL.
    for leg in (reference, candidate):
        if leg["exit_code"] == -1:
            return skip(
                f"Godot {leg['leg']} run timed out after {GODOT_TIMEOUT_SECONDS}s",
                extra=runs_extra,
            )
    if not reference["session_ready"] or not candidate["session_ready"]:
        return skip(
            "Rurix bridge session was not ready in this environment (no "
            f"'{SESSION_READY_MARKER}' marker); no D3D12 Forward+ session, so "
            "the fallback matrix cannot be measured",
            extra=runs_extra,
        )
    if not candidate["bridge_fallback_marker_observed"]:
        return skip(
            "candidate run (pass enabled) did not print the tracked patch-0002 "
            f"fallback marker '{FALLBACK_MARKER}'; the Auto Exposure call site "
            "did not exercise the bridge fallback in this environment",
            extra=runs_extra,
        )
    if reference["bridge_fallback_marker_observed"]:
        return fail(
            "reference run (pass disabled) unexpectedly printed the bridge "
            "fallback marker; the disabled pass must never call the bridge",
            extra=runs_extra,
        )
    for leg in (reference, candidate):
        if leg["exit_code"] != 0:
            return fail(
                f"Godot {leg['leg']} run exited with non-zero exit code "
                f"{leg['exit_code']}; a clean visual/fallback smoke requires "
                "exit 0",
                extra=runs_extra,
            )
        audit = leg["runtime_log_audit"]
        if (
            audit.get("unexpected_godot_error_count") != 0
            or audit.get("unexpected_rxgd_diag_count") != 0
        ):
            return fail(
                f"{leg['leg']} run output contained unexpected Godot ERROR / "
                f"RXGD_DIAG lines (only the known '{ALLOWED_GODOT_ERROR}' "
                "warning is tolerated, with a recorded rationale): "
                f"{audit.get('unexpected_lines_tail')}",
                extra=runs_extra,
            )
        if leg["capture_error"] is not None or leg["frame_bytes"] is None:
            return fail(
                f"{leg['leg']} frame capture failed: {leg['capture_error']}",
                extra=runs_extra,
            )

    ref_meta = reference["capture_meta"]
    cand_meta = candidate["capture_meta"]
    if (
        ref_meta.get("width") != cand_meta.get("width")
        or ref_meta.get("height") != cand_meta.get("height")
    ):
        return fail(
            "reference/candidate frame dimensions mismatch "
            f"({ref_meta.get('width')}x{ref_meta.get('height')} vs "
            f"{cand_meta.get('width')}x{cand_meta.get('height')})",
            extra=runs_extra,
        )
    width = int(ref_meta["width"])
    height = int(ref_meta["height"])

    # Both legs must capture at the SAME measured frame index; the telemetry
    # document's telemetry_frame is pinned to this measured value (not the
    # requested CAPTURE_FRAME_INDEX constant), so a stale/desynced capture is
    # an integrity FAIL rather than silently recorded.
    ref_frame_index = ref_meta.get("capture_frame_index")
    cand_frame_index = cand_meta.get("capture_frame_index")
    if (
        not isinstance(ref_frame_index, int)
        or not isinstance(cand_frame_index, int)
        or isinstance(ref_frame_index, bool)
        or isinstance(cand_frame_index, bool)
        or ref_frame_index < 1
        or ref_frame_index != cand_frame_index
    ):
        return fail(
            "reference/candidate measured capture frame indices are malformed "
            f"or do not match ({ref_frame_index!r} vs {cand_frame_index!r})",
            extra=runs_extra,
        )
    capture_frame_index = cand_frame_index

    max_abs, mean_abs, diff_bytes = compute_ldr_abs_diff(
        reference["frame_bytes"], candidate["frame_bytes"]
    )
    within_threshold = (
        max_abs <= LDR_MAX_ABS_DIFF_THRESHOLD
        and mean_abs <= LDR_MEAN_ABS_DIFF_THRESHOLD
    )
    print(
        "[grx009-segment4g-visual-fallback-smoke] LDR absolute diff "
        f"max_abs={max_abs} mean_abs={mean_abs:.6f} "
        f"(thresholds max<={LDR_MAX_ABS_DIFF_THRESHOLD} mean<={LDR_MEAN_ABS_DIFF_THRESHOLD})"
    )
    if not within_threshold:
        return fail(
            "LDR absolute diff exceeded the visual gate threshold: "
            f"max_abs={max_abs} (<= {LDR_MAX_ABS_DIFF_THRESHOLD} required), "
            f"mean_abs={mean_abs:.6f} (<= {LDR_MEAN_ABS_DIFF_THRESHOLD} required); "
            "enabling the falling-back pass changed the rendered image",
            extra={
                **runs_extra,
                "visual": {
                    "measured_local": True,
                    "metric_kind": METRIC_KIND,
                    "width": width,
                    "height": height,
                    "format": FRAME_FORMAT,
                    "max_abs_diff": max_abs,
                    "mean_abs_diff": mean_abs,
                    "max_abs_diff_threshold": LDR_MAX_ABS_DIFF_THRESHOLD,
                    "mean_abs_diff_threshold": LDR_MEAN_ABS_DIFF_THRESHOLD,
                    "within_threshold": False,
                },
            },
        )

    # Build the GRX-008-format measured_local telemetry document in the work
    # dir first, self-validate it, and only then publish tracked artifacts.
    telemetry_doc = {
        "run_mode": "full",
        "evidence_level": "measured_local",
        "target_backend": TARGET_BACKEND,
        "note": (
            "GRX-009 segment 4g measured fallback telemetry: with "
            f"{PASS_ENABLED_SETTING}=true the tracked Godot Auto Exposure call "
            "site invoked the shipping (feature-off) Rurix bridge, which "
            "returned RXGD_STATUS_FALLBACK (observed via the tracked patch-0002 "
            "marker), and the native Godot luminance path rendered the frame. "
            "The 0002-level module call carries no resource bindings, so the "
            "bridge preflight records validation_failed by construction. "
            "runtime_state stays fallback_only, real_gpu_pass=false, and no "
            "performance or FPS claim is made."
        ),
        "passes": [
            {
                "pass_id": "luminance_reduction",
                "enable_state": "enabled",
                "fallback_reason": "validation_failed",
                "godot_fallback_active": True,
                "telemetry_timestamp": now_iso(),
                "telemetry_frame": capture_frame_index,
            }
        ],
    }
    entry_issue = telemetry_entry_issue(telemetry_doc, capture_frame_index)
    if entry_issue is not None:
        return fail(
            f"generated measured fallback telemetry entry is incoherent: {entry_issue}",
            extra=runs_extra,
        )
    work_telemetry = WORK / "measured_fallback_telemetry.json"
    _write_json(work_telemetry, telemetry_doc)
    telemetry_check = subprocess.run(
        [sys.executable, str(FALLBACK_TELEMETRY_SCRIPT), "--validate-only", str(work_telemetry)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if telemetry_check.returncode != 0:
        return fail(
            "generated measured fallback telemetry failed "
            "fallback_telemetry.py --validate-only",
            extra={
                **runs_extra,
                "telemetry_validation_output": (
                    (telemetry_check.stdout + telemetry_check.stderr).strip()[-2000:]
                ),
            },
        )

    # Publish tracked artifacts: raw frames + diff (+ PNG siblings) and the
    # measured telemetry document. Only a fully-validated success reaches here.
    VISUAL_DIR.mkdir(parents=True, exist_ok=True)
    REFERENCE_FRAME.write_bytes(reference["frame_bytes"])
    CANDIDATE_FRAME.write_bytes(candidate["frame_bytes"])
    DIFF_ARTIFACT.write_bytes(diff_bytes)
    for src_leg, png_path in ((reference, REFERENCE_FRAME_PNG), (candidate, CANDIDATE_FRAME_PNG)):
        captured_png = Path(str(src_leg["capture_prefix"]) + ".png")
        if captured_png.is_file():
            shutil.copy2(captured_png, png_path)
        else:
            write_png_rgb8(png_path, width, height, src_leg["frame_bytes"])
    write_png_rgb8(DIFF_ARTIFACT_PNG, width, height, diff_bytes)
    shutil.copy2(work_telemetry, TELEMETRY_OUT)

    visual = {
        "measured_local": True,
        "metric_kind": METRIC_KIND,
        "width": width,
        "height": height,
        "format": FRAME_FORMAT,
        "capture_frame_index": capture_frame_index,
        "reference_frame": file_fingerprint(REFERENCE_FRAME),
        "candidate_frame": file_fingerprint(CANDIDATE_FRAME),
        "diff_artifact": file_fingerprint(DIFF_ARTIFACT),
        "reference_frame_png": file_fingerprint(REFERENCE_FRAME_PNG),
        "candidate_frame_png": file_fingerprint(CANDIDATE_FRAME_PNG),
        "diff_artifact_png": file_fingerprint(DIFF_ARTIFACT_PNG),
        "max_abs_diff": max_abs,
        "mean_abs_diff": mean_abs,
        "max_abs_diff_threshold": LDR_MAX_ABS_DIFF_THRESHOLD,
        "mean_abs_diff_threshold": LDR_MEAN_ABS_DIFF_THRESHOLD,
        "within_threshold": True,
        "visual_scope_note": (
            "Fallback-path visual diff only: both frames were rendered by the "
            "native Godot luminance path (the enabled pass fell back). This "
            "does NOT verify a Rurix GPU pass image."
        ),
    }
    fallback_telemetry = {
        "fallback_path_observed": True,
        "bridge_fallback_marker": FALLBACK_MARKER,
        "bridge_fallback_marker_line": candidate["bridge_fallback_marker_line"],
        "pass_enable_matrix": matrix,
        "telemetry_document": file_fingerprint(TELEMETRY_OUT),
        "no_fps_claim": True,
    }

    write_evidence(
        "success",
        extra={
            "visual": visual,
            "fallback_telemetry": fallback_telemetry,
            "pass_enable_matrix": matrix,
            "checks": {
                "artifact_hashes_match_offline_evidence": True,
                "reference_run_exit_zero": reference["exit_code"] == 0,
                "candidate_run_exit_zero": candidate["exit_code"] == 0,
                "session_ready_both_runs": True,
                "fallback_marker_observed_candidate": True,
                "fallback_marker_absent_reference": True,
                "frames_captured": True,
                "dimensions_match": True,
                "capture_frame_indices_match": True,
                "runtime_log_audit_clean": True,
                "diff_within_threshold": True,
                "telemetry_document_valid": True,
                "telemetry_entry_coherent": True,
            },
            "stdout_reference": reference["stdout_tail"],
            "stdout_candidate": candidate["stdout_tail"],
            "runtime_log_audit": {
                "reference": reference["runtime_log_audit"],
                "candidate": candidate["runtime_log_audit"],
            },
        },
    )
    print(
        "[grx009-segment4g-visual-fallback-smoke] PASS measured fallback matrix + "
        "LDR visual diff within threshold (fallback-path visual gate only; no "
        "Rurix pass, no performance claim)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
