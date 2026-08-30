#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""F6 逐字恢复机核：恢复的原形态函数体/struct 体 vs HEAD 提取文本位级比对。

提取律法：从 `fn NAME(`（或 `struct NAME {`/`const NAME:`）行起，至首个列 0 的
`}` 行止（fn/struct）；const 单行。签名行起在前面的 #[...] 属性与 doc 注释不入
比对域（F6 在 doc 注释追加了双形态导航注记——函数体本身逐字恢复）。
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CUR = ROOT / "src" / "rurix-render" / "src" / "bin" / "g14_3_lane" / "g14_3_lane_body.rs"
ORIG = ROOT / "artifacts" / "day_0828" / "f_emissive" / "f6_lane_body_orig.rs"

FNS = [
    "geo_patch_proxy_tritex",
    "g31_dds_decode_rgba8",
    "g31_tex_probes",
    "g31_tex_host_sample",
    "g31_tex_load",
    "g31_tex_probe_device",
    "g31_tex_sampler_leg",
    "g31_tex_probe_evaluate",
    "g34_slab_premod_texmeta",
    "g31_tex_linlut",
    "g31_tex_host_sample_srgb",
]
STRUCTS = ["G31TexSlot", "G31TexAssets", "G31TexCensus", "G31TexProbeReport", "G34TexSideTable"]
CONSTS = ["G31_TEX_N_MAPPED", "G31_TEX_TILE", "G31_TEX_GRID_COLS", "G31_TEX_PROBES_PER_SLOT"]


def extract_fn(text: str, name: str) -> str | None:
    m = re.search(rf"^fn {re.escape(name)}\(", text, re.M)
    if not m:
        return None
    body = text[m.start():]
    end = re.search(r"^\}", body, re.M)
    return body[: end.end()] if end else None


def extract_struct(text: str, name: str) -> str | None:
    m = re.search(rf"^struct {re.escape(name)} \{{", text, re.M)
    if not m:
        return None
    body = text[m.start():]
    end = re.search(r"^\}", body, re.M)
    return body[: end.end()] if end else None


def extract_const(text: str, name: str) -> str | None:
    m = re.search(rf"^const {re.escape(name)}: .*$", text, re.M)
    return m.group(0) if m else None


def main() -> int:
    cur = CUR.read_text(encoding="utf-8")
    orig = ORIG.read_text(encoding="utf-8")
    rows = []
    fails = 0
    for name in FNS:
        a, b = extract_fn(cur, name), extract_fn(orig, name)
        ok = a is not None and a == b
        rows.append({"kind": "fn", "name": name, "verbatim": ok,
                     "cur_found": a is not None, "orig_found": b is not None})
        fails += 0 if ok else 1
    for name in STRUCTS:
        a, b = extract_struct(cur, name), extract_struct(orig, name)
        ok = a is not None and a == b
        rows.append({"kind": "struct", "name": name, "verbatim": ok,
                     "cur_found": a is not None, "orig_found": b is not None})
        fails += 0 if ok else 1
    for name in CONSTS:
        a, b = extract_const(cur, name), extract_const(orig, name)
        ok = a is not None and a == b
        rows.append({"kind": "const", "name": name, "verbatim": ok, "cur": a, "orig": b})
        fails += 0 if ok else 1
    out = {"schema": "rurix.day0828.f_emissive.f6_verbatim.v1", "fails": fails, "rows": rows}
    (ROOT / "artifacts" / "day_0828" / "f_emissive" / "f6_verbatim_check.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(out, ensure_ascii=False, indent=1))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
