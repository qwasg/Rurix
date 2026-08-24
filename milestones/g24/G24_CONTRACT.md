---
contract: G24
title: G24 呈现与尾门清理期（毛发精确 OIT 重判 + HDR 标定重判 + BistroExterior 转换臂复查 + SAFE-GPU/历史 RD 清册）
status: active
implementation_status: unlocked
active_scope: g24_presentation_tail_gate_cleanup
version: v1.0
date: 2026-08-24
timebox: "G24.1 治理波即刻执行（G23 已 closed，tag g23-closed）；G24.2~G24.4 严格波次 + G24.5 P2/soak + G24.6 close-out；用户「帮我一次性完成G19-G25」战役字面（G24 = 七期串行战役第六期，里程碑间不越级）"
rfc_required: "G24.1 领取 RFC-0041（实测 RFC next_free=41）——呈现与尾门清理：M114-strand/M118-hdr-cal/G10-N6 三行机器取证重判 + SAFE-GPU 立项评估处置 + 历史 open RD 清册重判程序；经对抗评审后 Agent Approved 方可实现；maintain/go/defer 均合法终态。"
upstream_docs:
  - "milestones/g23/G23_P2_DECISIONS.md §1 defer-to-G24+ 四行 + G23_CONTRACT.md §8.7 承接锚"
  - "evidence/g9_m120_oit_benchmark_*.json（M120 七算法对照 measured 裁决数据，只读消费）"
  - "registry/deferred.json 历史 open RD（RD-007/011/012/014/015/026/027/030/032/033/036）"
implementation_unlock:
  required_all:
    - "G24.1 治理门全部完成且有真实验证记录"
    - "ci/g24_interlock_check.py --require-ready 输出 READY"
    - "用户 G24.2 开工指令留痕（「帮我一次性完成G19-G25」战役字面）"
in_scope:
  - g24_1_governance_only
  - hair_strand_oit_rejudgment
  - hdr_calibration_rejudgment
  - bistro_exterior_conversion_rejudgment
  - safe_gpu_and_legacy_rd_disposition
  - closed_gate_no_regression
  - g24_p2_decisions_soak_closeout_tag
out_of_scope:
  - hair_strand_precise_oit_implementation
  - hdr_display_pipeline_implementation
  - bistro_exterior_asset_conversion_execution
  - safe_gpu_platform_implementation
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g23_frozen_registries
deliverables:
  - id: D-G24-1
    check: "G24.1 四件套 + 候选决策 9 行 + 验收映射 5 P0 + RFC-0041 + 治理三门 413/414/415"
acceptance_gates:
  - id: G-G24-1
    check: "G24.1 完成门：D-G24-1 齐备；治理三门 PASS"
  - id: G-G24-2
    check: "互锁门：ci/g24_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G24-3
    check: "G24.2 退出门：M-a/M-b P0 全绿"
  - id: G-G24-4
    check: "G24.3 退出门：M-c/M-d P0 全绿"
  - id: G-G24-5
    check: "G24.4 退出门：M-e P0 全绿"
  - id: G-G24-6
    check: "P2 + soak + close-out → tag g24-closed"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G24-2"
  - "旧门只读消费：g9_m120 绿件禁 --gate 重跑（g24_ 前缀不抢 latest）"
  - "历史 RD 清册只追加：status 翻转仅当 backfill 条件字面成立，禁静默改判"
  - "no-go/defer/not-available/maintain 均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G24 呈现与尾门清理期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G19-G25**（2026-08-24，七期串行战役全期授权）。G24 = 战役第六期：G18 承接池最后四行（M114-strand/M118-hdr-cal/G10-N6/SAFE-GPU）机器取证重判 + 历史 open RD 全量清册逐条重判（G25 全量终审前的尾门清理）。

G24.0 不可变 ref = `2e3e8ae2d1f59a0752ad66ab359bd77512e69d18`（G23 close-out flip commit，tag `g23-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G24.1 | 治理波 + RFC-0041 起草 + baseline 快检 | G-G24-1 |
| 互锁 | `--require-ready` READY | G-G24-2 |
| G24.2 | M-a 毛发精确 OIT 重判 + M-b HDR 标定重判 | G-G24-3 |
| G24.3 | M-c BistroExterior 转换臂复查 + M-d SAFE-GPU/历史 RD 清册 | G-G24-4 |
| G24.4 | M-e 旧门零降级（全量测试波） | G-G24-5 |
| G24.5~6 | P2/soak/close-out/tag | G-G24-6 |

## 3. 治理波交付物

D-G24-1：PLAN/CONTRACT/CI_GATES/g24_budget.json + G24_CANDIDATE_DECISIONS + G24_ACCEPTANCE_MAP + RFC-0041 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | M114-strand 重判兑现：M120 七算法 OIT benchmark 裁决数据只读盘点（measured 绿件在案性核验）+ strand 档生产需求面核验（压测闭集毛发资产存在性）；两半分别登记；maintain-card-mesh/go 均合法终态 | G24.2 |
| **M-b** | M118-hdr-cal 重判兑现：HDR 设备面实测（vulkaninfo 表面色彩空间枚举取证落 g24_hdr_probe_results.json）+ HDR 资产/产品需求面核验；两半分别登记；maintain-SDR/go 均合法终态 | G24.2 |
| **M-c** | G10-N6 重判兑现：FBX2glTF/替代转换臂工具链在树性实测 + BistroExterior 源资产在树性核验 + 场景闭集裁决登记 g24_bistro_exterior_recheck.json；maintain-双场景闭集/go 均合法终态 | G24.3 |
| **M-d** | SAFE-GPU 立项评估处置 + 历史 open RD 清册逐条重判：RD-007/011/012/014/015/026/027/030/032/033/036 十一条逐条 disposition 闭集登记 g24_legacy_rd_registry.json + 逐条 history 只追加；maintain/close/inherit 均合法终态 | G24.3 |
| **M-e** | G23 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g24_` 前缀不抢 latest | G24.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g24.wave.1.acceptance_map` | `ci/g24_acceptance_map_check.py` | 413 |
| 候选决策 | `g24.wave.1.candidate_decisions` | `ci/g24_candidate_decisions_check.py` | 414 |
| 互锁 | `g24.gov.implementation_interlock` | `ci/g24_interlock_check.py` | 415 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G24_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G23 defer-to-G24+ 四行本波全部 go（M-a/M-b/M-c/M-d 承载——承接池清零窗）。
2. 主轨 = 四行机器取证重判 + 历史 open RD 全量清册（G25 终审前尾门清理）。
3. RFC-0041 本波起草 + 对抗评审。
4. 治理三门步骤 413/414/415 顺位领取（落盘前实测 CI_step.next_free=413）。
5. 用户战役指令「帮我一次性完成G19-G25」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先优化后测试：G24.2~G24.3 纯实现；G24.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G24-2 implementation_status 解锁记录（2026-08-24）

- **事实门全绿**：G23 closed + tag `g23-closed` + G24.0 不可变 ref `2e3e8ae2d1f59a0752ad66ab359bd77512e69d18`；候选表 9 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G19-G25」字面 + workflow 末号 415 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g24_interlock_check.py --gate g24.gov.implementation_interlock` VERDICT=READY；治理三门 413/414/415 PASS。
- **解锁**：`implementation_status: blocked → unlocked`。G24.2+ 实现波（M-a~M-e）现可开工。
