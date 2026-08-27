#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G31+ 波 C Task C10：registry/deferred.json RD-027 history 只追加（CRLF 补丁法——
# io.open newline="" 字节面保全，条目其余部分 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：本批 history 行已驻留即只做核验（_patch_g31_rd027_schemas.py 同法）。
# 纪律：status 维持 open、backfill_condition 原文 0-byte（绕行登记非修复确证，
# 不预支升档回填）；只追加 history 一行。
import io
import json
import sys

P = r"h:\rurix\registry\deferred.json"

NEW_ITEM = (
    '        },\r\n'
    '        {\r\n'
    '          "date": "2026-08-26",\r\n'
    '          "event": "G31+ 波 C Task C10 毒径定位修复批处置（agent 完全自主 D-406 v3.0）：'
    '(一) E8 全网格实测毒区图落盘（spp∈{8,32,64,128,256}×bounces∈{1,2,3,4} 20 格,'
    '生产档 720p/N=131072/REND_FRAMES=1/SUBSTEPS=4 sim 后数据,proc_guard 60s 判定线+挂起后金丝雀门）'
    '——b1 全 spp 绿;b2 spp≤64 绿/≥128 毒;b3·b4 全 spp 毒;distinct PTX digests=1 单 artifact 复确认;'
    'G3.1 事实矩阵五格 (8,2)(32,2) 绿/(8,3)(8,4)(256,2) 挂全复现零漂移。'
    '(二) 根因层复确认：判别腿 (8,3) @O1 挂/@O0 完成 = O0→O1 绿挂分界在当前工具链'
    '（ptxas 13.3 V13.3.33/driver 620.02）维持,归因 nvidia_optimizing_backends（层③④定罪,M1′ 机理）不动摇;'
    '根因修复 = 上游 NVIDIA 本体,本仓不可修（双装载路同挂在案,上游 DRAFT 备包维持 do-NOT-file owner 复核门）。'
    '(三) 落档绕行登记：MR-0011 RURIXC_PTXAS_OPT=0 护栏常驻（(8,3)(8,4)(256,2) 护栏腿终止+digest 基线在图）'
    '+ 毒区参数面 fail-closed 拒绝（未测绘组合按毒处理）经新门 g31.waveC.rd027 常驻回归'
    '（ci/g31_rd027_poison_guard.py：静态 fail-closed 判 params.rx 切片 + 边界绿腿 digest 命中 + '
    '护栏双腿终止 + 毒确认腿 hang_timeout 维持;三态 DEV_ENV_DEGRADE;evidence 经 check_schemas 路由）。'
    '**维持 open**（绕行登记非修复确证;backfill_condition 原文 0-byte 不预支——上游修复落地或 '
    'owner 另裁护栏档口径后按字面兑现 256spp/4 弹射升档重测回填）",\r\n'
    '          "evidence": "milestones/g31/g31_rd027_poison_zone_map.json / '
    'ci/g31_rd027_poison_guard.py / evidence/g31_rd027_poison_guard_20260826*.json / '
    'milestones/g31/g31_rd027_poison_guard_evidence_schema.json / milestones/g31/CI_GATES.md §2.17 / '
    'build/spike-rd027/campaign.jsonl（E8 逐 run 工件,不入库）"\r\n'
    '        }\r\n'
)

# RD-027 history 数组收尾 → RD-029 条目头的唯一字节锚（CRLF 面）。
ANCHOR = (
    '        }\r\n'
    '      ]\r\n'
    '    },\r\n'
    '    {\r\n'
    '      "id": "RD-029",'
)
TOKEN = "g31.waveC.rd027 常驻回归"


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # 切片定位（RD-027 条目体;幂等核验只在切片内计数,他条目/文档同串不受累）。
    i = src.index('"id": "RD-027"')
    j = src.index('"id": "RD-029"')
    seg = src[i:j]
    n_tok = seg.count(TOKEN)
    if n_tok == 1:
        print("[patch_rd027_deferred] history 已驻留（幂等跳过插入）")
    elif n_tok == 0:
        if src.count(ANCHOR) != 1:
            print(f"[patch_rd027_deferred] FAIL: 锚出现 {src.count(ANCHOR)} 次（≠1 拒改）",
                  file=sys.stderr)
            return 1
        src = src.replace(ANCHOR, NEW_ITEM + '      ]\r\n    },\r\n    {\r\n      "id": "RD-029",', 1)
        with io.open(P, "w", encoding="utf-8", newline="") as f:
            f.write(src)
        print("[patch_rd027_deferred] history 插入完成")
    else:
        print(f"[patch_rd027_deferred] FAIL: token 驻留数 {n_tok}（≠0≠1 拒改）", file=sys.stderr)
        return 1

    # 立即重读验证驻留 + JSON 结构核验 + RD-027 面 0-byte 核验（status/backfill 原文）。
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    doc = json.loads(back)
    rd = next(e for e in doc["entries"] if e["id"] == "RD-027")
    if rd["status"] != "open":
        print("[patch_rd027_deferred] FAIL: status ≠ open", file=sys.stderr)
        return 1
    if not any(TOKEN in h.get("event", "") for h in rd["history"]):
        print("[patch_rd027_deferred] FAIL: 重读未驻留", file=sys.stderr)
        return 1
    if "修复后把 ms1.bench.uc07_offline_frame_s" not in rd["backfill_condition"]:
        print("[patch_rd027_deferred] FAIL: backfill_condition 原文漂移", file=sys.stderr)
        return 1
    print(f"[patch_rd027_deferred] PASS：RD-027 history 只追加驻留（共 {len(rd['history'])} 行）"
          "+ status=open + backfill 原文 0-byte + JSON 结构绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
