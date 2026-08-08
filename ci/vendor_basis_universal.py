#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.3 M83:vendor `basis_universal`(BinomialLLC)到 `src/rurix-basis-sys/vendor/basis_universal/`。

设计案 §3.6 MUST:vendor 快照 + 精确 tag/commit + source digest + LICENSE digest。
**禁 cmake**:本脚本只拉源码;编译由 `build.rs` + `cc` crate 用显式 .cpp 清单完成。

pin 见 PIN_* 常量;拉取后写 `vendor_manifest.json`(逐文件 sha256 + 聚合 digest),
供 `VENDOR.md` / `SBOM.md` 登记与 smoke 复核。

用法:
  py -3 ci/vendor_basis_universal.py            # 拉取(已存在且 digest 相符则跳过)
  py -3 ci/vendor_basis_universal.py --verify   # 只校验已落盘树的 digest
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VENDOR_DIR = ROOT / "src" / "rurix-basis-sys" / "vendor" / "basis_universal"
MANIFEST = VENDOR_DIR / "vendor_manifest.json"

PIN_UPSTREAM = "https://github.com/BinomialLLC/basis_universal"
PIN_TAG = "1.16.4"
PIN_COMMIT = "900e40fb5d2502927360fe2f31762bdbb624455f"
RAW_BASE = f"https://raw.githubusercontent.com/BinomialLLC/basis_universal/{PIN_COMMIT}/"

# 显式清单(对齐上游 CMakeLists BASISU_SRC_LIST,剔除 basisu_tool.cpp(CLI main)、
# zstd/(禁 supercompression)、OpenCL/CL 头(禁 OpenCL))。
ENCODER_CPP = [
    "basisu_backend.cpp",
    "basisu_basis_file.cpp",
    "basisu_bc7enc.cpp",
    "basisu_comp.cpp",
    "basisu_enc.cpp",
    "basisu_etc.cpp",
    "basisu_frontend.cpp",
    "basisu_gpu_texture.cpp",
    "basisu_kernels_sse.cpp",
    "basisu_opencl.cpp",
    "basisu_pvrtc1_4.cpp",
    "basisu_resample_filters.cpp",
    "basisu_resampler.cpp",
    "basisu_ssim.cpp",
    "basisu_uastc_enc.cpp",
    "jpgd.cpp",
    "pvpngreader.cpp",
]
ENCODER_H = [
    "basisu_backend.h",
    "basisu_basis_file.h",
    "basisu_bc7enc.h",
    "basisu_comp.h",
    "basisu_enc.h",
    "basisu_etc.h",
    "basisu_frontend.h",
    "basisu_gpu_texture.h",
    "basisu_kernels_declares.h",
    "basisu_kernels_imp.h",
    "basisu_miniz.h",
    "basisu_ocl_kernels.h",
    "basisu_opencl.h",
    "basisu_pvrtc1_4.h",
    "basisu_resampler.h",
    "basisu_resampler_filters.h",
    "basisu_ssim.h",
    "basisu_uastc_enc.h",
    "cppspmd_flow.h",
    "cppspmd_math.h",
    "cppspmd_math_declares.h",
    "cppspmd_sse.h",
    "cppspmd_type_aliases.h",
    "jpgd.h",
    "pvpngreader.h",
]
TRANSCODER_CPP = ["basisu_transcoder.cpp"]
TRANSCODER_H = [
    "basisu.h",
    "basisu_containers.h",
    "basisu_containers_impl.h",
    "basisu_file_headers.h",
    "basisu_transcoder.h",
    "basisu_transcoder_internal.h",
    "basisu_transcoder_uastc.h",
]
TRANSCODER_INC = [
    "basisu_transcoder_tables_astc.inc",
    "basisu_transcoder_tables_astc_0_255.inc",
    "basisu_transcoder_tables_atc_55.inc",
    "basisu_transcoder_tables_atc_56.inc",
    "basisu_transcoder_tables_bc7_m5_alpha.inc",
    "basisu_transcoder_tables_bc7_m5_color.inc",
    "basisu_transcoder_tables_dxt1_5.inc",
    "basisu_transcoder_tables_dxt1_6.inc",
    "basisu_transcoder_tables_pvrtc2_45.inc",
    "basisu_transcoder_tables_pvrtc2_alpha_33.inc",
]
LICENSE_FILES = [
    "LICENSE",
    "LICENSES/Apache-2.0.txt",
    "LICENSES/BSD.txt",
    "LICENSES/Zlib.txt",
]

FILES: list[str] = (
    LICENSE_FILES
    + [f"encoder/{f}" for f in ENCODER_CPP + ENCODER_H]
    + [f"transcoder/{f}" for f in TRANSCODER_CPP + TRANSCODER_H + TRANSCODER_INC]
)

# build.rs 的编译单元清单(与 VENDOR.md §2 字面一致)。
COMPILE_UNITS = [f"encoder/{f}" for f in ENCODER_CPP] + [
    f"transcoder/{f}" for f in TRANSCODER_CPP
]


def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def fetch(rel: str) -> bytes:
    url = RAW_BASE + rel
    req = urllib.request.Request(url, headers={"User-Agent": "rurix-vendor"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def aggregate_digest(per_file: dict[str, str]) -> str:
    """聚合 digest = sha256(逐行 "sha256␠path\n",path 升序)。"""
    h = hashlib.sha256()
    for p in sorted(per_file):
        h.update(f"{per_file[p]} {p}\n".encode("utf-8"))
    return h.hexdigest()


def do_verify() -> int:
    if not MANIFEST.is_file():
        print(f"[vendor] FAIL manifest 缺失: {MANIFEST}", file=sys.stderr)
        return 1
    man = json.loads(MANIFEST.read_text(encoding="utf-8"))
    bad = 0
    per_file: dict[str, str] = {}
    for rel, want in man["files"].items():
        p = VENDOR_DIR / rel
        if not p.is_file():
            print(f"[vendor] MISSING {rel}", file=sys.stderr)
            bad += 1
            continue
        got = sha256_bytes(p.read_bytes())
        per_file[rel] = got
        if got != want:
            print(f"[vendor] DIGEST MISMATCH {rel}\n  want {want}\n  got  {got}", file=sys.stderr)
            bad += 1
    agg = aggregate_digest(per_file)
    if bad == 0 and agg != man["source_digest"]:
        print(f"[vendor] FAIL 聚合 digest 不符: {agg} != {man['source_digest']}", file=sys.stderr)
        bad += 1
    if bad:
        print(f"[vendor] VERDICT=FAIL ({bad} 项)", file=sys.stderr)
        return 1
    print(f"[vendor] VERDICT=OK {len(per_file)} 文件, source_digest={agg}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--verify", action="store_true")
    ap.add_argument("--force", action="store_true")
    args = ap.parse_args()
    if args.verify:
        return do_verify()

    VENDOR_DIR.mkdir(parents=True, exist_ok=True)
    per_file: dict[str, str] = {}
    total = 0
    for i, rel in enumerate(FILES, 1):
        dst = VENDOR_DIR / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        if dst.is_file() and not args.force:
            data = dst.read_bytes()
        else:
            print(f"[vendor] ({i}/{len(FILES)}) fetch {rel}")
            data = fetch(rel)
            dst.write_bytes(data)
        per_file[rel] = sha256_bytes(data)
        total += len(data)

    lic = {p: per_file[p] for p in LICENSE_FILES}
    man = {
        "upstream": PIN_UPSTREAM,
        "tag": PIN_TAG,
        "commit": PIN_COMMIT,
        "raw_base": RAW_BASE,
        "file_count": len(per_file),
        "total_bytes": total,
        "source_digest": aggregate_digest(per_file),
        "license_digest": aggregate_digest(lic),
        "license_spdx": "Apache-2.0",
        "compile_units": COMPILE_UNITS,
        "patches": [],
        "excluded": {
            "basisu_tool.cpp": "上游 CLI main,非库面",
            "zstd/": "禁 zstd supercompression(BASISD_SUPPORT_KTX2_ZSTD=0)",
            "OpenCL/CL/": "禁 OpenCL(BASISU_SUPPORT_OPENCL=0)",
            "webgl/, contrib/": "非库面",
        },
        "files": per_file,
    }
    MANIFEST.write_text(json.dumps(man, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"[vendor] wrote {MANIFEST.relative_to(ROOT)}")
    print(f"[vendor] file_count={len(per_file)} total_bytes={total}")
    print(f"[vendor] source_digest={man['source_digest']}")
    print(f"[vendor] license_digest={man['license_digest']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
