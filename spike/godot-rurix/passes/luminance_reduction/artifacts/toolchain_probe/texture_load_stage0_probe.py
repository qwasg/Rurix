#!/usr/bin/env python3
"""GRX-009 texture-line stage 0 probe: upstream texture load.level acceptance.

Zero-change precursor to the texture-line rurixc rewire. Answers one question
with hard evidence: does the local DXIL-capable llc
(``H:\\llvm-clean-82c5bce5-build\\bin\\llc.exe``, LLVM 23.0.0git, which merged the
upstream texture ``load.level`` path, PR #193343) ACCEPT the UPSTREAM texture
load form and emit a byte-stable DXIL object?

Forward case (expect ACCEPT, byte-stable):
  * ``case_K.ll`` : ``llvm.dx.resource.load.level`` +
    ``target("dx.Texture", float, 0, 0, 0, 2)`` (SRV Texture2D<float>), texel
    kept live by a rawbuffer store sink.

Reverse cases (expect REJECT, unchanged blocker) — proves the self-invented
form rurixc currently emits is still refused, so the ACCEPT above is due to the
upstream spelling, not a newly-permissive llc:
  * ``case_A.ll`` : ``llvm.dx.resource.load.texture.2d`` (self-invented)
  * ``case_H.ll`` : ``llvm.dx.resource.store.texture.2d`` (self-invented)

The forward case is emitted ``REPEAT`` (default 8) times; all runs must exit 0,
produce an object, and share one sha256 (deterministic emission). Stdlib-only,
``py -3`` friendly, writes ``texture_load_stage0_probe_results.json`` (LF) next
to this script. Discovers llc from ``$RURIX_LLC`` then the known local build.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
RESULTS_PATH = SCRIPT_DIR / "texture_load_stage0_probe_results.json"

DEFAULT_LLC = r"H:\llvm-clean-82c5bce5-build\bin\llc.exe"
REPEAT = 8
TIMEOUT_SECONDS = 60
STDERR_EXCERPT_LEN = 2048


def resolve_llc() -> str:
    for raw in (os.environ.get("RURIX_LLC", ""), DEFAULT_LLC):
        if raw and Path(raw).exists():
            return raw
    return ""


def get_llc_version(llc_path: str) -> str:
    try:
        proc = subprocess.run(
            [llc_path, "--version"], capture_output=True, text=True, timeout=30
        )
    except (subprocess.SubprocessError, OSError):
        return ""
    for line in ((proc.stdout or "") + (proc.stderr or "")).splitlines():
        if "LLVM version" in line:
            return line.strip()
    return ""


def emit_once(llc_path: str, ll_path: Path, obj_dir: Path, tag: str) -> dict[str, object]:
    obj_path = obj_dir / f"{ll_path.stem}.{tag}.obj"
    rec: dict[str, object] = {}
    try:
        proc = subprocess.run(
            [llc_path, str(ll_path), "-filetype=obj", "-o", str(obj_path)],
            capture_output=True,
            text=True,
            timeout=TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        return {"exit_code": None, "obj_produced": False, "note": "timeout"}
    except (OSError, subprocess.SubprocessError) as exc:
        return {"exit_code": None, "obj_produced": False, "note": f"{type(exc).__name__}: {exc}"}
    rec["exit_code"] = proc.returncode
    rec["stderr_excerpt"] = (proc.stderr or "")[:STDERR_EXCERPT_LEN].strip()
    if obj_path.exists():
        rec["obj_produced"] = True
        rec["obj_sha256"] = hashlib.sha256(obj_path.read_bytes()).hexdigest()
        rec["obj_size"] = obj_path.stat().st_size
        try:
            obj_path.unlink()
        except OSError:
            pass
    else:
        rec["obj_produced"] = False
    return rec


def forward_case(llc_path: str, obj_dir: Path) -> dict[str, object]:
    ll_path = SCRIPT_DIR / "case_K.ll"
    if not ll_path.is_file():
        return {"case": "case_K.ll", "verdict": "unavailable", "note": "case .ll not found"}
    runs = [emit_once(llc_path, ll_path, obj_dir, f"r{i}") for i in range(REPEAT)]
    shas = {r.get("obj_sha256") for r in runs if r.get("obj_produced")}
    all_ok = all(r.get("exit_code") == 0 and r.get("obj_produced") for r in runs)
    stable = len(shas) == 1 and None not in shas
    return {
        "case": "case_K.ll",
        "intrinsic": "llvm.dx.resource.load.level",
        "target_ext_type": 'target("dx.Texture", float, 0, 0, 0, 2)',
        "expected": "ACCEPT",
        "repeat": REPEAT,
        "all_exit_zero_and_obj": all_ok,
        "byte_stable": stable,
        "distinct_sha256": sorted(s for s in shas if s),
        "verdict": "accept_stable" if (all_ok and stable) else "unstable_or_reject",
        "runs": runs,
    }


def reverse_case(llc_path: str, case_file: str, intrinsic: str, obj_dir: Path) -> dict[str, object]:
    ll_path = SCRIPT_DIR / case_file
    if not ll_path.is_file():
        return {"case": case_file, "verdict": "unavailable", "note": "case .ll not found"}
    rec = emit_once(llc_path, ll_path, obj_dir, "rev")
    unknown_marker = f"unknown intrinsic '{intrinsic}'"
    stderr = str(rec.get("stderr_excerpt", ""))
    if rec.get("exit_code") == 0 and rec.get("obj_produced"):
        verdict = "unexpected_accept"
    elif unknown_marker in stderr:
        verdict = "reject_unknown_intrinsic"
    else:
        verdict = "reject_other"
    return {
        "case": case_file,
        "intrinsic": intrinsic,
        "expected": "REJECT",
        "verdict": verdict,
        "exit_code": rec.get("exit_code"),
        "obj_produced": rec.get("obj_produced"),
        "stderr_excerpt": stderr,
    }


def write_results(payload: dict[str, object]) -> None:
    RESULTS_PATH.write_text(
        json.dumps(payload, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def main() -> int:
    llc_path = resolve_llc()
    if not llc_path:
        write_results({"status": "skip", "skip_reason": "no_llc_found"})
        print("[grx009-texload-stage0] status=skip skip_reason=no_llc_found")
        return 0
    with tempfile.TemporaryDirectory(prefix="grx009_texload_s0_") as tmp:
        obj_dir = Path(tmp)
        forward = forward_case(llc_path, obj_dir)
        reverse = [
            reverse_case(llc_path, "case_A.ll", "llvm.dx.resource.load.texture.2d", obj_dir),
            reverse_case(llc_path, "case_H.ll", "llvm.dx.resource.store.texture.2d", obj_dir),
        ]
    forward_ok = forward.get("verdict") == "accept_stable"
    reverse_ok = all(r.get("verdict") == "reject_unknown_intrinsic" for r in reverse)
    payload = {
        "status": "complete",
        "probe_run_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "llc_path": llc_path,
        "llc_version": get_llc_version(llc_path),
        "forward_load_level": forward,
        "reverse_self_invented": reverse,
        "stage0_pass": bool(forward_ok and reverse_ok),
        "conclusion": (
            "Upstream texture load.level ACCEPTS and emits a byte-stable DXIL object "
            "while the self-invented load.texture.2d / store.texture.2d forms remain "
            "rejected by name. Texture LOAD is unblocked on this llc; STORE still needs "
            "the local llvm.dx.resource.store.texture patch (segment texture stage 1)."
            if (forward_ok and reverse_ok)
            else "Stage 0 gate not met; inspect forward_load_level / reverse_self_invented."
        ),
    }
    write_results(payload)
    print(
        f"[grx009-texload-stage0] status=complete stage0_pass={payload['stage0_pass']} "
        f"forward={forward.get('verdict')} reverse="
        + ",".join(str(r.get("verdict")) for r in reverse)
    )
    print(f"[grx009-texload-stage0] results: {RESULTS_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
