#!/usr/bin/env python3
"""Shared patch-stack checks for the ignored Godot snapshot."""

from __future__ import annotations

import pathlib
import shutil
import subprocess
import tempfile


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


def evaluate_followup_patch_applyability(
    root: pathlib.Path,
    external_godot: pathlib.Path,
    patch: pathlib.Path,
    patch_id: str,
) -> dict[str, object]:
    forward = patch_check(root, external_godot, patch)
    details: dict[str, object] = {
        f"{patch_id}_forward_exit_code": forward.returncode,
        f"{patch_id}_forward_stderr": _trim(forward.stderr),
    }
    if forward.returncode == 0:
        return {
            "ok": True,
            "ready": True,
            "reason": f"patch {patch_id} is forward-applicable",
            "details": details,
        }
    return {
        "ok": False,
        "ready": False,
        "reason": f"patch {patch_id} is not forward-applicable",
        "details": details,
    }


def patch_touched_paths(patch: pathlib.Path) -> list[str]:
    """Return the b-side relative paths named by ``diff --git`` lines."""
    paths: list[str] = []
    for line in patch.read_text(encoding="utf-8").splitlines():
        if not line.startswith("diff --git a/"):
            continue
        marker = " b/"
        index = line.rfind(marker)
        if index < 0:
            continue
        candidate = line[index + len(marker) :].strip()
        if candidate and candidate not in paths:
            paths.append(candidate)
    return paths


def evaluate_stacked_patch_applyability(
    root: pathlib.Path,
    external_godot: pathlib.Path,
    prereq_patches: list[pathlib.Path],
    patch: pathlib.Path,
    patch_id: str,
) -> dict[str, object]:
    """Check that ``patch`` applies after ``prereq_patches`` without touching
    the ignored Godot snapshot.

    The snapshot working tree is read-only for this check: every file the
    patches touch is copied into a temporary scratch git repository, the
    prerequisite patches are applied for real inside the scratch copy, and
    only then is the candidate patch verified with ``git apply --check``.
    """
    details: dict[str, object] = {}
    touched: list[str] = []
    for candidate in [*prereq_patches, patch]:
        if not candidate.is_file():
            details[f"{patch_id}_missing_patch_file"] = str(candidate)
            return {
                "ok": False,
                "ready": False,
                "reason": f"patch {patch_id} stacked check is missing a patch file",
                "details": details,
            }
        for rel_path in patch_touched_paths(candidate):
            if rel_path not in touched:
                touched.append(rel_path)
    details[f"{patch_id}_stack_touched_paths"] = touched

    with tempfile.TemporaryDirectory() as tmp:
        scratch = pathlib.Path(tmp) / "godot-patch-stack"
        scratch.mkdir()
        init = run_capture(
            scratch, ["git", "-c", "core.autocrlf=false", "init", "--quiet", "."]
        )
        details[f"{patch_id}_scratch_init_exit_code"] = init.returncode
        if init.returncode != 0:
            details[f"{patch_id}_scratch_init_stderr"] = _trim(init.stderr)
            return {
                "ok": False,
                "ready": False,
                "reason": f"patch {patch_id} stacked check could not init a scratch repo",
                "details": details,
            }
        for rel_path in touched:
            source = external_godot / rel_path
            if not source.is_file():
                # New files created by an earlier patch in the stack are fine;
                # the prerequisite apply will create them in the scratch copy.
                continue
            target = scratch / rel_path
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        for index, prereq in enumerate(prereq_patches):
            applied = run_capture(
                scratch,
                ["git", "-c", "core.autocrlf=false", "apply", str(prereq.resolve())],
            )
            details[f"{patch_id}_prereq_{index}_apply_exit_code"] = applied.returncode
            if applied.returncode != 0:
                details[f"{patch_id}_prereq_{index}_apply_stderr"] = _trim(applied.stderr)
                details[f"{patch_id}_prereq_{index}_patch"] = prereq.name
                return {
                    "ok": False,
                    "ready": False,
                    "reason": (
                        f"patch {patch_id} stacked check could not apply prerequisite "
                        f"{prereq.name} to the scratch copy"
                    ),
                    "details": details,
                }
        check = run_capture(
            scratch,
            ["git", "-c", "core.autocrlf=false", "apply", "--check", str(patch.resolve())],
        )
        details[f"{patch_id}_stacked_check_exit_code"] = check.returncode
        details[f"{patch_id}_stacked_check_stderr"] = _trim(check.stderr)
    if check.returncode == 0:
        return {
            "ok": True,
            "ready": True,
            "reason": f"patch {patch_id} applies on top of its prerequisite stack",
            "details": details,
        }
    return {
        "ok": False,
        "ready": False,
        "reason": f"patch {patch_id} does not apply on top of its prerequisite stack",
        "details": details,
    }


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


def main() -> int:
    """Standalone GRX-009 patch-stack report.

    Prints the shared 0001/0002/0003 stack state, the 0004 forward
    applyability against the ignored snapshot, and the 0005, 0006, 0007,
    0008, and 0009 stacked applyability (verified in a temporary scratch
    copy; the snapshot working tree is never modified). Exits non-zero when
    any check is not ok.
    """
    root = pathlib.Path(__file__).resolve().parents[1]
    external_godot = root / "external" / "godot-master"
    patches_dir = root / "spike" / "godot-rurix" / "patches"
    patch1 = patches_dir / "0001-rurix-accel-module-scaffold.patch"
    patch2 = patches_dir / "0002-rurix-accel-luminance-pass-gate.patch"
    patch3 = patches_dir / "0003-rurix-accel-luminance-core-callsite-wiring.patch"
    patch4 = patches_dir / "0004-rurix-accel-luminance-resource-mapping-scaffold.patch"
    patch5 = (
        patches_dir / "0005-rurix-accel-luminance-runtime-binding-preflight.patch"
    )
    patch6 = (
        patches_dir / "0006-rurix-accel-luminance-gated-dispatch-bringup.patch"
    )
    patch7 = (
        patches_dir
        / "0007-rurix-accel-luminance-native-resource-handle-mapping.patch"
    )
    patch8 = (
        patches_dir
        / "0008-rurix-accel-luminance-godot-runtime-bridge-recording-smoke.patch"
    )
    patch9 = patches_dir / "0009-rurix-accel-luminance-real-pass-optin.patch"
    patch10 = (
        patches_dir / "0010-rurix-accel-luminance-real-pass-result-writeback.patch"
    )
    patch11 = (
        patches_dir / "0011-rurix-accel-tonemap-pass-gate-and-callsite.patch"
    )
    patch12 = (
        patches_dir / "0012-rurix-accel-tonemap-runtime-resource-binding.patch"
    )
    patch13 = (
        patches_dir
        / "0013-rurix-accel-tonemap-recording-smoke-and-real-pass-optin.patch"
    )

    ok = True
    stack = evaluate_patch_stack(root, external_godot, patch1, patch2, patch3)
    print(f"[godot-patch-stack] stack_state: {stack.get('state')}")
    print(f"[godot-patch-stack] stack_reason: {stack.get('reason')}")
    if stack.get("ok") is not True:
        print(f"[godot-patch-stack] stack_details: {stack.get('details')}")
        ok = False

    patch4_result = evaluate_followup_patch_applyability(
        root, external_godot, patch4, "0004"
    )
    print(
        "[godot-patch-stack] patch_0004_forward_applyable: "
        + ("true" if patch4_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0004_reason: {patch4_result.get('reason')}")
    if patch4_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0004_details: {patch4_result.get('details')}")
        ok = False

    patch5_result = evaluate_stacked_patch_applyability(
        root, external_godot, [patch4], patch5, "0005"
    )
    print(
        "[godot-patch-stack] patch_0005_stacked_applyable: "
        + ("true" if patch5_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0005_reason: {patch5_result.get('reason')}")
    if patch5_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0005_details: {patch5_result.get('details')}")
        ok = False

    patch6_result = evaluate_stacked_patch_applyability(
        root, external_godot, [patch4, patch5], patch6, "0006"
    )
    print(
        "[godot-patch-stack] patch_0006_stacked_applyable: "
        + ("true" if patch6_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0006_reason: {patch6_result.get('reason')}")
    if patch6_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0006_details: {patch6_result.get('details')}")
        ok = False

    patch7_result = evaluate_stacked_patch_applyability(
        root, external_godot, [patch4, patch5, patch6], patch7, "0007"
    )
    print(
        "[godot-patch-stack] patch_0007_stacked_applyable: "
        + ("true" if patch7_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0007_reason: {patch7_result.get('reason')}")
    if patch7_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0007_details: {patch7_result.get('details')}")
        ok = False

    patch8_result = evaluate_stacked_patch_applyability(
        root, external_godot, [patch4, patch5, patch6, patch7], patch8, "0008"
    )
    print(
        "[godot-patch-stack] patch_0008_stacked_applyable: "
        + ("true" if patch8_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0008_reason: {patch8_result.get('reason')}")
    if patch8_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0008_details: {patch8_result.get('details')}")
        ok = False

    patch9_result = evaluate_stacked_patch_applyability(
        root, external_godot, [patch4, patch5, patch6, patch7, patch8], patch9, "0009"
    )
    print(
        "[godot-patch-stack] patch_0009_stacked_applyable: "
        + ("true" if patch9_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0009_reason: {patch9_result.get('reason')}")
    if patch9_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0009_details: {patch9_result.get('details')}")
        ok = False

    patch10_result = evaluate_stacked_patch_applyability(
        root,
        external_godot,
        [patch4, patch5, patch6, patch7, patch8, patch9],
        patch10,
        "0010",
    )
    print(
        "[godot-patch-stack] patch_0010_stacked_applyable: "
        + ("true" if patch10_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0010_reason: {patch10_result.get('reason')}")
    if patch10_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0010_details: {patch10_result.get('details')}")
        ok = False

    patch11_result = evaluate_stacked_patch_applyability(
        root,
        external_godot,
        [patch4, patch5, patch6, patch7, patch8, patch9, patch10],
        patch11,
        "0011",
    )
    print(
        "[godot-patch-stack] patch_0011_stacked_applyable: "
        + ("true" if patch11_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0011_reason: {patch11_result.get('reason')}")
    if patch11_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0011_details: {patch11_result.get('details')}")
        ok = False

    patch12_result = evaluate_stacked_patch_applyability(
        root,
        external_godot,
        [patch4, patch5, patch6, patch7, patch8, patch9, patch10, patch11],
        patch12,
        "0012",
    )
    print(
        "[godot-patch-stack] patch_0012_stacked_applyable: "
        + ("true" if patch12_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0012_reason: {patch12_result.get('reason')}")
    if patch12_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0012_details: {patch12_result.get('details')}")
        ok = False

    patch13_result = evaluate_stacked_patch_applyability(
        root,
        external_godot,
        [
            patch4,
            patch5,
            patch6,
            patch7,
            patch8,
            patch9,
            patch10,
            patch11,
            patch12,
        ],
        patch13,
        "0013",
    )
    print(
        "[godot-patch-stack] patch_0013_stacked_applyable: "
        + ("true" if patch13_result.get("ok") is True else "false")
    )
    print(f"[godot-patch-stack] patch_0013_reason: {patch13_result.get('reason')}")
    if patch13_result.get("ok") is not True:
        print(f"[godot-patch-stack] patch_0013_details: {patch13_result.get('details')}")
        ok = False

    print("[godot-patch-stack] " + ("PASS" if ok else "FAIL"))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
