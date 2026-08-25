---
contract: G28
title: G28 光照 device 化期（ReSTIR device kernel 兑现 + 空间重用加性臂 + M52/RD-040 workload 重判 + RD-034 上游复查）
status: active
implementation_status: unlocked
active_scope: g28_lighting_device_realization
version: v1.0
date: 2026-08-25
timebox: "G28.1 治理波即刻执行（G27 已 closed，tag g27-closed）；G28.2~G28.4 严格波次 + G28.5 P2/soak + G28.6 close-out；用户「帮我一次性完成G26-G30」战役字面（G28 = 五期串行战役第三期，里程碑间不越级）"
rfc_required: "G28.1 领取 RFC-0045（实测 RFC next_free=45）——G28 光照 device 化:ReSTIR device kernel 兑现语义 + 空间重用加性臂 + M52/RD-040 workload 重判程序 + RD-034 上游复查程序（文件名 rfcs/0045-lighting-device-realization.md）；经对抗评审后 Agent Approved 方可实现；达标/maintain/defer/诚实红均合法终态。"
upstream_docs:
  - "milestones/g25/g25_campaign_handover_registry.json M100-high/M52 行"
  - "registry/deferred.json RD-034/RD-040"
  - "milestones/g21/g21_rd040_subitem_registry.json(五分项 reeval_anchor)"
  - "src/rurix-render/src/gi/restir_reservoir.rs(G21 host 参考臂,本期 0-byte 冻结面)"
implementation_unlock:
  required_all:
    - "G28.1 治理门全部完成且有真实验证记录"
    - "ci/g28_interlock_check.py --require-ready 输出 READY"
    - "用户 G28.2 开工指令留痕（「帮我一次性完成G26-G30」战役字面）"
in_scope:
  - g28_1_governance_only
  - restir_device_kernel
  - restir_spatial_reuse_arm
  - m52_rd040_workload_rejudgment
  - rd034_upstream_recheck
  - closed_gate_no_regression
  - g28_p2_decisions_soak_closeout_tag
out_of_scope:
  - host_reference_arm_rewrite
  - rt_pipeline_sbt_implementation
  - m100_production_lane_integration
  - nrd_smrt_worldrc_omm_subitem_implementation
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g27_frozen_registries
deliverables:
  - id: D-G28-1
    check: "G28.1 四件套 + 候选决策 8 行 + 验收映射 5 P0 + RFC-0045 + 治理三门 477/478/479"
acceptance_gates:
  - id: G-G28-1
    check: "G28.1 完成门：D-G28-1 齐备；治理三门 PASS"
  - id: G-G28-2
    check: "互锁门：ci/g28_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G28-3
    check: "G28.2 退出门：M-a/M-b P0 全绿"
  - id: G-G28-4
    check: "G28.3 退出门：M-c/M-d P0 全绿"
  - id: G-G28-5
    check: "G28.4 退出门：M-e P0 全绿"
  - id: G-G28-6
    check: "P2 + soak + close-out → tag g28-closed（五期串行战役第三期收口）"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G28-2"
  - "device 化零冒充：kernel 车道实测不可达以 measured 证据落 maintain/defer 不冒充 implemented"
  - "加性纪律：host 参考臂 gi/restir_reservoir.rs 与 gi/multi_light.rs 生产默认面 vs g27-closed 0-byte git-diff 机核"
  - "no-go/defer/not-available/maintain/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---
<!-- Assisted-by: Cursor Agent(G28.1 治理波) -->

# G28 光照 device 化期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G26-G30**（2026-08-25，五期串行战役全期授权；G28 = 第三期）。G28 = 光照 device 化期：ReSTIR device kernel 兑现（WRS/RIS reservoir 更新链 device 化，与 host 金标准同输入逐 trial 对拍）、空间重用加性臂兑现（bin-local 网格邻域 reservoir 合并）、M52/RD-040 workload 重判（两半盘点 + 五分项逐锚重判）、RD-034 上游探针新鲜复查、G27 链零降级。

G28.0 不可变 ref = `3122653014f6e7e39b626e7d932065014f30ce47`（G27 close-out flip commit，tag `g27-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G28.1 | 治理波 + RFC-0045 起草 + baseline 快检 | G-G28-1 |
| 互锁 | `--require-ready` READY | G-G28-2 |
| G28.2 | M-a ReSTIR device kernel 兑现 + M-b 空间重用加性臂 | G-G28-3 |
| G28.3 | M-c M52/RD-040 workload 重判 + M-d RD-034 上游复查 | G-G28-4 |
| G28.4 | M-e 旧门零降级（全量测试波） | G-G28-5 |
| G28.5~6 | P2/soak/close-out/tag（第三期收口） | G-G28-6 |

## 3. 治理波交付物

D-G28-1：PLAN/CONTRACT/CI_GATES/g28_budget.json + G28_CANDIDATE_DECISIONS + G28_ACCEPTANCE_MAP + RFC-0045 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据(逐字) | 波次 |
|---|---|---|
| **M-a** | ReSTIR device kernel 兑现:kernels/g28_restir.rx(rurixc --target vulkan 产 SPV + spirv-val 通过)经 vk::run_compute 派发——WRS/RIS reservoir 更新链 device 化(候选流与均匀随机数由 host 单源预生成上传,device 不重生成 RNG——PCG32 u64 状态面留 host;逐 trial 单 invocation 顺序 WRS 链保浮点序)+ device vs host 金标准(gi/restir_reservoir.rs estimate_ris)同输入逐 trial 对拍(p100 ≤ 标定容差,threshold = measured × 2.0 冻结 k 程序产禁手写;实测位级可达则登记零容差)+ 无偏 3σ 维持 + device 双跑位级一致 + kernel-bias RED 臂检出;host 参考臂 0-byte;device 环境不可用时 SKIP 如实登记不冒充 | G28.2 |
| **M-b** | 空间重用加性臂兑现(bin-local,host 参考臂 0-byte):多着色点网格邻域 reservoir 合并(Reservoir::merge 语义同构 m_cap 截断,时域/空间同律)——无偏 3σ 维持(空间合并不引入偏差,等验证预算 measured 对照)+ 空间合并方差再收益 measured 登记(程序产对照,收益值如实登记不设通过线)+ 双跑位级一致 + M100 低档 MegaLights 生产默认面 0-byte 机核 | G28.2 |
| **M-c** | M52/RD-040 workload 重判:M52 两半盘点——capability 半边(G21 vulkaninfo 三 token available 取证只读盘点 + 新鲜 vulkaninfo 复测)+ workload 半边(RT pipeline/SBT 宿主车道树内检索,searched-paths manifest 必填)——两半全齐方改判;未全齐 → maintain-defer 只追加;RD-040 五分项逐锚重判(五分项 reeval_anchor 树内实测逐项登记)——全未命中 → 维持 defer;RD-040 history 只追加 | G28.3 |
| **M-d** | RD-034 上游复查:真跑 ci/meshrt_probe_smoke.py(spirv-cross 拒 raygen 探针新鲜——非零退出 = blocked 证据新鲜;意外成功翻红提醒复评)+ deferred.json RD-034 status/history 核验(G28.3 行只追加)——解锁/维持 blocked 均合法诚实终态零冒充 | G28.3 |
| **M-e** | G27 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g28_` 前缀不抢 latest | G28.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g28.wave.1.acceptance_map` | `ci/g28_acceptance_map_check.py` | 477 |
| 候选决策 | `g28.wave.1.candidate_decisions` | `ci/g28_candidate_decisions_check.py` | 478 |
| 互锁 | `g28.gov.implementation_interlock` | `ci/g28_interlock_check.py` | 479 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G28_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G25 交接登记表行（M100-high + M52 + RD-034）本波逐行 disposition（候选决策表 §1 三行）。
2. 主轨 = 光照 device 化四面（ReSTIR device kernel 兑现/空间重用加性臂/M52/RD-040 workload 重判/RD-034 上游复查）。
3. RFC-0045 本波起草 + 对抗评审。
4. 治理三门步骤 477/478/479 顺位领取（落盘前实测 CI_step.next_free=477）。
5. 用户战役指令「帮我一次性完成G26-G30」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先实现后测试：G28.2~G28.3 实现与重判波；G28.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G28-2 implementation_status 解锁记录（2026-08-25）

- **事实门全绿**：G27 closed + tag `g27-closed` + G28.0 不可变 ref `3122653014f6e7e39b626e7d932065014f30ce47`；候选表 8 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G26-G30」字面 + workflow 末号 479 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g28_interlock_check.py --require-ready` VERDICT=READY；治理三门 477/478/479 绿件；RFC-0045 经 D-409 对抗评审（12 findings 全 disposition，v0.2 修法批——含 F3 f64 幻影态删除）Agent Approved。
- **解锁**：`implementation_status: blocked → unlocked`。G28.2+ 实现波（M-a~M-e）现可开工。
