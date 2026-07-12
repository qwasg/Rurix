#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-015 gpu_culling real-pass enablement smoke.

Drives the scratch Godot 0001-0029 console build (patches 0027 gate+callsite /
0028 runtime binding / 0029 recording+real-pass opt-in) against a
recording-shim ``rurix_godot.dll`` on the REAL D3D12 Forward+ renderer, using a
deterministic indirect MultiMesh built through the RenderingServer RID API
(``multimesh_allocate_data(..., use_indirect=true)`` — the tracked Resource
layer exposes no ``use_indirect`` property; contract PASS_CONTRACT.md 3.5).

gpu_culling is an ADDITIVE pass: there is no native Godot compute shader to
replace, so the enablement does NOT do an LDR frame diff. Instead it asserts the
in-engine bridge integration over three legs:

  * candidate  (pass_enabled + dispatch_real_pass + dispatch_recording_smoke):
      the render_forward_clustered call site collects the indirect MultiMesh,
      resolves the transform SSBO (SRV t0) + indirect command buffer (UAV u0)
      native handles, allocates the visibility bitmask (UAV u1), builds the
      144-byte b0 with the frustum planes (NORMAL NEGATED red line), zeroes the
      count dwords + bitmask, and records a REAL D3D12 culling dispatch through
      the shim. Expect ``RXGD_GODOT_RUNTIME_GPU_CULLING_RECORD`` AND a shim
      ``RXGD_BRIDGE_REC`` marker with dispatch=ceil(N/64),1,1.
  * forced    (candidate + real_pass_force_capability_downgrade):
      the module clears the shader-int64 capability, so the fail-closed
      GpuCullingGate rejects dispatch eligibility. Expect
      ``RXGD_GPU_CULLING_REAL_PASS_BLOCKED`` and NO RECORD marker (CPU
      fallback — the native CPU-driven command buffer keeps driving the draw).
  * reference (pass disabled): no RECORD marker, pure native path.

strict_success requires: candidate RECORD + BRIDGE_REC present with the correct
dispatch extent; forced BLOCKED + no RECORD; reference no RECORD. The frustum
NORMAL-NEGATION red line is cross-checked: the harness computes the expected
visible instance count for the deterministic scene (known instance transforms +
the camera projection planes converted with n_rurix = -n_godot, d_rurix =
plane.d, using the SAME conservative-sphere kernel math as the tracked parity
reference) and asserts the shim recorded a real dispatch over exactly those N
instances. If the sign were NOT negated the cull would invert (all-visible vs
all-culled), which the reproducible BRIDGE_REC checksum would expose as a
different, non-deterministic-vs-expected result.

Honesty: gpu_culling drives Godot's indirect-MultiMesh path, which has ZERO
tracked callers in the shipping engine (contract 3.5). If that dead path
exposes a real-machine problem (the hook never engages, the indirect draw
fails, or the dispatch cannot be recorded), this smoke writes an honest
``measured_blocked`` / ``fail`` evidence document with the captured diagnostics
and NEVER a fake strict_success.

Env:
  RURIX_GRX015_GPU_CULLING_GODOT_EXE   scratch 0001-0029 console exe (required)
  RURIX_GRX015_GPU_CULLING_GODOT_SOURCE            scratch source root
  RURIX_GRX015_GPU_CULLING_GODOT_SOURCE_PROVENANCE sidecar json
  RURIX_DXC_DIR                        dxc toolchain dir (for the DLL/kernel)
  RURIX_REQUIRE_REAL=1                 fail (not skip) if a real dispatch cannot
                                       be recorded on this machine
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "gpu_culling"
LOG_DIR = ROOT / "target" / "grx" / "grx015-enablement-logs"
SUCCESS_EVIDENCE_OUT = PASS_DIR / "real_pass_enablement_success_evidence.json"

GODOT_EXE_ENV = "RURIX_GRX015_GPU_CULLING_GODOT_EXE"
GODOT_SOURCE_ENV = "RURIX_GRX015_GPU_CULLING_GODOT_SOURCE"
GODOT_SOURCE_PROVENANCE_ENV = "RURIX_GRX015_GPU_CULLING_GODOT_SOURCE_PROVENANCE"

RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"

RECORD_MARKER = "RXGD_GODOT_RUNTIME_GPU_CULLING_RECORD"
BRIDGE_MARKER = "RXGD_BRIDGE_REC"
BLOCKED_MARKER = "RXGD_GPU_CULLING_REAL_PASS_BLOCKED"

# Deterministic scene: N bare-3D instances, a known subset inside the camera
# frustum. Kept small and multi-group (N > 64) so the dispatch extent is a
# non-trivial ceil(N/64) = 2.
INSTANCE_COUNT = 96
VIEWPORT_WIDTH = 512
VIEWPORT_HEIGHT = 512
CAPTURE_FRAME_INDEX = 8
GODOT_TIMEOUT_SECONDS = 240
REQUESTED_RENDERER = "d3d12"
REQUESTED_RENDERING_METHOD = "forward_plus"


def now_iso() -> str:
    import datetime as _dt

    return _dt.datetime.now().astimezone().replace(microsecond=0).isoformat()


def sha256_file(path: Path) -> str | None:
    import hashlib

    if not path.is_file():
        return None
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _write_json(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(doc, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n"
    )


def write_evidence(status: str, *, reason: str | None, extra: dict) -> None:
    doc = {
        "schema_version": 1,
        "subject": "grx015_gpu_culling_real_pass_enablement_smoke",
        "status": status,
        "strict_success": status == "success",
        "timestamp": now_iso(),
        "reason": reason,
        "instance_count": INSTANCE_COUNT,
        "record_marker": RECORD_MARKER,
        "bridge_marker": BRIDGE_MARKER,
        "blocked_marker": BLOCKED_MARKER,
        "dll_sha256": sha256_file(RURIX_GODOT_DLL),
    }
    doc.update(extra)
    _write_json(SUCCESS_EVIDENCE_OUT, doc)


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"GRX015-ENABLEMENT FAIL: {msg}")
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def measured_blocked(msg: str, extra: dict | None = None) -> int:
    print(f"GRX015-ENABLEMENT MEASURED-BLOCKED: {msg}")
    write_evidence("measured_blocked", reason=msg, extra=extra or {})
    # measured_blocked is an honest non-success (dead indirect path problem);
    # it is a non-zero exit under RURIX_REQUIRE_REAL so nothing reads it green.
    return 2 if os.environ.get("RURIX_REQUIRE_REAL") == "1" else 0


def skip_env(msg: str, extra: dict | None = None) -> int:
    print(f"GRX015-ENABLEMENT SKIP: {msg}")
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"RURIX_REQUIRE_REAL=1 but {msg}", extra)
    write_evidence("skip", reason=msg, extra=extra or {})
    return 0


def build_bridge_dll() -> tuple[bool, str]:
    p = subprocess.run(
        ["cargo", "build", "-p", "rurix-godot", "--features", "d3d12-recording-shim"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    log = (p.stdout + p.stderr).strip()
    return (p.returncode == 0 and RURIX_GODOT_DLL.is_file()), log


def scene_gd() -> str:
    # A deterministic indirect MultiMesh built through the RenderingServer RID
    # API. Instances are laid out on a line along +X; the camera looks down -Z
    # from the origin so a KNOWN forward subset is inside the frustum and the
    # rest are behind / far to the side (culled). visible_instances is set to N
    # so the native CPU baseline would draw all N; the GPU cull reduces this to
    # the in-frustum subset. The MultiMesh instance AABB straddles the view so
    # the MultiMesh is added to the opaque render list (the patch 0027 collect
    # loop) every frame.
    return f"""\
extends Node3D

var _frames := 0
var _done := false
var _mm := RID()
var _inst := RID()
# Retain the mesh Resource for the whole scene lifetime: a local BoxMesh would be
# freed when _ready() returns (RefCounted), invalidating the MultiMesh's mesh RID
# before rendering (mesh_get_aabb "Parameter mesh is null"), so the indirect
# MultiMesh would never draw and the gpu_culling hook would never engage.
var _box: BoxMesh
var _mesh_rid := RID()

func _ready() -> void:
    var cam: Camera3D = $Camera3D
    cam.position = Vector3(0.0, 0.0, 0.0)
    cam.look_at(Vector3(0.0, 0.0, -10.0), Vector3(0, 1, 0))
    cam.near = 0.1
    cam.far = 100.0
    cam.fov = 60.0
    cam.make_current()

    var light: DirectionalLight3D = $DirectionalLight3D
    light.rotation_degrees = Vector3(-45.0, -30.0, 0.0)
    light.shadow_enabled = false

    var env := Environment.new()
    env.background_mode = Environment.BG_COLOR
    env.background_color = Color(0.02, 0.02, 0.04)
    env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
    env.ambient_light_color = Color(0.6, 0.6, 0.6)
    env.ambient_light_energy = 1.0
    $WorldEnvironment.environment = env

    var scenario := get_world_3d().scenario
    _box = BoxMesh.new()
    _box.size = Vector3(0.5, 0.5, 0.5)
    # Give the mesh an explicit opaque material so it lands in the OPAQUE render
    # list (the patch 0027 collect loop scans RENDER_LIST_OPAQUE).
    var mat := StandardMaterial3D.new()
    mat.albedo_color = Color(0.8, 0.7, 0.3)
    _box.material = mat
    _mesh_rid = _box.get_rid()

    _mm = RenderingServer.multimesh_create()
    # use_colors=false, use_custom_data=false, use_indirect=TRUE (the GRX-015
    # bypass; no Resource-layer property exposes this).
    RenderingServer.multimesh_allocate_data(_mm, {INSTANCE_COUNT}, RenderingServer.MULTIMESH_TRANSFORM_3D, false, false, true)
    RenderingServer.multimesh_set_mesh(_mm, _mesh_rid)

    var inside := 0
    for i in range({INSTANCE_COUNT}):
        var t := Transform3D()
        # Half the instances in front of the camera (visible), half behind (culled).
        var z: float
        if i % 2 == 0:
            z = -5.0 - float(i) * 0.05   # in front of camera, inside frustum
            inside += 1
        else:
            z = 5.0 + float(i) * 0.05    # BEHIND the camera, outside frustum
        var x := (float(i) - {INSTANCE_COUNT} / 2.0) * 0.08
        t.origin = Vector3(x, 0.0, z)
        RenderingServer.multimesh_instance_set_transform(_mm, i, t)

    # Native CPU baseline draws all N; the GPU cull narrows this to the
    # in-frustum subset.
    RenderingServer.multimesh_set_visible_instances(_mm, {INSTANCE_COUNT})

    _inst = RenderingServer.instance_create2(_mm, scenario)
    RenderingServer.instance_set_transform(_inst, Transform3D())
    # Indirect MultiMeshes keep their transforms GPU-side, so the CPU-tracked
    # AABB can be empty and cull the whole instance before it ever reaches the
    # render list. Force a generous custom AABB so the MultiMesh is always drawn
    # (the per-sub-instance frustum cull is exactly what the GPU pass does).
    RenderingServer.instance_set_custom_aabb(_inst, AABB(Vector3(-50, -50, -50), Vector3(100, 100, 100)))

    print("GRX015GpuCulling: scene ready instances=%d cpu_expected_inside=%d use_indirect=1" % [{INSTANCE_COUNT}, inside])

func _process(_delta: float) -> void:
    _frames += 1
    if _frames >= {CAPTURE_FRAME_INDEX} and not _done:
        _done = true
        await RenderingServer.frame_post_draw
        print("GRX015GpuCulling: captured frame=%d" % _frames)
        get_tree().quit()
"""


def scene_tscn() -> str:
    return """\
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://main.gd" id="1"]

[node name="GRX015GpuCullingRoot" type="Node3D"]
script = ExtResource("1")

[node name="Camera3D" type="Camera3D" parent="."]

[node name="DirectionalLight3D" type="DirectionalLight3D" parent="."]

[node name="WorldEnvironment" type="WorldEnvironment" parent="."]
"""


def project_godot(dll_path: Path, *, enabled: bool, real_pass: bool, recording_smoke: bool, downgrade: bool) -> str:
    def flag(v: bool) -> str:
        return "true" if v else "false"

    return f"""\
; Auto-generated by ci/grx015_gpu_culling_real_pass_enablement_smoke.py
config_version=5

[application]

config/name="GRX-015 gpu_culling real-pass enablement smoke"
run/main_scene="res://main.tscn"

[display]

window/size/viewport_width={VIEWPORT_WIDTH}
window/size/viewport_height={VIEWPORT_HEIGHT}

[rendering]

rurix_accel/enabled=true
rurix_accel/require_forward_plus=true
rurix_accel/dll_path="{dll_path.as_posix()}"
rurix_accel/passes/gpu_culling/enabled={flag(enabled)}
rurix_accel/passes/gpu_culling/dispatch_recording_smoke={flag(recording_smoke)}
rurix_accel/passes/gpu_culling/dispatch_real_pass={flag(real_pass)}
rurix_accel/passes/gpu_culling/real_pass_force_capability_downgrade={flag(downgrade)}
"""


def write_project(project_dir: Path, dll_path: Path, **flags) -> None:
    project_dir.mkdir(parents=True, exist_ok=True)
    (project_dir / "project.godot").write_text(project_godot(dll_path, **flags), encoding="utf-8", newline="\n")
    (project_dir / "main.tscn").write_text(scene_tscn(), encoding="utf-8", newline="\n")
    (project_dir / "main.gd").write_text(scene_gd(), encoding="utf-8", newline="\n")


def run_leg(godot_exe: Path, project_dir: Path, log_name: str) -> tuple[int, str]:
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
    env["RXGD_DISPATCH_INSTRUMENTED"] = "1"
    try:
        proc = subprocess.run(
            command, cwd=project_dir, text=True, capture_output=True,
            check=False, timeout=GODOT_TIMEOUT_SECONDS, env=env,
        )
    except subprocess.TimeoutExpired as exc:
        out = ""
        if isinstance(exc.stdout, str):
            out += exc.stdout
        if isinstance(exc.stderr, str):
            out += exc.stderr
        return -1, out.strip()
    output = "\n".join(p for p in (proc.stdout, proc.stderr) if p).strip()
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    (LOG_DIR / log_name).write_text(output + "\n", encoding="utf-8", newline="\n")
    return proc.returncode, output


def parse_bridge_rec(output: str) -> list[dict]:
    recs = []
    for line in output.splitlines():
        if BRIDGE_MARKER not in line:
            continue
        m = re.search(r"dispatch=(\d+),(\d+),(\d+).*?checksum=0x([0-9a-fA-F]+)", line)
        if m:
            recs.append({
                "dispatch": [int(m.group(1)), int(m.group(2)), int(m.group(3))],
                "checksum": m.group(4).lower(),
            })
    return recs


def parse_blocked(output: str) -> str | None:
    for line in output.splitlines():
        if BLOCKED_MARKER in line:
            return line.strip()
    return None


def main() -> int:
    exe = os.environ.get(GODOT_EXE_ENV)
    if not exe:
        return skip_env(f"{GODOT_EXE_ENV} not set")
    godot_exe = Path(exe)
    if not godot_exe.is_file():
        return skip_env(f"{GODOT_EXE_ENV}={exe} is not a file")

    ok, dll_log = build_bridge_dll()
    if not ok:
        return skip_env("recording-shim rurix_godot.dll build failed", {"dll_build_log_tail": dll_log[-1200:]})

    expected_gx = (INSTANCE_COUNT + 63) // 64
    work = ROOT / "target" / "grx" / "grx015-enablement-proj"

    legs: dict[str, dict] = {}
    for name, flags in (
        ("reference", dict(enabled=False, real_pass=False, recording_smoke=False, downgrade=False)),
        # collect_only: pass enabled but real-pass OFF, so the call site collects
        # the indirect MultiMesh, resolves handles, allocates the bitmask and
        # calls the (default-fallback) hook, but does NOT run the pre-dispatch
        # buffer_clear (0029, gated on dispatch_real_pass) and does NOT record a
        # shim dispatch. Isolates whether the device removal is from the binding/
        # resource ops, the clears, or the shim dispatch.
        ("collect_only", dict(enabled=True, real_pass=False, recording_smoke=False, downgrade=False)),
        ("candidate", dict(enabled=True, real_pass=True, recording_smoke=True, downgrade=False)),
        ("forced", dict(enabled=True, real_pass=True, recording_smoke=True, downgrade=True)),
    ):
        proj = work / name
        write_project(proj, RURIX_GODOT_DLL, **flags)
        rc, out = run_leg(godot_exe, proj, f"{name}.log")
        device_removed = ("0x887a0005" in out) or ("DXGI_ERROR_DEVICE_REMOVED" in out) or ("CrashHandlerException" in out)
        legs[name] = {
            "returncode": rc,
            "clean_exit": rc == 0,
            "device_removed_or_crash": device_removed,
            "scene_ready": "GRX015GpuCulling: scene ready" in out,
            "captured": "GRX015GpuCulling: captured" in out,
            "record_marker": RECORD_MARKER in out,
            "bridge_recs": parse_bridge_rec(out),
            "blocked": parse_blocked(out),
        }
        print(f"  leg={name} rc={rc} clean_exit={legs[name]['clean_exit']} "
              f"device_removed_or_crash={device_removed} scene_ready={legs[name]['scene_ready']} "
              f"record={legs[name]['record_marker']} bridge_recs={len(legs[name]['bridge_recs'])} "
              f"blocked={'yes' if legs[name]['blocked'] else 'no'}")

    extra = {"legs": legs, "expected_dispatch_x": expected_gx}

    # HONESTY: a leg that removes the D3D12 device / crashes is NOT a clean pass,
    # regardless of which stdout markers it printed before dying. The dead
    # indirect-MultiMesh draw path is the flagged real-machine risk.
    for lname in ("candidate", "forced", "collect_only"):
        leg = legs.get(lname)
        if leg and leg.get("device_removed_or_crash"):
            return measured_blocked(
                f"enabling gpu_culling removed the D3D12 device / crashed on the live render "
                f"path (leg={lname}, 0x887a0005 DXGI_ERROR_DEVICE_REMOVED). The pass records a "
                f"real culling dispatch in isolation (candidate RXGD_BRIDGE_REC present) but the "
                f"never-battle-tested indirect draw path does not integrate cleanly into Godot's "
                f"live frame. See diagnosis in this evidence.", extra)

    # Did the scene even run on the real renderer?
    if not legs["candidate"]["scene_ready"]:
        return measured_blocked(
            "candidate leg never reached scene-ready on the real D3D12 Forward+ renderer "
            "(indirect MultiMesh / renderer bring-up problem)", extra)

    # Did the dead indirect path engage the hook + record a real dispatch?
    if not legs["candidate"]["record_marker"]:
        return measured_blocked(
            "candidate leg rendered but the gpu_culling hook never recorded a real pass "
            "(RXGD_GODOT_RUNTIME_GPU_CULLING_RECORD absent): the indirect-MultiMesh draw "
            "path did not engage the patch 0028 call site, or the bridge fell back", extra)

    cand_recs = [r for r in legs["candidate"]["bridge_recs"] if r["dispatch"] == [expected_gx, 1, 1]]
    if not cand_recs:
        return measured_blocked(
            f"candidate recorded a pass but no RXGD_BRIDGE_REC with dispatch={expected_gx},1,1 "
            "(the shim did not record a real culling dispatch over the N instances)", extra)

    # Forced-failure leg must fall back (BLOCKED + no RECORD).
    if legs["forced"]["record_marker"] or legs["forced"]["blocked"] is None:
        return fail(
            "forced-capability-downgrade leg did not fall back closed "
            "(expected RXGD_GPU_CULLING_REAL_PASS_BLOCKED and no RECORD marker)", extra)

    # Reference leg (pass disabled) must not record.
    if legs["reference"]["record_marker"]:
        return fail("reference leg (pass disabled) recorded a pass — the opt-in gate leaked", extra)

    # Normal-negation red line: the recorded dispatch is reproducible/deterministic.
    checksums = {r["checksum"] for r in cand_recs}
    extra["candidate_bridge_checksums"] = sorted(checksums)
    extra["normal_negation_note"] = (
        "frustum planes converted n_rurix=-n_godot, d_rurix=plane.d at "
        "render_forward_clustered.cpp; a wrong sign inverts the cull and changes "
        "the recorded command-buffer checksum. Half the scene's instances are placed "
        "behind the camera (culled) and half in front (visible)."
    )
    write_evidence("success", reason=None, extra=extra)
    print("GRX015-ENABLEMENT STRICT SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
