---
contract: G26
title: G26 时域/帧生成 device 化期（FG/MFG device kernel 兑现 + device 帧时口径登记 + RD-045 backfill 重判 + G17-MD-F1 重判窗）
status: active
implementation_status: unlocked
active_scope: g26_framegen_device_realization
version: v1.0
date: 2026-08-25
timebox: "G26.1 治理波即刻执行（G25 已 closed，tag g25-closed）；G26.2~G26.4 严格波次 + G26.5 P2/soak + G26.6 close-out；用户「帮我一次性完成G26-G30」战役字面（G26 = 五期串行战役第一期，里程碑间不越级）"
rfc_required: "G26.1 领取 RFC-0043（实测 RFC next_free=43）——G26 时域/帧生成 device 化:FG/MFG device kernel 兑现语义 + RD-045 backfill 重判程序 + G17-MD-F1 重判窗程序（文件名 rfcs/0043-framegen-device-kernel-realization.md）；经对抗评审后 Agent Approved 方可实现；达标/maintain/defer/诚实红均合法终态。"
upstream_docs:
  - "milestones/g25/g25_campaign_handover_registry.json（G26+ 唯一法定输入面）"
  - "registry/deferred.json RD-045 条目（backfill 三件条件字面）"
  - "src/rurix-render/src/temporal/framegen.rs（G19 host 参考臂，本期 0-byte 冻结面）"
implementation_unlock:
  required_all:
    - "G26.1 治理门全部完成且有真实验证记录"
    - "ci/g26_interlock_check.py --require-ready 输出 READY"
    - "用户 G26.2 开工指令留痕（「帮我一次性完成G26-G30」战役字面）"
in_scope:
  - g26_1_governance_only
  - framegen_device_kernel
  - framegen_device_bench_accounting
  - rd045_backfill_rejudgment
  - g17_md_f1_rejudgment_window
  - closed_gate_no_regression
  - g26_p2_decisions_soak_closeout_tag
out_of_scope:
  - host_reference_arm_rewrite
  - presentation_default_arm_change
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g25_frozen_registries
  - new_feature_work_beyond_framegen_device
deliverables:
  - id: D-G26-1
    check: "G26.1 四件套 + 候选决策 8 行 + 验收映射 5 P0 + RFC-0043 + 治理三门 445/446/447"
acceptance_gates:
  - id: G-G26-1
    check: "G26.1 完成门：D-G26-1 齐备；治理三门 PASS"
  - id: G-G26-2
    check: "互锁门：ci/g26_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G26-3
    check: "G26.2 退出门：M-a/M-b P0 全绿"
  - id: G-G26-4
    check: "G26.3 退出门：M-c/M-d P0 全绿"
  - id: G-G26-5
    check: "G26.4 退出门：M-e P0 全绿"
  - id: G-G26-6
    check: "P2 + soak + close-out → tag g26-closed（五期串行战役第一期收口）"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G26-2"
  - "device 化零冒充：kernel 车道实测不可达以 measured 证据落 maintain/defer 不冒充 implemented"
  - "加性纪律：host 参考臂 temporal/framegen.rs 与默认渲染臂 vs g25-closed 0-byte git-diff 机核"
  - "no-go/defer/not-available/maintain/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---
<!-- Assisted-by: Cursor Agent(G26.1 治理波) -->

# G26 时域/帧生成 device 化期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G26-G30**（2026-08-25，五期串行战役全期授权；G26 = 第一期）。G26 = 时域/帧生成 device 化期：FG/MFG device kernel 兑现（device vs host 金标准对拍 + 程序产标定容差）、FG device 车道帧时 measured 登记与口径纪律回验、RD-045 backfill 三件重判、G17-MD-F1 重判窗条件核验、G25 链零降级。

G26.0 不可变 ref = `ae49a15b73083953c268d24cf4e2df64c17ddc6a`（G25 close-out flip commit，tag `g25-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G26.1 | 治理波 + RFC-0043 起草 + baseline 快检 | G-G26-1 |
| 互锁 | `--require-ready` READY | G-G26-2 |
| G26.2 | M-a FG/MFG device kernel 兑现 + M-b device 帧时与口径登记 | G-G26-3 |
| G26.3 | M-c RD-045 backfill 重判 + M-d G17-MD-F1 重判窗 | G-G26-4 |
| G26.4 | M-e 旧门零降级（全量测试波） | G-G26-5 |
| G26.5~6 | P2/soak/close-out/tag（第一期收口） | G-G26-6 |

## 3. 治理波交付物

D-G26-1：PLAN/CONTRACT/CI_GATES/g26_budget.json + G26_CANDIDATE_DECISIONS + G26_ACCEPTANCE_MAP + RFC-0043 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据(逐字) | 波次 |
|---|---|---|
| **M-a** | FG/MFG device kernel 兑现:kernels/g26_framegen.rx(rurixc --target vulkan 产 SPV + spirv-val 通过)经 vk::run_compute 派发 + device vs host 金标准(temporal/framegen.rs)同输入逐帧对拍——×2/×3/×4 三档合成运动场景逐帧逐像素最大绝对差 p100 ≤ 标定容差(threshold = measured × 2.0 冻结 k,标定腿两跑位级一致程序产,禁手写)+ SSIM(interp)>SSIM(frame-hold) 程序产对照继承 + device 双跑位级一致 + kernel-bias RED 臂检出;host 参考臂 temporal/framegen.rs 0-byte;device 环境不可用时 SKIP 如实登记不冒充 | G26.2 |
| **M-b** | FG device 车道帧时 measured 登记 + 口径纪律回验:device 全链路(打包+dispatch+回读)warmup+timed 逐帧墙钟登记(回归守护语义,不构成帧率对标通过线,生成帧禁计入真实渲染帧率)+ FgAccounting 真渲/presented 两口径类型面分离核验 + 性能面 g14_3_pipeline_perf 0-byte 机核 vs g25-closed | G26.2 |
| **M-c** | RD-045 backfill 三件重判:新鲜观察窗真跑(RD-045 焦点车道 canonical 双跑 digest 轨迹多轮零漂移登记)+ 三件条件逐项机器盘点(根因定位/生产化修复/Full RFC 评估——树内证据闭集实测)——全齐 → close;未齐 → maintain-open 只追加扩窗零冒充;deferred history 只追加 | G26.3 |
| **M-d** | G17-MD-F1 重判窗条件核验:NGX 分解 profiling 证据与 UE 侧插桩证据两半树内闭集搜索实测(evidence/ 检索面登记)——任一命中 → 重判程序启动;两半均未命中 → 维持 17/18 诚实红 carry(终判归 G30 商用终审),搜索面闭集只追加登记零冒充 | G26.3 |
| **M-e** | G25 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g26_` 前缀不抢 latest | G26.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g26.wave.1.acceptance_map` | `ci/g26_acceptance_map_check.py` | 445 |
| 候选决策 | `g26.wave.1.candidate_decisions` | `ci/g26_candidate_decisions_check.py` | 446 |
| 互锁 | `g26.gov.implementation_interlock` | `ci/g26_interlock_check.py` | 447 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G26_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G25 交接登记表行（G13-N7 + RD-045-window + G17-MD-F1）本波逐行 disposition（候选决策表 §1 三行）。
2. 主轨 = FG/MFG device 化四面（device kernel 兑现/device 帧时口径登记/RD-045 backfill 重判/G17-MD-F1 重判窗）。
3. RFC-0043 本波起草 + 对抗评审。
4. 治理三门步骤 445/446/447 顺位领取（落盘前实测 CI_step.next_free=445）。
5. 用户战役指令「帮我一次性完成G26-G30」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先实现后测试：G26.2~G26.3 实现与重判波；G26.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G26-2 implementation_status 解锁记录（2026-08-25）

- **事实门全绿**：G25 closed + tag `g25-closed` + G26.0 不可变 ref `ae49a15b73083953c268d24cf4e2df64c17ddc6a`；候选表 8 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G26-G30」字面 + workflow 末号 447 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g26_interlock_check.py --require-ready` VERDICT=READY；治理三门 445/446/447 绿件（acceptance_map/candidate_decisions PASS + interlock READY）；RFC-0043 经 D-409 对抗评审（11 findings 全 disposition，v0.2 修法批）Agent Approved。
- **解锁**：`implementation_status: blocked → unlocked`。G26.2+ 实现波（M-a~M-e）现可开工。

### §8.6 G26.5 P2 穷举 + stabilization soak 验收记录（2026-08-25）——G-G26-6 前置：P2 穷举决策门（g26.wave.5a.decisions，步骤 458，VERDICT=PASS）+ 稳定门 soak（g26.wave.5a.soak，步骤 459，8/8 facts VERDICT=PASS——67 迭代 wall=1950.7s ≥1800s 零失败）

- **① P2 穷举定盘**：`G26_P2_DECISIONS.md` 穷举闭集 **8 行零空行**（§1 三行 closed-go 3〔G13-N7 device kernel 兑现 implemented + RD-045-window 重判兑现（maintain-open，三件 0/3）+ G17-MD-F1 重判兑现（maintain 17/18 诚实红 carry 终判归 G30）〕；§3 期内行五行 closed-go 5；§2 open RD 八条维持 open）。
- **② soak 定盘**：VERDICT=PASS 8/8——**wall=1950.7s ≥1800s + 67 迭代零失败（含五车道探针轮换穿插 13 次：g19 framegen/g20 hzb/g21 restir/g22 slab 四实现件 + g26 framegen device --probe 快车道）+ active==wall + 零 sleep**；同窗独立第二实例 8/8 同绿（wall=1843.8s/67/0，双证并行）。
- **③ 命令输出**：P2 门 → VERDICT=PASS（g26_p2_decisions_check_20260825T030532Z）；wave2~wave6 聚合门五绿（守卫三件 + budget_eval 现场通过）。
- **④ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G26.5 P2/soak 波）`。
