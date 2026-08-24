---
contract: G19
title: G19 帧生成独立层兑现期（FG/MFG host 参考臂实现 + vendor disposition + RD-045 长窗观察）
status: active
implementation_status: unlocked
active_scope: g19_frame_generation_realization
version: v1.0
date: 2026-08-24
timebox: "G19.1 治理波即刻执行（G18 已 closed，tag g18-closed）；G19.2~G19.4 严格波次 + G19.5 P2/soak + G19.6 close-out；用户「帮我一次性完成G19-G25」战役字面（G19 = 七期串行战役第一期，里程碑间不越级）"
rfc_required: "G19.1 领取 RFC-0036（实测 RFC next_free=36）——帧生成独立层兑现（host 参考臂 + MFG 多档 + vendor disposition 三臂）；经对抗评审后 Agent Approved 方可实现；RFC-0035 按只追加程序落重判记录；实现/no-go/defer 均合法终态。"
upstream_docs:
  - "milestones/g18/G18_P2_DECISIONS.md §1 defer-to-G19+ 九行 + G18_CONTRACT.md §8.7 承接锚"
  - "registry/deferred.json RD-045（间歇 digest 漂移生产化缺陷）"
  - "milestones/g14/g14_3_stage_a_digest_anchor.json（18 格 digest 冻结锚）"
implementation_unlock:
  required_all:
    - "G19.1 治理门全部完成且有真实验证记录"
    - "ci/g19_interlock_check.py --require-ready 输出 READY"
    - "用户 G19.2 开工指令留痕（「帮我一次性完成G19-G25」战役字面）"
in_scope:
  - g19_1_governance_only
  - frame_generation_host_realization
  - frame_generation_vendor_disposition
  - rd045_drift_observation_window
  - fps_parity_window_registration
  - closed_gate_no_regression
  - g19_p2_decisions_soak_closeout_tag
out_of_scope:
  - frame_generation_device_kernel_lane
  - mesh_shader_p4_hzb_cluster
  - restir_high_tier_reservoir
  - jolt_56_adoption_three_pieces
  - hdr_device_calibration_layer
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g18_frozen_registries
deliverables:
  - id: D-G19-1
    check: "G19.1 四件套 + 候选决策 14 行 + 验收映射 5 P0 + RFC-0036 + RFC-0035 重判记录 + 治理三门 333/334/335"
acceptance_gates:
  - id: G-G19-1
    check: "G19.1 完成门：D-G19-1 齐备；治理三门 PASS"
  - id: G-G19-2
    check: "互锁门：ci/g19_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G19-3
    check: "G19.2 退出门：M-a/M-b P0 全绿"
  - id: G-G19-4
    check: "G19.3 退出门：M-c P0 全绿"
  - id: G-G19-5
    check: "G19.4 退出门：M-d/M-e P0 全绿"
  - id: G-G19-6
    check: "P2 + soak + close-out → tag g19-closed"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G19-2"
  - "真实渲染帧率口径 0-byte：FG/MFG 生成帧禁计入真实渲染帧率与 upscale ratio；presented 口径独立登记面"
  - "默认臂 Stage A digest 锚 18 格零漂移红线；g14_3_pipeline_perf 本期 0-byte 只读消费"
  - "no-go/defer/not-available/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G19 帧生成独立层兑现期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G19-G25**（2026-08-24，七期串行战役全期授权；波次内可并行、里程碑间不越级）。G19 = 战役第一期：兑现 G18 唯一 M 级 defer（G13-N7 帧生成 FG/MFG 独立层，RFC-0035 重判条件命中——「RFC-0035 终态落档后按只追加程序重判」），host 参考臂真实实现 + vendor 三臂 disposition；同期消化 RD-045 长窗观察与 G17-MD-F1 重评窗登记。

G19.0 不可变 ref = `9dda737bca0b2026f1e9672c5e70f6b807c172b9`（G18 close-out flip commit，tag `g18-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G19.1 | 治理波 + RFC-0036 起草 + baseline 快检 | G-G19-1 |
| 互锁 | `--require-ready` READY | G-G19-2 |
| G19.2 | M-a FG host 参考臂实现 + M-b vendor disposition | G-G19-3 |
| G19.3 | M-c RD-045 长窗观察 | G-G19-4 |
| G19.4 | M-d fps 重评窗登记 + M-e 旧门零降级（全量测试波） | G-G19-5 |
| G19.5~6 | P2/soak/close-out/tag | G-G19-6 |

## 3. 治理波交付物

D-G19-1：PLAN/CONTRACT/CI_GATES/g19_budget.json + G19_CANDIDATE_DECISIONS + G19_ACCEPTANCE_MAP + RFC-0036 + RFC-0035 重判记录 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | FG/MFG 独立层 host 参考臂实现（mv 后向双向 warp + 遮挡感知混合 + MFG ×2/×3/×4 档）；插帧质量程序产对照阈（interp SSIM > frame-hold SSIM 逐帧，禁手写阈）；双跑位级确定性；真实渲染帧率口径 0-byte（presented 口径独立登记，禁混入 upscale/FG ratio）；默认臂 Stage A digest 锚 18 格零漂移（g14_3_pipeline_perf 本期 0-byte） | G19.2 |
| **M-b** | RFC-0035 重判兑现：FSR3-FG / DLSS-G / SL-310.6.0 三 vendor 臂 disposition（integrated/rejected/not-available 均合法终态）；g19_vendor_sdk_registry.json provenance 登记；310.5.2 生产默认维持或换版程序面留痕 | G19.2 |
| **M-c** | RD-045 长窗观察兑现：bistro-interior/t50/tsr_device 连续 ≥12 轮 --expect-digest 锚对拍零漂移取证 + registry history 只追加登记；close/maintain-open 均合法诚实终态 | G19.3 |
| **M-d** | G17-MD-F1 重评窗登记：G14 M-d 最新 18 格 evidence 如实登记（met 计数 + 焦点格 ratio）；FG 生成帧禁计入真实渲染帧率；达标判定归 G25 终判窗 | G19.4 |
| **M-e** | G18 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g19_` 前缀不抢 latest | G19.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g19.wave.1.acceptance_map` | `ci/g19_acceptance_map_check.py` | 333 |
| 候选决策 | `g19.wave.1.candidate_decisions` | `ci/g19_candidate_decisions_check.py` | 334 |
| 互锁 | `g19.gov.implementation_interlock` | `ci/g19_interlock_check.py` | 335 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G19_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G18 defer-to-G19+ 九行本波逐行 disposition（候选决策表 §1）。
2. 主轨 = 帧生成独立层兑现（G13-N7 go，M-a/M-b 承载）；其余八行按七期战役排程 defer-to-G20+（承接锚点名具体期别）。
3. RD-045 观察窗（M-c）+ G17-MD-F1 重评窗登记（M-d）同期消化。
4. RFC-0036 本波起草 + 对抗评审；RFC-0035 只追加重判记录。
5. 治理三门步骤 333/334/335 顺位领取（落盘前实测 CI_step.next_free=333）。
6. 用户战役指令「帮我一次性完成G19-G25」登记（本契约 §1 字面；不占共享 U 段——U 段为 unsafe 审计命名空间）。
7. 先优化后测试：G19.2~G19.3 纯实现；G19.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G19-2 implementation_status 解锁记录（2026-08-24）

- **事实门全绿**：G18 closed + tag `g18-closed` + G19.0 不可变 ref `9dda737bca0b2026f1e9672c5e70f6b807c172b9`；候选表 14 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G19-G25」字面 + workflow 末号 335 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g19_interlock_check.py --gate g19.gov.implementation_interlock` VERDICT=READY（evidence/g19_interlock_check_20260824T145212Z.json）；治理三门 333/334/335 PASS（`g19_acceptance_map_check` 20260824T145202Z / `g19_candidate_decisions_check` 20260824T145203Z / `g19_interlock_check`）。
- **解锁**：`implementation_status: blocked → unlocked`。G19.2+ 实现波（M-a~M-e）现可开工。
