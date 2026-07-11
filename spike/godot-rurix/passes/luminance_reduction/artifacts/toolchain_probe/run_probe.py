#!/usr/bin/env python3
"""GRX009 toolchain probe: re-run minimal .ll cases through patched llc.

Reproduces the accept/reject/crash table for the patched llc at
H:\\llvm-dxil\\build\\bin\\llc.exe (or $RURIX_LLC if set). Writes
probe_results.json next to this script. Safe to run in CI: if llc
isn't found or no case_*.ll files exist, writes a skip JSON and
exits 0.

For the cross-version texture-intrinsic confirmation -- the case_A
(load.texture.2d) and case_H (store.texture.2d) reject cases run through
EVERY llc build on this machine, not just $RURIX_LLC -- see the companion
cross_version_texture_intrinsic_probe.py, which writes
cross_version_probe_results.json and feeds the cross_toolchain_confirmation
section of ../../texture_intrinsic_toolchain_blocker.json.

Stdlib-only. Windows + `py -3` friendly.
"""

from __future__ import annotations

import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

# --- paths ---------------------------------------------------------------

SCRIPT_DIR = Path(__file__).resolve().parent
FALLBACK_LLC = r"H:\llvm-dxil\build\bin\llc.exe"
RESULTS_PATH = SCRIPT_DIR / "probe_results.json"

# llc subprocess timeout (seconds). 60s is generous for tiny .ll cases.
TIMEOUT_SECONDS = 60

# How much of stderr to keep.
STDERR_EXCERPT_LEN = 2048


def locate_llc() -> str | None:
    """Prefer RURIX_LLC env var, else fall back to the known path.

    Returns the llc path if it exists on disk, else None.
    """
    env_path = os.environ.get("RURIX_LLC")
    if env_path:
        env_p = Path(env_path)
        if env_p.exists():
            return str(env_p)
    fb = Path(FALLBACK_LLC)
    if fb.exists():
        return str(fb)
    return None


def get_llc_version(llc_path: str) -> str:
    """Run `llc --version`, return the first 'LLVM version' line.

    Returns an empty string if the version can't be extracted.
    """
    try:
        proc = subprocess.run(
            [llc_path, "--version"],
            capture_output=True,
            text=True,
            timeout=30,
        )
    except (subprocess.SubprocessError, OSError):
        return ""
    out = (proc.stdout or "") + (proc.stderr or "")
    for line in out.splitlines():
        if "LLVM version" in line:
            return line.strip()
    return ""


def parse_expected(ll_path: Path) -> str | None:
    """Read the .ll header comment for the expected verdict.

    Matches any comment line (starts with `;`) that contains the word
    "expected" (case-insensitive), then scans the line for one of the
    tokens ACCEPT, REJECT, CRASH (case-insensitive). This handles both
    `; expected: REJECT` and `; Expected result:        REJECT` styles.
    Returns None if no such header/token is found.
    """
    try:
        text = ll_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith(";"):
            continue
        low = stripped.lower()
        if "expected" not in low:
            continue
        # Scan for the verdict token anywhere on the line.
        for token in ("accept", "reject", "crash"):
            # Match as a whole word (bounded by non-letter boundaries)
            # so e.g. "accept" inside another word doesn't false-match.
            # A simple regex would be cleaner but we keep stdlib-only;
            # splitting on non-alphanumeric and checking membership is enough.
            words = [w for w in low.replace(":", " ").split() if w.isalpha()]
            if token in words:
                return token.upper()
    return None


def run_one_case(llc_path: str, ll_path: Path) -> dict:
    """Run llc on one .ll case, capture results, clean up the .obj."""
    label = ll_path.stem.replace("case_", "")
    obj_path = ll_path.with_suffix(".obj")
    expected = parse_expected(ll_path)

    actual_exit_code: int | None = None
    actual_stderr_excerpt = ""
    obj_produced = False
    run_error = None

    try:
        proc = subprocess.run(
            [llc_path, str(ll_path), "-filetype=obj", "-o", str(obj_path)],
            capture_output=True,
            text=True,
            timeout=TIMEOUT_SECONDS,
        )
        actual_exit_code = proc.returncode
        stderr_full = proc.stderr or ""
        actual_stderr_excerpt = stderr_full[:STDERR_EXCERPT_LEN].strip()
        obj_produced = obj_path.exists()
    except subprocess.TimeoutExpired as e:
        run_error = f"timeout_after_{TIMEOUT_SECONDS}s"
        actual_exit_code = None
        actual_stderr_excerpt = (e.stderr or b"").decode("utf-8", "replace")[:STDERR_EXCERPT_LEN].strip() if isinstance(e.stderr, (bytes, bytearray)) else str(e)[:STDERR_EXCERPT_LEN]
        obj_produced = obj_path.exists()
    except (OSError, subprocess.SubprocessError) as e:
        run_error = f"subprocess_error: {type(e).__name__}: {e}"
        actual_exit_code = None
        actual_stderr_excerpt = ""
        obj_produced = obj_path.exists()

    # Clean up the .obj file regardless of outcome so it doesn't get committed.
    try:
        if obj_path.exists():
            obj_path.unlink()
    except OSError:
        pass

    verdict, note = classify(expected, actual_exit_code, actual_stderr_excerpt, obj_produced, run_error)

    return {
        "label": label,
        "ll": ll_path.name,
        "expected": expected if expected is not None else "",
        "actual_exit_code": actual_exit_code,
        "actual_stderr_excerpt": actual_stderr_excerpt,
        "obj_produced": obj_produced,
        "verdict": verdict,
        "note": note,
    }


def classify(expected, exit_code, stderr_excerpt, obj_produced, run_error) -> tuple[str, str]:
    """Determine verdict per the spec rules. Returns (verdict, note)."""
    if run_error is not None:
        return "unexpected", f"run error: {run_error}"

    if expected is None:
        return "unexpected", "no `; expected:` header found in .ll file"

    stderr_low = (stderr_excerpt or "").lower()

    if expected == "ACCEPT":
        if exit_code == 0 and obj_produced:
            return "accept", ""
        return "unexpected", f"expected accept but exit_code={exit_code}, obj_produced={obj_produced}"

    if expected == "REJECT":
        if exit_code != 0 and "unknown intrinsic" in stderr_low:
            return "reject", ""
        return "unexpected", f"expected reject (exit!=0 and 'unknown intrinsic' in stderr) but exit_code={exit_code}, stderr_has_unknown_intrinsic={'unknown intrinsic' in stderr_low}"

    if expected == "CRASH":
        crash_markers = ("crash", "access violation", "dxcontainer")
        has_marker = any(m in stderr_low for m in crash_markers)
        if exit_code != 0 and has_marker:
            return "crash", ""
        return "unexpected", f"expected crash (exit!=0 and stderr mentions crash/access violation/DXContainer) but exit_code={exit_code}, stderr_has_crash_marker={has_marker}"

    return "unexpected", f"unknown expected value: {expected}"


def write_results(payload: dict) -> None:
    """Write probe_results.json with LF line endings."""
    text = json.dumps(payload, indent=2, ensure_ascii=True)
    RESULTS_PATH.write_text(text, encoding="utf-8", newline="\n")


def main() -> int:
    # --- locate llc -----------------------------------------------------
    llc_path = locate_llc()

    if llc_path is None:
        write_results({
            "status": "skip",
            "skip_reason": "llc_not_found",
            "llc_path": None,
            "cases": [],
        })
        print("[grx009-toolchain-probe] status=skip, skip_reason=llc_not_found")
        print(f"[grx009-toolchain-probe] results: {RESULTS_PATH}")
        return 0

    # --- discover cases -------------------------------------------------
    cases = sorted(SCRIPT_DIR.glob("case_*.ll"))

    if not cases:
        write_results({
            "status": "skip",
            "skip_reason": "no_cases_found",
            "llc_path": llc_path,
            "cases": [],
        })
        print("[grx009-toolchain-probe] status=skip, skip_reason=no_cases_found")
        print(f"[grx009-toolchain-probe] results: {RESULTS_PATH}")
        return 0

    # --- llc version ----------------------------------------------------
    llc_version = get_llc_version(llc_path)

    # --- run each case --------------------------------------------------
    case_results = []
    for ll_path in cases:
        result = run_one_case(llc_path, ll_path)
        case_results.append(result)

    # --- counts ---------------------------------------------------------
    accepts = sum(1 for c in case_results if c["verdict"] == "accept")
    rejects = sum(1 for c in case_results if c["verdict"] == "reject")
    crashes = sum(1 for c in case_results if c["verdict"] == "crash")
    unexpected = sum(1 for c in case_results if c["verdict"] == "unexpected")

    payload = {
        "status": "complete",
        "llc_path": llc_path,
        "llc_version": llc_version,
        "probe_run_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "cases": case_results,
    }
    write_results(payload)

    print(f"[grx009-toolchain-probe] status=complete, cases={len(case_results)}, accepts={accepts}, rejects={rejects}, crashes={crashes}, unexpected={unexpected}")
    print(f"[grx009-toolchain-probe] results: {RESULTS_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
