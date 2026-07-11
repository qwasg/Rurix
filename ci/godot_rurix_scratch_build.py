#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Automate the GRX Godot *scratch* build convention.

The GRX downstream enablement smokes (segment 4h luminance, GRX-010 tonemap,
future GRX-011 ssao_blur) require a **scratch** Godot console executable rebuilt
from the ignored ``external/godot-master`` snapshot with the FULL per-milestone
patch stack applied. That build has, until now, been a purely manual habit
(copy the tree, ``git apply`` 0001..000N in order, run SCons with
``num_jobs=1`` for the MSVC C1001 ICE workaround, then hand-write a provenance
sidecar). This script automates every step, fail-closed, and prints the exact
enablement-smoke environment variable lines so a scratch build can be pointed at
its downstream gate without hand-editing anything.

Design invariants (do NOT relax):

  * ``external/godot-master`` is never written to. The scratch tree is a fresh,
    writable copy (robocopy, multithreaded) that lives entirely under ``target/``
    (gitignored). It is normalised back to a pristine base (reverse-applying any
    already-applied patches) and then the requested stack is applied fresh, so
    the scratch tree's committed content is *exactly* base + the tracked patch
    stack and nothing else.
  * The generated provenance sidecar is byte-compatible with what
    ``ci/grx009_segment4f_godot_source_provenance.py`` produces and what the
    downstream smokes' ``verify_source_provenance_sidecar`` accepts: the sidecar
    records the tracked patch stack (name + sha256 + size), the independent
    base→per-patch→final commit/tree audit chain, ``final_head``/``final_tree``,
    the build command, the build log path, and the console exe sha256. The
    independent expected chain is rebuilt in a throwaway repo and its final tree
    must equal the scratch tree's HEAD tree (``tracked_patch_stack_only``); a
    mismatch is a loud failure, never a silent success.
  * Fail-closed everywhere: a failed ``git apply``, a failed SCons build, or a
    missing console exe is a non-zero exit with an honest status document. No
    fake success is ever produced.
  * Byte-level LF only (the repo pins ``* -text``); every text artifact this
    script writes uses ``newline="\\n"``.

Typical usage::

    # Validate the current 0001..0013 stack is fully stackable (no copy/build):
    py -3 ci/godot_rurix_scratch_build.py --check-only --patches 0001-0013

    # Prepare a scratch source tree (copy + apply) without building, e.g. to
    # inspect the generated provenance sidecar:
    py -3 ci/godot_rurix_scratch_build.py --patches 0001-0013 --skip-build \\
        --dest target/grx/godot-scratch-0001..0013

    # Full scratch build (copy + apply + SCons num_jobs=1), then print the
    # enablement-smoke env lines for the matching downstream gate:
    py -3 ci/godot_rurix_scratch_build.py --patches 0001-0013

    # Faster rebuild reusing an existing scratch tree's SCons cache:
    py -3 ci/godot_rurix_scratch_build.py --patches 0001-0013 \\
        --from-scratch-tree target/grx/godot-scratch-prev
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

# Reuse the tracked patch-stack applyability helpers (--check-only).
from ci.godot_rurix_patch_stack import (  # noqa: E402
    evaluate_followup_patch_applyability,
    evaluate_patch_stack,
    evaluate_stacked_patch_applyability,
)

# Reuse the tracked segment 4f provenance machinery so the sidecar this script
# emits is byte-compatible with what the downstream smokes verify.
from ci.grx009_segment4f_godot_source_provenance import (  # noqa: E402
    commit_all,
    generate_expected_stack,
    patch_entries,
    rel,
    rev_parse,
    run_git,
    status_lines,
)

# Reuse the tracked SCons build wrapper's toolchain discovery / vcvars wrapping /
# proven ICE-workaround argument set so the scratch build matches the in-tree
# build command exactly.
from ci.godot_rurix_scons_build import (  # noqa: E402
    SCONS_BASE_ARGS,
    SCONS_ICE_ARGS,
    capture_vcvars_environment,
    discover_scons_launcher,
    discover_vs_toolchain,
    render_command,
)

EXTERNAL_GODOT = ROOT / "external" / "godot-master"
PATCHES_DIR = ROOT / "spike" / "godot-rurix" / "patches"
DEFAULT_DEST_PARENT = ROOT / "target" / "grx"
# The console executable the downstream enablement smokes point their
# ``*_GODOT_EXE`` env var at, relative to the scratch source root.
CONSOLE_EXE_REL = Path("bin") / "godot.windows.template_debug.x86_64.console.exe"

# Directories/files that are gitignored in a Godot tree and are excluded from the
# scratch copy for speed and to avoid dragging in a (possibly BSOD-corrupted)
# stale build. Because every name here is gitignored, excluding it from the copy
# is tree-neutral: the committed scratch tree is identical whether or not these
# are physically present. Mirrors the segment 4f copy_base_snapshot exclusions.
COPY_EXCLUDE_DIRS = ("bin", ".git", ".vs", "__pycache__")
COPY_EXCLUDE_FILES = (".sconsign.dblite",)

# Downstream enablement smokes and the env-var contract each one reads. The
# suffixes are shared (``_GODOT_EXE`` / ``_GODOT_SOURCE`` /
# ``_GODOT_SOURCE_PROVENANCE`` / ``_GODOT_BUILD_COMMAND`` / ``_GODOT_BUILD_LOG``);
# only the prefix and the exact patch stack each gate demands differ.
ENABLEMENT_GROUPS = (
    {
        "label": "GRX-009 segment 4h luminance real-pass enablement smoke",
        "script": "ci/grx009_segment4h_real_pass_enablement_smoke.py",
        "prefix": "RURIX_GRX009_SEGMENT4H",
        "expected_stack": "0001..0010",
    },
    {
        "label": "GRX-010 tonemap real-pass enablement smoke",
        "script": "ci/grx010_tonemap_real_pass_enablement_smoke.py",
        "prefix": "RURIX_GRX010_TONEMAP",
        "expected_stack": "0001..0013",
    },
    {
        "label": (
            "GRX-011 ssao_blur enablement smoke (anticipated: the smoke and its "
            "0014..0016 patches are not in the tree yet; prefix/stack may change "
            "when the gate lands)"
        ),
        "script": "ci/grx011_ssao_blur_real_pass_enablement_smoke.py (future)",
        "prefix": "RURIX_GRX011_SSAO_BLUR",
        "expected_stack": "0001..0016",
    },
)
ENV_SUFFIXES = (
    "_GODOT_EXE",
    "_GODOT_SOURCE",
    "_GODOT_SOURCE_PROVENANCE",
    "_GODOT_BUILD_COMMAND",
    "_GODOT_BUILD_LOG",
)

TAG = "[godot-scratch-build]"


# --------------------------------------------------------------------------- #
# small utilities
# --------------------------------------------------------------------------- #
def now_iso() -> str:
    return _dt.datetime.now().astimezone().replace(microsecond=0).isoformat()


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def format_mtime_utc(timestamp: float) -> str:
    return (
        _dt.datetime.fromtimestamp(timestamp, tz=_dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def write_json(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def info(msg: str) -> None:
    print(f"{TAG} {msg}")


def warn(msg: str) -> None:
    print(f"{TAG} WARN {msg}", file=sys.stderr)


# --------------------------------------------------------------------------- #
# patch selection
# --------------------------------------------------------------------------- #
_NUM_RE = re.compile(r"^(\d{4})$")
_RANGE_RE = re.compile(r"^(\d{4})-(\d{4})$")


def patch_number_index() -> dict[str, Path]:
    """Map the 4-digit ordinal prefix of every patch in PATCHES_DIR to its file."""
    index: dict[str, Path] = {}
    for path in sorted(PATCHES_DIR.glob("*.patch")):
        match = re.match(r"^(\d{4})-", path.name)
        if match:
            index[match.group(1)] = path
    return index


def resolve_patches(spec: str) -> tuple[list[Path], list[str]]:
    """Resolve a ``--patches`` spec into an ordered list of patch files.

    Accepts a contiguous range (``0001-0013``), an explicit comma/space list of
    ordinals (``0001,0004,0007``), or full patch filenames. Returns (paths,
    errors); a non-empty error list means the spec is unresolvable.
    """
    index = patch_number_index()
    errors: list[str] = []
    ordered: list[str] = []

    tokens = [t for t in re.split(r"[,\s]+", spec.strip()) if t]
    if not tokens:
        return [], ["--patches is empty"]

    for token in tokens:
        range_match = _RANGE_RE.match(token)
        if range_match:
            lo, hi = int(range_match.group(1)), int(range_match.group(2))
            if hi < lo:
                errors.append(f"range {token} is descending")
                continue
            for n in range(lo, hi + 1):
                ordered.append(f"{n:04d}")
            continue
        num_match = _NUM_RE.match(token)
        if num_match:
            ordered.append(num_match.group(1))
            continue
        if token.endswith(".patch"):
            file_match = re.match(r"^(\d{4})-", token)
            if file_match and (PATCHES_DIR / token).is_file():
                ordered.append(file_match.group(1))
                continue
        errors.append(f"unrecognised --patches token: {token!r}")

    paths: list[Path] = []
    for ordinal in ordered:
        path = index.get(ordinal)
        if path is None:
            errors.append(f"no patch file for ordinal {ordinal} in {rel(PATCHES_DIR)}")
            continue
        paths.append(path)
    return paths, errors


def stack_id_for(patch_paths: list[Path]) -> str:
    """Compute the canonical stack id string (``0001..0013`` when contiguous)."""
    ordinals = [p.name[:4] for p in patch_paths]
    nums = [int(o) for o in ordinals]
    contiguous = all(nums[i] + 1 == nums[i + 1] for i in range(len(nums) - 1))
    if contiguous and len(nums) >= 2:
        return f"{ordinals[0]}..{ordinals[-1]}"
    if len(nums) == 1:
        return ordinals[0]
    return ",".join(ordinals)


# --------------------------------------------------------------------------- #
# robocopy
# --------------------------------------------------------------------------- #
def robocopy(
    src: Path,
    dst: Path,
    *,
    mirror: bool,
    exclude_dirs: tuple[str, ...],
    exclude_files: tuple[str, ...],
) -> tuple[bool, str]:
    """Multithreaded copy via robocopy. robocopy exit codes < 8 are success."""
    dst.mkdir(parents=True, exist_ok=True)
    args = [
        "robocopy",
        str(src),
        str(dst),
        "/MIR" if mirror else "/E",
        "/MT:16",
        "/R:1",
        "/W:1",
        "/NFL",
        "/NDL",
        "/NJH",
        "/NJS",
        "/NP",
    ]
    for name in exclude_dirs:
        args += ["/XD", name]
    for name in exclude_files:
        args += ["/XF", name]
    proc = subprocess.run(args, capture_output=True, text=True, check=False)
    output = ((proc.stdout or "") + (proc.stderr or "")).strip()
    # robocopy: 0-7 == success (bit flags), 8+ == at least one failure.
    return proc.returncode < 8, output


# --------------------------------------------------------------------------- #
# scratch tree assembly
# --------------------------------------------------------------------------- #
def git(args: list[str], repo: Path) -> subprocess.CompletedProcess[str]:
    return run_git(args, repo)


def normalize_to_base(repo: Path, patch_paths_all: list[Path]) -> tuple[bool, list[str], str | None]:
    """Reverse-apply every currently-applied patch (highest ordinal first) until
    the tree is at pristine base. Works for any already-applied prefix (the
    tracked ``external/godot-master`` carries 0001+0002+0003; a prior scratch
    tree may carry a deeper stack)."""
    reversed_names: list[str] = []
    ordered = sorted(patch_paths_all, key=lambda p: p.name, reverse=True)
    for patch in ordered:
        check = git(
            ["-c", "core.autocrlf=false", "apply", "--reverse", "--check", str(patch.resolve())],
            repo,
        )
        if check.returncode != 0:
            continue
        applied = git(
            ["-c", "core.autocrlf=false", "apply", "--reverse", str(patch.resolve())],
            repo,
        )
        if applied.returncode != 0:
            return False, reversed_names, (applied.stderr.strip() or applied.stdout.strip())
        reversed_names.append(patch.name)
    return True, reversed_names, None


def build_scratch_tree(
    scratch: Path, patch_paths: list[Path]
) -> tuple[bool, dict, str | None]:
    """git init the scratch copy, normalise to base, apply the requested stack
    committing after each patch, and return the actual commit/tree chain."""
    init = git(["-c", "core.autocrlf=false", "init", "--quiet", "."], scratch)
    if init.returncode != 0:
        return False, {}, f"scratch git init failed: {init.stderr.strip() or init.stdout.strip()}"
    git(["config", "core.autocrlf", "false"], scratch)
    git(["config", "user.name", "GRX Scratch Build"], scratch)
    git(["config", "user.email", "grx-scratch-build@example.invalid"], scratch)

    all_patches = sorted(patch_number_index().values(), key=lambda p: p.name)
    normalized, reversed_names, norm_err = normalize_to_base(scratch, all_patches)
    if not normalized:
        return False, {}, f"could not normalise scratch tree to base: {norm_err}"

    base_head, base_tree, base_err = commit_all(scratch, "rurix scratch base snapshot")
    if base_err is not None:
        return False, {}, f"scratch base commit failed: {base_err}"

    audit: list[dict] = []
    entries = patch_entries(PATCHES_DIR, tuple(p.name for p in patch_paths))
    for entry in entries:
        patch = PATCHES_DIR / str(entry["patch"])
        applied = git(
            ["-c", "core.autocrlf=false", "apply", str(patch.resolve())], scratch
        )
        if applied.returncode != 0:
            return (
                False,
                {"failed_patch": entry, "patch_application_audit": audit},
                f"git apply failed for {entry['patch']}: "
                f"{applied.stderr.strip() or applied.stdout.strip()}",
            )
        commit, tree, commit_err = commit_all(scratch, f"rurix scratch apply {entry['patch']}")
        if commit_err is not None:
            return False, {}, f"scratch commit failed for {entry['patch']}: {commit_err}"
        audit.append({**entry, "commit": commit, "tree": tree})

    clean, status = status_lines(scratch)
    return (
        True,
        {
            "base_head": base_head,
            "base_tree": base_tree,
            "base_snapshot_reversed_patches": reversed_names,
            "patch_application_audit": audit,
            "final_head": rev_parse(scratch, "HEAD"),
            "final_tree": rev_parse(scratch, "HEAD^{tree}"),
            "final_status_clean": clean,
            "final_status": status,
        },
        None,
    )


def prepare_scratch_copy(
    scratch: Path, from_scratch_tree: Path | None
) -> tuple[bool, str, str | None]:
    """Populate the scratch directory. Returns (ok, mode, error).

    Incremental (``--from-scratch-tree``) mirrors an existing scratch tree
    (keeping its ``bin``/``.sconsign`` for a fast rebuild) then rebuilds the git
    patch layer from scratch. On any incremental failure it falls back to a
    clean full copy from ``external/godot-master``. The full-copy path always
    excludes gitignored build junk; the resulting committed tree is identical
    either way.
    """
    if from_scratch_tree is not None:
        try:
            if not from_scratch_tree.is_dir():
                raise RuntimeError(
                    f"--from-scratch-tree {from_scratch_tree} is not a directory"
                )
            info(f"incremental: robocopy /MIR {from_scratch_tree} -> {scratch}")
            ok, _ = robocopy(
                from_scratch_tree,
                scratch,
                mirror=True,
                exclude_dirs=(".git",),
                exclude_files=(),
            )
            if not ok:
                raise RuntimeError("robocopy /MIR from --from-scratch-tree failed")
            # Drop any copied git metadata; the patch layer is rebuilt fresh.
            shutil.rmtree(scratch / ".git", ignore_errors=True)
            return True, "incremental", None
        except Exception as exc:  # noqa: BLE001 - degrade to full copy
            warn(f"incremental copy failed ({exc}); falling back to a full copy")
            shutil.rmtree(scratch, ignore_errors=True)

    if scratch.exists():
        info(f"removing stale scratch dir {scratch} for a clean full copy")
        shutil.rmtree(scratch, ignore_errors=True)
    top_cache = EXTERNAL_GODOT / "cache"
    if top_cache.is_dir():
        warn(
            "external/godot-master has a top-level cache/ dir; it is copied but "
            "gitignored so the committed tree is unaffected"
        )
    info(f"full copy: robocopy /E {EXTERNAL_GODOT} -> {scratch}")
    ok, output = robocopy(
        EXTERNAL_GODOT,
        scratch,
        mirror=False,
        exclude_dirs=COPY_EXCLUDE_DIRS,
        exclude_files=COPY_EXCLUDE_FILES,
    )
    if not ok:
        return False, "full", f"robocopy from external/godot-master failed: {output[-800:]}"
    return True, "full", None


# --------------------------------------------------------------------------- #
# sidecar
# --------------------------------------------------------------------------- #
def build_sidecar(
    sidecar_path: Path,
    scratch: Path,
    patch_paths: list[Path],
    stack_id: str,
    *,
    build_command: str,
    build_cwd: str,
    build_log: Path | None,
    build_skipped: bool,
    console_exe: Path,
) -> dict:
    """Assemble a provenance sidecar aligned with
    ci/grx009_segment4f_godot_source_provenance.py's build_sidecar output, with
    the build command / log / exe fingerprint added."""
    stack_names = tuple(p.name for p in patch_paths)
    # Independent expected chain, rebuilt from external/godot-master in a
    # throwaway repo. Its final tree MUST equal the scratch tree's HEAD tree.
    expected = generate_expected_stack(EXTERNAL_GODOT, PATCHES_DIR, stack_names)

    actual_clean, actual_status = status_lines(scratch)
    actual_head = rev_parse(scratch, "HEAD")
    actual_tree = rev_parse(scratch, "HEAD^{tree}")
    final_tree = expected.get("final_tree") if isinstance(expected, dict) else None
    tracked_only = (
        isinstance(expected, dict)
        and expected.get("ok") is True
        and expected.get("final_status_clean") is True
        and actual_clean
        and isinstance(actual_tree, str)
        and actual_tree == final_tree
    )

    exe_exists = console_exe.is_file()
    exe_doc = {
        "path": str(console_exe),
        "path_rel_to_source_root": str(CONSOLE_EXE_REL).replace("\\", "/"),
        "exists": exe_exists,
        "sha256": sha256_file(console_exe) if exe_exists else None,
        "size_bytes": console_exe.stat().st_size if exe_exists else None,
        "mtime_utc": format_mtime_utc(console_exe.stat().st_mtime) if exe_exists else None,
        "note": (
            "Scratch Godot build binaries are NOT committed; only this "
            "fingerprint is recorded so the downstream evidence stays auditable."
        ),
    }

    doc: dict = {
        "schema_version": 1,
        "generated_by": "ci/godot_rurix_scratch_build.py",
        "generated_at": now_iso(),
        "base_snapshot": rel(EXTERNAL_GODOT),
        "patches_dir": rel(PATCHES_DIR),
        "stack": stack_id,
        "patch_count": len(stack_names),
        "source_root_at_generation": str(scratch),
        "actual_source_root_at_generation": str(scratch),
        "actual_head": actual_head,
        "actual_tree": actual_tree,
        "actual_status_clean": actual_clean,
        "actual_status": actual_status,
        "tracked_patch_stack_only": tracked_only,
        "applied_patch_stack": {
            "patches_dir": rel(PATCHES_DIR),
            "stack": stack_id,
            "patches": patch_entries(PATCHES_DIR, stack_names),
        },
        "build": {
            "command": build_command,
            "cwd": build_cwd,
            "log_path": str(build_log) if build_log is not None else None,
            "skipped": build_skipped,
            "num_jobs": 1,
            "ice_workaround": "MSVC C1001 num_jobs=1",
        },
        "console_exe": exe_doc,
    }
    if isinstance(expected, dict):
        doc.update({k: v for k, v in expected.items() if k != "ok"})
        doc["expected_stack_ok"] = expected.get("ok") is True
    write_json(sidecar_path, doc)
    return doc


# --------------------------------------------------------------------------- #
# SCons
# --------------------------------------------------------------------------- #
def scons_command_string() -> str:
    launcher, _ = discover_scons_launcher()
    launcher = launcher or ["scons"]
    return render_command(launcher + SCONS_BASE_ARGS + SCONS_ICE_ARGS)


def run_scons_build(scratch: Path, build_log: Path) -> tuple[bool, str, str | None]:
    """Run the SCons build (num_jobs=1 ICE workaround) in the scratch tree."""
    launcher, launcher_src = discover_scons_launcher()
    if launcher is None:
        return False, "", "no usable SCons launcher was found"
    toolchain = discover_vs_toolchain()
    if not toolchain:
        return False, "", "no usable Visual Studio toolchain was found"
    try:
        env = capture_vcvars_environment(toolchain["vcvarsall_bat"])
    except RuntimeError as exc:
        return False, "", f"vcvarsall.bat setup failed: {exc}"

    args = launcher + SCONS_BASE_ARGS + SCONS_ICE_ARGS
    command = render_command(args)
    info(f"scons launcher: {launcher_src}")
    info(f"building (this can take hours at num_jobs=1): {command}")
    proc = subprocess.run(
        args, cwd=scratch, env=env, text=True, capture_output=True, check=False
    )
    output = "\n".join(part for part in (proc.stdout, proc.stderr) if part).strip()
    build_log.parent.mkdir(parents=True, exist_ok=True)
    header = (
        f"# GRX Godot scratch SCons build log\n"
        f"# command: {command}\n"
        f"# cwd: {scratch}\n"
        f"# scons_source: {launcher_src}\n"
        f"# exit_code: {proc.returncode}\n\n"
    )
    build_log.write_text(header + output + "\n", encoding="utf-8", newline="\n")
    if proc.returncode != 0:
        return False, command, f"scons exited with code {proc.returncode} (see {build_log})"
    return True, command, None


# --------------------------------------------------------------------------- #
# enablement env lines
# --------------------------------------------------------------------------- #
def print_enablement_lines(
    scratch: Path,
    sidecar_path: Path,
    console_exe: Path,
    build_command: str,
    build_log: Path | None,
    stack_id: str,
) -> None:
    values = {
        "_GODOT_EXE": str(console_exe),
        "_GODOT_SOURCE": str(scratch),
        "_GODOT_SOURCE_PROVENANCE": str(sidecar_path),
        "_GODOT_BUILD_COMMAND": build_command,
        "_GODOT_BUILD_LOG": str(build_log) if build_log is not None else "",
    }
    print("")
    print(f"{TAG} enablement-smoke environment (PowerShell) for this scratch build:")
    for group in ENABLEMENT_GROUPS:
        match = group["expected_stack"] == stack_id
        marker = "MATCH" if match else f"stack mismatch (needs {group['expected_stack']})"
        print("")
        print(f"# {group['label']}")
        print(f"#   smoke: {group['script']}")
        print(f"#   stack: this scratch is {stack_id} -> {marker}")
        if not match:
            print(
                f"#   rebuild with --patches {group['expected_stack'].replace('..', '-')} "
                "to drive this gate"
            )
        for suffix in ENV_SUFFIXES:
            print(f'$env:{group["prefix"]}{suffix} = "{values[suffix]}"')


# --------------------------------------------------------------------------- #
# check-only
# --------------------------------------------------------------------------- #
def run_check_only(patch_paths: list[Path]) -> int:
    names = [p.name for p in patch_paths]
    ordinals = [n[:4] for n in names]
    info(f"--check-only patch stack applyability: {', '.join(ordinals)}")
    ok = True

    if len(patch_paths) >= 3 and ordinals[:3] == ["0001", "0002", "0003"]:
        stack = evaluate_patch_stack(
            ROOT, EXTERNAL_GODOT, patch_paths[0], patch_paths[1], patch_paths[2]
        )
        info(f"stack_0001_0002_0003 state={stack.get('state')} ok={stack.get('ok')}")
        if not stack.get("ok"):
            info(f"  reason: {stack.get('reason')}")
            ok = False
        if len(patch_paths) >= 4:
            r4 = evaluate_followup_patch_applyability(
                ROOT, EXTERNAL_GODOT, patch_paths[3], ordinals[3]
            )
            info(f"patch_{ordinals[3]}_forward_applyable={r4.get('ok') is True}")
            if r4.get("ok") is not True:
                info(f"  reason: {r4.get('reason')}")
                ok = False
        for i in range(4, len(patch_paths)):
            prereqs = patch_paths[3:i]
            result = evaluate_stacked_patch_applyability(
                ROOT, EXTERNAL_GODOT, prereqs, patch_paths[i], ordinals[i]
            )
            info(f"patch_{ordinals[i]}_stacked_applyable={result.get('ok') is True}")
            if result.get("ok") is not True:
                info(f"  reason: {result.get('reason')}")
                ok = False
    else:
        # A subset that does not start at the tracked 0001..0003 base prefix:
        # prove it assembles by rebuilding the whole expected chain in a
        # throwaway repo (no persistent copy).
        info(
            "requested stack does not start at the 0001..0003 base prefix; "
            "verifying via a throwaway expected-chain assembly"
        )
        expected = generate_expected_stack(
            EXTERNAL_GODOT, PATCHES_DIR, tuple(names)
        )
        assembled = (
            isinstance(expected, dict)
            and expected.get("ok") is True
            and expected.get("final_status_clean") is True
        )
        info(f"expected_chain_assembled={assembled}")
        if not assembled:
            info(f"  error: {expected.get('error') if isinstance(expected, dict) else expected}")
            ok = False

    print(f"{TAG} check-only " + ("PASS" if ok else "FAIL"))
    return 0 if ok else 1


# --------------------------------------------------------------------------- #
# main
# --------------------------------------------------------------------------- #
def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="godot_rurix_scratch_build.py",
        description="Automate the GRX Godot scratch build (copy + apply + SCons).",
    )
    parser.add_argument(
        "--source",
        default=str(EXTERNAL_GODOT),
        help="base Godot snapshot to copy (default: external/godot-master; never written to)",
    )
    parser.add_argument(
        "--dest",
        default=None,
        help="scratch source root (default: target/grx/godot-scratch-<stack-id>)",
    )
    parser.add_argument(
        "--patches",
        default="0001-0013",
        help="patch stack: a range (0001-0013), an ordinal list (0001,0004), or filenames",
    )
    parser.add_argument(
        "--from-scratch-tree",
        default=None,
        help="reuse an existing scratch tree's SCons cache for a faster incremental rebuild",
    )
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help="prepare the source tree and sidecar only; do not run SCons",
    )
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="only verify patch-stack applyability (no copy, no build); reuses evaluate_patch_stack",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)

    if not EXTERNAL_GODOT.is_dir():
        info(f"FAIL base snapshot missing: {rel(EXTERNAL_GODOT)}")
        return 2
    if not PATCHES_DIR.is_dir():
        info(f"FAIL patches dir missing: {rel(PATCHES_DIR)}")
        return 2

    patch_paths, errors = resolve_patches(args.patches)
    if errors:
        for e in errors:
            info(f"FAIL --patches: {e}")
        return 2
    if not patch_paths:
        info("FAIL --patches resolved to zero patches")
        return 2
    stack_id = stack_id_for(patch_paths)
    info(f"resolved stack {stack_id}: {', '.join(p.name for p in patch_paths)}")

    if args.check_only:
        return run_check_only(patch_paths)

    source = Path(args.source)
    if source.resolve() != EXTERNAL_GODOT.resolve():
        info(f"note: --source override {source} (base snapshot copies are read-only)")

    dest = (
        Path(args.dest)
        if args.dest
        else DEFAULT_DEST_PARENT / f"godot-scratch-{stack_id}"
    )
    dest = dest.resolve()
    dest.parent.mkdir(parents=True, exist_ok=True)
    sidecar_path = dest.parent / f"{dest.name}.source_provenance.json"
    build_log = dest.parent / f"{dest.name}.scons_build.log"
    status_path = dest.parent / f"{dest.name}.build_status.json"
    console_exe = dest / CONSOLE_EXE_REL
    from_tree = Path(args.from_scratch_tree).resolve() if args.from_scratch_tree else None

    def emit_status(status: str, reason: str | None, extra: dict | None = None) -> None:
        doc = {
            "schema_version": 1,
            "subject": "godot_rurix_scratch_build",
            "status": status,
            "timestamp": now_iso(),
            "stack": stack_id,
            "patches": [p.name for p in patch_paths],
            "source_root": str(dest),
            "sidecar_path": str(sidecar_path),
            "build_log": str(build_log),
            "skip_build": bool(args.skip_build),
            "console_exe": str(console_exe),
        }
        if reason is not None:
            doc["reason"] = reason
        if extra:
            doc.update(extra)
        write_json(status_path, doc)
        info(f"wrote status {rel_or_abs(status_path)} status={status}")

    # 1) populate the scratch copy
    ok, mode, copy_err = prepare_scratch_copy(dest, from_tree)
    if not ok:
        emit_status("fail", copy_err)
        info(f"FAIL {copy_err}")
        return 1
    info(f"scratch copy ready (mode={mode}) at {dest}")

    # 2) build the git patch layer (normalise to base + apply the stack)
    ok, chain, tree_err = build_scratch_tree(dest, patch_paths)
    if not ok:
        emit_status("fail", tree_err, extra={"chain": chain})
        info(f"FAIL {tree_err}")
        return 1
    info(
        f"applied {len(patch_paths)} patch(es); final_tree={chain.get('final_tree')} "
        f"clean={chain.get('final_status_clean')}"
    )
    if not chain.get("final_status_clean"):
        emit_status(
            "fail",
            "scratch tree is not clean after applying the patch stack",
            extra={"final_status": chain.get("final_status")},
        )
        info("FAIL scratch tree not clean after apply")
        return 1

    # 3) SCons build (unless skipped)
    build_command = scons_command_string()
    build_ran = False
    if args.skip_build:
        info("--skip-build: source tree prepared; SCons not run")
        build_log_for_sidecar = None
    else:
        built, build_command, build_err = run_scons_build(dest, build_log)
        build_ran = True
        build_log_for_sidecar = build_log
        if not built:
            build_sidecar(
                sidecar_path,
                dest,
                patch_paths,
                stack_id,
                build_command=build_command,
                build_cwd=str(dest),
                build_log=build_log,
                build_skipped=False,
                console_exe=console_exe,
            )
            emit_status("fail", build_err)
            info(f"FAIL {build_err}")
            return 1

    # 4) console exe detection (required after a real build)
    if build_ran and not console_exe.is_file():
        build_sidecar(
            sidecar_path,
            dest,
            patch_paths,
            stack_id,
            build_command=build_command,
            build_cwd=str(dest),
            build_log=build_log_for_sidecar,
            build_skipped=args.skip_build,
            console_exe=console_exe,
        )
        emit_status("fail", f"console exe missing after build: {console_exe}")
        info(f"FAIL console exe missing after build: {console_exe}")
        return 1

    # 5) provenance sidecar
    sidecar = build_sidecar(
        sidecar_path,
        dest,
        patch_paths,
        stack_id,
        build_command=build_command,
        build_cwd=str(dest),
        build_log=build_log_for_sidecar,
        build_skipped=args.skip_build,
        console_exe=console_exe,
    )
    info(
        f"wrote sidecar {rel_or_abs(sidecar_path)} "
        f"tracked_patch_stack_only={sidecar.get('tracked_patch_stack_only')}"
    )
    if sidecar.get("tracked_patch_stack_only") is not True:
        emit_status(
            "fail",
            "sidecar tracked_patch_stack_only is not true (scratch tree does not "
            "match base + the tracked patch stack)",
            extra={
                "expected_final_tree": sidecar.get("final_tree"),
                "actual_tree": sidecar.get("actual_tree"),
            },
        )
        info("FAIL scratch tree does not match the expected base + patch stack")
        return 1

    # 6) success — emit status and the enablement env lines
    status = "success" if build_ran else "prepared"
    emit_status(
        status,
        None,
        extra={
            "copy_mode": mode,
            "tracked_patch_stack_only": True,
            "console_exe_present": console_exe.is_file(),
            "final_head": sidecar.get("final_head"),
            "final_tree": sidecar.get("final_tree"),
        },
    )
    print_enablement_lines(
        dest, sidecar_path, console_exe, build_command, build_log_for_sidecar, stack_id
    )
    print(f"{TAG} {status.upper()} (stack={stack_id}, source_root={dest})")
    return 0


def rel_or_abs(path: Path) -> str:
    try:
        return rel(path)
    except Exception:  # noqa: BLE001
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
