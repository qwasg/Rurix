---
contract: G29
title: G29 材质 device 集成期（slab device kernel 兑现 + 侧表供参加性臂 + SVT/KTX2 差距重判 + Work Graphs/DGC capability 复测）
status: active
implementation_status: unlocked
active_scope: g29_material_device_integration
version: v1.0
date: 2026-08-25
timebox: "G29.1 治理波即刻执行（G28 已 closed，tag g28-closed）；G29.2~G29.4 严格波次 + G29.5 P2/soak + G29.6 close-out；用户「帮我一次性完成G26-G30」战役字面（G29 = 五期串行战役第四期，里程碑间不越级）"
rfc_required: "G29.1 领取 RFC-0046（实测 RFC next_free=46）——G29 材质 device 集成:slab device kernel 兑现语义 + 侧表供参加性臂 + SVT/KTX2 差距重判程序 + Work Graphs/DGC capability 复测程序（文件名 rfcs/0046-material-device-integration.md）；经对抗评审后 Agent Approved 方可实现；达标/maintain/defer/诚实红均合法终态。"
upstream_docs:
  - "milestones/g25/g25_campaign_handover_registry.json RD-041-slab/RD-041-svt-ktx2-wg 两行"
  - "registry/deferred.json RD-041"
  - "milestones/g22/g22_svt_gap.json(SVT 四行)"
  - "milestones/g22/g22_ktx2_disposition.json(KTX2 三行)"
  - "milestones/g22/g22_work_graphs_probe_results.json(WG absent/DGC available 实测)"
  - "src/rurix-render/src/material/slab.rs(G22 host 参考臂,本期 0-byte 冻结面)"
implementation_unlock:
  required_all:
    - "G29.1 治理门全部完成且有真实验证记录"
    - "ci/g29_interlock_check.py --require-ready 输出 READY"
    - "用户 G29.2 开工指令留痕（「帮我一次性完成G26-G30」战役字面）"
in_scope:
  - g29_1_governance_only
  - slab_device_kernel
  - slab_side_table_arm
  - svt_ktx2_gap_rejudgment
  - wg_dgc_capability_recheck
  - closed_gate_no_regression
  - g29_p2_decisions_soak_closeout_tag
out_of_scope:
  - host_reference_arm_rewrite
  - material_closure_32b_rewrite
  - svt_ktx2_wg_implementation
  - production_material_lane_integration
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g28_frozen_registries
deliverables:
  - id: D-G29-1
    check: "G29.1 四件套 + 候选决策 7 行 + 验收映射 5 P0 + RFC-0046 + 治理三门 493/494/495"
acceptance_gates:
  - id: G-G29-1
    check: "G29.1 完成门：D-G29-1 齐备；治理三门 PASS"
  - id: G-G29-2
    check: "互锁门：ci/g29_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G29-3
    check: "G29.2 退出门：M-a/M-b P0 全绿"
  - id: G-G29-4
    check: "G29.3 退出门：M-c/M-d P0 全绿"
  - id: G-G29-5
    check: "G29.4 退出门：M-e P0 全绿"
  - id: G-G29-6
    check: "P2 + soak + close-out → tag g29-closed（五期串行战役第四期收口）"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G29-2"
  - "device 化零冒充：kernel 车道实测不可达以 measured 证据落 maintain/defer 不冒充 implemented"
  - "加性纪律：host 参考臂 material/slab.rs 与 graph/types.rs MaterialClosure 冻结面 vs g28-closed 0-byte git-diff 机核"
  - "no-go/defer/not-available/maintain/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---
<!-- Assisted-by: Cursor Agent(G29.1 治理波) -->

# G29 材质 device 集成期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G26-G30**（2026-08-25，五期串行战役全期授权；G29 = 第四期）。G29 = 材质 device 集成期：slab device kernel 兑现（slab 能量守恒闭式 device 化，与 host 参考臂同输入逐样本对拍）、侧表供参加性臂兑现（bin-local 多材质槽 slab 参数侧表，冻结面 0-byte）、SVT/KTX2 差距重判（七行逐锚 reeval）、Work Graphs/DGC capability 复测、G28 链零降级。

G29.0 不可变 ref = `2553abe651bc8daa3c044947e1ace9051db1b4d5`（G28 close-out flip commit，tag `g28-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G29.1 | 治理波 + RFC-0046 起草 + baseline 快检 | G-G29-1 |
| 互锁 | `--require-ready` READY | G-G29-2 |
| G29.2 | M-a slab device kernel 兑现 + M-b 侧表供参加性臂 | G-G29-3 |
| G29.3 | M-c SVT/KTX2 差距重判 + M-d WG/DGC capability 复测 | G-G29-4 |
| G29.4 | M-e 旧门零降级（全量测试波） | G-G29-5 |
| G29.5~6 | P2/soak/close-out/tag（第四期收口） | G-G29-6 |

## 3. 治理波交付物

D-G29-1：PLAN/CONTRACT/CI_GATES/g29_budget.json + G29_CANDIDATE_DECISIONS + G29_ACCEPTANCE_MAP + RFC-0046 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据(逐字) | 波次 |
|---|---|---|
| **M-a** | slab device kernel 兑现:kernels/g29_slab.rx(rurixc --target vulkan 产 SPV + spirv-val 通过)经 vk::run_compute 派发——slab 能量守恒闭式 device 化(公式面与 host material/slab.rs 逐字同源:闭式反照率/白炉恒等/能量上界/lerp 连续)+ device vs host 同输入逐样本对拍(16641 样本网格同 g22_slab_probe 口径〔GRID=128 经 furnace_audit (grid+1)² 格点〕;p100 ≤ 标定容差 threshold = measured × 2.0 程序产禁手写,实测位级可达则登记零容差零条目)+ 白炉恒等 device 复现(dev 如实登记)+ device 双跑位级一致 + kernel-bias RED 臂检出;host 参考臂 material/slab.rs 0-byte;device 环境不可用时 SKIP 如实登记不冒充 | G29.2 |
| **M-b** | 侧表供参加性臂兑现(bin-local,冻结面 0-byte):多材质槽 slab 参数侧表(bin 内合成独立 SSBO,MaterialClosure 32B 与 reserved 拓扑位零触碰)——device kernel 逐槽消费侧表求值 + 与 host 逐槽对拍(p100 同 M-a 容差协议)+ 逐槽白炉恒等维持 + 双跑位级一致 + graph/types.rs 0-byte 机核 | G29.2 |
| **M-c** | SVT/KTX2 差距重判:SVT 四行(g22_svt_gap.json)+ KTX2 三行(g22_ktx2_disposition.json)逐行 reeval——各行现面实现痕迹树内实测(逐行检索清单 + 锚关键词映射入 evidence)——兑现 → 该行 closed-go;零实现 → 维持 defer 登记 milestones/g29/g29_svt_ktx2_rejudgment.json(g22 原表 0-byte 不回写);RD-041 history 只追加 | G29.3 |
| **M-d** | Work Graphs/DGC capability 复测:VK_AMDX_shader_enqueue 新鲜 vulkaninfo 复测(三态闭集:absent 维持 not-available/present 翻转复评启动/SKIP 如实登记)+ DGC 三扩展 available 复测互核 + FSR 3.1.5 maintain 盘点(vendor_upscale 面 0-byte)——not-available 维持/复评启动均合法诚实终态零冒充 | G29.3 |
| **M-e** | G28 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g29_` 前缀不抢 latest | G29.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g29.wave.1.acceptance_map` | `ci/g29_acceptance_map_check.py` | 493 |
| 候选决策 | `g29.wave.1.candidate_decisions` | `ci/g29_candidate_decisions_check.py` | 494 |
| 互锁 | `g29.gov.implementation_interlock` | `ci/g29_interlock_check.py` | 495 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G29_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G25 交接登记表行（RD-041-slab + RD-041-svt-ktx2-wg）本波逐行 disposition（候选决策表 §1 两行）。
2. 主轨 = 材质 device 集成四面（slab device kernel 兑现/侧表供参加性臂/SVT/KTX2 差距重判/Work Graphs/DGC capability 复测）。
3. RFC-0046 本波起草 + 对抗评审。
4. 治理三门步骤 493/494/495 顺位领取（落盘前实测 CI_step.next_free=493）。
5. 用户战役指令「帮我一次性完成G26-G30」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先实现后测试：G29.2~G29.3 实现与重判波；G29.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G29-2 implementation_status 解锁记录（2026-08-25）

- **事实门全绿**：G28 closed + tag `g28-closed` + G29.0 不可变 ref `2553abe651bc8daa3c044947e1ace9051db1b4d5`；候选表 7 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G26-G30」字面 + workflow 末号 495 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g29_interlock_check.py --require-ready` VERDICT=READY；治理三门 493/494/495 绿件；RFC-0046 经 D-409 对抗评审（11 findings 全 disposition，v0.2 修法批——blocker F2 角点门形以修法 A 消除 + F3 有限性一等断言先于开工已落）Agent Approved。
- **解锁**：`implementation_status: blocked → unlocked`。G29.2+ 实现波（M-a~M-e）现可开工。
