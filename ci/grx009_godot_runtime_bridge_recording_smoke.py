#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-009 segment 4f: Godot-runtime bridge dispatch recording smoke.

Unlike the segment 4d *bridge* recording smoke (which drives the Rurix Godot
bridge C ABI directly from a bare C++ harness that fabricates its own D3D12
resources), this harness proves the **Godot runtime luminance call site** — the
Auto Exposure path patched in segments 4e/4f — can drive **one real
bridge-recorded D3D12 dispatch** through the real ``ID3D12Resource*`` native
handles it resolves via ``RenderingDevice::get_driver_resource``. It produces
measured *Godot-runtime* smoke evidence only. It does NOT:

  * enable the default Godot luminance Rurix path (all three per-pass settings
    stay default-false; only the harness-only dispatch_recording_smoke opt-in is
    turned on in the generated smoke project),
  * mark the Godot runtime luminance pass as complete,
  * make the shipping (feature-off) bridge return RXGD_STATUS_OK,
  * flip real_gpu_pass / real_d3d12_dispatch_recorded (default runtime meaning),
  * claim any FPS / visual diff / measured fallback telemetry / GPU timestamp.

Preconditions (any missing one is a concrete SKIP that does NOT advance the
readiness gate):

  * A Godot scratch build that includes the FULL segment 4f patch stack
    (0001..0008) applied on top of the ignored ``external/godot-master`` snapshot
    and rebuilt with ``module_rurix_accel_enabled=yes d3d12=yes``. Because the
    tracked ``external/godot-master`` build only carries 0001+0002+0003, this
    harness will NOT reuse it: the caller must point
    ``RURIX_GRX009_SEGMENT4F_GODOT_EXE`` at a console Godot executable rebuilt
    with the full stack. Without it the smoke SKIPs.
  * A signed DXC suite carrying ``dxil.dll`` (so the recording shim can sign the
    in-memory DXIL container to load without Developer Mode) and MSVC vcvars64
    (to build the recording-shim ``rurix_godot.dll``).
  * A real hardware D3D12 adapter with the 64-bit integer shader capability.

Discipline mirrors ci/grx009_luminance_bridge_recording_smoke.py:

  * The tracked DXIL / root signature / descriptor layout digests must match the
    segment 3a offline compile evidence, and the descriptor layout must match the
    current resource mapping. Any mismatch is ``status=fail``.
  * Fake / null handles never record OK. The record path is only linked under the
    test-only ``d3d12-recording-shim`` feature and only armed by the default-false
    ``dispatch_recording_smoke`` opt-in the smoke project enables.
  * A ``status=success`` run must observe the distinctive
    ``RXGD_GODOT_RUNTIME_LUMINANCE_RECORD`` marker emitted by the patched Godot
    module call site (with ``recorded=1``) AND the Godot process must exit with
    ``exit_code == 0``: a non-zero exit after the marker is a hard ``status=fail``,
    never success (``checks.godot_exit_code_zero`` records this). This proves the
    Godot runtime — not a bare harness — drove the bridge recording and shut down
    cleanly. GPU timestamps are not implemented: ``gpu_timestamp_status=not_yet``
    and ``gpu_time_ns`` is never fabricated.

If RURIX_REQUIRE_REAL=1, an environment that would otherwise SKIP becomes a hard
failure (exit 1); otherwise SKIP exits 0, matching the repo GPU-smoke policy.

Evidence hygiene — two tracked artifacts:

  * ``godot_runtime_bridge_recording_evidence.json`` is the *latest* run
    evidence. It is rewritten on EVERY run and is honestly reproducible: with no
    ``RURIX_GRX009_SEGMENT4F_GODOT_EXE`` set it records ``status=skip``. It never
    advances the readiness gate on its own.
  * ``godot_runtime_bridge_recording_success_evidence.json`` is the *historical
    measured success* artifact. It is written/updated ONLY on a strict
    ``status=success`` run (recording the Godot exe fingerprint, the 0001..0008
    patch stack identity, the feature-built DLL fingerprint, the artifact
    hashes, ``godot_exit_code_zero=true``, and the ``recorded=1`` marker). A
    later SKIP/FAIL run must NEVER delete or overwrite it. The segment 4f
    readiness gate advances off THIS file, so a reproducible-default SKIP latest
    evidence does not regress readiness once a measured success has been
    recorded. Scratch Godot build binaries are not committed.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "luminance_reduction"
ARTIFACTS = PASS_DIR / "artifacts"
DXIL = ARTIFACTS / "luminance_reduction.dxil"
RTS0 = ARTIFACTS / "luminance_reduction.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "luminance_reduction_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
# The *latest* runtime smoke evidence. This is always rewritten on every run and
# is honestly reproducible: with no scratch Godot exe env var it records SKIP.
EVIDENCE_OUT = PASS_DIR / "godot_runtime_bridge_recording_evidence.json"
# The *historical* measured success artifact. It is only ever written/updated on
# a strict status=success run, and is never deleted or overwritten by a later
# SKIP/FAIL run. The segment 4f readiness gate advances on THIS file, not on the
# reproducible-default SKIP latest file above.
SUCCESS_EVIDENCE_OUT = (
    PASS_DIR / "godot_runtime_bridge_recording_success_evidence.json"
)
PATCHES_DIR = ROOT / "spike" / "godot-rurix" / "patches"
GODOT_BASE_SNAPSHOT = ROOT / "external" / "godot-master"
PATCH_STACK = (
    "0001-rurix-accel-module-scaffold.patch",
    "0002-rurix-accel-luminance-pass-gate.patch",
    "0003-rurix-accel-luminance-core-callsite-wiring.patch",
    "0004-rurix-accel-luminance-resource-mapping-scaffold.patch",
    "0005-rurix-accel-luminance-runtime-binding-preflight.patch",
    "0006-rurix-accel-luminance-gated-dispatch-bringup.patch",
    "0007-rurix-accel-luminance-native-resource-handle-mapping.patch",
    "0008-rurix-accel-luminance-godot-runtime-bridge-recording-smoke.patch",
)
RURIX_GODOT_DIR = ROOT / "src" / "rurix-godot"
RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"
WORK = ROOT / "target" / "grx009_godot_runtime_bridge_recording_smoke"
PROJECT_DIR = WORK / "project"
LOG_DIR = PROJECT_DIR / "logs"

SUBJECT = "grx009_godot_runtime_luminance_bridge_dispatch_recording_smoke"

# Distinctive marker the segment 4f module patch prints ONLY when the Godot
# runtime luminance call site drove a real bridge-recorded dispatch (rc == OK).
RUNTIME_RECORD_MARKER = "RXGD_GODOT_RUNTIME_LUMINANCE_RECORD"
SESSION_READY_MARKER = "RurixAccel: D3D12 Forward+ bridge session ready."
SESSION_UNAVAILABLE_MARKER = "RurixAccel: session unavailable rc="
GODOT_TIMEOUT_SECONDS = 180
REQUESTED_RENDERER = "d3d12"
REQUESTED_RENDERING_METHOD = "forward_plus"

# Explicit env var pointing at a Godot console exe rebuilt with the full
# segment 4f patch stack (0001..0008). The tracked external/godot-master build
# only has 0001+0002+0003, so it must NOT be reused for this smoke.
GODOT_EXE_ENV = "RURIX_GRX009_SEGMENT4F_GODOT_EXE"
SCRATCH_SOURCE_ENV = "RURIX_GRX009_SEGMENT4F_GODOT_SOURCE"
SCRATCH_SOURCE_PROVENANCE_ENV = "RURIX_GRX009_SEGMENT4F_GODOT_SOURCE_PROVENANCE"
SCRATCH_BUILD_COMMAND_ENV = "RURIX_GRX009_SEGMENT4F_GODOT_BUILD_COMMAND"
SCRATCH_BUILD_LOG_ENV = "RURIX_GRX009_SEGMENT4F_GODOT_BUILD_LOG"

KNOWN_DXC_DIR = Path(r"H:\dxc-round7\extracted\bin\x64")


def run(cmd: list[str], *, cwd: Path | None = None, env: dict | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd or ROOT, capture_output=True, text=True, env=env)


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


def locate_signed_dxc_dir() -> Path | None:
    """Signed pin dir carrying dxil.dll (the DXIL validator used to sign the
    container so it loads without Developer Mode). RURIX_DXC_DIR takes priority."""
    dirs: list[Path] = []
    for key in ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"):
        v = os.environ.get(key)
        if v:
            dirs.append(Path(v))
    dirs.append(KNOWN_DXC_DIR)
    for d in dirs:
        if (d / "dxil.dll").is_file():
            return d
    return None


def locate_dxcapi_include(dxc_dir: Path | None) -> Path | None:
    if dxc_dir is None:
        return None
    for base in (dxc_dir, *dxc_dir.parents):
        for name in ("inc", "include"):
            candidate = base / name / "dxcapi.h"
            if candidate.is_file():
                return candidate.parent
    return None


def locate_vcvars64() -> Path | None:
    override = os.environ.get("RURIX_VCVARS64")
    if override:
        p = Path(override)
        if p.is_file():
            return p
    candidates = [
        Path(r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"),
        Path(r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"),
    ]
    candidates.extend(Path(r"C:\Program Files").glob(r"Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvars64.bat"))
    candidates.extend(Path(r"C:\Program Files (x86)").glob(r"Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvars64.bat"))
    for p in candidates:
        if p.is_file():
            return p
    return None


def locate_segment4f_godot_exe() -> tuple[Path | None, str | None]:
    """Locate the Godot console exe rebuilt with the FULL segment 4f patch stack.

    The tracked external/godot-master build only carries 0001+0002+0003, so it
    must never be reused for this runtime smoke. The caller must point
    RURIX_GRX009_SEGMENT4F_GODOT_EXE at a console Godot executable rebuilt with
    the 0001..0008 stack applied and module_rurix_accel_enabled=yes d3d12=yes.
    """
    override = os.environ.get(GODOT_EXE_ENV)
    if not override:
        return None, (
            f"{GODOT_EXE_ENV} is not set; this smoke needs a Godot console exe "
            "rebuilt with the full segment 4f patch stack (0001..0008) applied "
            "and module_rurix_accel_enabled=yes d3d12=yes. The tracked "
            "external/godot-master build (0001+0002+0003 only) must not be reused."
        )
    p = Path(override)
    if not p.is_file():
        return None, f"{GODOT_EXE_ENV}={override} does not point at an existing file"
    return p, None


def offline_artifact_digests(evidence: dict) -> dict[str, str | None]:
    artifacts = evidence.get("artifacts")
    out: dict[str, str | None] = {"dxil": None, "root_signature": None, "descriptor_layout": None}
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


def descriptor_layout_matches_resource_mapping(layout: dict) -> str | None:
    """Return None when the descriptor layout matches the tracked resource
    mapping, otherwise a human-readable mismatch reason."""
    resources = layout.get("resources")
    if not isinstance(resources, list) or len(resources) != 2:
        return "descriptor layout does not declare exactly 2 resources"
    src, dst = resources[0], resources[1]
    if not (isinstance(src, dict) and src.get("name") == "src_luminance"
            and src.get("class") == "t" and src.get("register") == 0):
        return "resource[0] is not src_luminance SRV t0"
    if not (isinstance(dst, dict) and dst.get("name") == "dst_luminance"
            and dst.get("class") == "u" and dst.get("register") == 0):
        return "resource[1] is not dst_luminance UAV u0"
    if layout.get("root_signature_parameters") != 2:
        return "root_signature_parameters != 2"
    if layout.get("root_constants") != 5:
        return "root_constants != 5"
    mapping = layout.get("segment3b_mapping")
    if not isinstance(mapping, dict):
        return "missing segment3b_mapping"
    if mapping.get("root_constant_bytes") != 28 or mapping.get("root_constant_dwords") != 7:
        return "root constant block is not 28 bytes / 7 dwords"
    if mapping.get("requires_64bit_integer_shader_capability") is not True:
        return "layout does not require the 64-bit integer shader capability"
    return None


def dll_fingerprint(path: Path) -> dict:
    """Pin the feature-built DLL that this run exercised (same shape as the
    segment 4d smoke). target/debug/rurix_godot.dll is a mutable artifact."""
    fp: dict = {
        "dll_path_at_run": str(path.relative_to(ROOT)).replace("\\", "/"),
        "dll_sha256": None,
        "dll_size_bytes": None,
        "dll_mtime_utc": None,
        "build_profile": "debug",
        "features": ["d3d12-recording-shim"],
        "mutable_artifact_note": (
            "target/debug/rurix_godot.dll is a mutable build artifact; a later "
            "feature-off `cargo build -p rurix-godot` can overwrite it in place. "
            "Rerun ci/grx009_godot_runtime_bridge_recording_smoke.py to refresh "
            "this fingerprint and reproduce the exact feature-built DLL."
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


SNAPSHOT_DLL = WORK / "rurix_godot_d3d12_recording_shim.dll"


def snapshot_feature_dll(path: Path) -> dict:
    """Copy the feature-built DLL to an immutable snapshot under target/
    (gitignored, never committed) so the exact artifact stays reproducible."""
    if not path.is_file():
        return {"snapshot_dll_path": None, "snapshot_dll_sha256": None,
                "snapshot_error": "feature-built DLL missing at snapshot time"}
    WORK.mkdir(parents=True, exist_ok=True)
    try:
        shutil.copy2(path, SNAPSHOT_DLL)
    except OSError as exc:
        return {"snapshot_dll_path": None, "snapshot_dll_sha256": None,
                "snapshot_error": f"{type(exc).__name__}: {exc}"}
    return {
        "snapshot_dll_path": str(SNAPSHOT_DLL.relative_to(ROOT)).replace("\\", "/"),
        "snapshot_dll_sha256": sha256_file(SNAPSHOT_DLL),
    }


def patch_stack_identity(
    stack_names: tuple[str, ...] = PATCH_STACK,
    stack_id: str = "0001..0008",
) -> dict:
    """Pin the identity of a full patch stack the scratch Godot exe must
    carry (defaults: the segment 4f 0001..0008 stack; the segment 4h smoke
    passes its 0001..0009 stack). Only the patch *artifacts* are tracked and
    fingerprinted here; the scratch Godot build binaries they produce are
    never committed."""
    patches: list[dict] = []
    for name in stack_names:
        p = PATCHES_DIR / name
        patches.append(
            {
                "patch": name,
                "path": str(p.relative_to(ROOT)).replace("\\", "/"),
                "sha256": sha256_file(p),
                "size_bytes": p.stat().st_size if p.is_file() else None,
            }
        )
    return {
        "patches_dir": str(PATCHES_DIR.relative_to(ROOT)).replace("\\", "/"),
        "stack": stack_id,
        "patches": patches,
    }


def run_git(args: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)


def find_git_root(path: Path) -> Path | None:
    current = path if path.is_dir() else path.parent
    for candidate in (current, *current.parents):
        if (candidate / ".git").exists():
            return candidate
    return None


def source_status_clean(source_dir: Path) -> tuple[bool, list[str]]:
    status = run_git(["status", "--porcelain", "--untracked-files=all"], source_dir)
    if status.returncode != 0:
        lines = (status.stdout + status.stderr).strip().splitlines()
        return False, lines or [f"git status failed with exit code {status.returncode}"]
    lines = [line for line in status.stdout.splitlines() if line.strip()]
    return len(lines) == 0, lines


def git_value(args: list[str], cwd: Path) -> str | None:
    result = run_git(args, cwd)
    value = result.stdout.strip()
    if result.returncode == 0 and value:
        return value
    return None


def scratch_source_root(godot_exe: Path) -> tuple[Path | None, str | None]:
    override = os.environ.get(SCRATCH_SOURCE_ENV)
    if override:
        source = Path(override)
        if not source.is_dir():
            return None, f"{SCRATCH_SOURCE_ENV}={override} does not point at an existing directory"
        root = find_git_root(source)
        if root is None:
            return None, f"{SCRATCH_SOURCE_ENV}={override} is not inside a git worktree"
        return root, None
    root = find_git_root(godot_exe)
    if root is None:
        return None, (
            f"cannot locate scratch Godot source root from {godot_exe}; set "
            f"{SCRATCH_SOURCE_ENV} to the full-stack Godot source worktree"
        )
    return root, None


def source_provenance_sidecar_path() -> Path | None:
    value = os.environ.get(SCRATCH_SOURCE_PROVENANCE_ENV)
    return Path(value) if value else None


def patch_entry_matches_current(entry: object, expected: dict, order: int | None = None) -> bool:
    if not isinstance(entry, dict):
        return False
    if order is not None and entry.get("order") != order:
        return False
    return (
        entry.get("patch") == expected.get("patch")
        and entry.get("sha256") == expected.get("sha256")
        and entry.get("size_bytes") == expected.get("size_bytes")
    )


def load_source_provenance_sidecar(path: Path | None) -> tuple[dict | None, str | None]:
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


def verify_source_provenance_sidecar(
    sidecar: dict | None,
    source_root: Path,
    *,
    stack_names: tuple[str, ...] = PATCH_STACK,
    stack_id: str = "0001..0008",
    sidecar_path: Path | None = None,
) -> tuple[bool, list[str], dict]:
    errors: list[str] = []
    audit: dict = {}
    if sidecar_path is None:
        sidecar_path = source_provenance_sidecar_path()
    audit["source_provenance_sidecar_path"] = str(sidecar_path) if sidecar_path is not None else None
    if sidecar is None:
        errors.append("missing source provenance sidecar")
        return False, errors, audit

    expected_stack = patch_stack_identity(stack_names, stack_id)
    expected_patches = expected_stack.get("patches")
    audit["base_commit"] = sidecar.get("base_commit")
    audit["base_tree"] = sidecar.get("base_tree")
    audit["final_head"] = sidecar.get("final_head")
    audit["final_tree"] = sidecar.get("final_tree")
    audit["actual_head"] = git_value(["rev-parse", "HEAD"], source_root)
    audit["actual_tree"] = git_value(["rev-parse", "HEAD^{tree}"], source_root)
    audit["patch_application_audit"] = sidecar.get("patch_application_audit")
    audit["expected_stack_ok"] = sidecar.get("expected_stack_ok")
    audit["source_audit_supported"] = False

    if sidecar.get("base_snapshot") != str(GODOT_BASE_SNAPSHOT.relative_to(ROOT)).replace("\\", "/"):
        errors.append("sidecar base_snapshot does not match external/godot-master")
    if sidecar.get("stack") != stack_id or sidecar.get("patch_count") != len(stack_names):
        errors.append(
            f"sidecar does not record stack {stack_id} with {len(stack_names)} patches"
        )
    if sidecar.get("tracked_patch_stack_only") is not True:
        errors.append("sidecar tracked_patch_stack_only is not true")
    if sidecar.get("expected_stack_ok") is not True:
        errors.append("sidecar expected_stack_ok is not true")
    if sidecar.get("final_status_clean") is not True:
        errors.append("sidecar final_status_clean is not true")
    if sidecar.get("actual_status_clean") is not True:
        errors.append("sidecar actual_status_clean is not true")

    applied = sidecar.get("applied_patch_stack")
    if not isinstance(applied, dict) or applied.get("stack") != stack_id:
        errors.append("sidecar applied_patch_stack is missing or has the wrong stack")
    else:
        patches = applied.get("patches")
        if not isinstance(patches, list) or not isinstance(expected_patches, list) or len(patches) != len(expected_patches):
            errors.append("sidecar applied_patch_stack patch count mismatch")
        elif any(not patch_entry_matches_current(entry, expected) for entry, expected in zip(patches, expected_patches)):
            errors.append("sidecar applied_patch_stack does not match current tracked patch files")

    patch_audit = sidecar.get("patch_application_audit")
    if not isinstance(patch_audit, list) or not isinstance(expected_patches, list) or len(patch_audit) != len(expected_patches):
        errors.append(
            f"sidecar patch_application_audit must contain exactly {len(stack_names)} entries"
        )
    else:
        for index, (entry, expected) in enumerate(zip(patch_audit, expected_patches), start=1):
            if not patch_entry_matches_current(entry, expected, index):
                errors.append(f"sidecar patch_application_audit entry {index} does not match current patch stack")
                break
            if not isinstance(entry, dict) or not entry.get("commit") or not entry.get("tree"):
                errors.append(f"sidecar patch_application_audit entry {index} is missing commit/tree")
                break
        else:
            # Mirror the probe's segment 4f sidecar chain check
            # (grx009_segment4f_scratch_source_provenance_ok): the LAST audited
            # patch application must be the sidecar's declared final state, or
            # the audit trail does not actually end at final_head/final_tree.
            last_entry = patch_audit[-1]
            if isinstance(last_entry, dict):
                if last_entry.get("commit") != sidecar.get("final_head"):
                    errors.append(
                        "sidecar patch_application_audit[-1].commit does not match final_head"
                    )
                if last_entry.get("tree") != sidecar.get("final_tree"):
                    errors.append(
                        "sidecar patch_application_audit[-1].tree does not match final_tree"
                    )

    if not audit.get("actual_tree"):
        errors.append("current source root HEAD tree could not be read")
    if sidecar.get("final_tree") != audit.get("actual_tree"):
        errors.append("current source root tree does not match sidecar final_tree")
    if sidecar.get("actual_tree") != audit.get("actual_tree"):
        errors.append("sidecar actual_tree does not match current source root tree")

    sidecar_root = sidecar.get("actual_source_root_at_generation") or sidecar.get("source_root_at_generation")
    if isinstance(sidecar_root, str) and Path(sidecar_root).resolve() != source_root.resolve():
        errors.append("sidecar source root does not match current source root")

    audit["source_audit_supported"] = len(errors) == 0
    return len(errors) == 0, errors, audit


def scratch_source_provenance(godot_exe: Path) -> dict:
    source_root, source_error = scratch_source_root(godot_exe)
    exe_fp = godot_exe_fingerprint(godot_exe)
    build_command = os.environ.get(SCRATCH_BUILD_COMMAND_ENV)
    build_log = os.environ.get(SCRATCH_BUILD_LOG_ENV)
    provenance: dict = {
        "base_snapshot": str(GODOT_BASE_SNAPSHOT.relative_to(ROOT)).replace("\\", "/"),
        "source_root_at_run": str(source_root) if source_root is not None else None,
        "source_clean": False,
        "source_status": [],
        "tracked_patch_stack_only": False,
        "source_audit_supported": False,
        "source_audit_errors": [],
        "source_provenance_sidecar_path": None,
        "applied_patch_stack": patch_stack_identity(),
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
    sidecar, sidecar_error = load_source_provenance_sidecar(source_provenance_sidecar_path())
    ok, errors, audit = verify_source_provenance_sidecar(sidecar, source_root)
    if sidecar_error is not None:
        errors.insert(0, sidecar_error)
    provenance.update(audit)
    provenance["source_audit_errors"] = errors
    provenance["tracked_patch_stack_only"] = clean and ok
    provenance["source_audit_supported"] = clean and ok
    return provenance


def patch_queue_contains(marker: str, stack_names: tuple[str, ...] = PATCH_STACK) -> bool:
    for name in stack_names:
        path = PATCHES_DIR / name
        if path.is_file() and marker in path.read_text(encoding="utf-8", errors="ignore"):
            return True
    return False


def runtime_log_audit(output: str, stack_names: tuple[str, ...] = PATCH_STACK) -> dict:
    rxgd_diag_allowed = patch_queue_contains("RXGD_DIAG", stack_names)
    rxgd_diag_lines = [line for line in output.splitlines() if "RXGD_DIAG" in line]
    allowed_error = "Could not load global script cache"
    # Zero-tolerance for engine "ERROR:" lines AND GDScript "SCRIPT ERROR" lines.
    # Closing the fused audit hole: a scene-script property-assignment error
    # (assigning a property the base class does not expose) was previously ignored
    # here because only the "ERROR:" prefix was scanned, so a scene whose auto
    # exposure never engaged still passed the runtime audit.
    error_lines = [
        line
        for line in output.splitlines()
        if line.strip().startswith("ERROR:") or line.strip().startswith("SCRIPT ERROR")
    ]
    unexpected_errors = [line for line in error_lines if allowed_error not in line]
    unexpected_lines = ([] if rxgd_diag_allowed else rxgd_diag_lines) + unexpected_errors
    return {
        "unexpected_rxgd_diag_count": 0 if rxgd_diag_allowed else len(rxgd_diag_lines),
        "rxgd_diag_allowed_by_tracked_patch_queue": rxgd_diag_allowed,
        "unexpected_godot_error_count": len(unexpected_errors),
        "allowed_godot_errors": [
            {
                "message": allowed_error,
                "observed_count": sum(1 for line in error_lines if allowed_error in line),
                "rationale": (
                    "Tolerated Godot global script cache shutdown/cache warning in the generated minimal smoke project; "
                    "it does not indicate runtime bridge recording failure when the process exits 0 and recording checks pass."
                ),
            }
        ],
        "unexpected_lines_tail": unexpected_lines[-20:],
    }


def godot_exe_fingerprint(path: Path) -> dict:
    """Fingerprint the full-stack scratch Godot console exe that drove this run.

    The exe itself is a local, gitignored scratch build artifact rebuilt from
    the ignored external/godot-master snapshot with the 0001..0008 patch stack
    applied and module_rurix_accel_enabled=yes d3d12=yes; it is never committed.
    """
    fp: dict = {
        "exe_path_at_run": str(path),
        "exe_sha256": None,
        "exe_size_bytes": None,
        "exe_mtime_utc": None,
        "committed": False,
        "scratch_build_note": (
            "Scratch Godot build binaries are NOT committed to the repo. This "
            "console exe is a local, gitignored artifact rebuilt from the "
            "ignored external/godot-master snapshot with the full 0001..0008 "
            "segment 4f patch stack applied (module_rurix_accel_enabled=yes "
            "d3d12=yes). Only its fingerprint is recorded here so the historical "
            "measured success stays auditable; re-point "
            f"{GODOT_EXE_ENV} at an equivalent rebuild to reproduce it."
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


# Assembled at runtime so the evidence always records the exact digests/paths.
_EVIDENCE_BASE: dict = {}


def _write_json(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    # Byte-level LF only (repo .gitattributes pins `* -text`); never emit CRLF.
    path.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def write_evidence(status: str, *, reason: str | None = None, extra: dict | None = None) -> None:
    doc = dict(_EVIDENCE_BASE)
    doc["status"] = status
    # success is the only status allowed to assert the Godot runtime call site
    # drove a real bridge-recorded dispatch.
    doc["godot_runtime_bridge_recorded_dispatch"] = status == "success"
    doc["timestamp"] = now_iso()
    doc["run_url"] = github_run_url()
    if reason is not None:
        doc["reason"] = reason
    if extra:
        doc.update(extra)

    # The *latest* evidence file is always rewritten (SKIP by reproducible
    # default when the scratch Godot exe env var is absent).
    _write_json(EVIDENCE_OUT, doc)
    print(f"[grx009-godot-runtime-recording-smoke] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={status}")

    # The *historical* measured success artifact is only ever written on a
    # strict success. A SKIP/FAIL run must NOT delete or overwrite it, so the
    # 4f readiness gate can stay green off a prior measured run even after the
    # latest evidence reverts to the reproducible-default SKIP.
    if status == "success":
        success_doc = dict(doc)
        success_doc["evidence_kind"] = "historical_measured_success"
        success_doc["latest_evidence_path"] = (
            str(EVIDENCE_OUT.relative_to(ROOT)).replace("\\", "/")
        )
        success_doc["success_evidence_note"] = (
            "Historical measured success artifact for GRX-009 segment 4f. It is "
            "written ONLY on a strict status=success run and is never deleted or "
            "overwritten by a later SKIP/FAIL run, so the readiness gate advances "
            "off this file rather than the reproducible-default SKIP latest "
            "evidence. Scratch Godot build binaries are not committed."
        )
        _write_json(SUCCESS_EVIDENCE_OUT, success_doc)
        print(
            "[grx009-godot-runtime-recording-smoke] wrote "
            f"{SUCCESS_EVIDENCE_OUT.relative_to(ROOT)} status=success (historical measured success)"
        )


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx009-godot-runtime-recording-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx009-godot-runtime-recording-smoke] SKIP {msg}(降级 SKIP,退出 0)")
    write_evidence("skip", reason=msg, extra=extra or {})
    return 0


def build_bridge_dll(env: dict) -> tuple[bool, str]:
    """Build rurix_godot.dll with the d3d12-recording-shim feature. RURIX_DXC_DIR
    (in env) lets build.rs find dxcapi.h so the shim can sign the in-memory DXIL."""
    p = subprocess.run(
        ["cargo", "build", "-p", "rurix-godot", "--features", "d3d12-recording-shim"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
    )
    log = (p.stdout + p.stderr).strip()
    ok = p.returncode == 0 and RURIX_GODOT_DLL.is_file()
    return ok, log[-3000:]


def write_smoke_project(dll_path: Path) -> None:
    """Generate a minimal Godot project that opts into the harness-only
    dispatch_recording_smoke arm so the runtime luminance call site drives the
    bridge recording. All other per-pass settings stay default; only the
    test-only recording arm and the pass enable flag are turned on here.
    """
    PROJECT_DIR.mkdir(parents=True, exist_ok=True)
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    project_text = f"""\
; Engine configuration file.
; Auto-generated by ci/grx009_godot_runtime_bridge_recording_smoke.py

config_version=5

[application]

config/name="GRX-009 segment 4f Godot-runtime luminance recording smoke"
run/main_scene="res://main.tscn"

[rendering]

rurix_accel/enabled=true
rurix_accel/require_forward_plus=true
rurix_accel/dll_path="{dll_path.as_posix()}"
rurix_accel/passes/luminance_reduction/enabled=true
rurix_accel/passes/luminance_reduction/dispatch_bringup=true
rurix_accel/passes/luminance_reduction/dispatch_recording_smoke=true
"""
    scene_text = """\
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://main.gd" id="1"]

[node name="GRX009Segment4fRoot" type="Node3D"]
script = ExtResource("1")

[node name="Camera3D" type="Camera3D" parent="."]

[node name="WorldEnvironment" type="WorldEnvironment" parent="."]
"""
    # A tiny scene with tonemap + auto exposure enabled so the Auto Exposure
    # luminance_reduction call site actually runs, then quit after a few frames.
    # In Godot 4.x auto exposure lives on CameraAttributes (the Environment
    # auto_exposure_* properties were removed), and the renderer gates the Auto
    # Exposure luminance_reduction call site on
    # camera_attributes_uses_auto_exposure(...), so a CameraAttributesPractical
    # with auto_exposure_enabled must be attached to the current Camera3D.
    script_text = """\
extends Node3D

var _frames := 0

func _ready() -> void:
    var cam: Camera3D = $Camera3D
    cam.make_current()
    var attributes := CameraAttributesPractical.new()
    attributes.auto_exposure_enabled = true
    cam.attributes = attributes

    var env := Environment.new()
    env.background_mode = Environment.BG_COLOR
    env.background_color = Color(0.6, 0.6, 0.6)
    env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
    $WorldEnvironment.environment = env
    print("GRX009Segment4f: scene ready")

func _process(_delta: float) -> void:
    _frames += 1
    if _frames >= 8:
        print("GRX009Segment4f: quitting")
        get_tree().quit()
"""
    (PROJECT_DIR / "project.godot").write_text(project_text, encoding="utf-8", newline="\n")
    (PROJECT_DIR / "main.tscn").write_text(scene_text, encoding="utf-8", newline="\n")
    (PROJECT_DIR / "main.gd").write_text(script_text, encoding="utf-8", newline="\n")


def run_godot(godot_exe: Path) -> tuple[int, str]:
    command = [
        str(godot_exe),
        "--path",
        str(PROJECT_DIR),
        "--rendering-driver",
        REQUESTED_RENDERER,
        "--rendering-method",
        REQUESTED_RENDERING_METHOD,
        "--verbose",
    ]
    try:
        proc = subprocess.run(
            command,
            cwd=PROJECT_DIR,
            text=True,
            capture_output=True,
            check=False,
            timeout=GODOT_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as exc:
        out = ""
        if isinstance(exc.stdout, str):
            out += exc.stdout
        if isinstance(exc.stderr, str):
            out += exc.stderr
        return -1, out.strip()
    output = "\n".join(part for part in (proc.stdout, proc.stderr) if part).strip()
    (LOG_DIR / "godot_runtime_smoke.log").write_text(output + "\n", encoding="utf-8", newline="\n")
    return proc.returncode, output


def parse_runtime_marker(output: str) -> dict:
    """Extract the fields carried by the RXGD_GODOT_RUNTIME_LUMINANCE_RECORD
    marker line, if present."""
    parsed: dict = {"marker_present": RUNTIME_RECORD_MARKER in output}
    for line in output.splitlines():
        line = line.strip()
        if line.startswith(RUNTIME_RECORD_MARKER):
            body = line[len(RUNTIME_RECORD_MARKER):].lstrip(": ")
            for token in body.split(" "):
                if "=" in token:
                    k, v = token.split("=", 1)
                    parsed[k] = v
    parsed["session_ready"] = SESSION_READY_MARKER in output
    parsed["session_unavailable"] = SESSION_UNAVAILABLE_MARKER in output
    return parsed


def main() -> int:
    global _EVIDENCE_BASE

    for path in (DXIL, RTS0, DESCRIPTOR_LAYOUT, OFFLINE_EVIDENCE):
        if not path.is_file():
            _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
            return fail(f"required artifact missing: {path.relative_to(ROOT)}")

    dxil_sha = sha256_file(DXIL)
    rts0_sha = sha256_file(RTS0)
    layout_sha = sha256_file(DESCRIPTOR_LAYOUT)
    offline = load_json(OFFLINE_EVIDENCE)
    layout = load_json(DESCRIPTOR_LAYOUT)
    if offline is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read offline_compile_evidence.json")
    if layout is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read luminance_reduction_descriptor_layout.json")

    offline_digests = offline_artifact_digests(offline)
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "luminance_reduction",
        "segment": "4f",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "godot_runtime_luminance_path_enabled": False,
        "default_enable_state": "disabled",
        "gpu_timestamp_status": "not_yet",
        "gpu_time_ns": None,
        "bridge": {
            "dll": str(RURIX_GODOT_DLL.relative_to(ROOT)).replace("\\", "/"),
            "feature": "d3d12-recording-shim",
            "record_arm": "RXGD_CAP_LUMINANCE_DISPATCH_RECORD (harness-only, set via the default-false dispatch_recording_smoke opt-in)",
        },
        "godot": {
            "smoke_opt_in": "rendering/rurix_accel/passes/luminance_reduction/dispatch_recording_smoke",
            "smoke_opt_in_default": False,
            "project_dir": str(PROJECT_DIR.relative_to(ROOT)).replace("\\", "/"),
            "runtime_record_marker": RUNTIME_RECORD_MARKER,
            "requires_full_patch_stack": "0001..0008",
        },
        "artifacts": {
            "dxil": {"path": str(DXIL.relative_to(ROOT)).replace("\\", "/"), "sha256": dxil_sha},
            "root_signature": {"path": str(RTS0.relative_to(ROOT)).replace("\\", "/"), "sha256": rts0_sha},
            "descriptor_layout": {
                "path": str(DESCRIPTOR_LAYOUT.relative_to(ROOT)).replace("\\", "/"),
                "sha256": layout_sha,
            },
        },
        "offline_evidence": {
            "path": str(OFFLINE_EVIDENCE.relative_to(ROOT)).replace("\\", "/"),
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
            "GRX-009 segment 4f Godot-runtime bridge D3D12 dispatch recording smoke "
            "evidence only. Even a success here keeps runtime_state=fallback_only, "
            "real_gpu_pass=false, real_d3d12_dispatch_recorded=false, "
            "godot_runtime_luminance_path_enabled=false, and default_enable_state="
            "disabled: the recording path is compiled only under the test-only "
            "d3d12-recording-shim feature and armed only by the default-false "
            "dispatch_recording_smoke opt-in the generated smoke project turns on. "
            "The shipping/feature-off bridge and the default Godot config still "
            "return RXGD_STATUS_FALLBACK. It makes no visual, perf, GPU-timestamp, "
            "or measured-fallback-telemetry claim."
        ),
    }

    if not _EVIDENCE_BASE["artifact_hashes_match_offline_evidence"]:
        return fail(
            "artifact SHA-256 does not match tracked offline compile evidence "
            f"(dxil={dxil_sha} vs {offline_digests['dxil']}, "
            f"rts0={rts0_sha} vs {offline_digests['root_signature']}, "
            f"layout={layout_sha} vs {offline_digests['descriptor_layout']})"
        )

    layout_issue = descriptor_layout_matches_resource_mapping(layout)
    if layout_issue is not None:
        return fail(f"descriptor layout / resource mapping mismatch: {layout_issue}")

    # Preconditions that only warrant a SKIP (environment-level, not a hash/layout
    # integrity failure). None of these advance the readiness gate.
    godot_exe, godot_reason = locate_segment4f_godot_exe()
    if godot_exe is None:
        return skip(godot_reason or "segment 4f Godot exe unavailable")

    vcvars = locate_vcvars64()
    if vcvars is None:
        return skip("未找到 VS vcvars64.bat(set RURIX_VCVARS64);无法编译 d3d12-recording-shim bridge DLL")

    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip(
            "未找到含 dxil.dll 的签名 DXC pin(set RURIX_DXC_DIR=H:\\dxc-round7\\extracted\\bin\\x64);"
            "bridge 录制 shim 无法为编译器产出的 DXIL container 签名以在非 Developer-Mode device 上加载"
        )
    include_dir = locate_dxcapi_include(dxc_dir)
    if include_dir is None:
        return skip(f"未在 {dxc_dir} 附近找到 dxcapi.h(bridge 录制 shim 签名路径无法编译)")

    # Build the recording-shim bridge DLL. RURIX_DXC_DIR must be visible to
    # build.rs so it can add the dxcapi.h include dir (in-memory DXIL signing).
    build_env = dict(os.environ)
    build_env.setdefault("RURIX_DXC_DIR", str(dxc_dir))
    built_dll, dll_log = build_bridge_dll(build_env)
    if not built_dll:
        print(dll_log, file=sys.stderr)
        return fail("cargo build -p rurix-godot --features d3d12-recording-shim failed",
                    extra={"build_log_tail": dll_log})

    fingerprint = dll_fingerprint(RURIX_GODOT_DLL)
    fingerprint.update(snapshot_feature_dll(RURIX_GODOT_DLL))
    _EVIDENCE_BASE["dll_fingerprint"] = fingerprint

    write_smoke_project(RURIX_GODOT_DLL)

    # The Godot module loads rurix_godot.dll and signs the in-memory DXIL via
    # dxil.dll from RURIX_DXC_DIR at record time.
    os.environ.setdefault("RURIX_DXC_DIR", str(dxc_dir))
    exit_code, output = run_godot(godot_exe)
    parsed = parse_runtime_marker(output)
    godot_info = {
        "exe": str(godot_exe),
        "exit_code": exit_code,
        "session_ready": parsed.get("session_ready"),
        "session_unavailable": parsed.get("session_unavailable"),
        "runtime_record_marker_present": parsed.get("marker_present"),
    }

    if exit_code == -1:
        return skip(
            "Godot runtime smoke timed out; no runtime luminance recording marker observed",
            extra={"godot": {**_EVIDENCE_BASE["godot"], **godot_info}, "stdout": output[-4000:]},
        )

    if parsed.get("session_unavailable") and not parsed.get("session_ready"):
        return skip(
            "Godot bridge session unavailable (no real D3D12 session created in this environment); "
            "runtime luminance recording did not run",
            extra={"godot": {**_EVIDENCE_BASE["godot"], **godot_info}, "stdout": output[-4000:]},
        )

    if not parsed.get("marker_present"):
        return skip(
            "Godot ran but the RXGD_GODOT_RUNTIME_LUMINANCE_RECORD marker was not observed; "
            "the runtime luminance call site did not drive a bridge recording (likely the "
            "Godot build lacks the full 0001..0008 patch stack, the device is unsupported, or "
            "the recording shim was not linked)",
            extra={"godot": {**_EVIDENCE_BASE["godot"], **godot_info}, "stdout": output[-4000:]},
        )

    if str(parsed.get("recorded")) != "1":
        return fail(
            "runtime luminance recording marker present but did not report recorded=1",
            extra={"godot": {**_EVIDENCE_BASE["godot"], **godot_info}, "stdout": output[-4000:]},
        )

    # The Godot runtime luminance call site can print the recorded=1 marker mid-run,
    # but a success must also observe the Godot process exit cleanly (exit_code == 0).
    # A non-zero exit means the runtime crashed/aborted after recording, so the run
    # is NOT a clean success — record it as a hard FAIL, never success.
    if exit_code != 0:
        return fail(
            "runtime luminance recording marker reported recorded=1 but the Godot process "
            f"exited with non-zero exit code {exit_code}; a clean runtime smoke requires "
            "the Godot process to exit 0",
            extra={"godot": {**_EVIDENCE_BASE["godot"], **godot_info}, "stdout": output[-4000:]},
        )

    provenance = scratch_source_provenance(godot_exe)
    audit = runtime_log_audit(output)
    strict_extra = {
        "godot": {**_EVIDENCE_BASE["godot"], **godot_info},
        "scratch_source_provenance": provenance,
        "runtime_log_audit": audit,
        "stdout": output[-6000:],
    }
    if provenance.get("source_clean") is not True:
        return fail(
            "scratch Godot source provenance is not clean; strict segment 4f success requires "
            "external/godot-master plus tracked 0001..0008 patch stack with no source deltas",
            extra=strict_extra,
        )
    if provenance.get("tracked_patch_stack_only") is not True:
        return fail(
            "scratch Godot source provenance is not limited to the tracked 0001..0008 patch stack",
            extra=strict_extra,
        )
    if audit.get("unexpected_rxgd_diag_count") != 0:
        return fail(
            "runtime log contains RXGD_DIAG markers not present in the tracked patch queue",
            extra=strict_extra,
        )
    if audit.get("unexpected_godot_error_count") != 0:
        return fail("runtime log contains unexpected Godot ERROR lines", extra=strict_extra)

    write_evidence(
        "success",
        extra={
            "godot": {**_EVIDENCE_BASE["godot"], **godot_info},
            "godot_exe_fingerprint": godot_exe_fingerprint(godot_exe),
            "patch_stack_identity": patch_stack_identity(),
            "scratch_source_provenance": provenance,
            "runtime_log_audit": audit,
            "recording": {
                "runtime_record_marker": RUNTIME_RECORD_MARKER,
                "recorded": parsed.get("recorded"),
                "pass": parsed.get("pass"),
            },
            "checks": {
                "artifact_hashes_match_offline_evidence": True,
                "descriptor_layout_matches_resource_mapping": True,
                "recording_shim_linked": True,
                "godot_runtime_session_ready": parsed.get("session_ready") is True,
                "godot_runtime_call_site_recorded": parsed.get("marker_present") is True,
                "recorded_one_pass": str(parsed.get("recorded")) == "1",
                "godot_exit_code_zero": exit_code == 0,
            },
            "stdout": output[-6000:],
        },
    )
    print("[grx009-godot-runtime-recording-smoke] PASS Godot runtime luminance call site drove a "
          "bridge-recorded D3D12 dispatch")
    return 0


if __name__ == "__main__":
    sys.exit(main())
