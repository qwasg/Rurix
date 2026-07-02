#!/usr/bin/env python3
"""Shared patch-stack checks for the ignored Godot snapshot."""

from __future__ import annotations

import pathlib
import subprocess


def run_capture(root: pathlib.Path, cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=root,
        text=True,
        capture_output=True,
        check=False,
    )


def patch_check(
    root: pathlib.Path,
    external_godot: pathlib.Path,
    patch: pathlib.Path,
    *,
    reverse: bool = False,
) -> subprocess.CompletedProcess[str]:
    try:
        directory = external_godot.relative_to(root).as_posix()
    except ValueError:
        directory = str(external_godot)
    cmd = ["git", "apply"]
    if reverse:
        cmd.append("--reverse")
    cmd.extend(["--check", f"--directory={directory}", str(patch)])
    return run_capture(root, cmd)


def _trim(value: str | None) -> str:
    return (value or "").strip()


def evaluate_patch_stack(
    root: pathlib.Path,
    external_godot: pathlib.Path,
    patch1: pathlib.Path,
    patch2: pathlib.Path,
    patch3: pathlib.Path,
) -> dict[str, object]:
    """Evaluate the legal 0001/0002/0003 patch stack states.

    Legal states:
    - base (nothing applied)
    - 0001-only
    - 0001+0002
    - 0001+0002+0003
    """

    forward1 = patch_check(root, external_godot, patch1)
    details: dict[str, object] = {
        "0001_forward_exit_code": forward1.returncode,
        "0001_forward_stderr": _trim(forward1.stderr),
    }
    if forward1.returncode == 0:
        return {
            "ok": True,
            "state": "base",
            "ready": False,
            "reason": "patch 0001 is forward-applicable; ignored Godot snapshot is at base state",
            "details": details,
        }

    reverse3 = patch_check(root, external_godot, patch3, reverse=True)
    details["0003_reverse_exit_code"] = reverse3.returncode
    details["0003_reverse_stderr"] = _trim(reverse3.stderr)
    if reverse3.returncode == 0:
        return {
            "ok": True,
            "state": "0001+0002+0003",
            "ready": True,
            "reason": "all three tracked Godot patches are applied in stack order",
            "details": details,
        }

    reverse2 = patch_check(root, external_godot, patch2, reverse=True)
    details["0002_reverse_exit_code"] = reverse2.returncode
    details["0002_reverse_stderr"] = _trim(reverse2.stderr)
    if reverse2.returncode == 0:
        forward3 = patch_check(root, external_godot, patch3)
        details["0003_forward_exit_code"] = forward3.returncode
        details["0003_forward_stderr"] = _trim(forward3.stderr)
        if forward3.returncode == 0:
            return {
                "ok": True,
                "state": "0001+0002",
                "ready": False,
                "reason": "patches 0001 and 0002 are applied; patch 0003 is not applied yet",
                "details": details,
            }
        return {
            "ok": False,
            "state": "drift",
            "ready": False,
            "reason": (
                "patch 0003 drift detected; 0001+0002 is applied but 0003 neither "
                "forward- nor reverse-applies"
            ),
            "details": details,
        }

    reverse1 = patch_check(root, external_godot, patch1, reverse=True)
    details["0001_reverse_exit_code"] = reverse1.returncode
    details["0001_reverse_stderr"] = _trim(reverse1.stderr)
    if reverse1.returncode == 0:
        forward2 = patch_check(root, external_godot, patch2)
        details["0002_forward_exit_code"] = forward2.returncode
        details["0002_forward_stderr"] = _trim(forward2.stderr)
        if forward2.returncode == 0:
            return {
                "ok": True,
                "state": "0001-only",
                "ready": False,
                "reason": "patch 0001 is applied; patch 0002 is not applied yet",
                "details": details,
            }
        return {
            "ok": False,
            "state": "drift",
            "ready": False,
            "reason": (
                "patch 0002 drift detected; 0001 is applied but 0002 neither "
                "forward- nor reverse-applies"
            ),
            "details": details,
        }

    return {
        "ok": False,
        "state": "drift",
        "ready": False,
        "reason": (
            "patch drift detected; the tree matches neither base, 0001-only, "
            "0001+0002, nor 0001+0002+0003"
        ),
        "details": details,
    }
