---
contract: G21
title: G21 光照 P3+ 深化期（ReSTIR 高档 reservoir host 参考臂 + SER 重判 + RD-040 分项处置 + RD-034 上游复查）
status: active
implementation_status: unlocked
active_scope: g21_lighting_p3_deepening
version: v1.0
date: 2026-08-24
timebox: "G21.1 治理波即刻执行（G20 已 closed，tag g20-closed）；G21.2~G21.4 严格波次 + G21.5 P2/soak + G21.6 close-out；用户「帮我一次性完成G19-G25」战役字面（G21 = 七期串行战役第三期，里程碑间不越级）"
rfc_required: "G21.1 领取 RFC-0038（实测 RFC next_free=38）——光照 P3+ 深化：ReSTIR 高档 reservoir host 参考臂 + SER capability disposition + RD-040 分项处置 + RD-034 上游复查程序；经对抗评审后 Agent Approved 方可实现；实现/no-go/defer 均合法终态。"
upstream_docs:
  - "milestones/g20/G20_P2_DECISIONS.md §1 defer-to-G21+ 七行 + G20_CONTRACT.md §8.7 承接锚"
  - "milestones/g18/G18_P2_DECISIONS.md §1 M100-high 行（closed-go 重判锚：G19+ 高档 reservoir 证据齐备）"
  - "registry/deferred.json RD-040（光照 P3+ 长线）/ RD-034（DXIL RT 腿 blocked）"
implementation_unlock:
  required_all:
    - "G21.1 治理门全部完成且有真实验证记录"
    - "ci/g21_interlock_check.py --require-ready 输出 READY"
    - "用户 G21.2 开工指令留痕（「帮我一次性完成G19-G25」战役字面）"
in_scope:
  - g21_1_governance_only
  - restir_high_reservoir_realization
  - ser_capability_disposition
  - rd040_subitem_disposition
  - rd034_upstream_recheck
  - closed_gate_no_regression
  - g21_p2_decisions_soak_closeout_tag
out_of_scope:
  - restir_device_kernel_lane
  - rt_pipeline_sbt_host_lane_implementation
  - smrt_world_radiance_cache_implementation
  - nrd_omm_vendor_integration
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g20_frozen_registries
deliverables:
  - id: D-G21-1
    check: "G21.1 四件套 + 候选决策 13 行 + 验收映射 5 P0 + RFC-0038 + 治理三门 365/366/367"
acceptance_gates:
  - id: G-G21-1
    check: "G21.1 完成门：D-G21-1 齐备；治理三门 PASS"
  - id: G-G21-2
    check: "互锁门：ci/g21_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G21-3
    check: "G21.2 退出门：M-a/M-b P0 全绿"
  - id: G-G21-4
    check: "G21.3 退出门：M-c/M-d P0 全绿"
  - id: G-G21-5
    check: "G21.4 退出门：M-e P0 全绿"
  - id: G-G21-6
    check: "P2 + soak + close-out → tag g21-closed"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G21-2"
  - "M100 低档 MegaLights 生产默认面 0-byte：multi_light.rs 与其 fail-closed 登记面不接线不改写"
  - "阈值零手写：无偏 3σ 检验 + 方差收益 measured 对照"
  - "no-go/defer/not-available/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G21 光照 P3+ 深化期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G19-G25**（2026-08-24，七期串行战役全期授权）。G21 = 战役第三期：M100-high 重判条件「高档 reservoir 证据齐备」的证据产出（WRS/RIS + 时域合并 host 参考臂），M52 SER 两半条件实测重判（capability 设备面 + workload 宿主车道面），RD-040 五分项处置闭集，RD-034 上游探针复查。

G21.0 不可变 ref = `2b521523a660a7dd3c98106d08c4470e295a03fc`（G20 close-out flip commit，tag `g20-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G21.1 | 治理波 + RFC-0038 起草 + baseline 快检 | G-G21-1 |
| 互锁 | `--require-ready` READY | G-G21-2 |
| G21.2 | M-a ReSTIR 高档 reservoir + M-b SER 重判 | G-G21-3 |
| G21.3 | M-c RD-040 分项处置 + M-d RD-034 上游复查 | G-G21-4 |
| G21.4 | M-e 旧门零降级（全量测试波） | G-G21-5 |
| G21.5~6 | P2/soak/close-out/tag | G-G21-6 |

## 3. 治理波交付物

D-G21-1：PLAN/CONTRACT/CI_GATES/g21_budget.json + G21_CANDIDATE_DECISIONS + G21_ACCEPTANCE_MAP + RFC-0038 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | ReSTIR DI 高档 reservoir host 参考臂实现（WRS/RIS 无偏估计 + 时域 reservoir 合并 M-cap）；程序产判据（无偏 3σ 检验 + 等验证预算方差收益 var(uniform)/var(RIS) > 2 + 时域再收益 > 1.2，均 measured 禁手写）；双跑位级确定性；M100 低档生产默认面 0-byte（multi_light.rs 与其 fail-closed 登记面不接线） | G21.2 |
| **M-b** | M52 SER 重判兑现：rt.ser 设备 capability 实测（vulkaninfo 扩展枚举取证落 g21_ser_capability_probe_results.json）+ 高分歧 RT workload 宿主车道核验（RT pipeline/SBT 车道存在性）；capability/workload 两半分别登记；maintain-defer/go 均合法终态 | G21.2 |
| **M-c** | RD-040 分项处置闭集：SMRT/世界辐射缓存演进/NRD 降噪/OMM/RT pipeline+SBT 五分项 disposition 登记 g21_rd040_subitem_registry.json + RD-040 history 只追加；go/no-go/defer 均合法终态 | G21.3 |
| **M-d** | RD-034 上游复查兑现：blocked 恒跑探针真跑（ci/meshrt_probe_smoke.py --verify-latest 或 --gate）+ 复查结论 RD-034 history 只追加；解锁/维持 blocked 均合法诚实终态 | G21.3 |
| **M-e** | G20 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g21_` 前缀不抢 latest | G21.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g21.wave.1.acceptance_map` | `ci/g21_acceptance_map_check.py` | 365 |
| 候选决策 | `g21.wave.1.candidate_decisions` | `ci/g21_candidate_decisions_check.py` | 366 |
| 互锁 | `g21.gov.implementation_interlock` | `ci/g21_interlock_check.py` | 367 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G21_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G20 defer-to-G21+ 七行 + M100-high 重判锚行本波逐行 disposition（候选决策表 §1 八行）。
2. 主轨 = ReSTIR 高档 reservoir 证据产出（M-a）+ SER 两半实测重判（M-b）+ RD-040/RD-034 处置（M-c/M-d）。
3. RFC-0038 本波起草 + 对抗评审。
4. 治理三门步骤 365/366/367 顺位领取（落盘前实测 CI_step.next_free=365）。
5. 用户战役指令「帮我一次性完成G19-G25」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先优化后测试：G21.2~G21.3 纯实现；G21.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G21-2 implementation_status 解锁记录（2026-08-24）

- **事实门全绿**：G20 closed + tag `g20-closed` + G21.0 不可变 ref `2b521523a660a7dd3c98106d08c4470e295a03fc`；候选表 13 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G19-G25」字面 + workflow 末号 367 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g21_interlock_check.py --gate g21.gov.implementation_interlock` VERDICT=READY；治理三门 365/366/367 PASS。
- **解锁**：`implementation_status: blocked → unlocked`。G21.2+ 实现波（M-a~M-e）现可开工。

### §8.6 G21.5 P2 穷举 + stabilization soak 验收记录（2026-08-24）——G-G21-6 前置：P2 穷举决策门（g21.wave.5a.decisions，步骤 378，VERDICT=PASS）+ 稳定门 soak（g21.wave.5a.soak，步骤 379，8/8 facts VERDICT=PASS——69 迭代 wall=1854.7s ≥1800s 零失败）

- **① P2 穷举定盘**：`G21_P2_DECISIONS.md` 穷举闭集 **13 行零空行**（§1 八行：closed-go 2〔M100-high 兑现 + M52 重判窗兑现（裁决 maintain-defer 留档）〕 + defer-to-G22+ 6；§3 期内行五行 closed-go 5；§2 open RD 八条维持 open——RD-040/RD-034 history 只追加）。
- **② soak 定盘**：`py -3 ci/g21_stabilization_soak.py --gate` → VERDICT=PASS 8/8（evidence/g21_stabilization_soak_20260824T174318Z 系列最新件）——M-d 前置绿 + **wall=1854.7s ≥1800s + 69 迭代零失败（含 ReSTIR 方差收益车道穿插 13 次复跑）+ active==wall + 零 sleep**。
- **③ 命令输出**：P2 门 → VERDICT=PASS；budget_eval --strict 279 pass 零 skip 零 estimated；cargo test -p rurix-render --lib 486 passed 0 failed。
- **④ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G21.5 P2/soak 波）`。

### §8.7 G21.6 close-out 终审签署块（2026-08-24）——G-G21-6 字面兑现：close-out 终审门（g21.wave.6b.closeout，步骤 380）八 facts 全绿 **VERDICT=READY** → status active→closed + tag `g21-closed`

- **① 终审八 facts 逐条**：five_p0_evidence_green / p2_exhaustive_zero_empty / restir_realization_chain（方差收益 15.955×、时域再收益 7.27×、无偏 3σ、双跑位级）/ rfc_0038_archived / old_gates_no_regression / rd_open_maintained（RD 八条 open；RD-040 五分项闭集 + RD-034 复查 history 只追加）/ soak_ge_1800_zero_fail / closeout_ready —— 全 PASS。
- **② 终审命令逐字输出**：`py -3 ci/g21_closeout_check.py --gate` → **VERDICT=READY，exit=0**。
- **③ 收口裁决**：光照 P3+ 深化字面兑现——**M-a ReSTIR 高档 reservoir implemented**（M100-high「高档 reservoir 证据齐备」兑现：var(uniform)/var(RIS)=15.955×、时域 7.27×、无偏 3σ、双跑位级；低档 MegaLights 生产默认面 0-byte）；**M-b M52 SER 重判 = maintain-defer**（capability 半边实测 available〔vulkaninfo 三 token〕+ workload 半边未命中〔RT pipeline/SBT 车道零实现〕；语言层不加 SER 原语兜底 0-byte）；**M-c RD-040 五分项全 defer**（SMRT/世界辐射缓存/NRD/OMM/RT-pipeline+SBT 各附 basis+reeval_anchor）；**M-d RD-034 复查 = 维持 blocked**（探针真跑：spirv-cross 仍拒 raygen）；M-e 旧门零降级全绿。**maintain-defer/blocked 维持均为合法收口态**；G22 承接锚齐备（slab 预制件已在树：material/slab.rs 5 测全绿）。
- **④ status flip 与 tag**：§8 只追加区本块落盘后，`status: active → closed`；`implementation_status: unlocked` 字面不动。flip commit 独立洁净落盘，随后 tag `g21-closed`。
- **⑤ 签署块**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G21 战役第三期收口）`。
