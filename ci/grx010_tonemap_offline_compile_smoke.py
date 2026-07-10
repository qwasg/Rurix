#!/usr/bin/env python3
"""GRX-010: tonemap offline compile smoke.

Runs the pass compile script
(``spike/godot-rurix/passes/tonemap/compile_hlsl_bridge.py``: DXC cs_6_0
compile + DXV validation + descriptor layout + Rurix-owned RTS0 via the
``emit_grx010_tonemap_rts0`` example) and then audits the produced
``offline_compile_evidence.json``:

* ``status=success`` with ``provenance=hlsl_bridge_workaround``,
  ``rurix_owned=false``, and ``runtime_mappable=true`` (the owner-approved
  GRX-009 texture artifact provenance policy applies to every texture
  compute pass);
* all three canonical artifacts (DXIL container, RTS0 root signature,
  descriptor layout) exist on disk and their recomputed SHA-256 digests
  match the evidence byte for byte;
* the DXV validation leg recorded ``status=pass``.

A missing DXC/DXV toolchain records an honest SKIP (exit 0 unless
``RURIX_REQUIRE_REAL=1``). Any mismatch is a hard FAIL. Success here is
offline compile evidence only: it does NOT enable the pass, does NOT imply
real_gpu_pass=true, and makes no visual/perf claim.
"""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "tonemap"
COMPILE_SCRIPT = PASS_DIR / "compile_hlsl_bridge.py"
EVIDENCE_PATH = PASS_DIR / "offline_compile_evidence.json"


def sha256_of_file(path: pathlib.Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fail(msg: str) -> int:
    print(f"[grx010-offline-compile-smoke] FAIL {msg}", file=sys.stderr)
    return 1


def main() -> int:
    completed = subprocess.run(
        [sys.executable, str(COMPILE_SCRIPT)], cwd=ROOT, text=True, check=False
    )
    if completed.returncode != 0:
        return fail(f"compile script exited {completed.returncode}")

    if not EVIDENCE_PATH.is_file():
        return fail("offline_compile_evidence.json missing after compile run")
    evidence = json.loads(EVIDENCE_PATH.read_text(encoding="utf-8"))

    status = evidence.get("status")
    if status == "skip":
        msg = f"toolchain unavailable ({evidence.get('blocker_category')})"
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            return fail(f"(RURIX_REQUIRE_REAL) {msg}")
        print(f"[grx010-offline-compile-smoke] SKIP {msg} (exit 0)")
        return 0
    if status != "success":
        return fail(f"offline compile status={status} blocker={evidence.get('blocker_category')}")

    if evidence.get("provenance") != "hlsl_bridge_workaround":
        return fail(f"unexpected provenance {evidence.get('provenance')!r}")
    if evidence.get("rurix_owned") is not False:
        return fail("rurix_owned must stay false for the hlsl_bridge workaround package")
    if evidence.get("runtime_mappable") is not True:
        return fail("runtime_mappable must be true on a success evidence")
    validation = (evidence.get("dxil_provenance") or {}).get("validation") or {}
    if validation.get("status") != "pass":
        return fail(f"dxv validation status={validation.get('status')!r}")

    artifacts = evidence.get("artifacts")
    if not isinstance(artifacts, dict):
        return fail("evidence artifacts block missing")
    for key in ("dxil", "root_signature", "descriptor_layout"):
        entry = artifacts.get(key)
        if not isinstance(entry, dict):
            return fail(f"artifact entry {key} missing")
        path = ROOT / str(entry.get("path"))
        if not path.is_file():
            return fail(f"artifact {key} missing on disk: {entry.get('path')}")
        actual = sha256_of_file(path)
        if actual != entry.get("sha256"):
            return fail(
                f"artifact {key} hash mismatch: disk={actual} evidence={entry.get('sha256')}"
            )

    print(
        "[grx010-offline-compile-smoke] PASS status=success "
        "provenance=hlsl_bridge_workaround artifacts verified on disk"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
