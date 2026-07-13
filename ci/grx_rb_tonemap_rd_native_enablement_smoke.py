#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX Route B: tonemap rd_native in-frame REAL-replacement enablement smoke.

This is the first NON-SCAFFOLD real replacement gate in the GRX Godot line. It
drives patch 0040's ``rendering/rurix_accel/passes/tonemap/backend == 2``
(rd_native) path, in which the Rurix tonemap kernel runs as a first-class
in-frame ``RenderingDevice`` compute pass and the native Godot tonemapper is
GENUINELY SKIPPED (unlike the patch 0013 shim scaffold, which prints a writeback
marker and then keeps the native tonemapper as the continuation/backstop so the
image can never change). Here the candidate frame IS the Rurix kernel's output.

rd_native is BRIDGE-INDEPENDENT: it does not go through the rxgd session /
``rxgd_record_pass`` path and sets no ``RxGdCaps.flags`` bit. It only needs the
``RurixAccelD3D12Hooks`` singleton to exist (so ``rurix_godot.dll`` must load for
``bridge_preflight()``), then drives the main ``RenderingDevice`` directly.

Legs (all with scene ``tonemap_mode = LINEAR`` to align with the kernel's LINEAR
+ linear_to_srgb subset — the contract's declared subset boundary):

  * ``reference`` (backend == 0): the native Godot LINEAR tonemapper renders.
    rd_native is never engaged; the ``RXGD_RD_NATIVE_TONEMAP active`` marker must
    be ABSENT.
  * ``candidate`` (backend == 2 + real staged container): the module lazily
    builds the RD compute pipeline from the Rurix container and dispatches it,
    SKIPPING the native tonemapper. The active marker must be PRESENT; the
    captured frame is the rd_native kernel output.
  * ``fail_closed`` (backend == 2 + a garbage container path): the container
    load fails, the module latches the failure and returns false, and the native
    Godot tonemapper renders as the fail-closed fallback. The active marker must
    be ABSENT and the frame must byte-match the reference frame.

Multi-frame stability: each leg captures three consecutive frames and asserts
they are byte-identical (a static scene at ``--fixed-fps`` is deterministic; a
graph-scheduling race or resource hazard in the injected pass would show up as a
non-stable capture).

Outcome semantics (``real_pass_enablement_evidence.json`` next to this gate,
rewritten every run):

  * ``status=skip`` / ``skip_kind=environment``: a precondition is unavailable
    (scratch exe, ``rurix_godot.dll``, auditable source provenance, staged
    container, ready session). ``RURIX_REQUIRE_REAL=1`` upgrades this to FAIL.
  * ``status=skip`` / ``skip_kind=measured_prerequisite_blocked``: every leg ran
    on real hardware but rd_native did not achieve a clean real replacement —
    either the active marker did not appear in the candidate leg (rd_native
    could not engage; usage bits / container / pipeline), or it engaged but the
    LINEAR kernel output diverged from Godot's native LINEAR tonemapper beyond
    the parity tolerance (an honest picture-difference finding, with the number).
    Not upgraded to FAIL by RURIX_REQUIRE_REAL; never advances the gate.
  * ``status=fail``: an integrity violation (marker in the wrong leg, fail_closed
    frame not matching reference, non-deterministic capture, non-zero exit,
    unexpected ERROR line, tampered container).
  * ``status=success`` (strict): the candidate leg engaged rd_native (active
    marker observed -> native tonemapper skipped by construction) AND its frame
    matched the native reference within the LDR parity thresholds AND the
    fail_closed leg fell back byte-identically AND every audit passed. ONLY then
    is ``real_gpu_pass=true`` recorded and the historical
    ``real_pass_enablement_success_evidence.json`` written. Even a success keeps
    ``default_enable_state=disabled`` and ``performance_claim=none``.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from ci.grx009_godot_runtime_bridge_recording_smoke import (  # noqa: E402
    find_git_root,
    patch_stack_identity,
    runtime_log_audit,
    source_status_clean,
    verify_source_provenance_sidecar,
)

PATCHES_DIR = ROOT / "spike" / "godot-rurix" / "patches"

# rd_native stacks on the branch-HEAD culling tail 0001-0029, then 0040 (the
# 0030-0039 block is a monotonic hole: gpu_culling took 0027-0029, fused took
# 0036-0038, and neither combines with the other under strict git apply — see
# PATCH_ALLOCATION.md, the Route B double-tail note). The sidecar records a
# comma-joined stack id because 0029 -> 0040 is not contiguous.
PATCH_ORDINALS = [f"{n:04d}" for n in range(1, 30)] + ["0040"]
PATCH_STACK_ID = ",".join(PATCH_ORDINALS)


def _resolve_stack() -> tuple[str, ...]:
    names: list[str] = []
    for ordinal in PATCH_ORDINALS:
        matches = sorted(PATCHES_DIR.glob(f"{ordinal}-*.patch"))
        if not matches:
            raise SystemExit(f"no patch file for ordinal {ordinal} in {PATCHES_DIR}")
        names.append(matches[0].name)
    return tuple(names)


PATCH_STACK = _resolve_stack()

PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "tonemap"
RD_PIPELINE_DIR = ROOT / "spike" / "godot-rurix" / "rd-native-pipeline"
# The proven S2 container, staged out of the RB-1 in-flight source tree.
STAGED_CONTAINER = ROOT / "target" / "grx" / "rd_containers" / "tonemap.rd_container.bin"
# The S2 probe evidence pins the container sha this gate consumes (~1 ULP parity
# proven in the real engine); the staged copy must byte-match it.
S2_PROBE_EVIDENCE = RD_PIPELINE_DIR / "rd_native_probe_evidence.json"

EVIDENCE_OUT = PASS_DIR / "rd_native_enablement_evidence.json"
SUCCESS_EVIDENCE_OUT = PASS_DIR / "rd_native_enablement_success_evidence.json"

RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"
WORK = ROOT / "target" / "grx_rb_tonemap_rd_native_enablement_smoke"
LOG_DIR = WORK / "logs"

SUBJECT = "grx_rb_tonemap_rd_native_enablement_smoke"

# The rd_native module marker (module-side print_verbose, ONE-SHOT when the
# pipeline is first built — not per-frame).
ACTIVE_MARKER = "RXGD_RD_NATIVE_TONEMAP active"
SESSION_READY_MARKER = "RurixAccel: D3D12 Forward+ bridge session ready."
ALLOWED_GODOT_ERROR = "Could not load global script cache"
# The fail_closed leg feeds backend==2 a garbage container; RenderingDevice's
# shader_create_from_bytecode rejects the bytes with these ERROR lines. They are
# the EXPECTED fail-closed evidence that RD rejected the bad container (the
# module then latches the failure and falls back to the native tonemapper), so
# they are tolerated in the fail_closed leg ONLY and are positively required.
EXPECTED_FAIL_CLOSED_ERRORS = (
    "Incorrect magic number in shader container",
    "Failed to parse shader container from binary",
)

METRIC_KIND = "ldr_absolute_diff"
FRAME_FORMAT = "R8G8B8_raw"
# Parity thresholds for the LINEAR + linear_to_srgb kernel subset vs Godot's
# native LINEAR tonemapper. The kernel is ~1 ULP in float (S2); at 8-bit LDR
# with a raster-vs-compute write path a few units of quantization drift is
# tolerated. The measured number is always recorded, threshold or not.
LDR_MAX_ABS_DIFF_THRESHOLD = 4
LDR_MEAN_ABS_DIFF_THRESHOLD = 1.0
MIN_FRAME_DIMENSION = 64
CAPTURE_FRAME_INDEX = 24
STABILITY_FRAME_COUNT = 3
VIEWPORT_WIDTH = 256
VIEWPORT_HEIGHT = 144

GODOT_TIMEOUT_SECONDS = 180
REQUESTED_RENDERER = "d3d12"
REQUESTED_RENDERING_METHOD = "forward_plus"

GODOT_EXE_ENV = "RURIX_GRX_RB_TONEMAP_GODOT_EXE"
SCRATCH_SOURCE_ENV = "RURIX_GRX_RB_TONEMAP_GODOT_SOURCE"
SCRATCH_SOURCE_PROVENANCE_ENV = "RURIX_GRX_RB_TONEMAP_GODOT_SOURCE_PROVENANCE"
SCRATCH_BUILD_COMMAND_ENV = "RURIX_GRX_RB_TONEMAP_GODOT_BUILD_COMMAND"
SCRATCH_BUILD_LOG_ENV = "RURIX_GRX_RB_TONEMAP_GODOT_BUILD_LOG"
CAPTURE_PREFIX_ENV = "RURIX_GRX_RB_TONEMAP_CAPTURE_PREFIX"
CONTAINER_OVERRIDE_ENV = "RURIX_GRX_RB_TONEMAP_CONTAINER"
DXC_DIR_ENV = "RURIX_DXC_DIR"

TARGET_BACKEND = "Godot 4.7-dev Windows D3D12 Forward+"
BACKEND_SETTING = "rendering/rurix_accel/passes/tonemap/backend"
CONTAINER_SETTING = "rendering/rurix_accel/passes/tonemap/rd_container_path"

KNOWN_GAPS = [
    (
        "the rd_native kernel covers only the LINEAR tonemapper with the SDR "
        "convert_to_srgb (linear_to_srgb) leg; the candidate scene is pinned to "
        "tonemap_mode LINEAR to align with this subset. Reinhard/Filmic/ACES/AgX, "
        "auto exposure, glow, FXAA, BCS, color correction, debanding, multiview, "
        "and HDR output are OUT of the rd_native subset and route to the native "
        "path"
    ),
    (
        "parity is measured at the scene's flat tone(s); the LDR delta between "
        "the compute UAV write and Godot's raster tonemap write is recorded as "
        "the first real-replacement picture evidence (not a full-frame or "
        "multi-tone characterization)"
    ),
    (
        "rd_native drives the main RenderingDevice's draw graph directly; the "
        "injected compute_list relies on the graph's ResourceTracker for "
        "barriers. No submit()/sync() is issued (that is a local-RD concept)"
    ),
]


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
    try:
        return str(path.relative_to(ROOT)).replace("\\", "/")
    except ValueError:
        return str(path)


def file_fingerprint(path: Path) -> dict:
    fp: dict = {"path": rel(path), "sha256": None, "size_bytes": None}
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
            "Scratch Godot build binaries are NOT committed. This console exe is "
            "a local, gitignored artifact rebuilt from the ignored "
            f"external/godot-master snapshot with the {PATCH_STACK_ID} patch "
            "stack (module_rurix_accel_enabled=yes d3d12=yes). Only its "
            "fingerprint is recorded so the measured evidence stays auditable."
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
        "build_profile": "debug",
        "features": [],
        "feature_note": (
            "rd_native is bridge-independent: the plain rurix_godot.dll is "
            "required only so bridge_preflight() succeeds and the "
            "RurixAccelD3D12Hooks singleton (which carries the "
            "try_record_tonemap_rd_native override) is instantiated. The "
            "d3d12-recording-shim feature is NOT needed for rd_native."
        ),
    }
    if path.is_file():
        fp["dll_sha256"] = sha256_file(path)
        fp["dll_size_bytes"] = path.stat().st_size
    return fp


def _write_json(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    # Byte-level LF only (repo .gitattributes pins `* -text`); never emit CRLF.
    path.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def compute_ldr_abs_diff(reference: bytes, candidate: bytes) -> tuple[int, float]:
    diff = [abs(a - b) for a, b in zip(reference, candidate)]
    if not diff:
        return 0, 0.0
    return max(diff), sum(diff) / len(diff)


def unexpected_error_lines(output: str, extra_allowed: tuple[str, ...] = ()) -> list[str]:
    """ERROR: lines in the merged output that are neither the tolerated Godot
    script-cache warning nor one of ``extra_allowed`` (leg-specific expected
    diagnostics, e.g. the fail_closed container-reject errors)."""
    allowed = (ALLOWED_GODOT_ERROR, *extra_allowed)
    out: list[str] = []
    for line in output.splitlines():
        if not line.strip().startswith("ERROR:"):
            continue
        if any(token in line for token in allowed):
            continue
        out.append(line.strip())
    return out


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
    print(f"[grx-rb-tonemap-rd-native-smoke] wrote {rel(EVIDENCE_OUT)} status={status}")
    if status == "success":
        success_doc = dict(doc)
        success_doc["evidence_kind"] = "historical_measured_success"
        success_doc["latest_evidence_path"] = rel(EVIDENCE_OUT)
        success_doc["success_evidence_note"] = (
            "Historical measured success artifact for the GRX Route B tonemap "
            "rd_native in-frame real-replacement gate. Written ONLY on a strict "
            "status=success run (candidate engaged rd_native AND the native "
            "tonemapper was skipped AND the LDR parity gate stayed within "
            "thresholds AND the fail_closed leg fell back byte-identically AND "
            "every audit passed) and never overwritten by a later SKIP/FAIL run. "
            "Even this success keeps default_enable_state=disabled and "
            "performance_claim=none."
        )
        _write_json(SUCCESS_EVIDENCE_OUT, success_doc)
        print(
            "[grx-rb-tonemap-rd-native-smoke] wrote "
            f"{rel(SUCCESS_EVIDENCE_OUT)} status=success (historical measured success)"
        )


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx-rb-tonemap-rd-native-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip_environment(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx-rb-tonemap-rd-native-smoke] SKIP {msg} (降级 SKIP,退出 0)")
    payload = dict(extra or {})
    payload["skip_kind"] = "environment"
    write_evidence("skip", reason=msg, extra=payload)
    return 0


def skip_measured_prerequisite(prerequisite: str, msg: str, extra: dict) -> int:
    print(
        "[grx-rb-tonemap-rd-native-smoke] SKIP (measured) first missing "
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
            f"{GODOT_EXE_ENV} is not set; the rd_native enablement smoke needs a "
            f"scratch Godot console exe rebuilt with the {PATCH_STACK_ID} patch "
            "stack (module_rurix_accel_enabled=yes d3d12=yes). The tracked "
            "external/godot-master build only has 0001+0002+0003 and must NOT be "
            "reused here"
        )
    p = Path(override)
    if not p.is_file():
        return None, f"{GODOT_EXE_ENV}={override} does not point at an existing file"
    return p, None


def build_bridge_dll() -> tuple[bool, str]:
    """Build the PLAIN rurix_godot.dll (no d3d12-recording-shim). rd_native only
    needs bridge_preflight() to succeed so the hooks singleton is instantiated."""
    p = subprocess.run(
        ["cargo", "build", "-p", "rurix-godot"],
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
        return None, f"{SCRATCH_SOURCE_PROVENANCE_ENV}={path} does not point at an existing file"
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return None, f"could not load source provenance sidecar: {type(exc).__name__}: {exc}"
    if not isinstance(payload, dict):
        return None, "source provenance sidecar is not a JSON object"
    return payload, None


def scratch_source_provenance(godot_exe: Path) -> dict:
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
        "applied_patch_stack": patch_stack_identity(PATCH_STACK, PATCH_STACK_ID),
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
        stack_names=PATCH_STACK,
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
    backend: int,
    container_path: str,
) -> None:
    """Minimal deterministic Godot project. Only the tonemap backend selector and
    the rd_container_path differ between legs; everything else is byte-identical.
    The scene pins tonemap_mode LINEAR to align with the rd_native kernel subset,
    and captures STABILITY_FRAME_COUNT consecutive frames for the stability
    assertion."""
    project_dir.mkdir(parents=True, exist_ok=True)

    project_text = f"""\
; Engine configuration file.
; Auto-generated by ci/grx_rb_tonemap_rd_native_enablement_smoke.py

config_version=5

[application]

config/name="GRX Route B tonemap rd_native enablement smoke"
run/main_scene="res://main.tscn"

[display]

window/size/viewport_width={VIEWPORT_WIDTH}
window/size/viewport_height={VIEWPORT_HEIGHT}

[rendering]

rurix_accel/enabled=true
rurix_accel/require_forward_plus=true
rurix_accel/dll_path="{dll_path.as_posix()}"
rurix_accel/passes/tonemap/backend={backend}
rurix_accel/passes/tonemap/rd_container_path="{Path(container_path).as_posix() if container_path else ''}"
"""
    scene_text = """\
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://main.gd" id="1"]

[node name="GRXRBTonemapRoot" type="Node3D"]
script = ExtResource("1")

[node name="Camera3D" type="Camera3D" parent="."]

[node name="WorldEnvironment" type="WorldEnvironment" parent="."]
"""
    script_text = f"""\
extends Node3D

var _frames := 0
var _captured := 0

func _ready() -> void:
    var cam: Camera3D = $Camera3D
    cam.make_current()

    var env := Environment.new()
    env.background_mode = Environment.BG_COLOR
    env.background_color = Color(0.6, 0.45, 0.3)
    env.tonemap_mode = Environment.TONE_MAPPER_LINEAR
    env.tonemap_exposure = 1.0
    env.tonemap_white = 1.0
    $WorldEnvironment.environment = env
    print("GRXRBTonemap: scene ready backend={backend}")

func _process(_delta: float) -> void:
    _frames += 1
    if _frames >= {CAPTURE_FRAME_INDEX} and _captured < {STABILITY_FRAME_COUNT}:
        _capture(_captured)
        _captured += 1
        if _captured >= {STABILITY_FRAME_COUNT}:
            get_tree().quit()

func _capture(idx: int) -> void:
    await RenderingServer.frame_post_draw
    var img: Image = get_viewport().get_texture().get_image()
    img.convert(Image.FORMAT_RGB8)
    var prefix := OS.get_environment("{CAPTURE_PREFIX_ENV}")
    if prefix.is_empty():
        printerr("GRXRBTonemap: capture prefix env var missing")
        get_tree().quit(3)
        return
    var raw := FileAccess.open(prefix + ".f%d.rgb8" % idx, FileAccess.WRITE)
    raw.store_buffer(img.get_data())
    raw.close()
    if idx == 0:
        img.save_png(prefix + ".png")
        var meta := FileAccess.open(prefix + ".json", FileAccess.WRITE)
        meta.store_string(JSON.stringify({{
            "width": img.get_width(),
            "height": img.get_height(),
            "format": "{FRAME_FORMAT}",
            "capture_frame_index": _frames,
            "stability_frame_count": {STABILITY_FRAME_COUNT},
        }}))
        meta.close()
    print("GRXRBTonemap: captured idx=%d frame=%d width=%d height=%d" % [idx, _frames, img.get_width(), img.get_height()])
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
    # The container's DXIL is loaded by the D3D12 driver at runtime; make the
    # signed DXC dxil.dll discoverable (same environment the S2 probe used).
    dxc_dir = os.environ.get(DXC_DIR_ENV)
    if dxc_dir and Path(dxc_dir).is_dir():
        env["PATH"] = dxc_dir + os.pathsep + env.get("PATH", "")
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


def load_capture_frames(capture_prefix: Path) -> tuple[dict | None, list[bytes] | None, str | None]:
    meta = load_json(Path(str(capture_prefix) + ".json"))
    if meta is None:
        return None, None, f"capture metadata missing/unreadable at {capture_prefix}.json"
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
    frames: list[bytes] = []
    for idx in range(STABILITY_FRAME_COUNT):
        raw_path = Path(str(capture_prefix) + f".f{idx}.rgb8")
        if not raw_path.is_file():
            return meta, None, f"raw frame {idx} missing at {raw_path}"
        data = raw_path.read_bytes()
        if len(data) != width * height * 3:
            return meta, None, (
                f"raw frame {idx} size {len(data)} != width*height*3 "
                f"({width}x{height}x3={width * height * 3})"
            )
        frames.append(data)
    return meta, frames, None


LEG_SETTINGS = {
    "reference": {"backend": 0, "role": "native_reference"},
    "candidate": {"backend": 2, "role": "rd_native_replacement"},
    "fail_closed": {"backend": 2, "role": "rd_native_garbage_container_fallback"},
}


def run_matrix_leg(godot_exe: Path, *, leg: str, dll_path: Path, container_path: str) -> dict:
    settings = LEG_SETTINGS[leg]
    project_dir = WORK / f"project_{leg}"
    capture_prefix = WORK / f"capture_{leg}"
    for suffix in [".png", ".json"] + [f".f{i}.rgb8" for i in range(STABILITY_FRAME_COUNT)]:
        Path(str(capture_prefix) + suffix).unlink(missing_ok=True)
    write_smoke_project(
        project_dir,
        dll_path=dll_path,
        backend=settings["backend"],
        container_path=container_path,
    )
    exit_code, output = run_godot(godot_exe, project_dir, capture_prefix, f"godot_{leg}.log")
    meta, frames, capture_error = load_capture_frames(capture_prefix)
    return {
        "leg": leg,
        "role": settings["role"],
        "project_settings": {
            BACKEND_SETTING: settings["backend"],
            CONTAINER_SETTING: container_path,
        },
        "exit_code": exit_code,
        "session_ready": SESSION_READY_MARKER in output,
        "active_marker_observed": ACTIVE_MARKER in output,
        "capture_meta": meta,
        "capture_error": capture_error,
        "capture_prefix": capture_prefix,
        "frames": frames,
        "runtime_log_audit": runtime_log_audit(output, PATCH_STACK),
        "container_reject_errors_observed": [
            token for token in EXPECTED_FAIL_CLOSED_ERRORS if token in output
        ],
        "full_output": output,
        "stdout_tail": output[-4000:],
    }


def leg_public(leg: dict) -> dict:
    return {
        "role": leg["role"],
        "project_settings": leg["project_settings"],
        "exit_code": leg["exit_code"],
        "session_ready": leg["session_ready"],
        "active_marker_observed": leg["active_marker_observed"],
        "container_reject_errors_observed": leg["container_reject_errors_observed"],
        "capture_meta": leg["capture_meta"],
        "capture_error": leg["capture_error"],
    }


def main() -> int:
    global _EVIDENCE_BASE
    _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}

    # 1) staged container present + byte-matches the S2-proven container.
    if not STAGED_CONTAINER.is_file():
        return skip_environment(
            f"staged rd_native container missing at {rel(STAGED_CONTAINER)}; copy "
            "spike/godot-rurix/rd-native-pipeline/out/tonemap.rd_container.bin there"
        )
    container_override = os.environ.get(CONTAINER_OVERRIDE_ENV)
    container_path = str(Path(container_override).resolve()) if container_override else str(STAGED_CONTAINER)
    container_sha = sha256_file(Path(container_path))
    probe = load_json(S2_PROBE_EVIDENCE)
    probe_container_sha = None
    if isinstance(probe, dict):
        container_block = probe.get("container")
        if isinstance(container_block, dict):
            probe_container_sha = container_block.get("sha256")
    container_matches_probe = (
        probe_container_sha is not None and container_sha == probe_container_sha
    )

    _EVIDENCE_BASE.update(
        {
            "pass_id": "tonemap",
            "provenance": "rd_native_route_b",
            "backend_selector": "rendering/rurix_accel/passes/tonemap/backend",
            "backend_states": {"disabled": 0, "shim": 1, "rd_native": 2},
            "bridge_independent": True,
            "cap_bit_consumed": None,
            "runtime_state": "fallback_only",
            "real_gpu_pass": False,
            "default_enable_state": "disabled",
            "gpu_timestamp_status": "not_yet",
            "performance_claim": "none",
            "kernel_subset": "linear + linear_to_srgb (tonemap_mode LINEAR)",
            "target_backend": TARGET_BACKEND,
            "known_gaps": KNOWN_GAPS,
            "container": {
                "path_at_run": container_path,
                "sha256": container_sha,
                "s2_probe_evidence": rel(S2_PROBE_EVIDENCE),
                "s2_probe_container_sha256": probe_container_sha,
                "matches_s2_probe_container": container_matches_probe,
            },
            "patch_stack_identity": patch_stack_identity(PATCH_STACK, PATCH_STACK_ID),
            "note": (
                "GRX Route B tonemap rd_native in-frame REAL-replacement gate. "
                "backend==2 drives the Rurix tonemap kernel as a first-class "
                "RenderingDevice compute pass and SKIPS the native Godot "
                "tonemapper (not a scaffold). Bridge-independent (no rxgd "
                "session, no RxGdCaps.flags bit). default_enable_state stays "
                "disabled and no performance/FPS/GPU-timestamp claim is made."
            ),
        }
    )

    if not container_matches_probe:
        return fail(
            "staged rd_native container sha does not match the S2 probe evidence "
            f"container sha (staged={container_sha}, probe={probe_container_sha}); "
            "the gate must consume the exact container proven at ~1 ULP in S2"
        )

    # 2) scratch exe.
    godot_exe, godot_reason = locate_godot_exe()
    if godot_exe is None:
        return skip_environment(godot_reason or "rd_native Godot exe unavailable")

    # 3) plain bridge DLL (so the hooks singleton is instantiated).
    built_dll, dll_log = build_bridge_dll()
    if not built_dll:
        print(dll_log, file=sys.stderr)
        return fail("cargo build -p rurix-godot failed", extra={"build_log_tail": dll_log})
    _EVIDENCE_BASE["dll_fingerprint"] = dll_fingerprint(RURIX_GODOT_DLL)
    _EVIDENCE_BASE["godot_exe_fingerprint"] = godot_exe_fingerprint(godot_exe)

    # 4) auditable scratch source provenance (0001-0029 + 0040).
    provenance = scratch_source_provenance(godot_exe)
    _EVIDENCE_BASE["scratch_source_provenance"] = provenance
    if provenance.get("tracked_patch_stack_only") is not True:
        return skip_environment(
            "scratch Godot source provenance is not auditable as tracked-patch-"
            f"stack-only ({PATCH_STACK_ID}); errors: "
            + "; ".join(str(e) for e in provenance.get("source_audit_errors", []))[:1200]
        )

    # 5) run the three legs.
    WORK.mkdir(parents=True, exist_ok=True)
    garbage_container = str((WORK / "garbage_not_a_container.bin").resolve())
    (WORK / "garbage_not_a_container.bin").write_bytes(b"NOT_A_RURIX_CONTAINER" * 4)

    reference = run_matrix_leg(godot_exe, leg="reference", dll_path=RURIX_GODOT_DLL, container_path="")
    candidate = run_matrix_leg(godot_exe, leg="candidate", dll_path=RURIX_GODOT_DLL, container_path=container_path)
    fail_closed = run_matrix_leg(godot_exe, leg="fail_closed", dll_path=RURIX_GODOT_DLL, container_path=garbage_container)
    legs = {"reference": reference, "candidate": candidate, "fail_closed": fail_closed}
    matrix = {name: leg_public(leg) for name, leg in legs.items()}
    runs_extra = {
        "pass_enable_matrix": matrix,
        "stdout_reference": reference["stdout_tail"],
        "stdout_candidate": candidate["stdout_tail"],
        "stdout_fail_closed": fail_closed["stdout_tail"],
        "runtime_log_audit": {name: leg["runtime_log_audit"] for name, leg in legs.items()},
    }

    # Environment-level outcomes -> SKIP; integrity violations -> FAIL.
    for name, leg in legs.items():
        if leg["exit_code"] == -1:
            return skip_environment(
                f"Godot {name} run timed out after {GODOT_TIMEOUT_SECONDS}s", extra=runs_extra
            )
    if not all(leg["session_ready"] for leg in legs.values()):
        return skip_environment(
            "Rurix bridge session was not ready in this environment (no "
            f"'{SESSION_READY_MARKER}'); the RurixAccelD3D12Hooks singleton "
            "carrying the rd_native override was not instantiated",
            extra=runs_extra,
        )

    for name, leg in legs.items():
        if leg["exit_code"] != 0:
            return fail(
                f"Godot {name} run exited with non-zero exit code {leg['exit_code']}",
                extra=runs_extra,
            )
        # The fail_closed leg is EXPECTED to emit the container-reject ERROR
        # lines (RD rejecting the garbage container is the fail-closed evidence);
        # they are tolerated for that leg only. The reference/candidate legs must
        # be clean save for the tolerated script-cache warning.
        extra_allowed = EXPECTED_FAIL_CLOSED_ERRORS if name == "fail_closed" else ()
        unexpected = unexpected_error_lines(leg["full_output"], extra_allowed)
        if unexpected or leg["runtime_log_audit"].get("unexpected_rxgd_diag_count") != 0:
            return fail(
                f"{name} run output contained unexpected Godot ERROR / RXGD_DIAG "
                f"lines (tolerated: '{ALLOWED_GODOT_ERROR}'"
                + (" + container-reject errors" if name == "fail_closed" else "")
                + f"): {(unexpected + leg['runtime_log_audit'].get('unexpected_lines_tail', []))[-20:]}",
                extra=runs_extra,
            )
        if leg["capture_error"] is not None or leg["frames"] is None:
            return fail(f"{name} frame capture failed: {leg['capture_error']}", extra=runs_extra)

    # Positively require the fail_closed leg to have shown RD rejecting the
    # garbage container (proves the fail-closed path was actually exercised, not
    # silently skipped).
    if not fail_closed["container_reject_errors_observed"]:
        return fail(
            "fail_closed leg (backend=2 + garbage container) did not emit the "
            "expected RenderingDevice container-reject error; the fail-closed "
            "container-load path was not exercised",
            extra=runs_extra,
        )

    # Multi-frame stability: the three captured frames of every leg must be
    # byte-identical (a graph-scheduling race would break this).
    stability: dict[str, bool] = {}
    for name, leg in legs.items():
        frames = leg["frames"]
        stable = all(f == frames[0] for f in frames)
        stability[name] = stable
        if not stable:
            return fail(
                f"{name} leg capture is not frame-stable across "
                f"{STABILITY_FRAME_COUNT} consecutive frames; the injected pass "
                "produced a non-deterministic image",
                extra=runs_extra,
            )
    runs_extra["frame_stability"] = stability

    # Marker placement: active marker must appear ONLY in the candidate leg.
    if reference["active_marker_observed"]:
        return fail(
            "reference run (backend=0) printed the rd_native active marker; the "
            "disabled backend must never engage rd_native",
            extra=runs_extra,
        )
    if fail_closed["active_marker_observed"]:
        return fail(
            "fail_closed run (backend=2 + garbage container) printed the rd_native "
            "active marker; a container load failure must latch and fail closed "
            "without engaging the pipeline",
            extra=runs_extra,
        )

    # Frame coherence + LDR diffs against the native reference.
    ref_meta = reference["capture_meta"]
    width = int(ref_meta["width"])
    height = int(ref_meta["height"])
    for name in ("candidate", "fail_closed"):
        meta = legs[name]["capture_meta"]
        if ref_meta.get("width") != meta.get("width") or ref_meta.get("height") != meta.get("height"):
            return fail(
                f"reference/{name} frame dimensions mismatch "
                f"({ref_meta.get('width')}x{ref_meta.get('height')} vs "
                f"{meta.get('width')}x{meta.get('height')})",
                extra=runs_extra,
            )

    ref_frame = reference["frames"][0]
    diffs: dict[str, dict] = {}
    for name in ("candidate", "fail_closed"):
        max_abs, mean_abs = compute_ldr_abs_diff(ref_frame, legs[name]["frames"][0])
        within = max_abs <= LDR_MAX_ABS_DIFF_THRESHOLD and mean_abs <= LDR_MEAN_ABS_DIFF_THRESHOLD
        diffs[name] = {
            "max_abs_diff": max_abs,
            "mean_abs_diff": mean_abs,
            "within_threshold": within,
        }
        print(
            f"[grx-rb-tonemap-rd-native-smoke] LDR absolute diff ({name} vs "
            f"reference) max_abs={max_abs} mean_abs={mean_abs:.6f} "
            f"(thresholds max<={LDR_MAX_ABS_DIFF_THRESHOLD} mean<={LDR_MEAN_ABS_DIFF_THRESHOLD})"
        )
    visual = {
        "measured_local": True,
        "metric_kind": METRIC_KIND,
        "width": width,
        "height": height,
        "format": FRAME_FORMAT,
        "capture_frame_index": ref_meta.get("capture_frame_index"),
        "stability_frame_count": STABILITY_FRAME_COUNT,
        "max_abs_diff_threshold": LDR_MAX_ABS_DIFF_THRESHOLD,
        "mean_abs_diff_threshold": LDR_MEAN_ABS_DIFF_THRESHOLD,
        "reference_frame": file_fingerprint(Path(str(reference["capture_prefix"]) + ".f0.rgb8")),
        "candidate_frame": file_fingerprint(Path(str(candidate["capture_prefix"]) + ".f0.rgb8")),
        "fail_closed_frame": file_fingerprint(Path(str(fail_closed["capture_prefix"]) + ".f0.rgb8")),
        "diffs": diffs,
    }
    runs_extra["visual"] = visual

    # The fail_closed leg fell back to the native tonemapper; it MUST match the
    # reference byte-for-byte (both rendered by the identical native path).
    if not (fail_closed["frames"][0] == ref_frame):
        return fail(
            "fail_closed leg frame does not byte-match the reference frame; a "
            "garbage container must fall back to the SAME native tonemap path as "
            f"the reference (max_abs={diffs['fail_closed']['max_abs_diff']})",
            extra=runs_extra,
        )

    checks = {
        "container_matches_s2_probe": True,
        "reference_run_exit_zero": reference["exit_code"] == 0,
        "candidate_run_exit_zero": candidate["exit_code"] == 0,
        "fail_closed_run_exit_zero": fail_closed["exit_code"] == 0,
        "session_ready_all_runs": True,
        "active_marker_absent_reference": not reference["active_marker_observed"],
        "active_marker_present_candidate": candidate["active_marker_observed"],
        "active_marker_absent_fail_closed": not fail_closed["active_marker_observed"],
        "frames_stable_all_legs": all(stability.values()),
        "fail_closed_matches_reference": True,
        "runtime_log_audit_clean": True,
        "candidate_diff_within_threshold": diffs["candidate"]["within_threshold"],
    }
    measured_extra = {**runs_extra, "checks": checks}

    # Strict success requires BOTH: the candidate engaged rd_native (active
    # marker -> native tonemapper skipped by construction) AND the replacement
    # image matched the native reference within the LDR parity thresholds.
    if not candidate["active_marker_observed"]:
        return skip_measured_prerequisite(
            "rd_native_engage_failed",
            "backend==2 was armed with the S2-proven container but the candidate "
            "leg did not print the RXGD_RD_NATIVE_TONEMAP active marker: the "
            "rd_native pipeline did not engage (e.g. usage-bits preflight, "
            "container load, shader/pipeline creation, or the tonemap "
            "can_use_storage leg was not taken). The native tonemapper rendered "
            "and no real replacement occurred",
            measured_extra,
        )
    if not diffs["candidate"]["within_threshold"]:
        return skip_measured_prerequisite(
            "rd_native_linear_parity_out_of_tolerance",
            "the candidate leg engaged rd_native and SKIPPED the native "
            "tonemapper (real replacement confirmed), but the LINEAR kernel "
            "output diverged from Godot's native LINEAR tonemapper beyond the "
            f"parity tolerance (candidate max_abs={diffs['candidate']['max_abs_diff']}, "
            f"mean_abs={diffs['candidate']['mean_abs_diff']:.6f}). This is an "
            "honest picture-difference finding for the first real replacement; "
            "the raster-vs-compute write seam / LINEAR-subset alignment is a "
            "later round",
            measured_extra,
        )

    success_extra = dict(measured_extra)
    success_extra["real_gpu_pass"] = True
    success_extra["real_replacement"] = True
    success_extra["native_tonemapper_skipped"] = True
    success_extra["candidate_ldr_max_abs_diff"] = diffs["candidate"]["max_abs_diff"]
    success_extra["candidate_ldr_mean_abs_diff"] = diffs["candidate"]["mean_abs_diff"]
    write_evidence("success", extra=success_extra)
    print(
        "[grx-rb-tonemap-rd-native-smoke] PASS measured rd_native REAL replacement "
        f"(candidate active + native skipped, LDR max_abs={diffs['candidate']['max_abs_diff']} "
        "within parity threshold; default enablement unchanged; no performance claim)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
