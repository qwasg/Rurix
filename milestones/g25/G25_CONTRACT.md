---
contract: G25
title: G25 全量商用终审收官期（画质终态维持核验 + 性能 18 格终判 + 全链零降级 + 战役承接锚归档）
status: active
implementation_status: unlocked
active_scope: g25_campaign_final_review
version: v1.0
date: 2026-08-24
timebox: "G25.1 治理波即刻执行（G24 已 closed，tag g24-closed）；G25.2~G25.4 严格波次 + G25.5 P2/soak + G25.6 close-out；用户「帮我一次性完成G19-G25」战役字面（G25 = 七期串行战役收官期，里程碑间不越级）"
rfc_required: "G25.1 领取 RFC-0042（实测 RFC next_free=42）——战役终审程序：画质终态维持核验 + fps 18 格终判（两态合法）+ 全链零降级 + 承接锚归档闭集；经对抗评审后 Agent Approved 方可实现；达标/诚实红均合法终态。"
upstream_docs:
  - "milestones/g24/G24_P2_DECISIONS.md §1 + G24_CONTRACT.md §8.7 承接锚"
  - "milestones/g18/G18_CONTRACT.md §8.7（M-d 画质终审达标 + M-f 17/18 诚实红终值 0.856326）"
  - "milestones/g19/G19_P2_DECISIONS.md（fps 终判归 G25 字面）+ registry/deferred.json 全量"
implementation_unlock:
  required_all:
    - "G25.1 治理门全部完成且有真实验证记录"
    - "ci/g25_interlock_check.py --require-ready 输出 READY"
    - "用户 G25.2 开工指令留痕（「帮我一次性完成G19-G25」战役字面）"
in_scope:
  - g25_1_governance_only
  - quality_final_state_verification
  - fps_parity_final_verdict
  - campaign_full_chain_no_regression
  - campaign_handover_ledger
  - closed_gate_no_regression
  - g25_p2_decisions_soak_closeout_tag
out_of_scope:
  - new_optimization_or_feature_work
  - ue_full_render_rerun_without_surface_change_evidence
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g24_frozen_registries
deliverables:
  - id: D-G25-1
    check: "G25.1 四件套 + 候选决策 7 行 + 验收映射 5 P0 + RFC-0042 + 治理三门 429/430/431"
acceptance_gates:
  - id: G-G25-1
    check: "G25.1 完成门：D-G25-1 齐备；治理三门 PASS"
  - id: G-G25-2
    check: "互锁门：ci/g25_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G25-3
    check: "G25.2 退出门：M-a/M-b P0 全绿"
  - id: G-G25-4
    check: "G25.3 退出门：M-c/M-d P0 全绿"
  - id: G-G25-5
    check: "G25.4 退出门：M-e P0 全绿"
  - id: G-G25-6
    check: "P2 + soak + close-out → tag g25-closed（战役收官）"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G25-2"
  - "终审零冒充：画质/性能终态以机器事实定盘（表面 0-byte 证明 + 最新绿件 + 焦点格新鲜实测），达标/诚实红均合法"
  - "战役期加性纪律回验：G19~G24 全部新增面对默认臂/冻结面 0-byte（git-diff 机核）"
  - "no-go/defer/not-available/maintain/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G25 全量商用终审收官期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G19-G25**（2026-08-24，七期串行战役全期授权）。G25 = 战役收官期：双端商用画质/性能终态定盘（G18 达标面维持核验 + fps 17/18 诚实红终判两态程序）、G13~G24 全链零降级、战役承接锚归档闭集（G26+ 法定输入面）。

G25.0 不可变 ref = `4b65631c354340d3b7359a5b2561e57897e982e2`（G24 close-out flip commit，tag `g24-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G25.1 | 治理波 + RFC-0042 起草 + baseline 快检 | G-G25-1 |
| 互锁 | `--require-ready` READY | G-G25-2 |
| G25.2 | M-a 画质终态维持核验 + M-b fps 18 格终判 | G-G25-3 |
| G25.3 | M-c 全链零降级 + M-d 承接锚归档 | G-G25-4 |
| G25.4 | M-e 旧门零降级（全量测试波） | G-G25-5 |
| G25.5~6 | P2/soak/close-out/tag（战役收官） | G-G25-6 |

## 3. 治理波交付物

D-G25-1：PLAN/CONTRACT/CI_GATES/g25_budget.json + G25_CANDIDATE_DECISIONS + G25_ACCEPTANCE_MAP + RFC-0042 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | 画质终态维持核验：G18 M-d 商用画质终审达标绿件只读盘点 + 战役期画质表面 0-byte 机核（presentation/显示链/默认渲染臂 vs g18-closed git-diff 闭集）+ G19~G24 加性面零接线核验；维持达标终态/降级检出均如实登记 | G25.2 |
| **M-b** | fps 18 格终判兑现：G14 M-d 最新 18 格 evidence 如实定盘 + 性能面 0-byte 机核（g14_3_pipeline_perf 全战役 0-byte）+ 焦点格新鲜单测真跑（bistro-interior/t100/dlss_sr canonical 160 帧 bench 一轮 ratio 登记）；≥1.00 → 18/18 或物理不可达维持 **17/18 诚实红终判**（两态均为战役合法收官态，G15 兜底同源） | G25.2 |
| **M-c** | 战役全链零降级：G24 受影响门 `--verify-latest` 全绿（递归链自动涵盖 G13~G23）+ budget_eval --strict 全量零 skip 零 estimated；禁 `--gate` 旧脚本 | G25.3 |
| **M-d** | 战役承接锚归档闭集：g25_campaign_handover_registry.json（七期 defer/maintain 行 + RD 八条 + 历史清册十一条 + SAFE-GPU 处置 + RD-045 累计观察复核）全量汇总闭集登记——G26+ 法定输入面；归档完整性机核 | G25.3 |
| **M-e** | G24 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g25_` 前缀不抢 latest | G25.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g25.wave.1.acceptance_map` | `ci/g25_acceptance_map_check.py` | 429 |
| 候选决策 | `g25.wave.1.candidate_decisions` | `ci/g25_candidate_decisions_check.py` | 430 |
| 互锁 | `g25.gov.implementation_interlock` | `ci/g25_interlock_check.py` | 431 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G25_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G24 defer-to-G25+ 行（SAFE-GPU）+ fps 终判锚（G17-MD-F1 链，G19 M-d「终判归 G25」字面）本波逐行 disposition（候选决策表 §1 两行）。
2. 主轨 = 终审四面（画质维持/性能终判/全链零降级/承接锚归档）。
3. RFC-0042 本波起草 + 对抗评审。
4. 治理三门步骤 429/430/431 顺位领取（落盘前实测 CI_step.next_free=429）。
5. 用户战役指令「帮我一次性完成G19-G25」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先优化后测试：G25.2~G25.3 纯核验；G25.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G25-2 implementation_status 解锁记录（2026-08-24）

- **事实门全绿**：G24 closed + tag `g24-closed` + G25.0 不可变 ref `4b65631c354340d3b7359a5b2561e57897e982e2`；候选表 7 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G19-G25」字面 + workflow 末号 431 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g25_interlock_check.py --gate g25.gov.implementation_interlock` VERDICT=READY；治理三门 429/430/431 PASS。
- **解锁**：`implementation_status: blocked → unlocked`。G25.2+ 实现波（M-a~M-e）现可开工。
