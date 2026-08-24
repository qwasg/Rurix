---
contract: G23
title: G23 物理平台深化期（Jolt 5.6 采纳臂重判 + 神经变形重判 + 研究轨/物理 P3+ 处置）
status: active
implementation_status: unlocked
active_scope: g23_physics_platform_deepening
version: v1.0
date: 2026-08-24
timebox: "G23.1 治理波即刻执行（G22 已 closed，tag g22-closed）；G23.2~G23.4 严格波次 + G23.5 P2/soak + G23.6 close-out；用户「帮我一次性完成G19-G25」战役字面（G23 = 七期串行战役第五期，里程碑间不越级）"
rfc_required: "G23.1 领取 RFC-0040（实测 RFC next_free=40）——物理平台深化：Jolt 5.6 采纳臂重判 + M127 神经变形两半实测重判 + RD-042/043/044 处置程序；经对抗评审后 Agent Approved 方可实现；maintain/adopt/go/defer 均合法终态。"
upstream_docs:
  - "milestones/g22/G22_P2_DECISIONS.md §1 defer-to-G23+ 六行 + G22_CONTRACT.md §8.7 承接锚"
  - "registry/deferred.json RD-042/RD-043/RD-044（物理研究轨与 P3+ 长线）"
  - "milestones/g9/g9_m125_jolt_56_ab_evaluation（5.6 评估臂 A/B 绿件，只读消费）"
implementation_unlock:
  required_all:
    - "G23.1 治理门全部完成且有真实验证记录"
    - "ci/g23_interlock_check.py --require-ready 输出 READY"
    - "用户 G23.2 开工指令留痕（「帮我一次性完成G19-G25」战役字面）"
in_scope:
  - g23_1_governance_only
  - jolt_56_adoption_rejudgment
  - neural_deform_rejudgment
  - research_track_disposition
  - physics_p3_subitem_disposition
  - closed_gate_no_regression
  - g23_p2_decisions_soak_closeout_tag
out_of_scope:
  - jolt_56_production_default_flip_without_demand_evidence
  - soft_body_cloth_fluid_implementation
  - taichi_mpm_production_import
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g22_frozen_registries
deliverables:
  - id: D-G23-1
    check: "G23.1 四件套 + 候选决策 11 行 + 验收映射 5 P0 + RFC-0040 + 治理三门 397/398/399"
acceptance_gates:
  - id: G-G23-1
    check: "G23.1 完成门：D-G23-1 齐备；治理三门 PASS"
  - id: G-G23-2
    check: "互锁门：ci/g23_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G23-3
    check: "G23.2 退出门：M-a/M-b P0 全绿"
  - id: G-G23-4
    check: "G23.3 退出门：M-c/M-d P0 全绿"
  - id: G-G23-5
    check: "G23.4 退出门：M-e P0 全绿"
  - id: G-G23-6
    check: "P2 + soak + close-out → tag g23-closed"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G23-2"
  - "5.3 基线生产默认面 0-byte：rurix-physics-sys VENDOR.md pin 不动；sys56 评估臂不升格除非需求证据成立"
  - "旧门只读消费：g9_m125 绿件禁 --gate 重跑（g23_ 前缀不抢 latest）"
  - "no-go/defer/not-available/maintain 均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G23 物理平台深化期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G19-G25**（2026-08-24，七期串行战役全期授权）。G23 = 战役第五期：物理面三条 open RD（RD-042/043/044）与两行 defer（M125-adopt3/M127）的机器取证重判——Jolt 5.6 评估臂新鲜度真跑 + 采纳三件条件核验，神经变形两半条件实测，研究轨与 P3+ 分项处置闭集。

G23.0 不可变 ref = `1ac8b12956eced1a3a08e03c1f91aa7e0949b23c`（G22 close-out flip commit，tag `g22-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G23.1 | 治理波 + RFC-0040 起草 + baseline 快检 | G-G23-1 |
| 互锁 | `--require-ready` READY | G-G23-2 |
| G23.2 | M-a Jolt 5.6 采纳臂重判 + M-b 神经变形重判 | G-G23-3 |
| G23.3 | M-c 研究轨处置 + M-d 物理 P3+ 分项处置 | G-G23-4 |
| G23.4 | M-e 旧门零降级（全量测试波） | G-G23-5 |
| G23.5~6 | P2/soak/close-out/tag | G-G23-6 |

## 3. 治理波交付物

D-G23-1：PLAN/CONTRACT/CI_GATES/g23_budget.json + G23_CANDIDATE_DECISIONS + G23_ACCEPTANCE_MAP + RFC-0040 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | M125-adopt3 重判兑现：5.6 评估臂在树核验（rurix-physics-sys56 + VENDOR56.md）+ g9_m125 A/B 最新绿件只读盘点 + 评估臂构建新鲜真跑（cargo check -p rurix-physics-sys56）+ 采纳三件成立条件核验（生产切换需求证据面）；maintain-5.3/adopt 均合法终态，登记 g23_jolt_adoption_registry.json | G23.2 |
| **M-b** | M127 重判兑现：离线工具链 corpus 语料在树性实测 + PhysicsAsset residual 消费方存在性核验（两半分别登记）；maintain-研究子轨/go 均合法终态 | G23.2 |
| **M-c** | RD-042/RD-043 观察轨处置闭集：Newton/Genesis/MuJoCo-Warp/wgrapier 逐轨 disposition 登记 g23_research_track_registry.json + 两条 RD history 只追加；观察存续/关闭均合法终态 | G23.3 |
| **M-d** | RD-044 分项处置闭集：Jolt 软体/布料/流体、Taichi MPM、Rapier 快路径（M126 maintain-no-go 在案转引）三分项 disposition 登记 g23_rd044_subitem_registry.json + RD-044 history 只追加；go/no-go/defer 均合法终态 | G23.3 |
| **M-e** | G22 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g23_` 前缀不抢 latest | G23.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g23.wave.1.acceptance_map` | `ci/g23_acceptance_map_check.py` | 397 |
| 候选决策 | `g23.wave.1.candidate_decisions` | `ci/g23_candidate_decisions_check.py` | 398 |
| 互锁 | `g23.gov.implementation_interlock` | `ci/g23_interlock_check.py` | 399 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G23_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G22 defer-to-G23+ 六行本波逐行 disposition（候选决策表 §1 六行）。
2. 主轨 = Jolt 5.6 采纳臂机器取证重判（M-a）+ M127 两半实测（M-b）+ 三条物理 RD 处置闭集（M-c/M-d）。
3. RFC-0040 本波起草 + 对抗评审。
4. 治理三门步骤 397/398/399 顺位领取（落盘前实测 CI_step.next_free=397）。
5. 用户战役指令「帮我一次性完成G19-G25」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先优化后测试：G23.2~G23.3 纯实现；G23.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G23-2 implementation_status 解锁记录（2026-08-24）

- **事实门全绿**：G22 closed + tag `g22-closed` + G23.0 不可变 ref `1ac8b12956eced1a3a08e03c1f91aa7e0949b23c`；候选表 11 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G19-G25」字面 + workflow 末号 399 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g23_interlock_check.py --gate g23.gov.implementation_interlock` VERDICT=READY；治理三门 397/398/399 PASS。
- **解锁**：`implementation_status: blocked → unlocked`。G23.2+ 实现波（M-a~M-e）现可开工。
