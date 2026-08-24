---
contract: G18
title: G18 全量方向一次性收口期（光线画质 + 性能 + 虚拟化几何 + 帧生成）
status: closed
implementation_status: unlocked
active_scope: g18_full_campaign
version: v1.0
date: 2026-08-24
timebox: "G18.1 治理波即刻执行（G17 已 closed，tag g17-closed）；G18.2~G18.7 严格波次 + G18.8 P2/soak + G18.9 close-out；用户 U-59「G18全量方向一次性完成计划」字面"
rfc_required: "G18.1 领取 RFC-0033/0034/0035（实测 RFC next_free=33）——光线画质 presentation 双 profile / mesh shader P3 / 帧生成独立层；各经 D-410~D-412 对抗评审后 Agent Approved 方可实现；approved/no-go/defer 均合法终态。"
upstream_docs:
  - "milestones/g17/G17_P2_DECISIONS.md §3 defer-to-G18+ 十六行 + G17_CONTRACT.md §8.7"
  - "milestones/g16/G16_CONTRACT.md M-g 商用画质 18/18 定盘"
  - "milestones/g14/g14_3_stage_a_digest_anchor.json"
implementation_unlock:
  required_all:
    - "G18.1 治理门全部完成且有真实验证记录"
    - "ci/g18_interlock_check.py --require-ready 输出 READY"
    - "用户 G18.2 开工指令留痕（U-59：G18全量方向一次性完成计划）"
in_scope:
  - g18_1_governance_only
  - rurix_light_transport_depth
  - presentation_pipeline_dual_profile
  - ue_arm_lighting_repair_and_render
  - dual_end_commercial_quality_verdict
  - sl_runtime_upgrade_disposition
  - fps_parity_reeval
  - virtualized_geometry_p3
  - frame_generation_independent_layer
  - closed_gate_no_regression
  - g18_p2_decisions_soak_closeout_tag
out_of_scope:
  - safe_gpu_operator_platform_independent_period
  - neural_deform_research_subtrack
  - bistro_exterior_scene_expansion
  - hdr_device_calibration_layer
  - jolt_56_adoption_three_pieces
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g17_frozen_registries
deliverables:
  - id: D-G18-1
    check: "G18.1 四件套 + 候选决策 25 行 + 验收映射 9 P0 + RFC-0033/0034/0035 + 治理三门 309/310/311"
acceptance_gates:
  - id: G-G18-1
    check: "G18.1 完成门：D-G18-1 齐备；治理三门 PASS"
  - id: G-G18-2
    check: "互锁门：ci/g18_interlock_check.py --require-ready READY + U-59 留痕"
  - id: G-G18-3
    check: "G18.2 退出门：M-a/M-b P0 全绿"
  - id: G-G18-4
    check: "G18.3 退出门：M-c P0 全绿"
  - id: G-G18-5
    check: "G18.4 退出门：M-e/M-f P0 全绿"
  - id: G-G18-6
    check: "G18.5 退出门：M-g P0 全绿"
  - id: G-G18-7
    check: "G18.6 退出门：M-h P0 全绿"
  - id: G-G18-8
    check: "G18.7 退出门：M-d/M-i P0 全绿"
  - id: G-G18-9
    check: "P2 + soak + close-out → tag g18-closed"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G18-2"
  - "默认臂 Stage A digest 锚零漂移为红线；新特性走加性 profile/新契约"
  - "G13 冻结契约 0-byte；presentation 走 g18_presentation_contract.json"
  - "no-go/defer/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G18 全量方向一次性收口期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户 U-59 指令字面：**G18全量方向一次性完成计划**——双臂(Rurix + UE5)光线与画质修复并输出商业化水平渲染图片(夜景+日景双 profile)，同时立项性能(SL 升级+17/18 格重评)、虚拟化几何 P3、帧生成独立层；**先优化后测试**压缩工期。

G18.0 不可变 ref = `3b8ac48ed657de90f1fa0365a4bb92b044c0e440`（G17 close-out flip commit，tag `g17-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G18.1 | 治理波 + RFC 起草 + baseline 快检 | G-G18-1 |
| 互锁 | `--require-ready` READY | G-G18-2 |
| G18.2 | M-a 光照纵深 + M-b presentation 出图 | G-G18-3 |
| G18.3 | M-c UE 臂修复与日景 | G-G18-4 |
| G18.4 | M-e SL 升级 + M-f 性能格重评 | G-G18-5 |
| G18.5 | M-g mesh shader P3 | G-G18-6 |
| G18.6 | M-h 帧生成独立层 | G-G18-7 |
| G18.7 | M-d 商业化画质终审 + M-i 旧门零降级 | G-G18-8 |
| G18.8~9 | P2/soak/close-out/tag | G-G18-9 |

## 3. 治理波交付物

D-G18-1：PLAN/CONTRACT/CI_GATES/g18_budget.json + G18_CANDIDATE_DECISIONS + G18_ACCEPTANCE_MAP + RFC-0033/0034/0035 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 九行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | 天光/IBL + 镜面反射 + 软阴影 + 降噪 + GI 纵深加性 profile 实现；默认臂 `--gi off` Stage A digest 锚 18 格零漂移；加性 profile 走 `--presentation-profile` 或 `--gi on` 独立登记面 | G18.2 |
| **M-b** | 后处理链（exposure/bloom/tonemap）接入 + PNG 出图 + `g18_presentation_contract.json` 夜/日双 profile；收敛帧 ≥128；G13 冻结契约 0-byte | G18.2 |
| **M-c** | UE 臂 bistro 灯光/曝光校准 + 日景关卡 variant（DirectionalLight+SkyLight）+ MRQ presentation 出图（夜/日 × 两场景）+ `-renderoffscreen` UE 5.8 可用性实测 | G18.3 |
| **M-d** | 双端商业化画质终审：AI 读图逐格 + SSIM/FLIP 程序产阈（p100×2.0 禁手写）；达标/诚实红均合法；G10-N17 FLIP 演进位 + G11-N5 暗帧数据集顺带兑现 | G18.7 |
| **M-e** | G17-MB-F1 兑现：新版 Streamline 换版/拒绝换版/not-available 均合法终态；provenance 登记 + 画质守护双门禁 | G18.4 |
| **M-f** | G17-MD-F1 兑现：G14 M-d 同口径 18 格重评；≥1.00 → 18/18；物理不可达 → 维持未达标登记不冒充 | G18.4 |
| **M-g** | RFC-0034 终态兑现：mesh shader VisBuffer 第三光栅路径实现 / no-go / defer 均合法；像素零差判据或评估证据留档 | G18.5 |
| **M-h** | RFC-0035 终态兑现：FG/MFG 独立层（真实渲染帧率口径，禁混入 upscale ratio）；实现 / no-go / defer 均合法 | G18.6 |
| **M-i** | G13~G17 受影响门 `--verify-latest` 全绿零降级；禁 `--gate`；`g18_` 前缀不抢 latest | G18.7 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g18.wave.1.acceptance_map` | `ci/g18_acceptance_map_check.py` | 309 |
| 候选决策 | `g18.wave.1.candidate_decisions` | `ci/g18_candidate_decisions_check.py` | 310 |
| 互锁 | `g18.gov.implementation_interlock` | `ci/g18_interlock_check.py` | 311 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G18_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G17 defer-to-G18+ 十六行本波逐行 disposition（候选决策表）。
2. 五轨全量 in_scope（画质 + 性能 + mesh shader + frame gen）。
3. bistro 增设日景 presentation profile（G13 契约 0-byte）。
4. RFC-0033/0034/0035 本波起草 + 对抗评审。
5. 治理三门步骤 309/310/311 顺位领取。
6. U-59 用户指令登记。
7. 先优化后测试：G18.2~G18.6 纯实现；G18.7 全量测试波一次。

## 8. Close-out 区

### §8.1 G-G18-2 implementation_status 解锁记录（2026-08-24）

- **事实门全绿**：G17 closed + tag `g17-closed` + G18.0 不可变 ref `3b8ac48ed657de90f1fa0365a4bb92b044c0e440`；候选表 16 行零空行 + MAP 九行 P0；用户 U-59「G18全量方向一次性完成计划」字面 + workflow 末号 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g18_interlock_check.py --require-ready` VERDICT=READY；治理三门 309/310/311 PASS（`g18_acceptance_map_check` / `g18_candidate_decisions_check` / `g18_interlock_check`）。
- **解锁**：`implementation_status: blocked → unlocked`。G18.2+ 实现波（M-a~M-i）现可开工。

### §8.6 G18.8 P2 穷举 + stabilization soak 验收记录（2026-08-24）——G-G18-9 前置：P2 穷举决策门（g18.wave.8a.decisions，步骤 330，VERDICT=PASS）+ 稳定门 soak（g18.wave.8a.soak，步骤 331，8/8 facts VERDICT=PASS——49 迭代 wall=1821.0s ≥1800s 零失败）

- **① P2 穷举定盘**：`G18_P2_DECISIONS.md` 穷举闭集 **25 行零空行**（§1 十六行：closed-go 7 + no-go 1 + defer-to-G19+ 8；§3 期内行九行 closed-go 9；§2 open RD 八条维持 open 0-byte）；汇总 closed-go 16 + defer-to-G19+ 9 + no-go 1。
- **② soak 定盘**：`py -3 ci/g18_stabilization_soak.py --gate` → VERDICT=PASS 8/8（evidence/g18_stabilization_soak_20260824T135718Z.json）——M-f 前置绿 + **wall=1821.0s ≥1800s + 49 迭代零失败（fails=0/49）+ active==wall drift=0.000 + 零 sleep** + 默认臂 `--gi off` 迭代体。
- **③ 命令输出**：P2 门 → VERDICT=PASS（evidence/g18_p2_decisions_check_20260824T132800Z.json）；soak schema 校准（`g18.wave.8a.soak` 与 workflow 步骤 331 对齐）后 evidence 绿；budget_eval --strict 271 pass 零 skip 零 estimated。
- **④ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G18.8 P2/soak 波）`。

### §8.7 G18.9 close-out 终审签署块（2026-08-24）——G-G18-9 字面兑现：close-out 终审门（g18.wave.9b.closeout，步骤 332）八 facts 全绿 **VERDICT=READY** → status active→closed + tag `g18-closed`

- **① 终审八 facts 逐条（evidence/g18_wave9b_closeout_20260824T135747Z.json）**：
  1. **nine_p0_evidence_green = PASS**（M-a~M-i 九 P0 最新 evidence host_section_pass 全真）。
  2. **p2_exhaustive_zero_empty = PASS**（P2 门 132800Z——25 行零空行）。
  3. **fps_reeval_chain = PASS**（M-f ratio 终值 **0.856326，维持未达标登记不冒充 17/18**——G17-MD-F1 重评窗字面兑现）。
  4. **rfc_0033/0034/0035_archived = PASS**（RFC-0033 presentation 双 profile / RFC-0034 mesh shader **no-go** / RFC-0035 帧生成 **defer**——三态均合法终态）。
  5. **old_gates_no_regression = PASS**（M-i 旧门零降级全绿）。
  6. **rd_open_maintained = PASS**（RD-034~045 八条 open 维持）。
  7. **soak_ge_1800_zero_fail = PASS**（135718Z：49 迭代 wall=1821.0s ≥1800s 零失败）。
  8. **closeout_ready = PASS**（VERDICT=READY）。
- **② 终审命令逐字输出**：`py -3 ci/g18_closeout_check.py --gate` → **VERDICT=READY，exit=0**。
- **③ 收口裁决**：五轨全量字面兑现——轨A 双臂光线画质与 presentation 双 profile 出图（M-a/M-b/M-c/M-d）；轨B SL 升级 **not-available** + 性能格 **17/18 诚实红**（M-e/M-f）；轨C mesh shader P3 **no-go**（M-g）；轨D 帧生成独立层 **defer-to-G19+**（M-h）；M-i 旧门零降级全绿。**no-go/defer/诚实红均为合法收口态**；G19+ 承接锚齐备。
- **④ status flip 与 tag**：§8 只追加区本块落盘后，`status: active → closed`；`implementation_status: unlocked` 字面不动。flip commit 独立洁净落盘，随后 tag `g18-closed`。
- **⑤ 签署块**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G18 一次性收口战役）`。
