#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G39 T1 lamp_restir:窗口 bin textures evidence 的 quality_arms 闭集枚举纯追加
# (milestones/g31/g31_texture_sampling_heap_evidence_schema.json——
# additionalProperties:false 面加 lamp_restir/lamp_restir_mcap 两 property;
# **不进 required**:evidence/ 既有归档件无此两键,check_schemas.py 全量复验
# 必须保绿——properties-only 追加令新旧两代 evidence 双绿)。
# io.open 补丁法 newline="" 字节面保全(_patch_g31_texture_heap_schemas.py 同
# 形);既有行 0-byte,幂等:已驻留即只做核验。禁改 ci/check_schemas.py 本体。
import io
import json
import sys
from pathlib import Path

P = str(
    Path(__file__).resolve().parent.parent
    / "milestones/g31/g31_texture_sampling_heap_evidence_schema.json"
)

ANCHOR_LINES = [
    '            "gi2_clamp": { "type": "number" }',
    "          }",
    "        },",
    "",
]
ADD_LINES = [
    '            "gi2_clamp": { "type": "number" },',
    '            "lamp_restir": { "type": "boolean" },',
    '            "lamp_restir_mcap": { "type": "integer" }',
    "          }",
    "        },",
    "",
]
TOKEN = '"lamp_restir": { "type": "boolean" }'


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # 换行面探测(文件为 CRLF/LF 皆容;锚/插入体按实际换行拼装,字节面保全)。
    nl = "\r\n" if "\r\n" in src else "\n"
    anchor = nl.join(ANCHOR_LINES)
    add = nl.join(ADD_LINES)

    n = src.count(TOKEN)
    if n == 1:
        print("[patch_g31_window_evidence] quality_arms 两 property 已驻留（幂等跳过插入）")
    elif n != 0:
        print(f"[patch_g31_window_evidence] FAIL: token 驻留数 {n}（≠0≠1 拒改）", file=sys.stderr)
        return 1
    else:
        if src.count(anchor) != 1:
            print(
                f"[patch_g31_window_evidence] FAIL: 锚出现 {src.count(anchor)} 次（≠1 拒改）",
                file=sys.stderr,
            )
            return 1
        src = src.replace(anchor, add, 1)
        with io.open(P, "w", encoding="utf-8", newline="") as f:
            f.write(src)
        print("[patch_g31_window_evidence] quality_arms 两 property 插入完成")

    # 重读核验:token 驻留 + JSON 可解析 + required 不含新键(旧档保绿判据)
    # + properties 恰含新键 + additionalProperties 仍 false。
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    if back.count(TOKEN) != 1:
        print("[patch_g31_window_evidence] FAIL: 重读未驻留 token", file=sys.stderr)
        return 1
    doc = json.loads(back)
    qa = doc["properties"]["textures"]["properties"]["quality_arms"]
    misses = []
    if "lamp_restir" in qa["required"] or "lamp_restir_mcap" in qa["required"]:
        misses.append("新键误入 required（旧档 evidence 必红,拒）")
    for k, t in [("lamp_restir", "boolean"), ("lamp_restir_mcap", "integer")]:
        if qa["properties"].get(k, {}).get("type") != t:
            misses.append(f"properties.{k} 缺失或 type ≠ {t}")
    if qa.get("additionalProperties") is not False:
        misses.append("additionalProperties 不为 false（闭集面破坏）")
    if misses:
        for m in misses:
            print(f"[patch_g31_window_evidence] FAIL {m}", file=sys.stderr)
        return 1
    print(
        "[patch_g31_window_evidence] PASS：quality_arms +lamp_restir/lamp_restir_mcap"
        "（properties-only 纯追加,required 不动,JSON 解析绿）"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
