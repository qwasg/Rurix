---
contract: G20
title: G20 虚拟化几何 P4 期（HZB 遮挡剔除 host 参考臂 + cluster 流送 P4 评估 + mesh shader/Far Field 重判）
status: closed
implementation_status: unlocked
active_scope: g20_virtualized_geometry_p4
version: v1.0
date: 2026-08-24
timebox: "G20.1 治理波即刻执行（G19 已 closed，tag g19-closed）；G20.2~G20.4 严格波次 + G20.5 P2/soak + G20.6 close-out；用户「帮我一次性完成G19-G25」战役字面（G20 = 七期串行战役第二期，里程碑间不越级）"
rfc_required: "G20.1 领取 RFC-0037（实测 RFC next_free=37）——虚拟化几何 P4：HZB 遮挡剔除 host 参考臂 + cluster 流送 P4 评估 + M61/M98-l4 重判程序；经对抗评审后 Agent Approved 方可实现；RFC-0034 按只追加程序落重判记录；实现/no-go/defer 均合法终态。"
upstream_docs:
  - "milestones/g19/G19_P2_DECISIONS.md §1 defer-to-G20+ 八行 + G19_CONTRACT.md §8.7 承接锚"
  - "milestones/g18/G18_P2_DECISIONS.md §1 M61 行（no-go 重判锚：G19+ HZB/cluster P4 触发条件齐备）"
  - "registry/deferred.json RD-039（虚拟化几何 P3+ 长线）"
implementation_unlock:
  required_all:
    - "G20.1 治理门全部完成且有真实验证记录"
    - "ci/g20_interlock_check.py --require-ready 输出 READY"
    - "用户 G20.2 开工指令留痕（「帮我一次性完成G19-G25」战役字面）"
in_scope:
  - g20_1_governance_only
  - hzb_occlusion_host_realization
  - cluster_streaming_p4_disposition
  - mesh_shader_rejudgment
  - far_field_l4_disposition
  - closed_gate_no_regression
  - g20_p2_decisions_soak_closeout_tag
out_of_scope:
  - hzb_device_kernel_lane
  - mesh_shader_hw_pipeline_implementation
  - mega_geometry_and_foliage_skeleton
  - restir_high_tier_reservoir
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g19_frozen_registries
deliverables:
  - id: D-G20-1
    check: "G20.1 四件套 + 候选决策 14 行 + 验收映射 5 P0 + RFC-0037 + RFC-0034 重判记录 + 治理三门 349/350/351"
acceptance_gates:
  - id: G-G20-1
    check: "G20.1 完成门：D-G20-1 齐备；治理三门 PASS"
  - id: G-G20-2
    check: "互锁门：ci/g20_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G20-3
    check: "G20.2 退出门：M-a/M-b P0 全绿"
  - id: G-G20-4
    check: "G20.3 退出门：M-c/M-d P0 全绿"
  - id: G-G20-5
    check: "G20.4 退出门：M-e P0 全绿"
  - id: G-G20-6
    check: "P2 + soak + close-out → tag g20-closed"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G20-2"
  - "保守零假阳性硬不变量：HZB 判遮挡 ⇒ 逐像素精确真值必同判（不得剔可见物）"
  - "既有 cull/visbuffer/streaming 面 0-byte 只读消费；默认臂 Stage A digest 锚红线"
  - "no-go/defer/not-available/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G20 虚拟化几何 P4 期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G19-G25**（2026-08-24，七期串行战役全期授权）。G20 = 战役第二期：兑现 geometry 模块头注「HZB 两阶段 P3 预留」第一阶段 host 面（M61 重判条件「G19+ HZB/cluster P4 触发条件齐备」的 HZB 半边），cluster 流送 P4 差距闭集评估，M61 mesh shader 与 M98-l4 Far Field 两行重判。

G20.0 不可变 ref = `3c138867f94af31101591b8b2103bb1622175d4c`（G19 close-out flip commit，tag `g19-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G20.1 | 治理波 + RFC-0037 起草 + baseline 快检 | G-G20-1 |
| 互锁 | `--require-ready` READY | G-G20-2 |
| G20.2 | M-a HZB host 参考臂 + M-b cluster P4 评估 | G-G20-3 |
| G20.3 | M-c M61 重判 + M-d M98-l4 重判 | G-G20-4 |
| G20.4 | M-e 旧门零降级（全量测试波） | G-G20-5 |
| G20.5~6 | P2/soak/close-out/tag | G-G20-6 |

## 3. 治理波交付物

D-G20-1：PLAN/CONTRACT/CI_GATES/g20_budget.json + G20_CANDIDATE_DECISIONS + G20_ACCEPTANCE_MAP + RFC-0037 + RFC-0034 重判记录 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | HZB 层级深度金字塔遮挡剔除 host 参考臂实现（farther-of 归约金字塔 + ≤2×2 纹素窗保守测试 + reverse-Z/standard-Z 双约定）；保守零假阳性硬不变量（确定性 rect 夹具 vs 逐像素精确真值零假阳性 + 剔除率非零）；双跑位级确定性；既有 cull/visbuffer 面 0-byte | G20.2 |
| **M-b** | RD-039 cluster 流送 P4 分项评估 disposition：streaming/ 现面盘点 + P4 差距闭集登记 g20_cluster_streaming_p4_gap.json；go/no-go/defer 均合法终态 | G20.2 |
| **M-c** | M61 重判兑现：RFC-0034 只追加重判记录（HZB host 面兑现事实 + mesh shader 性能差 measured 证据面核验 + VS fallback 维持裁决）；maintain-no-go/go 均合法终态 | G20.3 |
| **M-d** | M98-l4 重判兑现：HLOD 运行时接口面就绪核验（world/hlod.rs + g9_m111 门绿件）+ L4 计数可测性评估 + disposition 登记；实现/维持 L1/L2/L3 三级链均合法终态 | G20.3 |
| **M-e** | G19 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g20_` 前缀不抢 latest | G20.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g20.wave.1.acceptance_map` | `ci/g20_acceptance_map_check.py` | 349 |
| 候选决策 | `g20.wave.1.candidate_decisions` | `ci/g20_candidate_decisions_check.py` | 350 |
| 互锁 | `g20.gov.implementation_interlock` | `ci/g20_interlock_check.py` | 351 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G20_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G19 defer-to-G20+ 八行 + M61 重判锚行本波逐行 disposition（候选决策表 §1 九行）。
2. 主轨 = HZB host 参考臂（M-a）+ cluster P4 评估（M-b）+ M61/M98-l4 重判（M-c/M-d）。
3. RFC-0037 本波起草 + 对抗评审；RFC-0034 只追加重判记录（M-c 落档）。
4. 治理三门步骤 349/350/351 顺位领取（落盘前实测 CI_step.next_free=349）。
5. 用户战役指令「帮我一次性完成G19-G25」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先优化后测试：G20.2~G20.3 纯实现；G20.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G20-2 implementation_status 解锁记录（2026-08-24）

- **事实门全绿**：G19 closed + tag `g19-closed` + G20.0 不可变 ref `3c138867f94af31101591b8b2103bb1622175d4c`；候选表 14 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G19-G25」字面 + workflow 末号 351 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g20_interlock_check.py --gate g20.gov.implementation_interlock` VERDICT=READY；治理三门 349/350/351 PASS（acceptance_map 162039Z / candidate_decisions 162108Z / interlock）。
- **解锁**：`implementation_status: blocked → unlocked`。G20.2+ 实现波（M-a~M-e）现可开工。

### §8.6 G20.5 P2 穷举 + stabilization soak 验收记录（2026-08-24）——G-G20-6 前置：P2 穷举决策门（g20.wave.5a.decisions，步骤 362，VERDICT=PASS）+ 稳定门 soak（g20.wave.5a.soak，步骤 363，8/8 facts VERDICT=PASS——67 迭代 wall=1845.0s ≥1800s 零失败）

- **① P2 穷举定盘**：`G20_P2_DECISIONS.md` 穷举闭集 **14 行零空行**（§1 九行：maintain-no-go 2〔M61/M98-l4〕 + defer-to-G21+ 7；§3 期内行五行 closed-go 5；§2 open RD 八条维持 open）；汇总 closed-go 5 + maintain-no-go 2 + defer-to-G21+ 7。
- **② soak 定盘**：`py -3 ci/g20_stabilization_soak.py --gate` → VERDICT=PASS 8/8（evidence/g20_stabilization_soak_20260824T170025Z.json）——M-d 前置绿 + **wall=1845.0s ≥1800s + 67 迭代零失败（fails=0/67，含 HZB 零假阳性车道穿插 13 次复跑）+ active==wall + 零 sleep**。
- **③ 命令输出**：P2 门 → VERDICT=PASS（evidence/g20_p2_decisions_check_20260824T162931Z.json）；budget_eval --strict 277 pass 零 skip 零 estimated；cargo test -p rurix-render --lib 486 passed 0 failed（hzb 5 测 + slab 5 测〔G22 预制〕计入）。
- **④ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G20.5 P2/soak 波）`。

### §8.7 G20.6 close-out 终审签署块（2026-08-24）——G-G20-6 字面兑现：close-out 终审门（g20.wave.6b.closeout，步骤 364）八 facts 全绿 **VERDICT=READY** → status active→closed + tag `g20-closed`

- **① 终审八 facts 逐条（evidence/g20_wave6b_closeout_20260824T170042Z.json）**：
  1. **five_p0_evidence_green = PASS**（M-a~M-e 五 P0 最新 evidence host_section_pass 全真）。
  2. **p2_exhaustive_zero_empty = PASS**（14 行零空行）。
  3. **hzb_realization_chain = PASS**（g20_hzb_probe：双约定 800 rect 零假阳性 + 剔除 231/800 + 双跑位级）。
  4. **rfc_0037_archived = PASS**（RFC-0037 approved-implemented；RFC-0034 重判记录只追加在档）。
  5. **old_gates_no_regression = PASS**（M-e G19 链零降级全绿）。
  6. **rd_open_maintained = PASS**（RD 八条 open 维持）。
  7. **soak_ge_1800_zero_fail = PASS**（170025Z：67 迭代 wall=1845.0s ≥1800s 零失败）。
  8. **closeout_ready = PASS**（VERDICT=READY）。
- **② 终审命令逐字输出**：`py -3 ci/g20_closeout_check.py --gate` → **VERDICT=READY，exit=0**。
- **③ 收口裁决**：虚拟化几何 P4 字面兑现——**M-a HZB host 参考臂 implemented**（geometry 头注「HZB 两阶段 P3 预留」第一阶段兑现，保守零假阳性硬不变量）；**M-b cluster P4 defer**（差距闭集四行全 open 如实登记）；**M-c M61 maintain-no-go**（HZB 半边兑现 + cluster 半边未清零 + HW 性能差 measured 证据缺，RFC-0034 重判记录只追加）；**M-d M98-l4 维持三级链**（接口面就绪命中 + L4 计数可测未命中）；M-e 旧门零降级全绿。**maintain-no-go/defer 均为合法收口态**；G21 承接锚齐备（M100-high 重判窗 ReSTIR 预制件已在树：gi/restir_reservoir.rs 5 测全绿 + SER capability vulkaninfo 预测得）。
- **④ status flip 与 tag**：§8 只追加区本块落盘后，`status: active → closed`；`implementation_status: unlocked` 字面不动。flip commit 独立洁净落盘，随后 tag `g20-closed`。
- **⑤ 签署块**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G20 战役第二期收口）`。
