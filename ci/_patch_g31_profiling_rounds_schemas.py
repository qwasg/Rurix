#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G39 T4：g31 profiling 门 evidence schema 纯追加可选块 identity_rounds
# （identity 判据多轮中位鲁棒化的逐轮明细登记面）。本补丁只动
# milestones/g31/g31_profiling_evidence_schema.json 一个文件,且为可选字段
# 追加——required 15 闭集不变,存量 PASS evidence（无该字段）免疫;判据规则/
# 容差 [−0.10,2.00] 四面同源（脚本常量/profile 输出 schema/双 bin 字面/docs）
# 0-byte 零触碰,本件不在四面之列。
# 范式 = ci/_patch_g31_wp_hlod_schemas.py：io.open newline="" 字节面保全;
# 锚 = 当前在树字节文本（唯一性机核,≠1 拒改）;token 驻留 0/1 判定;插入后
# json.loads 自检 + 结构自检 + 重读验证;幂等可复跑（已驻留即只核验）。
# 注:名 ci/_patch_g31_profiling_schemas.py 已被 C7 check_schemas 注册补丁
# 占用（历史件不可覆写）,本件按 _patch_g31_sdk_dist_v2 同主题二号补丁先例另名。
import io
import json
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent.parent
        / "milestones" / "g31" / "g31_profiling_evidence_schema.json")

TOKEN = '"identity_rounds": {'

ANCHOR = '''  "zero_drift": {
   "type": "object",
'''

ADD = '''  "identity_rounds": {
   "description": "G39 T4 identity 多轮中位鲁棒化逐轮明细（可选纯追加块;判据消费 N 轮 gpu_sum/render_wall/host_residual mean 分量中位数,中位裁决落 profiles/*/identity_ok const true;逐轮 identity_ok 为 boolean 可红如实登记,不改判据规则与容差字面）",
   "type": "object",
   "required": [
    "rounds",
    "g31",
    "g14"
   ],
   "properties": {
    "rounds": {
     "type": "integer",
     "minimum": 1,
     "maximum": 9
    },
    "g31": {
     "type": "array",
     "minItems": 1,
     "maxItems": 9,
     "items": {
      "type": "object",
      "required": [
       "gpu_sum_mean_ms",
       "render_wall_mean_ms",
       "host_residual_mean_ms",
       "identity_ok"
      ],
      "properties": {
       "gpu_sum_mean_ms": {
        "type": "number"
       },
       "render_wall_mean_ms": {
        "type": "number"
       },
       "host_residual_mean_ms": {
        "type": "number"
       },
       "identity_ok": {
        "type": "boolean"
       }
      }
     }
    },
    "g14": {
     "type": "array",
     "minItems": 1,
     "maxItems": 9,
     "items": {
      "type": "object",
      "required": [
       "gpu_sum_mean_ms",
       "render_wall_mean_ms",
       "host_residual_mean_ms",
       "identity_ok"
      ],
      "properties": {
       "gpu_sum_mean_ms": {
        "type": "number"
       },
       "render_wall_mean_ms": {
        "type": "number"
       },
       "host_residual_mean_ms": {
        "type": "number"
       },
       "identity_ok": {
        "type": "boolean"
       }
      }
     }
    }
   }
  },
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    n = src.count(TOKEN)
    if n == 1:
        print("[patch_g31_profiling_rounds] identity_rounds 已驻留（幂等跳过插入,只核验）")
    elif n != 0:
        print(f"[patch_g31_profiling_rounds] FAIL: token 驻留数 {n}（≠0≠1 拒改）", file=sys.stderr)
        return 1
    else:
        anchor, add = ANCHOR, ADD
        if "\r\n" in src:  # 字节面保全:目标为 CRLF 时同形插入（当前在树为 LF）。
            anchor = anchor.replace("\n", "\r\n")
            add = add.replace("\n", "\r\n")
        if src.count(anchor) != 1:
            print(f"[patch_g31_profiling_rounds] FAIL: 锚出现 {src.count(anchor)} 次（≠1 拒改）",
                  file=sys.stderr)
            return 1
        src = src.replace(anchor, add + anchor, 1)
        print("[patch_g31_profiling_rounds] identity_rounds 插入完成（zero_drift 锚前纯追加）")

    # 驻留 + json 可 load + 结构自检（含容差同源面未动证明）。
    if src.count(TOKEN) != 1:
        print(f"[patch_g31_profiling_rounds] FAIL: 驻留核验 token 数 {src.count(TOKEN)} ≠ 1",
              file=sys.stderr)
        return 1
    try:
        doc = json.loads(src)
    except json.JSONDecodeError as e:
        print(f"[patch_g31_profiling_rounds] FAIL: json 不可 load {e}", file=sys.stderr)
        return 1
    ir = doc.get("properties", {}).get("identity_rounds", {})
    checks = [
        ("identity_rounds 在 properties", bool(ir)),
        ("identity_rounds 非 required（纯追加可选）", "identity_rounds" not in doc.get("required", [])),
        ("required 闭集 15 不变", len(doc.get("required", [])) == 15),
        ("rounds 闭集 [1,9]",
         ir.get("properties", {}).get("rounds", {}).get("minimum") == 1
         and ir.get("properties", {}).get("rounds", {}).get("maximum") == 9),
        ("逐轮行 required 四键",
         all(sorted(ir.get("properties", {}).get(k, {}).get("items", {}).get("required", []))
             == ["gpu_sum_mean_ms", "host_residual_mean_ms", "identity_ok", "render_wall_mean_ms"]
             for k in ("g31", "g14"))),
        ("中位裁决面未动:profiles.g31.identity_ok const true",
         doc["properties"]["profiles"]["properties"]["g31"]["properties"]["identity_ok"] == {"const": True}),
        ("中位裁决面未动:profiles.g14.identity_ok const true",
         doc["properties"]["profiles"]["properties"]["g14"]["properties"]["identity_ok"] == {"const": True}),
    ]
    bad = [name for name, ok in checks if not ok]
    if bad:
        for b in bad:
            print(f"[patch_g31_profiling_rounds] FAIL: 自检 {b}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    # 立即重读验证驻留 + 可 load（并发同窗改写面下重读为最终裁决）。
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    if back.count(TOKEN) != 1:
        print("[patch_g31_profiling_rounds] FAIL: 重读未驻留", file=sys.stderr)
        return 1
    try:
        json.loads(back)
    except json.JSONDecodeError as e:
        print(f"[patch_g31_profiling_rounds] FAIL: 重读 json 不可 load {e}", file=sys.stderr)
        return 1
    print("[patch_g31_profiling_rounds] PASS：identity_rounds 可选块驻留 + json load/结构自检 + 重读核验绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
