---
contract: G22
title: G22 材质/流送/时域期（slab 材质 host 参考臂 + SVT/KTX2/Work Graphs/FSR 分项处置）
status: closed
implementation_status: unlocked
active_scope: g22_material_streaming_temporal
version: v1.0
date: 2026-08-24
timebox: "G22.1 治理波即刻执行（G21 已 closed，tag g21-closed）；G22.2~G22.4 严格波次 + G22.5 P2/soak + G22.6 close-out；用户「帮我一次性完成G19-G25」战役字面（G22 = 七期串行战役第四期，里程碑间不越级）"
rfc_required: "G22.1 领取 RFC-0039（实测 RFC next_free=39）——材质/流送/时域 P3+：Substrate 类 slab 能量守恒闭合 host 参考臂 + SVT/KTX2/Work Graphs/FSR 四分项处置程序；经对抗评审后 Agent Approved 方可实现；实现/no-go/defer 均合法终态。"
upstream_docs:
  - "milestones/g21/G21_P2_DECISIONS.md §1 defer-to-G22+ 六行 + G21_CONTRACT.md §8.7 承接锚"
  - "registry/deferred.json RD-041（材质/流送/时域 P3+ 长线）"
implementation_unlock:
  required_all:
    - "G22.1 治理门全部完成且有真实验证记录"
    - "ci/g22_interlock_check.py --require-ready 输出 READY"
    - "用户 G22.2 开工指令留痕（「帮我一次性完成G19-G25」战役字面）"
in_scope:
  - g22_1_governance_only
  - slab_material_host_realization
  - svt_disposition
  - ktx2_basisu_disposition
  - work_graphs_fsr_reeval_disposition
  - closed_gate_no_regression
  - g22_p2_decisions_soak_closeout_tag
out_of_scope:
  - slab_device_kernel_and_side_table_integration
  - svt_page_table_implementation
  - basisu_vendor_transcoder_integration
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g21_frozen_registries
deliverables:
  - id: D-G22-1
    check: "G22.1 四件套 + 候选决策 11 行 + 验收映射 5 P0 + RFC-0039 + 治理三门 381/382/383"
acceptance_gates:
  - id: G-G22-1
    check: "G22.1 完成门：D-G22-1 齐备；治理三门 PASS"
  - id: G-G22-2
    check: "互锁门：ci/g22_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G22-3
    check: "G22.2 退出门：M-a/M-b P0 全绿"
  - id: G-G22-4
    check: "G22.3 退出门：M-c/M-d P0 全绿"
  - id: G-G22-5
    check: "G22.4 退出门：M-e P0 全绿"
  - id: G-G22-6
    check: "P2 + soak + close-out → tag g22-closed"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G22-2"
  - "白炉能量守恒硬不变量：任意参数域 R_total ≤ 1 + 白炉恒等 + 闭式↔级数恒等式"
  - "既有 material/closure 单层面与 G11.3 DDS 转码链 0-byte 只读消费"
  - "no-go/defer/not-available/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G22 材质/流送/时域期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G19-G25**（2026-08-24，七期串行战役全期授权）。G22 = 战役第四期：RD-041 材质/流送/时域长线的分项深化——slab 分层材质语义参考面实现 + SVT/KTX2/Work Graphs/FSR 四分项机器取证处置。

G22.0 不可变 ref = `0a4b1df397ed79ab30380fe0b12a822027a18d78`（G21 close-out flip commit，tag `g21-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G22.1 | 治理波 + RFC-0039 起草 + baseline 快检 | G-G22-1 |
| 互锁 | `--require-ready` READY | G-G22-2 |
| G22.2 | M-a slab 材质参考臂 + M-b SVT 处置 | G-G22-3 |
| G22.3 | M-c KTX2 处置 + M-d Work Graphs/FSR 重评 | G-G22-4 |
| G22.4 | M-e 旧门零降级（全量测试波） | G-G22-5 |
| G22.5~6 | P2/soak/close-out/tag | G-G22-6 |

## 3. 治理波交付物

D-G22-1：PLAN/CONTRACT/CI_GATES/g22_budget.json + G22_CANDIDATE_DECISIONS + G22_ACCEPTANCE_MAP + RFC-0039 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | Substrate 类双层 slab 能量守恒闭合 host 参考臂实现（无穷弹跳解析闭式 + farther 级数对拍）；白炉能量守恒硬不变量（白炉恒等 + 全参数域 R_total ≤ 1 + 对 base 反照率单调 + 闭式↔级数+尾和恒等式 1e-9）；层参数 lerp 连续性；双跑位级确定性；既有 material/closure 单层面 0-byte | G22.2 |
| **M-b** | RD-041 SVT 分项评估 disposition：streaming/ 页式现面 vs 虚拟纹理页表差距闭集登记 g22_svt_gap.json；go/no-go/defer 均合法终态 | G22.2 |
| **M-c** | RD-041 KTX2-BasisU 分项评估 disposition：G11.3 DDS 转码链现面盘点 + 转码器差距/收益登记 g22_ktx2_disposition.json；go/no-go/defer 均合法终态 | G22.3 |
| **M-d** | RD-041 Work Graphs + FSR 分项重评：Work Graphs Vulkan 车道设备实测（vulkaninfo AMDX/DGC 扩展枚举取证落 g22_work_graphs_probe_results.json）+ DGC 现面盘点（dgc.rs M102）+ FSR 3.1.5 第二超分臂重评维持登记；not-available/maintain 均合法终态 | G22.3 |
| **M-e** | G21 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g22_` 前缀不抢 latest | G22.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g22.wave.1.acceptance_map` | `ci/g22_acceptance_map_check.py` | 381 |
| 候选决策 | `g22.wave.1.candidate_decisions` | `ci/g22_candidate_decisions_check.py` | 382 |
| 互锁 | `g22.gov.implementation_interlock` | `ci/g22_interlock_check.py` | 383 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G22_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G21 defer-to-G22+ 六行本波逐行 disposition（候选决策表 §1 六行）。
2. 主轨 = slab 材质语义参考面（M-a）+ RD-041 四分项机器取证处置（M-b/M-c/M-d）。
3. RFC-0039 本波起草 + 对抗评审。
4. 治理三门步骤 381/382/383 顺位领取（落盘前实测 CI_step.next_free=381）。
5. 用户战役指令「帮我一次性完成G19-G25」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 先优化后测试：G22.2~G22.3 纯实现；G22.4 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G22-2 implementation_status 解锁记录（2026-08-24）

- **事实门全绿**：G21 closed + tag `g21-closed` + G22.0 不可变 ref `0a4b1df397ed79ab30380fe0b12a822027a18d78`；候选表 11 行零空行 + MAP 五行 P0；用户战役指令「帮我一次性完成G19-G25」字面 + workflow 末号 383 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g22_interlock_check.py --gate g22.gov.implementation_interlock` VERDICT=READY；治理三门 381/382/383 PASS（治理批 commit e304a275 落盘时 front matter 尚 blocked——本解锁记录与 flip 随实现批补落，符合 C2 双态一致性：解锁记录 ⇔ unlocked）。
- **解锁**：`implementation_status: blocked → unlocked`。G22.2+ 实现波（M-a~M-e）现可开工。

### §8.6 G22.5 P2 穷举 + stabilization soak 验收记录（2026-08-24）——G-G22-6 前置：P2 穷举决策门（g22.wave.5a.decisions，步骤 394，VERDICT=PASS）+ 稳定门 soak（g22.wave.5a.soak，步骤 395，8/8 facts VERDICT=PASS——69 迭代 wall=1846.2s ≥1800s 零失败）

- **① P2 穷举定盘**：`G22_P2_DECISIONS.md` 穷举闭集 **11 行零空行**（§1 六行 defer-to-G23+ 6；§3 期内行五行 closed-go 5；§2 open RD 八条维持 open——RD-041 四分项处置 history 只追加）。
- **② soak 定盘**：`py -3 ci/g22_stabilization_soak.py --gate` → VERDICT=PASS 8/8——M-d 前置绿 + **wall=1846.2s ≥1800s + 69 迭代零失败（含 slab 白炉审计车道穿插 13 次复跑）+ active==wall + 零 sleep**。
- **③ 命令输出**：P2 门 → VERDICT=PASS；budget_eval --strict 283 pass 零 skip 零 estimated；cargo test -p rurix-render --lib 486 passed 0 failed。
- **④ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G22.5 P2/soak 波）`。

### §8.7 G22.6 close-out 终审签署块（2026-08-24）——G-G22-6 字面兑现：close-out 终审门（g22.wave.6b.closeout，步骤 396）八 facts 全绿 **VERDICT=READY** → status active→closed + tag `g22-closed`

- **① 终审八 facts 逐条**：five_p0_evidence_green / p2_exhaustive_zero_empty / slab_realization_chain（白炉恒等 dev=0 + 恒等式 1e-15 + 双跑位级）/ rfc_0039_archived / old_gates_no_regression / rd_open_maintained（RD 八条 open；RD-041 history 只追加）/ soak_ge_1800_zero_fail / closeout_ready —— 全 PASS。
- **② 终审命令逐字输出**：`py -3 ci/g22_closeout_check.py --gate` → **VERDICT=READY，exit=0**。
- **③ 收口裁决**：材质/流送/时域字面兑现——**M-a slab 能量守恒参考臂 implemented**（RD-041 slab 分项兑现：白炉恒等 dev=0、能量上界 16641 样本、单调、恒等式 1e-15、lerp 连续、双跑位级；closure 单层生产面 0-byte）；**M-b SVT defer**（四行差距闭集）；**M-c KTX2-BasisU defer**（三行差距闭集，DDS 链维持）；**M-d Work Graphs not-available 实测**（AMDX absent）+ DGC available 实测 + FSR maintain；M-e 旧门零降级全绿。**defer/not-available/maintain 均为合法收口态**；G23 承接锚齐备（Jolt 5.6 评估臂/M127 重判窗）。
- **④ status flip 与 tag**：§8 只追加区本块落盘后，`status: active → closed`；`implementation_status: unlocked` 字面不动。flip commit 独立洁净落盘，随后 tag `g22-closed`。
- **⑤ 签署块**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G22 战役第四期收口）`。
