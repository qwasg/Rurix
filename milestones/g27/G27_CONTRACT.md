---
contract: G27
title: G27 几何 device 化期（HZB device kernel 兑现 + M61 mesh shader 重判窗 + cluster P4 差距闭集重判 + M98-l4 重判窗）
status: active
implementation_status: unlocked
active_scope: g27_geometry_device_realization
version: v1.0
date: 2026-08-25
timebox: "G27.1 治理波即刻执行（G26 已 closed，tag g26-closed）；G27.2~G27.4 严格波次 + G27.5 P2/soak + G27.6 close-out；用户「帮我一次性完成G26-G30」战役字面（G27 = 五期串行战役第二期，里程碑间不越级）"
rfc_required: "G27.1 领取 RFC-0044（实测 RFC next_free=44）——G27 几何 device 化:HZB device kernel 兑现语义 + M61 重判程序 + cluster P4 差距重判程序 + M98-l4 重判窗程序（文件名 rfcs/0044-geometry-device-realization.md）；经对抗评审后 Agent Approved 方可实现；达标/maintain/defer/诚实红均合法终态。"
upstream_docs:
  - "milestones/g25/g25_campaign_handover_registry.json M61/M98-l4 行"
  - "registry/deferred.json RD-039"
  - "milestones/g20/g20_cluster_streaming_p4_gap.json(四行差距闭集,本期只读重判不回写)"
  - "src/rurix-render/src/geometry/hzb.rs(G20 host 参考臂,本期 0-byte 冻结面)"
implementation_unlock:
  required_all:
    - "G27.1 治理门全部完成且有真实验证记录"
    - "ci/g27_interlock_check.py --require-ready 输出 READY"
    - "用户 G27.2 开工指令留痕（「帮我一次性完成G26-G30」战役字面）"
in_scope:
  - g27_1_governance_only
  - hzb_device_kernel
  - m61_mesh_shader_rejudgment
  - cluster_p4_gap_rejudgment
  - hlod_l4_counter_rejudgment
  - closed_gate_no_regression
  - g27_p2_decisions_soak_closeout_tag
out_of_scope:
  - host_reference_arm_rewrite
  - g20_gap_registry_rewrite
  - mesh_shader_hw_pipeline_implementation
  - hlod_proxy_device_leg_implementation
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g26_frozen_registries
deliverables:
  - id: D-G27-1
    check: "G27.1 四件套 + 候选决策 8 行 + 验收映射 5 P0 + RFC-0044 + 治理三门 461/462/463"
acceptance_gates:
  - id: G-G27-1
    check: "G27.1 完成门：D-G27-1 齐备；治理三门 PASS"
  - id: G-G27-2
    check: "互锁门：ci/g27_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G27-3
    check: "G27.2 退出门：M-a/M-b P0 全绿"
  - id: G-G27-4
    check: "G27.3 退出门：M-c/M-d P0 全绿"
  - id: G-G27-5
    check: "G27.4 退出门：M-e P0 全绿"
  - id: G-G27-6
    check: "P2 + soak + close-out → tag g27-closed（五期串行战役第二期收口）"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G27-2"
  - "device 化零冒充：kernel 车道实测不可达以 measured 证据落 maintain/defer 不冒充 implemented"
  - "加性纪律：host 参考臂 geometry/hzb.rs 与 geometry/cull.rs、geometry/visbuffer.rs vs g26-closed 0-byte git-diff 机核"
  - "no-go/defer/not-available/maintain/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---
<!-- Assisted-by: Cursor Agent(G27.1 治理波) -->

# G27 几何 device 化期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G26-G30**（2026-08-25，五期串行战役全期授权；G27 = 第二期）。G27 = 几何 device 化期：HZB device kernel 兑现（金字塔逐级 farther-of 归约 + rect 测试 device 化，与 host 金标准位级对拍）、M61 mesh shader 重判窗两半盘点、cluster P4 差距闭集四行重判、M98-l4 重判窗条件核验、G26 链零降级。

G27.0 不可变 ref = `fc8c9fa2c0360997b95c559da1d0d68af0c37159`（G26 close-out flip commit，tag `g26-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G27.1 | 治理波 + RFC-0044 起草 + baseline 快检 | G-G27-1 |
| 互锁 | `--require-ready` READY | G-G27-2 |
| G27.2 | M-a HZB device kernel 兑现 + M-b M61 重判窗 | G-G27-3 |
| G27.3 | M-c cluster P4 差距闭集重判 + M-d M98-l4 重判窗 | G-G27-4 |
| G27.4 | M-e 旧门零降级（全量测试波） | G-G27-5 |
| G27.5~6 | P2/soak/close-out/tag（第二期收口） | G-G27-6 |

## 3. 治理波交付物

D-G27-1：PLAN/CONTRACT/CI_GATES/g27_budget.json + G27_CANDIDATE_DECISIONS + G27_ACCEPTANCE_MAP + RFC-0044 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据(逐字) | 波次 |
|---|---|---|
| **M-a** | HZB device 化兑现:kernels/g27_hzb_reduce.rx + g27_hzb_test.rx(rurixc --target vulkan 产 SPV + spirv-val 通过)经 vk::run_compute 派发——金字塔逐级 farther-of 归约 device 化(mips 与 host HzbPyramid::build 逐级位级相等)+ rect 测试 device 化(mip 选择/≤2×2 窗/is_farther 判定与 host test_rect 逐字同源,800 rect × 双约定判定序列与 host 全等)+ 零假阳性硬不变量(device 判 Occluded ⇒ exact_rect_occluded 同判)+ device 双跑位级一致 + 篡改 RED 臂检出;host 参考臂 geometry/hzb.rs 0-byte;device 环境不可用时 SKIP 如实登记不冒充 | G27.2 |
| **M-b** | M61 重判窗兑现(RFC-0034 只追加程序):重判条件两半机器盘点——HZB device 化半边(M-a 绿件只读盘点)+ cluster P4 差距闭集清零半边(g20_cluster_streaming_p4_gap.json 四行 open 状态实测)+ mesh shader HW 性能差 measured 证据树内搜索(searched-paths manifest 必填)——条件未全齐 → maintain-no-go 只追加再判记录(RFC-0034 重判表 + 本期 evidence);全齐 → 重判程序启动;零冒充 | G27.2 |
| **M-c** | cluster P4 差距闭集重判:四行(P4-1~P4-4)逐行 reeval——P4-2 依赖面(HZB device 化)本期解除事实登记 + 各行现面零实现树内实测(streaming/ 模块 cluster 载荷面检索)——清零 → closed-go;未清零 → 维持 open 登记 milestones/g27/g27_cluster_p4_rejudgment.json(g20 差距表原文 0-byte 不回写);RD-039 history 只追加 | G27.3 |
| **M-d** | M98-l4 重判窗条件核验:重判条件两半树内实测——HLOD proxy 追踪 device 腿(src 检索零实现登记)+ L4 计数器接入(gi/fallback_chain.rs L4 槽位恒零/fail-closed 入口实测 + world/hlod.rs 接口面就绪盘点)——任一半命中 → 重判程序启动;均未命中 → 维持 L1/L2/L3 三级链诚实登记,承接锚只追加 | G27.3 |
| **M-e** | G26 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g27_` 前缀不抢 latest | G27.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g27.wave.1.acceptance_map` | `ci/g27_acceptance_map_check.py` | 461 |
| 候选决策 | `g27.wave.1.candidate_decisions` | `ci/g27_candidate_decisions_check.py` | 462 |
| 互锁 | `g27.gov.implementation_interlock` | `ci/g27_interlock_check.py` | 463 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G27_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G25 交接登记表行（M61 + M98-l4 + RD-039-mesh）本波逐行 disposition（候选决策表 §1 三行）。
2. 主轨 = 几何 device 化四面（HZB device kernel 兑现/M61 重判窗/cluster P4 差距重判/M98-l4 重判窗）。
3. RFC-0044 本波起草 + 对抗评审。
4. 治理三门步骤 461/462/463 顺位领取（落盘前实测 CI_step.next_free=461）。
5. 用户战役指令「帮我一次性完成G26-G30」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先实现后测试：G27.2~G27.3 实现与重判波；G27.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G27-2 implementation_status 解锁记录（2026-08-25）

- **事实门全绿**：G26 closed + tag `g26-closed` + G27.0 不可变 ref `fc8c9fa2c0360997b95c559da1d0d68af0c37159`；候选表 8 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G26-G30」字面 + workflow 末号 463 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g27_interlock_check.py --require-ready` VERDICT=READY；治理三门 461/462/463 绿件（acceptance_map/candidate_decisions PASS + interlock READY）；RFC-0044 经 D-409 对抗评审（11 findings 全 disposition，v0.2 修法批）Agent Approved。
- **解锁**：`implementation_status: blocked → unlocked`。G27.2+ 实现波（M-a~M-e）现可开工。
