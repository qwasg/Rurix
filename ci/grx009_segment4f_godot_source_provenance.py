#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_BASE_SNAPSHOT = ROOT / "external" / "godot-master"
DEFAULT_PATCHES_DIR = ROOT / "spike" / "godot-rurix" / "patches"
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
# Supported patch stacks: the segment 4f 0001..0008 stack (default), the
# segment 4h 0001..0009 stack (adds the real-pass opt-in patch), and the
# stage A5 0001..0010 stack (adds the real-pass result writeback scaffold).
# The sidecar audit chain is rebuilt in a temp repo either way, so the
# tree-equality proof works identically for all stacks.
SUPPORTED_STACKS = {
    "0001..0008": PATCH_STACK,
    "0001..0009": (
        *PATCH_STACK,
        "0009-rurix-accel-luminance-real-pass-optin.patch",
    ),
    "0001..0010": (
        *PATCH_STACK,
        "0009-rurix-accel-luminance-real-pass-optin.patch",
        "0010-rurix-accel-luminance-real-pass-result-writeback.patch",
    ),
    # GRX-010 tonemap runtime stack: the full luminance stack plus the
    # tonemap gate/call-site (0011), the tonemap runtime resource binding
    # (0012), and the tonemap recording-smoke / real-pass opt-in (0013).
    "0001..0013": (
        *PATCH_STACK,
        "0009-rurix-accel-luminance-real-pass-optin.patch",
        "0010-rurix-accel-luminance-real-pass-result-writeback.patch",
        "0011-rurix-accel-tonemap-pass-gate-and-callsite.patch",
        "0012-rurix-accel-tonemap-runtime-resource-binding.patch",
        "0013-rurix-accel-tonemap-recording-smoke-and-real-pass-optin.patch",
    ),
}
IGNORED_COPY_NAMES = {
    ".git",
    ".sconsign.dblite",
    ".vs",
    "bin",
    "__pycache__",
}


def run_git(args: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", *args], cwd=cwd, text=True, capture_output=True)


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


def status_lines(path: Path) -> tuple[bool, list[str]]:
    status = run_git(["status", "--porcelain", "--untracked-files=all"], path)
    if status.returncode != 0:
        lines = (status.stdout + status.stderr).strip().splitlines()
        return False, lines or [f"git status failed with exit code {status.returncode}"]
    lines = [line for line in status.stdout.splitlines() if line.strip()]
    return len(lines) == 0, lines


def rev_parse(path: Path, spec: str) -> str | None:
    result = run_git(["rev-parse", spec], path)
    value = result.stdout.strip()
    if result.returncode == 0 and value:
        return value
    return None


def rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT)).replace("\\", "/")
    except ValueError:
        return str(path).replace("\\", "/")


def patch_entries(patches_dir: Path, stack: tuple[str, ...] = PATCH_STACK) -> list[dict[str, object]]:
    entries: list[dict[str, object]] = []
    for index, name in enumerate(stack, start=1):
        patch = patches_dir / name
        entries.append(
            {
                "order": index,
                "patch": name,
                "path": rel(patch),
                "sha256": sha256_file(patch),
                "size_bytes": patch.stat().st_size if patch.is_file() else None,
            }
        )
    return entries


def copy_base_snapshot(base_snapshot: Path, scratch: Path) -> None:
    def ignore(directory: str, names: list[str]) -> set[str]:
        ignored = {name for name in names if name in IGNORED_COPY_NAMES}
        relative_dir = Path(directory).resolve().relative_to(base_snapshot.resolve())
        if relative_dir == Path(".") and "cache" in names:
            ignored.add("cache")
        return ignored

    shutil.copytree(base_snapshot, scratch, ignore=ignore)


def commit_all(repo: Path, message: str) -> tuple[str | None, str | None, str | None]:
    add = run_git(["add", "-A"], repo)
    if add.returncode != 0:
        return None, None, add.stderr.strip() or add.stdout.strip()
    commit = run_git(
        [
            "-c",
            "user.name=GRX009 Provenance",
            "-c",
            "user.email=grx009-provenance@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            message,
        ],
        repo,
    )
    if commit.returncode != 0:
        return None, None, commit.stderr.strip() or commit.stdout.strip()
    return rev_parse(repo, "HEAD"), rev_parse(repo, "HEAD^{tree}"), None


def normalize_snapshot_to_base(repo: Path, patches_dir: Path) -> tuple[bool, list[str]]:
    reversed_patches: list[str] = []
    for name in reversed(PATCH_STACK[:3]):
        patch = patches_dir / name
        check = run_git(["-c", "core.autocrlf=false", "apply", "--reverse", "--check", str(patch.resolve())], repo)
        if check.returncode != 0:
            continue
        applied = run_git(["-c", "core.autocrlf=false", "apply", "--reverse", str(patch.resolve())], repo)
        if applied.returncode != 0:
            return False, [applied.stderr.strip() or applied.stdout.strip()]
        reversed_patches.append(name)
    return True, reversed_patches


def generate_expected_stack(
    base_snapshot: Path, patches_dir: Path, stack: tuple[str, ...] = PATCH_STACK
) -> dict[str, object]:
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp) / "godot-source-provenance"
        copy_base_snapshot(base_snapshot, repo)
        init = run_git(["-c", "core.autocrlf=false", "init", "--quiet", "."], repo)
        if init.returncode != 0:
            return {"ok": False, "error": init.stderr.strip() or init.stdout.strip()}
        run_git(["config", "core.autocrlf", "false"], repo)
        normalized, reversed_patches = normalize_snapshot_to_base(repo, patches_dir)
        if not normalized:
            return {"ok": False, "error": "could not normalize base snapshot", "normalization_errors": reversed_patches}
        base_head, base_tree, base_error = commit_all(repo, "grx009 segment4f base snapshot")
        if base_error is not None:
            return {"ok": False, "error": base_error}
        patch_audit: list[dict[str, object]] = []
        entries = patch_entries(patches_dir, stack)
        for entry in entries:
            patch = patches_dir / str(entry["patch"])
            applied = run_git(["-c", "core.autocrlf=false", "apply", str(patch.resolve())], repo)
            if applied.returncode != 0:
                return {
                    "ok": False,
                    "error": applied.stderr.strip() or applied.stdout.strip(),
                    "failed_patch": entry,
                    "base_commit": base_head,
                    "base_tree": base_tree,
                    "patch_application_audit": patch_audit,
                }
            commit, tree, commit_error = commit_all(repo, f"grx009 segment4f apply {entry['patch']}")
            if commit_error is not None:
                return {"ok": False, "error": commit_error, "failed_patch": entry}
            patch_audit.append({**entry, "commit": commit, "tree": tree})
        final_clean, final_status = status_lines(repo)
        return {
            "ok": True,
            "base_commit": base_head,
            "base_tree": base_tree,
            "base_snapshot_reversed_patches": reversed_patches,
            "patch_application_audit": patch_audit,
            "final_head": rev_parse(repo, "HEAD"),
            "final_tree": rev_parse(repo, "HEAD^{tree}"),
            "final_status_clean": final_clean,
            "final_status": final_status,
        }


def build_sidecar(
    source_root: Path,
    output: Path,
    base_snapshot: Path,
    patches_dir: Path,
    stack_id: str = "0001..0008",
) -> dict[str, object]:
    stack = SUPPORTED_STACKS[stack_id]
    expected = generate_expected_stack(base_snapshot, patches_dir, stack)
    actual_clean, actual_status = status_lines(source_root)
    actual_head = rev_parse(source_root, "HEAD")
    actual_tree = rev_parse(source_root, "HEAD^{tree}")
    final_tree = expected.get("final_tree") if isinstance(expected, dict) else None
    tracked_only = (
        expected.get("ok") is True
        and expected.get("final_status_clean") is True
        and actual_clean
        and isinstance(actual_tree, str)
        and actual_tree == final_tree
    )
    doc: dict[str, object] = {
        "schema_version": 1,
        "generated_by": "ci/grx009_segment4f_godot_source_provenance.py",
        "generated_at": now_iso(),
        "base_snapshot": rel(base_snapshot),
        "patches_dir": rel(patches_dir),
        "stack": stack_id,
        "patch_count": len(stack),
        "source_root_at_generation": str(source_root),
        "actual_source_root_at_generation": str(source_root),
        "actual_head": actual_head,
        "actual_tree": actual_tree,
        "actual_status_clean": actual_clean,
        "actual_status": actual_status,
        "tracked_patch_stack_only": tracked_only,
        "applied_patch_stack": {
            "patches_dir": rel(patches_dir),
            "stack": stack_id,
            "patches": patch_entries(patches_dir, stack),
        },
    }
    if isinstance(expected, dict):
        doc.update({k: v for k, v in expected.items() if k != "ok"})
        doc["expected_stack_ok"] = expected.get("ok") is True
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    return doc


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--base-snapshot", default=str(DEFAULT_BASE_SNAPSHOT))
    parser.add_argument("--patches-dir", default=str(DEFAULT_PATCHES_DIR))
    parser.add_argument("--patch-stack", default="0001..0008")
    args = parser.parse_args()
    if args.patch_stack not in SUPPORTED_STACKS:
        print(
            "only --patch-stack " + " / ".join(sorted(SUPPORTED_STACKS)) + " is supported",
            file=sys.stderr,
        )
        return 2
    source_root = Path(args.source_root)
    base_snapshot = Path(args.base_snapshot)
    patches_dir = Path(args.patches_dir)
    if not source_root.is_dir():
        print(f"source root does not exist: {source_root}", file=sys.stderr)
        return 2
    if not base_snapshot.is_dir():
        print(f"base snapshot does not exist: {base_snapshot}", file=sys.stderr)
        return 2
    if not patches_dir.is_dir():
        print(f"patches dir does not exist: {patches_dir}", file=sys.stderr)
        return 2
    doc = build_sidecar(
        source_root, Path(args.output), base_snapshot, patches_dir, args.patch_stack
    )
    return 0 if doc.get("tracked_patch_stack_only") is True else 1


if __name__ == "__main__":
    sys.exit(main())
