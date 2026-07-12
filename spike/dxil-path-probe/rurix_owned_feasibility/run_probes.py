#!/usr/bin/env python3
"""Run rurix_owned feasibility probes through the full native DXIL chain.

rurixc (release, dxil-backend+shader-stages) -> patched llc -filetype=obj ->
dxv validator. Classifies each probe as dxv_pass / rurixc_reject / llc_fail /
dxv_reject and writes a machine-readable evidence JSON. Real toolchain only;
no estimates. LF-only output.
"""
from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]  # H:\rurix
RURIXC = ROOT / "target" / "release" / "rurixc.exe"
PROBES = HERE / "probes"
EVID = HERE / "evidence"
EVID.mkdir(parents=True, exist_ok=True)

ENV = dict(os.environ)
ENV["RURIX_LLC"] = r"H:/llvm-clean-82c5bce5-build/bin/llc.exe"
ENV["RURIX_DXC_DIR"] = r"H:/dxc-round7/extracted/bin/x64"

# probe file -> hypothesis note (the measured class is authoritative).
# "reason" documents WHAT the probe isolates.
EXPECT = {
    # --- expressible native subsets (expected dxv PASS) ---
    "cluster_store_arith_core.rx": "dxv_pass|cluster_store z-decode+min/max pack, output at own index (cast-free)",
    "probe_xor_clear_bit.rx": "dxv_pass|findLSB bit-walk cleared by XOR (unary-NOT substitute)",
    "particles_copy_align_disabled_lane.rx": "reject_alignment|ALIGN_DISABLED f32 lane; i64 root-const odd-dword alignment",
    "instance_compaction_scatter_lane.rx": "dxv_pass|D3 bit-preserving transform move given precomputed rank",
    "probe_texture_loadstore_2d.rx": "dxv_pass|SETTLES GRX-009 llc texture blocker (single-channel f32 texel load/store)",
    # --- capability boundaries (expected reject; isolate a specific gap) ---
    "cluster_store_native_probe.rx": "reject_cast|full pack needs u32->usize cast for scan-derived index",
    "probe_u32_bitmask_decode.rx": "reject_cast|instance index = word*32+bit needs u32->usize cast",
    "probe_cast_f32_u32.rx": "reject_cast|ssao edge-unpack (f32->u32->f32) casts",
    "probe_unary_not.rx": "reject|unary bitwise NOT not lowered on DXIL path",
    "probe_groupshared_barrier.rx": "reject|shared let + block.sync() (compaction prefix-scan)",
    "probe_atomic.rx": "reject|AtomicView + fetch_add (culling/indirect_args counts)",
    "probe_viewmut_read_rmw.rx": "reject|read-modify-write on ViewMut/UAV (cluster_store/culling merges)",
}


def classify(exit_code: int, stdout: str, stderr: str) -> str:
    blob = (stdout or "") + (stderr or "")
    if exit_code == 0 and "dxc validator accepted" in blob:
        return "dxv_pass"
    if "dxc validator rejected" in blob:
        return "dxv_reject"
    if "patched llc DXIL emit failed" in blob:
        return "llc_fail"
    if "RX" in blob:
        # extract the first RXxxxx code
        return "rurixc_reject"
    return "other"


def rx_code(blob: str) -> str | None:
    import re

    m = re.search(r"\bRX(\d{4})\b", blob)
    return f"RX{m.group(1)}" if m else None


def sha256(path: pathlib.Path) -> str | None:
    if not path.is_file():
        return None
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    results = []
    for name in sorted(EXPECT):
        src = PROBES / name
        out = EVID / (src.stem + ".dxil")
        cmd = [str(RURIXC), str(src), "--target", "dxil", "-o", str(out)]
        proc = subprocess.run(cmd, cwd=ROOT, env=ENV, text=True, capture_output=True, check=False)
        blob = (proc.stdout or "") + (proc.stderr or "")
        cls = classify(proc.returncode, proc.stdout, proc.stderr)
        code = rx_code(blob)
        # container size only meaningful when dxv passed
        dxil_sha = sha256(out) if cls == "dxv_pass" else None
        dxil_bytes = out.stat().st_size if (cls == "dxv_pass" and out.is_file()) else None
        results.append(
            {
                "probe": name,
                "expected": EXPECT[name],
                "measured_class": cls,
                "exit_code": proc.returncode,
                "rx_code": code,
                "dxil_container_sha256": dxil_sha,
                "dxil_container_bytes": dxil_bytes,
                "stderr_tail": "\n".join((proc.stderr or "").splitlines()[-6:]),
            }
        )
        # write per-probe stderr for the record
        log = EVID / (src.stem + ".stderr.txt")
        with log.open("w", encoding="utf-8", newline="\n") as fh:
            fh.write(proc.stderr or "")
        print(f"{name:38s} -> {cls:24s} exit={proc.returncode} code={code} bytes={dxil_bytes}")

    summary = {
        "generated_utc": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "toolchain": {
            "rurixc": str(RURIXC),
            "llc": ENV["RURIX_LLC"],
            "dxc_dir": ENV["RURIX_DXC_DIR"],
            "features": "dxil-backend shader-stages",
        },
        "note": "Real rurixc->llc->dxv chain. measured_class is authoritative; expected is the pre-run hypothesis.",
        "probes": results,
    }
    out_json = HERE / "capability_probe_evidence.json"
    with out_json.open("w", encoding="utf-8", newline="\n") as fh:
        json.dump(summary, fh, ensure_ascii=False, indent=2)
        fh.write("\n")
    print(f"\nwrote {out_json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
