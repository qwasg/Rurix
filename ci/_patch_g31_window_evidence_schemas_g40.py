#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G40 T1:窗口 bin textures evidence schema 纯追加二号补丁(G39
# _patch_g31_window_evidence_schemas.py 同律;milestones/g31/
# g31_texture_sampling_heap_evidence_schema.json):
#   ① quality_arms(additionalProperties:false)+lamp_restir_clamp/
#     lamp_restir_depth_rej/lamp_restir_nrm_rej(number)+
#     lamp_restir_verify(integer)——**不进 required**(旧档 evidence 无此
#     四键保绿;新档恒发射,off 面 = 缺省字面值,G39 mcap 恒发射先例)。
#   ② textures.properties +lamp_restir_verify_stats(["object","null"] 闭形;
#     off/非 verify 跑 = null——**不进 required** 同律)。
#     〔v2,2026-09-02 B3 run2 首红后两级判据升级:y_mismatch 由 const 0 放开为
#     ≥0 边界事件计数;新增 y_attributed(≥0)/ y_unattributed(const 0 = 硬门)/
#     margin_abs_p100(≥0)/ ulp_bound(>0),全部进该块 required。v1 块从未入库
#     (本役工作树内产物),施用方式 = schema 文件 git checkout 还原至 HEAD 后
#     重跑本补丁,一号补丁形态不留档。〕
# io.open newline="" 字节面保全;幂等:已驻留即只核验;禁改 check_schemas 本体。
import io
import json
import sys
from pathlib import Path

P = str(
    Path(__file__).resolve().parent.parent
    / "milestones/g31/g31_texture_sampling_heap_evidence_schema.json"
)

QA_ANCHOR = [
    '            "lamp_restir": { "type": "boolean" },',
    '            "lamp_restir_mcap": { "type": "integer" }',
]
QA_ADD = [
    '            "lamp_restir": { "type": "boolean" },',
    '            "lamp_restir_mcap": { "type": "integer" },',
    '            "lamp_restir_clamp": { "type": "number" },',
    '            "lamp_restir_depth_rej": { "type": "number" },',
    '            "lamp_restir_nrm_rej": { "type": "number" },',
    '            "lamp_restir_verify": { "type": "integer" }',
]
QA_TOKEN = '"lamp_restir_clamp": { "type": "number" }'

ST_ANCHOR = ['        "spv_texture": {']
ST_ADD = [
    '        "lamp_restir_verify_stats": {',
    '          "type": ["object", "null"],',
    '          "additionalProperties": false,',
    '          "required": ["frames", "pixels", "hit_pixels", "merged_pixels", "y_mismatch", "y_attributed", "y_unattributed", "margin_abs_p100", "ulp_bound", "m_mismatch", "wsum_absdiff_p100", "w_absdiff_p100"],',
    '          "properties": {',
    '            "frames": { "type": "integer", "minimum": 0 },',
    '            "pixels": { "type": "integer", "minimum": 0 },',
    '            "hit_pixels": { "type": "integer", "minimum": 0 },',
    '            "merged_pixels": { "type": "integer", "minimum": 0 },',
    '            "y_mismatch": { "type": "integer", "minimum": 0 },',
    '            "y_attributed": { "type": "integer", "minimum": 0 },',
    '            "y_unattributed": { "type": "integer", "const": 0 },',
    '            "margin_abs_p100": { "type": "number", "minimum": 0 },',
    '            "ulp_bound": { "type": "number", "exclusiveMinimum": 0 },',
    '            "m_mismatch": { "type": "integer", "minimum": 0 },',
    '            "wsum_absdiff_p100": { "type": "number", "minimum": 0 },',
    '            "w_absdiff_p100": { "type": "number", "minimum": 0 }',
    "          }",
    "        },",
    '        "spv_texture": {',
]
ST_TOKEN = '"lamp_restir_verify_stats": {'
ST_V2_TOKEN = '"y_unattributed": { "type": "integer", "const": 0 }'


def patch(src: str, nl: str, anchor_lines: list[str], add_lines: list[str],
          token: str, tag: str) -> str | None:
    anchor = nl.join(anchor_lines)
    add = nl.join(add_lines)
    n = src.count(token)
    if n == 1:
        print(f"[patch_g40] {tag} 已驻留（幂等跳过插入）")
        return src
    if n != 0:
        print(f"[patch_g40] FAIL: {tag} token 驻留数 {n}（≠0≠1 拒改）", file=sys.stderr)
        return None
    if src.count(anchor) != 1:
        print(f"[patch_g40] FAIL: {tag} 锚出现 {src.count(anchor)} 次（≠1 拒改）",
              file=sys.stderr)
        return None
    print(f"[patch_g40] {tag} 插入完成")
    return src.replace(anchor, add, 1)


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()
    nl = "\r\n" if "\r\n" in src else "\n"
    orig = src
    for anchor, add, token, tag in [
        (QA_ANCHOR, QA_ADD, QA_TOKEN, "quality_arms 四 property"),
        (ST_ANCHOR, ST_ADD, ST_TOKEN, "lamp_restir_verify_stats property"),
    ]:
        out = patch(src, nl, anchor, add, token, tag)
        if out is None:
            return 1
        src = out
    if src != orig:
        with io.open(P, "w", encoding="utf-8", newline="") as f:
            f.write(src)

    # 重读核验:JSON 可解析 + 新键不入 required + properties 闭集形状。
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        doc = json.loads(f.read())
    tex = doc["properties"]["textures"]
    qa = tex["properties"]["quality_arms"]
    misses = []
    for k, t in [
        ("lamp_restir_clamp", "number"),
        ("lamp_restir_depth_rej", "number"),
        ("lamp_restir_nrm_rej", "number"),
        ("lamp_restir_verify", "integer"),
    ]:
        if k in qa["required"]:
            misses.append(f"{k} 误入 required（旧档必红,拒）")
        if qa["properties"].get(k, {}).get("type") != t:
            misses.append(f"quality_arms.properties.{k} 缺失或 type ≠ {t}")
    vs = tex["properties"].get("lamp_restir_verify_stats")
    if "lamp_restir_verify_stats" in tex["required"]:
        misses.append("verify_stats 误入 textures.required（旧档必红,拒）")
    if not vs or vs.get("type") != ["object", "null"]:
        misses.append("lamp_restir_verify_stats 缺失或 type ≠ [object,null]")
    else:
        # v2 形核验(v1 块驻留 = 一号补丁产物未还原:按头注施用方式 git checkout 后重跑)。
        if ST_V2_TOKEN not in src:
            misses.append("verify_stats 驻留块为 v1 形（y_unattributed 缺失）——先 git checkout 还原 schema 再重跑本补丁")
        for k in ("y_mismatch", "y_attributed", "y_unattributed", "margin_abs_p100", "ulp_bound"):
            if k not in vs.get("required", []):
                misses.append(f"verify_stats.required 缺 {k}")
        if vs.get("properties", {}).get("y_unattributed", {}).get("const") != 0:
            misses.append("verify_stats.y_unattributed 非 const 0（硬门语义破坏）")
        if vs.get("properties", {}).get("y_mismatch", {}).get("minimum") != 0 or "const" in vs.get("properties", {}).get("y_mismatch", {}):
            misses.append("verify_stats.y_mismatch 应为 minimum 0 且无 const（边界事件计数语义）")
    if qa.get("additionalProperties") is not False or tex.get("additionalProperties") is not False:
        misses.append("additionalProperties 闭集面破坏")
    if misses:
        for m in misses:
            print(f"[patch_g40] FAIL {m}", file=sys.stderr)
        return 1
    print("[patch_g40] PASS：quality_arms +4 键 / textures +verify_stats（properties-only 纯追加,required 不动,JSON 解析绿）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
