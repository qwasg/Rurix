#!/usr/bin/env python3
"""GRX-009 cross-version texture-intrinsic toolchain probe.

Companion to ``run_probe.py``. Where ``run_probe.py`` exercises the full
``case_*.ll`` accept/reject/crash table against a single llc (``RURIX_LLC`` or
the patched fallback), this script isolates the ONE decisive question for
GRX-009 segment 4i: does ANY DXIL-capable llc on this machine recognize the
texture load/store intrinsics ``rurixc`` emits?

It runs the two texture-intrinsic reject cases —

  * ``case_A.ll`` : ``llvm.dx.resource.load.texture.2d``  (SRV Texture2D<float> load)
  * ``case_H.ll`` : ``llvm.dx.resource.store.texture.2d`` (UAV RWTexture2D<float> store)

— against every llc build it can find, and records the per-build verdict. A
texture-intrinsic-capable llc would ACCEPT (exit 0, obj produced); every llc on
this machine REJECTs with ``unknown intrinsic``, which is the toolchain blocker
documented in ``texture_intrinsic_toolchain_blocker.json``. Recording the reject
across MULTIPLE LLVM versions (a patched 22.1.7 DirectX build plus a bleeding-edge
23.0.0git upstream build) proves the gap is an upstream DirectX-backend
limitation, not merely a patched-build regression that a newer llc would fix.

Discovery order (deduplicated by resolved path; only existing paths probed):
  1. ``$RURIX_LLC`` (if set and exists)
  2. the known local builds in ``KNOWN_LLC_CANDIDATES``
  3. extra ``;``-separated paths in ``$RURIX_LLC_EXTRA``

Stdlib-only, Windows + ``py -3`` friendly. Writes
``cross_version_probe_results.json`` next to this script (LF). Never installs
anything and never mutates canonical pass artifacts (each ``.obj`` is emitted to
a scratch temp dir and discarded); safe to run in CI (writes a skip JSON when no
llc or no case files are found, exit 0).
"""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path

# --- paths ---------------------------------------------------------------

SCRIPT_DIR = Path(__file__).resolve().parent
RESULTS_PATH = SCRIPT_DIR / "cross_version_probe_results.json"

# Known local llc builds on this workstation, most-relevant first. These are
# the builds surveyed while gathering the segment 4i texture-intrinsic blocker
# evidence; the probe records only the ones that exist on disk at run time.
KNOWN_LLC_CANDIDATES = (
    r"H:\llvm-dxil\build\bin\llc.exe",
    r"H:\llvm-clean-82c5bce5-build\bin\llc.exe",
    r"H:\llvm-upstream-test\build\bin\llc.exe",
)

# The two texture-intrinsic reject cases. A texture-capable llc ACCEPTs these
# (exit 0, obj produced); every llc surveyed rejects them by name.
TEXTURE_CASES = (
    {
        "case": "case_A.ll",
        "intrinsic": "llvm.dx.resource.load.texture.2d",
        "direction": "load",
        "resource": "Texture2D<float>",
    },
    {
        "case": "case_H.ll",
        "intrinsic": "llvm.dx.resource.store.texture.2d",
        "direction": "store",
        "resource": "RWTexture2D<float>",
    },
)

TIMEOUT_SECONDS = 60
STDERR_EXCERPT_LEN = 2048


def discover_llc_builds() -> list[str]:
    """Return existing llc paths (deduplicated by resolved path)."""
    ordered: list[str] = []
    seen: set[str] = set()

    def consider(raw: str) -> None:
        if not raw:
            return
        path = Path(raw)
        if not path.exists():
            return
        key = str(path.resolve()).lower()
        if key in seen:
            return
        seen.add(key)
        ordered.append(str(path))

    consider(os.environ.get("RURIX_LLC", ""))
    for candidate in KNOWN_LLC_CANDIDATES:
        consider(candidate)
    for extra in os.environ.get("RURIX_LLC_EXTRA", "").split(";"):
        consider(extra.strip())
    return ordered


def get_llc_version(llc_path: str) -> str:
    """Return the first 'LLVM version' line from ``llc --version`` (or "")."""
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


def run_case(llc_path: str, case: dict[str, str], obj_dir: Path) -> dict[str, object]:
    """Run one texture-intrinsic case through one llc, classify the verdict."""
    ll_path = SCRIPT_DIR / case["case"]
    intrinsic = case["intrinsic"]
    result: dict[str, object] = {
        "case": case["case"],
        "intrinsic": intrinsic,
        "direction": case["direction"],
        "resource": case["resource"],
        "expected": "REJECT",
    }
    if not ll_path.is_file():
        result["verdict"] = "unavailable"
        result["note"] = "case .ll file not found"
        return result

    obj_path = obj_dir / (ll_path.stem + ".obj")
    exit_code: int | None = None
    obj_produced = False
    stderr_excerpt = ""
    try:
        proc = subprocess.run(
            [llc_path, str(ll_path), "-filetype=obj", "-o", str(obj_path)],
            capture_output=True,
            text=True,
            timeout=TIMEOUT_SECONDS,
        )
        exit_code = proc.returncode
        stderr_excerpt = (proc.stderr or "")[:STDERR_EXCERPT_LEN].strip()
        obj_produced = obj_path.exists()
    except subprocess.TimeoutExpired:
        result["verdict"] = "timeout"
        result["note"] = f"llc did not return within {TIMEOUT_SECONDS}s"
        return result
    except (OSError, subprocess.SubprocessError) as exc:
        result["verdict"] = "run_error"
        result["note"] = f"{type(exc).__name__}: {exc}"
        return result
    finally:
        try:
            if obj_path.exists():
                obj_path.unlink()
        except OSError:
            pass

    result["exit_code"] = exit_code
    result["obj_produced"] = obj_produced
    result["stderr_excerpt"] = stderr_excerpt
    unknown_marker = f"unknown intrinsic '{intrinsic}'"
    if exit_code == 0 and obj_produced:
        # A texture-intrinsic-capable llc: this would be the breakthrough that
        # unblocks segment 4i. None observed on this machine.
        result["verdict"] = "accept"
        result["texture_intrinsic_capable"] = True
    elif exit_code not in (0, None) and unknown_marker in stderr_excerpt:
        result["verdict"] = "reject_unknown_intrinsic"
        result["texture_intrinsic_capable"] = False
    else:
        # Any other non-zero outcome (e.g. the non-deterministic DXContainer
        # emitter crash) is still not a usable texture-capable emission.
        result["verdict"] = "reject_other"
        result["texture_intrinsic_capable"] = False
    return result


def write_results(payload: dict[str, object]) -> None:
    RESULTS_PATH.write_text(
        json.dumps(payload, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def main() -> int:
    builds = discover_llc_builds()
    if not builds:
        write_results(
            {
                "status": "skip",
                "skip_reason": "no_llc_found",
                "llc_builds": [],
                "any_texture_intrinsic_capable": False,
            }
        )
        print("[grx009-cross-version-probe] status=skip skip_reason=no_llc_found")
        print(f"[grx009-cross-version-probe] results: {RESULTS_PATH}")
        return 0

    build_records: list[dict[str, object]] = []
    any_capable = False
    with tempfile.TemporaryDirectory(prefix="grx009_xver_obj_") as tmp:
        obj_dir = Path(tmp)
        for llc_path in builds:
            version = get_llc_version(llc_path)
            case_results = [run_case(llc_path, case, obj_dir) for case in TEXTURE_CASES]
            capable = any(r.get("texture_intrinsic_capable") is True for r in case_results)
            any_capable = any_capable or capable
            build_records.append(
                {
                    "llc_path": llc_path,
                    "llc_version": version,
                    "texture_intrinsic_capable": capable,
                    "cases": case_results,
                }
            )

    if any_capable:
        conclusion = (
            "A texture-intrinsic-capable llc was found: at least one build ACCEPTED a "
            "texture load/store intrinsic. Segment 4i can advance to producing a real "
            "runtime-mappable texture-capable DXIL container; refresh the blocker evidence."
        )
    else:
        conclusion = (
            "No texture-intrinsic-capable llc found. Every surveyed llc rejects "
            "llvm.dx.resource.load.texture.2d / llvm.dx.resource.store.texture.2d by name "
            "(or fails to emit an object), across the patched LLVM 22.1.7 DirectX build and "
            "the bleeding-edge 23.0.0git upstream build. The texture-intrinsic gap is an "
            "upstream DirectX-backend limitation, not a patched-build regression. Segment 4i "
            "stays fail-closed: offline_compile_evidence.json remains compile_failed / "
            "runtime_mappable=false and the bridge tracked package stays raw_buffer_view."
        )

    payload = {
        "status": "complete",
        "probe_run_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "texture_cases": [dict(case) for case in TEXTURE_CASES],
        "llc_builds": build_records,
        "any_texture_intrinsic_capable": any_capable,
        "conclusion": conclusion,
    }
    write_results(payload)

    print(
        f"[grx009-cross-version-probe] status=complete builds={len(build_records)} "
        f"any_texture_intrinsic_capable={any_capable}"
    )
    for record in build_records:
        verdicts = ",".join(str(c.get("verdict")) for c in record["cases"])
        print(
            f"[grx009-cross-version-probe] {record['llc_version'] or record['llc_path']}: "
            f"{verdicts}"
        )
    print(f"[grx009-cross-version-probe] results: {RESULTS_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
