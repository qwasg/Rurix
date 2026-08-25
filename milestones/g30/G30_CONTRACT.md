---
contract: G30
title: G30 商用终审收官期（尾锚重判闭集 + 三面商用终审 + 全链零降级 + 战役承接锚归档）
status: active
implementation_status: unlocked
active_scope: g30_campaign_final_review
version: v1.0
date: 2026-08-25
timebox: "G30.1 治理波即刻执行（G29 已 closed，tag g29-closed）；G30.2~G30.4 严格波次 + G30.5 P2/soak + G30.6 close-out；用户「帮我一次性完成G26-G30」战役字面（G30 = 五期串行战役收官期，里程碑间不越级）"
rfc_required: "G30.1 领取 RFC-0047（实测 RFC next_free=47）——G30 战役商用终审收官程序:尾锚重判闭集 + 三面商用终审 + 全链零降级 + 承接锚归档（文件名 rfcs/0047-campaign-final-review.md）；经对抗评审后 Agent Approved 方可实现；达标/维持/诚实红均合法终态。"
upstream_docs:
  - "milestones/g25/g25_campaign_handover_registry.json（M125-adopt3/M127/M114-strand/M118-hdr-cal/G10-N6/SAFE-GPU/G17-MD-F1 七行）"
  - "registry/deferred.json（RD-034/039/040/041/042/043/044/045 八条）"
  - "milestones/g24/g24_legacy_rd_registry.json（历史清册）"
  - "milestones/g26/G26_P2_DECISIONS.md + milestones/g27/G27_P2_DECISIONS.md + milestones/g28/G28_P2_DECISIONS.md + milestones/g29/G29_P2_DECISIONS.md（G26~G29 四期 P2 表，战役期承接锚）"
implementation_unlock:
  required_all:
    - "G30.1 治理门全部完成且有真实验证记录"
    - "ci/g30_interlock_check.py --require-ready 输出 READY"
    - "用户 G30.2 开工指令留痕（「帮我一次性完成G26-G30」战役字面）"
in_scope:
  - g30_1_governance_only
  - tail_anchor_rejudgment_closure
  - commercial_final_review
  - campaign_full_chain_no_regression
  - campaign_handover_ledger
  - closed_gate_no_regression
  - g30_p2_decisions_soak_closeout_tag
out_of_scope:
  - new_optimization_or_feature_work
  - ue_full_render_rerun_without_surface_change_evidence
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g29_frozen_registries
  - jolt_56_switch_implementation
  - hdr_display_chain_implementation
deliverables:
  - id: D-G30-1
    check: "G30.1 四件套 + 候选决策 12 行 + 验收映射 5 P0 + RFC-0047 + 治理三门 509/510/511"
acceptance_gates:
  - id: G-G30-1
    check: "G30.1 完成门：D-G30-1 齐备；治理三门 PASS"
  - id: G-G30-2
    check: "互锁门：ci/g30_interlock_check.py --require-ready READY + 战役开工指令留痕"
  - id: G-G30-3
    check: "G30.2 退出门：M-a/M-b P0 全绿"
  - id: G-G30-4
    check: "G30.3 退出门：M-c/M-d P0 全绿"
  - id: G-G30-5
    check: "G30.4 退出门：M-e P0 全绿"
  - id: G-G30-6
    check: "P2 + soak + close-out → tag g30-closed（战役收官）"
guardrails:
  - "双状态机：status=active + implementation_status=blocked 直至 G-G30-2"
  - "终审零冒充：尾锚/画质/性能终态以机器事实定盘，达标/维持/诚实红均合法"
  - "战役期加性纪律回验：G26~G29 全部新增面对默认臂/冻结面 0-byte（git-diff 机核）"
  - "no-go/defer/not-available/maintain/诚实红均为合法终态"
  - "commit 带 Assisted-by: trailer 且不 push"
---
<!-- Assisted-by: Cursor Agent(G30.1 治理波) -->

# G30 商用终审收官期 契约

> front matter 双状态机：`status` 与 `implementation_status` 严格分离。

## 1. 目标

用户战役指令字面：**帮我一次性完成G26-G30**（2026-08-25，五期串行战役全期授权；G30 = 收官期）；用户原指令含**要求完成商业化使用标准收尾**字面一并登记。G30 = 商用终审收官期（G25 收官期同构）：尾锚重判闭集（六件外部条件类尾锚机器取证重判 + RD-042/043/044 同批逐锚）、三面商用终审（画质/性能/确定性终态定盘，G17-MD-F1 终判法定义务两态程序）、战役全链零降级（G29 递归链自动涵盖 G26~G28 及更早）、战役承接锚归档闭集（G31+ 唯一法定输入面）。

G30.0 不可变 ref = `0d0a4c8a2821c8bab672418b67bcd078d7f8b267`（G29 close-out flip commit，tag `g29-closed`）。

## 2. 范围与波次

| 波次 | 内容 | 门 |
|---|---|---|
| G30.1 | 治理波 + RFC-0047 起草 + baseline 快检 | G-G30-1 |
| 互锁 | `--require-ready` READY | G-G30-2 |
| G30.2 | M-a 尾锚重判闭集 + M-b 三面商用终审 | G-G30-3 |
| G30.3 | M-c 战役全链零降级 + M-d 战役承接锚归档 | G-G30-4 |
| G30.4 | M-e 旧门零降级（全量测试波） | G-G30-5 |
| G30.5~6 | P2/soak/close-out/tag（战役收官） | G-G30-6 |

## 3. 治理波交付物

D-G30-1：PLAN/CONTRACT/CI_GATES/g30_budget.json + G30_CANDIDATE_DECISIONS + G30_ACCEPTANCE_MAP + RFC-0047 + 对抗评审 + 治理三门。

## 4. P0 断言

### 4.2 五行 P0

| M 行 | 判据(逐字) | 波次 |
|---|---|---|
| **M-a** | 尾锚重判闭集:六件外部条件类尾锚机器取证重判——M125-adopt3(Jolt 5.6 需求证据三类树内实测 + sys56 评估臂 cargo check 新鲜)+ M127(corpus 目录 + PhysicsAsset residual 消费方检索)+ M114-strand(毛发资产入压测闭集检索)+ M118-hdr-cal(vulkaninfo HDR token 新鲜探针)+ G10-N6(fbx2gltf/assimp/blender 三工具 PATH 实测 + 源资产检索)+ SAFE-GPU(独立期资源窗 + 平台需求方文档检索);RD-042/043/044 三条 G30 尾锚窗同批逐锚重判;各件 searched-paths manifest 必填,全未命中 → 逐件维持诚实终态零冒充;deferred history 只追加 | G30.2 |
| **M-b** | 三面商用终审:画质面——画质表面闭集 0-byte 机核(vs g25-closed git-diff)+ 战役期加性面(四 kernel/四 device bin)零接线核验 + G18 M-d 达标绿件只读盘点;性能面——G14 M-d 最新 18 格 evidence 如实定盘 + 性能面 0-byte 机核 + 焦点格新鲜单测真跑(bistro-interior/t100/dlss_sr canonical 160 帧 ratio 登记,G17-MD-F1 终判法定义务:≥1.00 → 18/18 或物理不可达维持 17/18 诚实红终判,两态均为战役合法收官态);确定性面——Stage A 18 格 digest 锚在档 + 战役期四 device kernel 双跑位级绿件盘点;三面终态如实定盘零冒充 | G30.2 |
| **M-c** | 战役全链零降级:G29 受影响门 `--verify-latest` 全绿(递归链自动涵盖 G26~G28 及更早)+ budget_eval --strict 全量零 skip 零 estimated;禁 `--gate` 旧脚本 | G30.3 |
| **M-d** | 战役承接锚归档闭集:g30_campaign_handover_registry.json(五期 defer/maintain 行 + RD 八条 G31+ 锚 + 历史清册引用 + 尾锚六件重判终态)全量汇总闭集登记——G31+ 唯一法定输入面;归档完整性机核 | G30.3 |
| **M-e** | G29 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g30_` 前缀不抢 latest | G30.4 |

### 4.3 治理三门

| 门 | key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射 | `g30.wave.1.acceptance_map` | `ci/g30_acceptance_map_check.py` | 509 |
| 候选决策 | `g30.wave.1.candidate_decisions` | `ci/g30_candidate_decisions_check.py` | 510 |
| 互锁 | `g30.gov.implementation_interlock` | `ci/g30_interlock_check.py` | 511 |

## 5. Guardrails

见 front matter guardrails 逐字。

## 6. 实现互锁

同 G30_ACCEPTANCE_MAP §6。

## 7. 立项裁决

1. G25 交接登记表七行（M125-adopt3/M127/M114-strand/M118-hdr-cal/G10-N6/SAFE-GPU/G17-MD-F1）本波逐行 disposition（候选决策表 §1 七行）。
2. 主轨 = 终审四面（尾锚重判闭集/三面商用终审/全链零降级/承接锚归档）。
3. RFC-0047 本波起草 + 对抗评审。
4. 治理三门步骤 509/510/511 顺位领取（落盘前实测 CI_step.next_free=509）。
5. 用户战役指令「帮我一次性完成G26-G30」+「要求完成商业化使用标准收尾」登记（本契约 §1 字面；共享 D/U 段零消费）。
6. 纯核验波次序：G30.2~G30.3 纯核验；G30.4 全量测试波一次。

## 8. Close-out 区

G30.2+ 各波验收记录与收口签署块只追加登记于此（§8.1 解锁记录 → §8.6 P2/soak 验收记录 → §8.7 close-out 终审签署块 → status flip → tag `g30-closed`）；治理波零预写。

### §8.1 G-G30-2 implementation_status 解锁记录（2026-08-25）

- **事实门全绿**：G29 closed + close-out 签署块在位 + G30.0 不可变 ref `0d0a4c8a2821c8bab672418b67bcd078d7f8b267`；候选表 12 行零空行 + MAP 五行 P0 + §2 零 go P1 空集；用户战役指令「帮我一次性完成G26-G30」字面 + workflow 末号 511 == ledger on_tree_max。
- **机器事实**：`py -3 ci/g30_interlock_check.py --gate g30.gov.implementation_interlock` VERDICT=READY（步骤 511，8/8 facts PASS）；治理三门 509/510/511 绿件；RFC-0047 经 D-409 对抗评审（18 findings 全 disposition：blocker 0/major 5/minor 13——F1 Jolt 错标、F2 零接线恒真判据重写两层、F3 两半锚 G30 新鲜检索补程序、F4 verify-latest 语义如实化、F5 归档字段分 section 钉死均已落字面，v0.2）Agent Approved。
- **解锁**：`implementation_status: blocked → unlocked`。G30.2+ 纯核验波（M-a~M-e）现可开工。

### §8.6 G30.5 P2 穷举 + stabilization soak 验收记录（2026-08-25）——G-G30-6 前置：P2 穷举决策门（g30.wave.5a.decisions，步骤 522，VERDICT=PASS）+ 稳定门 soak（g30.wave.5a.soak，步骤 523，8/8 facts VERDICT=PASS——67 迭代 wall=1817.7s ≥1800s 零失败）

- **① P2 穷举定盘**：`G30_P2_DECISIONS.md` 穷举闭集 **12 行零空行**（§1 七行 closed-go 7〔六件尾锚重判兑现全维持：M125 maintain-5.3〔三类 1/3〕+ M127 研究子轨〔两半 0/2〕+ M114 card/mesh〔SKIP 兜底〕+ M118 maintain-SDR〔三 token absent〕+ G10-N6 双场景闭集〔三工具缺〕+ SAFE-GPU defer 改锚 defer-to-G31+ + **G17-MD-F1 17/18 诚实红终判定盘**〔焦点格 160 帧新鲜真跑 ratio=0.960479 < 1.00，两半锚 6 pattern 零命中〕〕；§3 期内行五行 closed-go 5；§2 open RD 八条维持 open〔RD-042/043/044 尾锚窗承载 + RD-045 复核承载〕）。
- **② soak 定盘**：VERDICT=PASS 8/8——**wall=1817.7s ≥1800s + 67 迭代零失败（含八车道探针轮换穿插 13 次：g19 framegen/g20 hzb/g21 restir/g22 slab 四实现件 + g26 framegen/g27 hzb/g28 restir/g29 slab 四 device 零容差快车道）+ active==wall + 零 sleep**。
- **③ 命令输出**：P2 门 → VERDICT=PASS（g30_p2_decisions_check_20260825T103353Z）；soak → VERDICT=PASS（g30_stabilization_soak_20260825T110425Z）；wave2~wave6 聚合门五绿（守卫三件 + budget_eval 现场通过）。
- **④ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Agent（G30.5 P2/soak 波）`。
