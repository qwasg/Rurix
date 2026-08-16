#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波）
"""G11.3 U2 修复面 — Bistro DDS → PNG 派生链转码驱动（G10-N7 承接锚兑现）。

链路（UE Interchange 不消费 .dds 的绕行面 = 派生链转码，G11_PLAN §2 G11.3
U2 行字面；M131 白名单面联动——派生产物为同一登记资产 bistro-orca 的
G11.3 转码面，逐文件 digest 入 manifest 机核）：

  K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/（G10 语料 0-byte 只读）
    ├─ *.dds（144 张实测：DXT1×54 / DXT5×20 / ATI2×70 legacy FourCC）
    │    → target/release/g11_3_dds_dump（bcdec 真实解码 mip0 → RGBA8 raw）
    │    → PIL PNG 编码（baseColor 直写 RGBA8；BC5 法线 XY 重建 Z 通道：
    │      z = sqrt(max(0,1−x²−y²))，PNG = 标准切线空间法线图）
    ├─ buffer.bin（39MB 逐字节复制，digest 对账）
    └─ BistroInterior.gltf → 派生 gltf（images[].uri 仅扩展名改写 .dds→.png，
       其余字节级结构不动）→ K:/rurix-ext/g11-assets/bistro-interior-ue/

产物登记：milestones/g11/g11_3_dds_transcode_manifest.json（逐文件源 digest +
产物 digest + 格式枚举 + 派生 gltf digest——未登记资产混入即 RED 的机核面）。

用法：
  py -3 milestones/g11/harness/g11_3_dds_transcode.py
"""
from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
SRC_DIR = Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior")
OUT_DIR = Path(r"K:\rurix-ext\g11-assets\bistro-interior-ue")
MANIFEST_PATH = ROOT / "milestones" / "g11" / "g11_3_dds_transcode_manifest.json"
DUMP_BIN = ROOT / "target" / "release" / "g11_3_dds_dump.exe"


def log(msg: str) -> None:
    print(f"[g11_3_dds_transcode] {msg}", flush=True)


def sha256_bytes(b: bytes) -> str:
    return "sha256:" + hashlib.sha256(b).hexdigest()


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()


def main() -> int:
    if not DUMP_BIN.is_file():
        print(f"缺解码器（先 cargo build --release -p rurix-asset --bin g11_3_dds_dump）: {DUMP_BIN}", file=sys.stderr)
        return 2
    gltf_path = SRC_DIR / "BistroInterior.gltf"
    doc = json.loads(gltf_path.read_text(encoding="utf-8"))
    images = doc.get("images", [])
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    entries = []
    formats: dict[str, int] = {}
    tmp_raw = OUT_DIR / "_tmp_decode.rgba8"
    for im in images:
        uri = im.get("uri", "")
        if not uri.lower().endswith(".dds"):
            return 2
        src = SRC_DIR / uri
        png_name = uri[:-4] + ".png"
        dst = OUT_DIR / png_name
        r = subprocess.run([str(DUMP_BIN), str(src), str(tmp_raw)], capture_output=True, text=True)
        if r.returncode != 0:
            print(f"解码失败 {uri}: {r.stderr[-400:]}", file=sys.stderr)
            return 1
        info = json.loads(r.stdout.strip().splitlines()[-1])
        w, h, fmt = info["width"], info["height"], info["format"]
        formats[fmt] = formats.get(fmt, 0) + 1
        arr = np.fromfile(tmp_raw, dtype=np.uint8).reshape(h, w, 4)
        if fmt == "bc5":
            # 法线 XY（R/G）→ 重建 Z 通道写标准切线空间法线 PNG。
            xy = arr[..., :2].astype(np.float64) / 255.0 * 2.0 - 1.0
            z2 = np.clip(1.0 - xy[..., 0] ** 2 - xy[..., 1] ** 2, 0.0, None)
            z = np.sqrt(z2)
            zb = np.round((z * 0.5 + 0.5) * 255.0).astype(np.uint8)
            out = arr.copy()
            out[..., 2] = zb
            out[..., 3] = 255
            arr = out
        Image.fromarray(arr, "RGBA").save(dst, format="PNG")
        entries.append({
            "source_uri": uri,
            "product_png": png_name,
            "dds_format": fmt,
            "width": w,
            "height": h,
            "mip_count": info["mip_count"],
            "source_digest": sha256_file(src),
            "rgba8_digest": info["rgba8_digest"],
            "product_digest": sha256_file(dst),
        })
        log(f"{uri} [{fmt} {w}x{h}] → {png_name}")

    tmp_raw.unlink(missing_ok=True)
    # buffer.bin 逐字节复制 + digest 对账。
    bin_src = SRC_DIR / "buffer.bin"
    bin_dst = OUT_DIR / "buffer.bin"
    shutil.copyfile(bin_src, bin_dst)
    bin_src_d = sha256_file(bin_src)
    bin_dst_d = sha256_file(bin_dst)
    if bin_src_d != bin_dst_d:
        print("buffer.bin 复制 digest 不符", file=sys.stderr)
        return 1

    # 派生 gltf：仅 images[].uri 扩展名改写（结构 0-byte 语义面）。
    for im in doc["images"]:
        im["uri"] = im["uri"][:-4] + ".png"
    derived_text = json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n"
    derived_path = OUT_DIR / "BistroInterior.gltf"
    derived_path.write_text(derived_text, encoding="utf-8", newline="\n")

    manifest = {
        "schema_version": 1,
        "registry": "g11_3_dds_transcode_manifest",
        "generated_by": "milestones/g11/harness/g11_3_dds_transcode.py",
        "semantics": (
            "U2 修复面派生链转码登记：bistro-orca 包内 .dds（UE Interchange 不消费）"
            "经 bcdec 真实解码 → PNG 派生（baseColor 直写 RGBA8；BC5 法线 XY 重建 Z）。"
            "源语料 0-byte 只读；产物 = 同一登记资产的 G11.3 转码面，逐文件 digest 机核。"
        ),
        "source_asset_id": "bistro-orca",
        "source_dir": str(SRC_DIR),
        "output_dir": str(OUT_DIR),
        "decoder": "target/release/g11_3_dds_dump.exe（rurix_asset::bcdec::decode_dds 真实解码）",
        "format_histogram": formats,
        "image_count": len(entries),
        "buffer_bin": {"source_digest": bin_src_d, "product_digest": bin_dst_d},
        "derived_gltf": {
            "path": str(derived_path),
            "digest": sha256_file(derived_path),
            "rewrite": "images[].uri 扩展名 .dds→.png（其余结构 0-byte 语义面）",
        },
        "entries": entries,
    }
    MANIFEST_PATH.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    log(f"manifest → {MANIFEST_PATH}（images={len(entries)} formats={formats}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
