#!/usr/bin/env python3
"""Directly compile each GRX pass's actual src/lib.rx through the native DXIL
chain (rurixc -> llc -> dxv) and record the measured outcome. No estimates.
LF-only output.
"""
from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
RURIXC = ROOT / "target" / "release" / "rurixc.exe"
PASSES = ROOT / "spike" / "godot-rurix" / "passes"
EVID = HERE / "evidence" / "pass_direct"
EVID.mkdir(parents=True, exist_ok=True)

ENV = dict(os.environ)
ENV["RURIX_LLC"] = r"H:/llvm-clean-82c5bce5-build/bin/llc.exe"
ENV["RURIX_DXC_DIR"] = r"H:/dxc-round7/extracted/bin/x64"

PASS_LIST = [
    "luminance_reduction",
    "tonemap",
    "ssao_blur",
    "taa_resolve",
    "particles_copy",
    "cluster_store",
    "gpu_culling",
    "instance_compaction",
    "indirect_args",
    "fused_post_chain",
]


def classify(rc: int, blob: str) -> str:
    if rc == 0 and "dxc validator accepted" in blob:
        return "dxv_pass"
    if "no compute `kernel fn` found" in blob:
        return "no_kernel_documentation_only"
    if "dxc validator rejected" in blob:
        return "dxv_reject"
    if "patched llc DXIL emit failed" in blob:
        return "llc_fail"
    m = re.search(r"\bRX(\d{4})\b", blob)
    if m:
        return f"rurixc_reject_RX{m.group(1)}"
    return "other"


def first_error(blob: str) -> str:
    for line in blob.splitlines():
        if "error[RX" in line:
            return line.strip()[:200]
    if "no compute `kernel fn` found" in blob:
        return "no compute kernel fn (documentation-only lib.rx)"
    for line in blob.splitlines():
        if "accepted" in line:
            return line.strip()[:200]
    return ""


def main() -> int:
    rows = []
    for p in PASS_LIST:
        src = PASSES / p / "src" / "lib.rx"
        out = EVID / f"{p}.dxil"
        proc = subprocess.run(
            [str(RURIXC), str(src), "--target", "dxil", "-o", str(out)],
            cwd=ROOT, env=ENV, text=True, capture_output=True, check=False,
        )
        blob = (proc.stdout or "") + (proc.stderr or "")
        cls = classify(proc.returncode, blob)
        dxil_sha = None
        dxil_bytes = None
        if cls == "dxv_pass" and out.is_file():
            b = out.read_bytes()
            dxil_sha = hashlib.sha256(b).hexdigest()
            dxil_bytes = len(b)
        rows.append({
            "pass": p,
            "src": str(src.relative_to(ROOT)).replace("\\", "/"),
            "measured_class": cls,
            "exit_code": proc.returncode,
            "first_signal": first_error(blob),
            "dxil_container_sha256": dxil_sha,
            "dxil_container_bytes": dxil_bytes,
        })
        print(f"{p:22s} {cls}")

    summary = {
        "generated_utc": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "toolchain": {"rurixc": str(RURIXC), "llc": ENV["RURIX_LLC"], "dxc_dir": ENV["RURIX_DXC_DIR"]},
        "note": "Direct compile of each pass's UNMODIFIED src/lib.rx. Documentation-only lib.rx (cluster_store/gpu_culling/indirect_args) carry no kernel fn.",
        "passes": rows,
    }
    outp = HERE / "pass_direct_compile_evidence.json"
    with outp.open("w", encoding="utf-8", newline="\n") as fh:
        json.dump(summary, fh, ensure_ascii=False, indent=2)
        fh.write("\n")
    print(f"wrote {outp}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
