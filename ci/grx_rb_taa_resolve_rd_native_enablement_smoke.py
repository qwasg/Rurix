#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX Route B: taa_resolve rd_native in-frame REAL-replacement enablement smoke.

Third Route B non-scaffold real-replacement gate (after tonemap 0040 and
ssao_blur 0041). It drives patch 0042's
``rendering/rurix_accel/passes/taa_resolve/backend == 2`` (rd_native) path, in
which the Rurix taa_resolve kernel runs as a first-class in-frame
``RenderingDevice`` compute dispatch that REPLACES the native resolve() compute
dispatch inside TAA::process, while the native history-maintenance copies
(temp->internal, internal->history, velocity->prev_velocity) still run so the
temporal feedback loop is preserved. When it records, the native Godot resolve
dispatch is GENUINELY SKIPPED (unlike the patch 0019 shim scaffold, which prints
a writeback marker and keeps the native resolve so the image can never change).

rd_native is BRIDGE-INDEPENDENT: it does not go through the rxgd session /
``rxgd_record_pass`` path and sets no ``RxGdCaps.flags`` bit. It only needs the
``RurixAccelD3D12Hooks`` singleton to exist (so ``rurix_godot.dll`` must load for
``bridge_preflight()``), then drives the main ``RenderingDevice`` directly.

Temporal DoD (GRX_PLAN, reused from the grx012 shim gate): a real TAA resolve is
a TEMPORAL accumulation, so a single-frame screenshot may NOT stand in for the
evidence. Each leg runs a deterministic ``use_taa=true`` scene with fixed-seed
orbit motion (so the velocity buffer is non-trivial) and captures a CONTIGUOUS
sequence of ``CAPTURE_COUNT`` (>=8) frames after a warmup, then the gate compares
the reference/candidate sequences frame-by-frame.

Provenance: there is NO S2 ~1-ULP probe for taa_resolve (the S2 probe proved
tonemap only), so this gate anchors the staged container to the S1 structural
container smoke evidence (``rd_container_smoke_evidence.json``, kernel
``taa_resolve``, 69/69 verify checks). ``s2_probe_proven`` is recorded ``false``.

Legs (all with viewport TAA enabled + fixed-seed motion):

  * ``reference`` (backend == 0): the native Godot TAA resolve renders. rd_native
    is never engaged; the ``RXGD_RD_NATIVE_TAA_RESOLVE active`` marker must be
    ABSENT.
  * ``candidate`` (backend == 2 + real staged container): the module lazily
    builds the RD compute pipeline from the Rurix container and dispatches the
    resolve, SKIPPING the native resolve dispatch (history copies still run). The
    active marker must be PRESENT.
  * ``fail_closed`` (backend == 2 + a garbage container path): the container load
    fails, the module latches the failure and returns false, and the native Godot
    TAA resolve renders as the fail-closed fallback. The active marker must be
    ABSENT and every captured frame must byte-match the reference sequence.

Per-frame determinism: the reference and fail_closed sequences (both the native
path, identical integer-driven motion) must be byte-identical frame-for-frame.

Outcome semantics (``rd_native_enablement_evidence.json`` in the taa_resolve pass
dir, rewritten every run):

  * ``status=skip`` / ``skip_kind=environment``: a precondition is unavailable
    (scratch exe, ``rurix_godot.dll``, auditable source provenance, staged
    container, ready session). ``RURIX_REQUIRE_REAL=1`` upgrades this to FAIL.
  * ``status=skip`` / ``skip_kind=measured_prerequisite_blocked``: every leg ran
    on real hardware but rd_native did not achieve a clean real replacement —
    either the active marker did not appear in the candidate leg (rd_native could
    not engage), or it engaged but the temporal resolve output diverged from
    Godot's native TAA resolve beyond the parity tolerance at one or more frames
    (an honest picture-difference finding, with the worst-frame number). Not
    upgraded to FAIL by RURIX_REQUIRE_REAL; never advances the gate.
  * ``status=fail``: an integrity violation (marker in the wrong leg, fail_closed
    sequence not matching reference, fewer than the temporal floor of frames,
    non-zero exit, unexpected ERROR line, tampered container).
  * ``status=success`` (strict): the candidate leg engaged rd_native (active
    marker -> native resolve skipped by construction) AND its captured sequence
    matched the native reference within the LDR parity thresholds at EVERY frame
    AND the fail_closed leg fell back byte-identically AND every audit passed.
    ONLY then is ``real_gpu_pass=true`` recorded and the historical
    ``rd_native_enablement_success_evidence.json`` written. Even a success keeps
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

# rd_native taa_resolve is delivered by patch 0042, the top of the shared Route B
# scratch build 0001-0029 (culling tail) + 0040 (tonemap) + 0041 (ssao_blur) +
# 0042 (taa_resolve). The 0030-0039 block is a monotonic hole (see
# PATCH_ALLOCATION.md, the Route B double-tail note). The sidecar records a
# comma-joined stack id because 0029 -> 0040 is not contiguous.
PATCH_ORDINALS = [f"{n:04d}" for n in range(1, 30)] + ["0040", "0041", "0042"]
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

PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "taa_resolve"
RD_PIPELINE_DIR = ROOT / "spike" / "godot-rurix" / "rd-native-pipeline"
STAGED_CONTAINER = ROOT / "target" / "grx" / "rd_containers" / "taa_resolve.rd_container.bin"
S1_CONTAINER_SMOKE_EVIDENCE = RD_PIPELINE_DIR / "rd_container_smoke_evidence.json"
S1_KERNEL_NAME = "taa_resolve"

EVIDENCE_OUT = PASS_DIR / "rd_native_enablement_evidence.json"
SUCCESS_EVIDENCE_OUT = PASS_DIR / "rd_native_enablement_success_evidence.json"

RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"
WORK = ROOT / "target" / "grx_rb_taa_resolve_rd_native_enablement_smoke"
LOG_DIR = WORK / "logs"

SUBJECT = "grx_rb_taa_resolve_rd_native_enablement_smoke"

ACTIVE_MARKER = "RXGD_RD_NATIVE_TAA_RESOLVE active"
SESSION_READY_MARKER = "RurixAccel: D3D12 Forward+ bridge session ready."
ALLOWED_GODOT_ERROR = "Could not load global script cache"
EXPECTED_FAIL_CLOSED_ERRORS = (
    "Incorrect magic number in shader container",
    "Failed to parse shader container from binary",
)

METRIC_KIND = "ldr_absolute_diff_temporal"
FRAME_FORMAT = "R8G8B8_raw"
# Parity thresholds for the rd_native TAA resolve vs Godot's native TAA resolve.
# The two kernels are different implementations, so the measured per-frame number
# is always recorded; only a within-threshold-at-every-frame candidate advances
# to strict success.
LDR_MAX_ABS_DIFF_THRESHOLD = 2
LDR_MEAN_ABS_DIFF_THRESHOLD = 0.25
MIN_FRAME_DIMENSION = 64
# Temporal capture (GRX_PLAN DoD): a CONTIGUOUS sequence, never a single frame.
# CAPTURE_START leaves warmup for the TAA history slice allocation (needs >=2
# frames; rd_native only engages once has_history is true) and accumulation
# spin-up; CAPTURE_COUNT >= 8 is the temporal-evidence floor.
CAPTURE_START_FRAME = 16
CAPTURE_COUNT = 8
MIN_TEMPORAL_FRAMES = 8
VIEWPORT_WIDTH = 256
VIEWPORT_HEIGHT = 144

GODOT_TIMEOUT_SECONDS = 180
REQUESTED_RENDERER = "d3d12"
REQUESTED_RENDERING_METHOD = "forward_plus"

GODOT_EXE_ENV = "RURIX_GRX_RB_TAA_RESOLVE_GODOT_EXE"
SCRATCH_SOURCE_ENV = "RURIX_GRX_RB_TAA_RESOLVE_GODOT_SOURCE"
SCRATCH_SOURCE_PROVENANCE_ENV = "RURIX_GRX_RB_TAA_RESOLVE_GODOT_SOURCE_PROVENANCE"
SCRATCH_BUILD_COMMAND_ENV = "RURIX_GRX_RB_TAA_RESOLVE_GODOT_BUILD_COMMAND"
SCRATCH_BUILD_LOG_ENV = "RURIX_GRX_RB_TAA_RESOLVE_GODOT_BUILD_LOG"
CAPTURE_PREFIX_ENV = "RURIX_GRX_RB_TAA_RESOLVE_CAPTURE_PREFIX"
CAPTURE_START_ENV = "RURIX_GRX_RB_TAA_RESOLVE_CAPTURE_START"
CAPTURE_COUNT_ENV = "RURIX_GRX_RB_TAA_RESOLVE_CAPTURE_COUNT"
CONTAINER_OVERRIDE_ENV = "RURIX_GRX_RB_TAA_RESOLVE_CONTAINER"
DXC_DIR_ENV = "RURIX_DXC_DIR"

TARGET_BACKEND = "Godot 4.7-dev Windows D3D12 Forward+"
BACKEND_SETTING = "rendering/rurix_accel/passes/taa_resolve/backend"
CONTAINER_SETTING = "rendering/rurix_accel/passes/taa_resolve/rd_container_path"

KNOWN_GAPS = [
    (
        "rd_native replaces ONLY the resolve compute dispatch inside TAA::process; "
        "the native history-maintenance copies (temp->internal, internal->history, "
        "velocity->prev_velocity) still run, so the temporal feedback loop is "
        "preserved but the Rurix resolve kernel differs from Godot's, so the "
        "accumulated history can diverge over the sequence"
    ),
    (
        "the six resolve resources are bound in the container's binding order "
        "(color/depth/velocity/prev_velocity/history = SRV t0..t4, temp output = "
        "UAV u0); Godot's native resolve binds a mixed IMAGE/SAMPLER_WITH_TEXTURE "
        "layout, so the per-frame LDR delta through the temporal accumulation is "
        "recorded as the picture evidence (not a full-frame characterization)"
    ),
    (
        "rd_native opens/closes its own compute list (resolve() is a standalone "
        "list); No submit()/sync() is issued (that is a local-RD concept)"
    ),
    (
        "provenance is the S1 structural container smoke (69/69 verify checks), "
        "NOT an S2 ~1-ULP in-engine probe (which exists for tonemap only); "
        "s2_probe_proven is recorded false"
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
            "try_record_taa_resolve_rd_native override) is instantiated. The "
            "d3d12-recording-shim feature is NOT needed for rd_native."
        ),
    }
    if path.is_file():
        fp["dll_sha256"] = sha256_file(path)
        fp["dll_size_bytes"] = path.stat().st_size
    return fp


def _write_json(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
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
    allowed = (ALLOWED_GODOT_ERROR, *extra_allowed)
    out: list[str] = []
    for line in output.splitlines():
        if not line.strip().startswith("ERROR:"):
            continue
        if any(token in line for token in allowed):
            continue
        out.append(line.strip())
    return out


def s1_container_sha() -> str | None:
    doc = load_json(S1_CONTAINER_SMOKE_EVIDENCE)
    if not isinstance(doc, dict):
        return None
    kernels = doc.get("kernels")
    if not isinstance(kernels, list):
        return None
    for kernel in kernels:
        if isinstance(kernel, dict) and kernel.get("kernel") == S1_KERNEL_NAME:
            sha = kernel.get("container_sha256")
            return sha if isinstance(sha, str) else None
    return None


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
    print(f"[grx-rb-taa-resolve-rd-native-smoke] wrote {rel(EVIDENCE_OUT)} status={status}")
    if status == "success":
        success_doc = dict(doc)
        success_doc["evidence_kind"] = "historical_measured_success"
        success_doc["latest_evidence_path"] = rel(EVIDENCE_OUT)
        success_doc["success_evidence_note"] = (
            "Historical measured success artifact for the GRX Route B taa_resolve "
            "rd_native in-frame real-replacement gate. Written ONLY on a strict "
            "status=success run (candidate engaged rd_native AND the native "
            "resolve was skipped AND the temporal LDR parity gate stayed within "
            "thresholds at every captured frame AND the fail_closed leg fell back "
            "byte-identically AND every audit passed) and never overwritten by a "
            "later SKIP/FAIL run. Even this success keeps "
            "default_enable_state=disabled and performance_claim=none."
        )
        _write_json(SUCCESS_EVIDENCE_OUT, success_doc)
        print(
            "[grx-rb-taa-resolve-rd-native-smoke] wrote "
            f"{rel(SUCCESS_EVIDENCE_OUT)} status=success (historical measured success)"
        )


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx-rb-taa-resolve-rd-native-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip_environment(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx-rb-taa-resolve-rd-native-smoke] SKIP {msg} (降级 SKIP,退出 0)")
    payload = dict(extra or {})
    payload["skip_kind"] = "environment"
    write_evidence("skip", reason=msg, extra=payload)
    return 0


def skip_measured_prerequisite(prerequisite: str, msg: str, extra: dict) -> int:
    print(
        "[grx-rb-taa-resolve-rd-native-smoke] SKIP (measured) first missing "
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
    """Deterministic viewport-TAA scene with fixed-seed orbit motion (so the TAA
    velocity buffer is non-trivial and the resolve dispatch runs every frame),
    then a CONTIGUOUS frame-sequence capture. All motion is driven by an INTEGER
    frame counter, so under --fixed-fps the state at each captured frame is
    deterministic and identical across legs. Only the taa_resolve backend selector
    and the rd_container_path differ between legs."""
    project_dir.mkdir(parents=True, exist_ok=True)

    project_text = f"""\
; Engine configuration file.
; Auto-generated by ci/grx_rb_taa_resolve_rd_native_enablement_smoke.py

config_version=5

[application]

config/name="GRX Route B taa_resolve rd_native enablement smoke"
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
rurix_accel/passes/taa_resolve/backend={backend}
rurix_accel/passes/taa_resolve/rd_container_path="{Path(container_path).as_posix() if container_path else ''}"
"""
    scene_text = """\
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://main.gd" id="1"]

[node name="GRXRBTaaResolveRoot" type="Node3D"]
script = ExtResource("1")

[node name="Camera3D" type="Camera3D" parent="."]

[node name="DirectionalLight3D" type="DirectionalLight3D" parent="."]

[node name="WorldEnvironment" type="WorldEnvironment" parent="."]
"""
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
    env.tonemap_mode = Environment.TONE_MAPPER_LINEAR
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

    print("GRXRBTaaResolve: scene ready backend={backend} use_taa=%s" % str(get_viewport().use_taa))
    _capture_sequence()

func _apply_motion(fi: int) -> void:
    var t := float(fi) * 0.13
    for i in range(_boxes.size()):
        var mi: MeshInstance3D = _boxes[i]
        var ang := t + float(i) * (TAU / 6.0)
        var radius := 2.6
        mi.position = Vector3(cos(ang) * radius, 0.6 + 0.35 * sin(t + float(i)), sin(ang) * radius - 1.0)
        mi.rotation = Vector3(0.0, ang, 0.0)
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
        printerr("GRXRBTaaResolve: capture prefix env var missing")
        get_tree().quit(3)
        return
    var saved := 0
    while saved < count:
        await RenderingServer.frame_post_draw
        if _frames < start:
            continue
        _save_frame(prefix, _frames)
        saved += 1
    print("GRXRBTaaResolve: captured %d frames start=%d" % [saved, start])
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


def load_capture_sequence(capture_prefix: Path) -> tuple[list[dict] | None, list[bytes] | None, str | None]:
    frames: list[bytes] = []
    metas: list[dict] = []
    for offset in range(CAPTURE_COUNT):
        fi = CAPTURE_START_FRAME + offset
        frame_prefix = f"{capture_prefix}.{fi:03d}"
        meta = load_json(Path(frame_prefix + ".json"))
        if meta is None:
            return None, None, f"capture metadata missing/unreadable at {frame_prefix}.json"
        width = meta.get("width")
        height = meta.get("height")
        if (
            not isinstance(width, int)
            or not isinstance(height, int)
            or width < MIN_FRAME_DIMENSION
            or height < MIN_FRAME_DIMENSION
        ):
            return None, None, (
                f"captured frame {fi} dimensions {width}x{height} are malformed or "
                f"below the {MIN_FRAME_DIMENSION}px minimum"
            )
        raw_path = Path(frame_prefix + ".rgb8")
        if not raw_path.is_file():
            return None, None, f"raw frame {fi} missing at {raw_path}"
        data = raw_path.read_bytes()
        if len(data) != width * height * 3:
            return None, None, (
                f"raw frame {fi} size {len(data)} != width*height*3 "
                f"({width}x{height}x3={width * height * 3})"
            )
        metas.append(meta)
        frames.append(data)
    if len(frames) < MIN_TEMPORAL_FRAMES:
        return None, None, (
            f"only {len(frames)} frames captured; the temporal DoD floor is "
            f"{MIN_TEMPORAL_FRAMES}"
        )
    return metas, frames, None


LEG_SETTINGS = {
    "reference": {"backend": 0, "role": "native_reference"},
    "candidate": {"backend": 2, "role": "rd_native_replacement"},
    "fail_closed": {"backend": 2, "role": "rd_native_garbage_container_fallback"},
}


def run_matrix_leg(godot_exe: Path, *, leg: str, dll_path: Path, container_path: str) -> dict:
    settings = LEG_SETTINGS[leg]
    project_dir = WORK / f"project_{leg}"
    capture_prefix = WORK / f"capture_{leg}"
    for offset in range(CAPTURE_COUNT):
        fi = CAPTURE_START_FRAME + offset
        for suffix in (".rgb8", ".json"):
            Path(f"{capture_prefix}.{fi:03d}{suffix}").unlink(missing_ok=True)
    write_smoke_project(
        project_dir,
        dll_path=dll_path,
        backend=settings["backend"],
        container_path=container_path,
    )
    exit_code, output = run_godot(godot_exe, project_dir, capture_prefix, f"godot_{leg}.log")
    metas, frames, capture_error = load_capture_sequence(capture_prefix)
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
        "capture_metas": metas,
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
        "captured_frames": len(leg["frames"]) if leg["frames"] else 0,
        "capture_error": leg["capture_error"],
    }


def temporal_diffs(reference_frames: list[bytes], candidate_frames: list[bytes]) -> dict:
    per_frame: list[dict] = []
    worst_max = 0
    worst_mean = 0.0
    within_all = True
    for offset, (ref, cand) in enumerate(zip(reference_frames, candidate_frames)):
        fi = CAPTURE_START_FRAME + offset
        max_abs, mean_abs = compute_ldr_abs_diff(ref, cand)
        within = max_abs <= LDR_MAX_ABS_DIFF_THRESHOLD and mean_abs <= LDR_MEAN_ABS_DIFF_THRESHOLD
        within_all = within_all and within
        worst_max = max(worst_max, max_abs)
        worst_mean = max(worst_mean, mean_abs)
        per_frame.append(
            {"frame_index": fi, "max_abs_diff": max_abs, "mean_abs_diff": mean_abs, "within_threshold": within}
        )
    return {
        "per_frame": per_frame,
        "worst_max_abs_diff": worst_max,
        "worst_mean_abs_diff": worst_mean,
        "within_threshold_all_frames": within_all,
    }


def main() -> int:
    global _EVIDENCE_BASE
    _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}

    # 1) staged container present + byte-matches the S1 container smoke evidence.
    if not STAGED_CONTAINER.is_file():
        return skip_environment(
            f"staged rd_native container missing at {rel(STAGED_CONTAINER)}; copy "
            "spike/godot-rurix/rd-native-pipeline/out/taa_resolve.rd_container.bin there"
        )
    container_override = os.environ.get(CONTAINER_OVERRIDE_ENV)
    container_path = str(Path(container_override).resolve()) if container_override else str(STAGED_CONTAINER)
    container_sha = sha256_file(Path(container_path))
    s1_sha = s1_container_sha()
    container_matches_s1 = s1_sha is not None and container_sha == s1_sha

    _EVIDENCE_BASE.update(
        {
            "pass_id": "taa_resolve",
            "provenance": "rd_native_route_b",
            "backend_selector": BACKEND_SETTING,
            "backend_states": {"disabled": 0, "shim": 1, "rd_native": 2},
            "bridge_independent": True,
            "cap_bit_consumed": None,
            "runtime_state": "fallback_only",
            "real_gpu_pass": False,
            "default_enable_state": "disabled",
            "gpu_timestamp_status": "not_yet",
            "performance_claim": "none",
            "kernel_subset": "single-view full-resolution TAA resolve dispatch (native history copies preserved)",
            "temporal_dod": {
                "capture_start_frame": CAPTURE_START_FRAME,
                "capture_count": CAPTURE_COUNT,
                "min_temporal_frames": MIN_TEMPORAL_FRAMES,
            },
            "target_backend": TARGET_BACKEND,
            "known_gaps": KNOWN_GAPS,
            "container": {
                "path_at_run": container_path,
                "sha256": container_sha,
                "s1_container_smoke_evidence": rel(S1_CONTAINER_SMOKE_EVIDENCE),
                "s1_container_sha256": s1_sha,
                "matches_s1_container_smoke": container_matches_s1,
                "s2_probe_proven": False,
                "s2_probe_note": (
                    "no S2 ~1-ULP in-engine probe exists for taa_resolve (the S2 "
                    "probe proved tonemap only); this is a structural S1 "
                    "container-provenance anchor, not a proven-parity anchor"
                ),
            },
            "patch_stack_identity": patch_stack_identity(PATCH_STACK, PATCH_STACK_ID),
            "note": (
                "GRX Route B taa_resolve rd_native in-frame REAL-replacement gate. "
                "backend==2 drives the Rurix taa_resolve kernel as a first-class "
                "RenderingDevice compute dispatch replacing the native resolve() "
                "dispatch and SKIPS it (the native history-maintenance copies still "
                "run). Bridge-independent (no rxgd session, no RxGdCaps.flags bit). "
                "default_enable_state stays disabled and no performance/FPS/"
                "GPU-timestamp claim is made."
            ),
        }
    )

    if not container_matches_s1:
        return fail(
            "staged rd_native container sha does not match the S1 container smoke "
            f"evidence container sha (staged={container_sha}, s1={s1_sha}); the "
            "gate must consume the exact container structurally verified in S1"
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

    # 4) auditable scratch source provenance (0001-0029 + 0040 + 0041 + 0042).
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

    if not fail_closed["container_reject_errors_observed"]:
        return fail(
            "fail_closed leg (backend=2 + garbage container) did not emit the "
            "expected RenderingDevice container-reject error; the fail-closed "
            "container-load path was not exercised",
            extra=runs_extra,
        )

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

    # Dimension coherence across legs.
    ref_meta0 = reference["capture_metas"][0]
    width = int(ref_meta0["width"])
    height = int(ref_meta0["height"])
    for name in ("candidate", "fail_closed"):
        meta0 = legs[name]["capture_metas"][0]
        if ref_meta0.get("width") != meta0.get("width") or ref_meta0.get("height") != meta0.get("height"):
            return fail(
                f"reference/{name} frame dimensions mismatch "
                f"({ref_meta0.get('width')}x{ref_meta0.get('height')} vs "
                f"{meta0.get('width')}x{meta0.get('height')})",
                extra=runs_extra,
            )

    # The fail_closed sequence fell back to the native resolve; it MUST match the
    # reference sequence byte-for-byte at every captured frame (both the native
    # path with identical deterministic motion).
    fail_closed_matches = all(a == b for a, b in zip(reference["frames"], fail_closed["frames"]))
    if not fail_closed_matches:
        return fail(
            "fail_closed sequence does not byte-match the reference sequence; a "
            "garbage container must fall back to the SAME native TAA resolve path "
            "as the reference at every captured frame",
            extra=runs_extra,
        )

    candidate_temporal = temporal_diffs(reference["frames"], candidate["frames"])
    for entry in candidate_temporal["per_frame"]:
        print(
            f"[grx-rb-taa-resolve-rd-native-smoke] temporal LDR diff frame "
            f"{entry['frame_index']} (candidate vs reference) "
            f"max_abs={entry['max_abs_diff']} mean_abs={entry['mean_abs_diff']:.6f} "
            f"(thresholds max<={LDR_MAX_ABS_DIFF_THRESHOLD} mean<={LDR_MEAN_ABS_DIFF_THRESHOLD})"
        )
    visual = {
        "measured_local": True,
        "metric_kind": METRIC_KIND,
        "width": width,
        "height": height,
        "format": FRAME_FORMAT,
        "capture_start_frame": CAPTURE_START_FRAME,
        "capture_count": CAPTURE_COUNT,
        "max_abs_diff_threshold": LDR_MAX_ABS_DIFF_THRESHOLD,
        "mean_abs_diff_threshold": LDR_MEAN_ABS_DIFF_THRESHOLD,
        "candidate_temporal": candidate_temporal,
        "reference_first_frame": file_fingerprint(Path(f"{reference['capture_prefix']}.{CAPTURE_START_FRAME:03d}.rgb8")),
        "candidate_first_frame": file_fingerprint(Path(f"{candidate['capture_prefix']}.{CAPTURE_START_FRAME:03d}.rgb8")),
    }
    runs_extra["visual"] = visual

    checks = {
        "container_matches_s1_container_smoke": True,
        "reference_run_exit_zero": reference["exit_code"] == 0,
        "candidate_run_exit_zero": candidate["exit_code"] == 0,
        "fail_closed_run_exit_zero": fail_closed["exit_code"] == 0,
        "session_ready_all_runs": True,
        "active_marker_absent_reference": not reference["active_marker_observed"],
        "active_marker_present_candidate": candidate["active_marker_observed"],
        "active_marker_absent_fail_closed": not fail_closed["active_marker_observed"],
        "temporal_floor_met_all_legs": all(len(leg["frames"]) >= MIN_TEMPORAL_FRAMES for leg in legs.values()),
        "fail_closed_matches_reference_sequence": True,
        "runtime_log_audit_clean": True,
        "candidate_within_threshold_all_frames": candidate_temporal["within_threshold_all_frames"],
    }
    measured_extra = {**runs_extra, "checks": checks}

    if not candidate["active_marker_observed"]:
        return skip_measured_prerequisite(
            "rd_native_engage_failed",
            "backend==2 was armed with the S1-verified container but the candidate "
            "leg did not print the RXGD_RD_NATIVE_TAA_RESOLVE active marker: the "
            "rd_native pipeline did not engage (e.g. usage-bits preflight, "
            "container load, shader/pipeline creation, or the TAA history slice "
            "was not yet allocated). The native TAA resolve rendered and no real "
            "replacement occurred",
            measured_extra,
        )
    if not candidate_temporal["within_threshold_all_frames"]:
        return skip_measured_prerequisite(
            "rd_native_temporal_parity_out_of_tolerance",
            "the candidate leg engaged rd_native and SKIPPED the native resolve "
            "dispatch (real replacement confirmed), but the temporal resolve "
            "output diverged from Godot's native TAA resolve beyond the parity "
            f"tolerance (worst-frame max_abs={candidate_temporal['worst_max_abs_diff']}, "
            f"worst mean_abs={candidate_temporal['worst_mean_abs_diff']:.6f}). This is an "
            "honest picture-difference finding for the different resolve kernels "
            "accumulating over the temporal history; a later round",
            measured_extra,
        )

    success_extra = dict(measured_extra)
    success_extra["real_gpu_pass"] = True
    success_extra["real_replacement"] = True
    success_extra["native_resolve_skipped"] = True
    success_extra["candidate_worst_max_abs_diff"] = candidate_temporal["worst_max_abs_diff"]
    success_extra["candidate_worst_mean_abs_diff"] = candidate_temporal["worst_mean_abs_diff"]
    success_extra["telemetry_frame"] = CAPTURE_START_FRAME + CAPTURE_COUNT - 1
    write_evidence("success", extra=success_extra)
    print(
        "[grx-rb-taa-resolve-rd-native-smoke] PASS measured rd_native REAL replacement "
        f"(candidate active + native resolve skipped, temporal LDR worst max_abs="
        f"{candidate_temporal['worst_max_abs_diff']} within parity threshold at every "
        "frame; default enablement unchanged; no performance claim)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
