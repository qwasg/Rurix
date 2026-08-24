---
contract: G17
title: G17 DLSS 性能缺口收口期（G15-MD-F1 字面兑现）
status: active
implementation_status: unlocked
active_scope: g17_full_campaign
version: v1.0
date: 2026-08-24
timebox: "G17.1 治理波即刻执行（G16 已 closed，tag g16-closed）；G17.2~G17.6 严格波次 + G17.7a P2/soak + G17.7b close-out；用户 2026-08-24「帮我一次性完成G17」一次性收口战役字面；工期由 measured baseline 校准"
rfc_required: "G17.1 领取 RFC-0032（2026-08-24 实测 namespaces.RFC next_free=32）——D3D12 宿主 NGX 车道 Full RFC（跨 device 同步面 / 单 device 化评估），必须经 D-409 对抗性评审（评审 provenance ≠ 起草 provenance，findings 逐条 disposition）后 Agent Approved 方可实现；approved / no-go / defer 三态均为合法终态（no-go/defer 须留档可机器核验评估证据）。触其余冻结面必须独立 Full RFC，编号按起草时实测 next_free 领取，禁推测号；判档争议向上取严（10 §3）。"
upstream_docs:
  - "milestones/g15/G15_P2_DECISIONS.md v1.0 §2 G15-MD-F1 行 + §4 承接锚清单 G15-MD-F1 行（三件套字面）+ §5 汇总（G17 全部范围由此导出）"
  - "milestones/g16/G16_CONTRACT.md §7 立项裁决 3（G15 defer-to-G16+ 十四行 + G15-MD-F1 → defer-to-G17+）"
  - "milestones/g15/G15_CONTRACT.md §8.5~§8.7（G15-MD-F1 四轮复跑 17/18 定盘 + UE 暖态位移诊断定论 + NGXCubinVulkan 取证 + 税源分解 in-stream≈1.90ms/提交≈0.10ms/scene≈1.02ms 地板 3.02ms）"
  - "milestones/g14/g14_fps_gap_registry.json（gap_id 51a150cb4523e8b6 门产登记行——只登记不拟合，判据面 0-byte）"
  - "milestones/g13/g13_vendor_sdk_registry.json（Streamline 2.10.3 + nvngx_dlss.dll 310.5.2 provenance 登记，0-byte；G17 换版评估走新登记面）"
  - "registry/deferred.json RD-034/039/040/041/042/043/044/045（存续 open RD 八条；只追加禁静默改判）"
implementation_unlock:
  required_all:
    - "G17.1 治理门全部完成且有真实验证记录"
    - "ci/g17_interlock_check.py --require-ready 输出 READY（互锁 validator 机器事实，不以叙述替代）"
    - "用户 G17.2 开工指令留痕（2026-08-24「帮我一次性完成G17」字面 + 同指令十阶段推进程序全期授权面）"
    - "共享编号按互锁开放时 actual next_free 重新校准；数字 CI 步骤不得沿用推测号与草案建议值"
in_scope:
  - g17_1_governance_only
  - candidate_decisions_and_rd_mapping
  - p0_acceptance_mapping
  - g17_governance_three_gates_materialize
  - rfc_0032_d3d12_host_ngx_lane_full_rfc
  - measured_baseline_dual_end_retest
  - g17_2_m_a_dual_end_retest_warm_recalib
  - g17_3_m_b_ngx_evolution_alignment
  - g17_4_m_c_d3d12_host_lane_disposition
  - g17_5_m_d_t100_final_verdict
  - g17_6_m_e_closed_gate_no_regression
  - g17_p2_decisions_soak_closeout_tag
out_of_scope:
  - gi_expression_depth_rd040_p2_p4
  - virtualized_geometry_rd039
  - material_streaming_temporal_p3_rd041
  - frame_generation_fg_mfg_independent_layer
  - g18_direction_implementation
  - handwritten_or_loosened_thresholds
  - rewriting_g13_g15_g16_frozen_gap_registries_and_closed_evidence
  - coordinate_scale_rewrite
  - gi_off_default_arm_rewrite
  - legacy_gate_reissue_only_verify_latest
  - speculative_number_preclaim
deferred_refs:
  - "registry/deferred.json RD-034/039/040/041/042/043/044/045（存续 open RD 八条；只追加禁静默改判）"
deliverables:
  - id: D-G17-1
    check: "G17.1 完成门：D-G17-1~5 齐备并通过结构/schema/ledger/guardrail 核验；验收映射无缺行；无 src/spec/conformance 语义实现（治理期）；本门通过不自动开放实现"
  - id: D-G17-2
    check: "G17_CANDIDATE_DECISIONS：§1 G16 defer-to-G17+ 十四行 + G15-MD-F1 承接 15 行逐行转引处置 + §2 open RD 八条逐条映射 + §3 G17 新增候选四行，零空行"
  - id: D-G17-3
    check: "G17_ACCEPTANCE_MAP：5 个 P0 独立 symbolic gate key / 稳定脚本名 / evidence schema 目标路径 / 逐字判据，与契约 §4.2 双向逐字一致"
  - id: D-G17-4
    check: "治理三门（acceptance_map / candidate_decisions / implementation_interlock）真脚本真步骤 materialize（步骤 293/294/295 = 落盘前实测 CI_step.next_free=293 顺位领取），互锁按事实诚实输出（BLOCKED/READY 均为正确结论字面，不充绿）"
  - id: D-G17-5
    check: "RFC-0032 起草 + D-409 对抗评审记录（findings 逐条 disposition）+ measured baseline（真实硬件双端现状复测入 evidence，G14 M-d 门同口径一轮）"
acceptance_gates:
  - id: G-G17-1
    check: "G17.1 完成门：D-G17-1~5 齐备并通过结构/schema/ledger/guardrail 核验；验收映射无缺行；无 src/spec/conformance 语义实现（治理期）；本门通过不自动开放实现"
  - id: G-G17-2
    check: "实现互锁门：ci/g17_interlock_check.py --require-ready 输出 READY + 用户 G17.2 开工指令留痕（2026-08-24「帮我一次性完成G17」字面）+ 共享编号按 actual next_free 重新校准。任一条件不满足均保持 implementation_status=blocked"
  - id: G-G17-3
    check: "G17.2 退出门：M-a P0 独立断言全绿——双端复测四轮全协议 + UE 暖态基线程序产重标定入 g17_budget + 新旧环境差异如实分解"
  - id: G-G17-4
    check: "G17.3 退出门：M-b P0 独立断言全绿——NGX 310.6.0+ 换版评估 provenance 登记 + 形态核验 + X2 分解重测 + 画质守护双门禁 + A/B 结论如实登记（采纳/拒绝/零收益均合法）"
  - id: G-G17-5
    check: "G17.4 退出门：M-c P0 独立断言全绿——RFC-0032 终态兑现（approved 实现 / no-go / defer 留档均合法终态，终态字面入 evidence）"
  - id: G-G17-6
    check: "G17.5 退出门：M-d P0 独立断言全绿——终判 18 格全协议复测证据链完整 + 终判判定如实登记（达标 18/18 或维持未达标登记不冒充，二者均合法收口）"
  - id: G-G17-7
    check: "G17.6 退出门：M-e P0 独立断言全绿——G13/G14/G15/G16 受影响门 --verify-latest 零降级，禁 --gate，g17_ 前缀不抢 latest"
  - id: G-G17-8
    check: "P2 穷举决策门 + soak：G17_P2_DECISIONS 十五行 + 新增候选穷举零空行（defer 必有 G18+ 承接锚）+ soak ≥1800s 零失败 + budget_eval --strict 零 estimated"
  - id: G-G17-9
    check: "close-out：终审八 facts 全绿 VERDICT=READY 后 status active→closed（独立洁净 commit）+ tag g17-closed；终判两种结局（18/18 或维持未达标如实登记）均允许 close"
guardrails:
  - "双状态不可混同：status=active 仅表示 G17.1 governance-only 已立项；在 G-G17-2 真实通过前 implementation_status=blocked，任何治理完成叙述不得冒充 G17.2 开工"
  - "G17.1 允许 milestones/g17、G17 专属治理三门（ci/g17_*_check.py + evidence schema + workflow 步骤 293/294/295 按落盘前实测 next_free）、rfcs/0032 + 对抗评审、baseline 复测 evidence、deferred history 只追加；src/spec/conformance 0-byte（治理期）；G13/G15/G16 冻结登记表与已收口 evidence 0-byte 不回写"
  - "G17 P0 实现门 CI 只冻结 symbolic gate key 与脚本名；numeric_step 一律写 post-interlock actual-next-free allocation。不得沿用推测号与草案建议值，不得预放空 workflow、空脚本或空 schema 壳（G17.1 治理三门为例外：本波即落盘真脚本真步骤）"
  - "每个 P0 必须独立布尔断言与独立 evidence subject；可共享一次进程执行，但聚合 PASS 不能遮蔽任一子断言 FAIL/SKIP"
  - "禁手写/放宽阈值（P-09）：UE 暖态基线重标定与一切通过线必须程序产；×1.00 通过线口径本身 0-byte 不可下调"
  - "度量复测 = G14 M-d 门同口径复跑（消费其复跑件，不改其门脚本/判据/锚/协议 0-byte）；受影响旧门复测 = 各门 --verify-latest，禁止 --gate；g17_ 前缀不抢旧门 latest"
  - "NGX 换版程序面：nvngx_dlss.dll 310.5.2→310.6.0+ 评估走新缓存目录 + G17 新 provenance 登记面（milestones/g17/g17_vendor_sdk_registry.json，sha256 实测登记）；external/streamline-2.10.3 既有缓存与 g13_vendor_sdk_registry.json 0-byte；换版采纳必须过画质守护双门禁（Stage A digest 锚零漂移 + 画质锚带复核带内），超带即拒绝换版如实登记"
  - "终判两态合法：bistro-interior/t100/dlss_sr ratio ≥ ×1.00 → 性能 18/18；物理地板/vendor 面使达标不可能 → 维持未达标登记不冒充（兜底字面与 G15 同源，用户 2026-08-19 授权面逐字承接）；ratio 终值必须来自 evidence JSON 命令输出"
  - "UE 侧暖态事件 ≠ Rurix 侧收益，禁混淆归因；scene 面优化 L0 位级探针漂移即弃，禁碰 NGX 税源物理地板冒充收益"
  - "RD-034/039/040/041/042/043/044/045 八条 open 维持（条目级 status 0-byte，history 只追加）；G18 方向（GI P2~P4 / 虚拟化几何 / 帧生成独立层等）本窗只重评触发条件，零实现，不齐备则顺延留档"
  - "既有 84+ 门零降级；G5~G16 closed 契约与判据 0-byte；一切编号以 py -3 ci/check_number_ledger.py 实测 next_free 顺位领取"
---

# G17 DLSS 性能缺口收口期 契约

> 本契约是 G17 里程碑唯一事实源。front matter 双状态机：`status`（治理激活）与 `implementation_status`（实现解锁）严格分离。

## 1. 目标与双门状态

**目标（用户 2026-08-24 指令字面兑现面）**：「帮我一次性完成G17——DLSS 性能缺口收口期（G15-MD-F1 字面兑现）」——立项并一次性收口 G17：性能单格未达标（bistro-interior/t100/dlss_sr ratio 17/18）的法定承接期。G17 只做三件事 + 一套穷举：① 双端同协议复测与暖态重标定；② NGX 版本演进面对齐评估（nvngx_dlss.dll 310.5.2→310.6.0+）；③ 车道架构面 D3D12 宿主 NGX（Full RFC，触冻结面，经 D-409 对抗评审）；④ P2 穷举决策（G16 defer-to-G17+ 十四行 + G15-MD-F1 + 本期新增候选，逐行零空行）。**终判**：ratio ≥ ×1.00 → 性能 18/18；若物理地板/vendor 面使达标不可能 → 维持未达标登记不冒充。两种结局都允许 close。

G17.0 不可变 ref = `8fc1fdaa0a9c23c53d5a31b419ad5c9759aeaff8`（G16 close-out flip commit，tag `g16-closed`，实测 `git rev-parse g16-closed`）。

**双门状态**：`status: active`（G17.1 治理齐备后）+ `implementation_status: blocked`（G-G17-2 事实门全绿前）。

## 2. 范围与波次

- **G17.1 治理波**（本波）：契约四件套（PLAN/CONTRACT/CI_GATES/非空 measured `g17_budget.json` 零 estimated）+ 候选决策表 + 验收映射 5 P0 + RFC-0032 起草与 D-409 对抗评审 + measured baseline（双端现状复测）+ 治理三门 materialize + 互锁按事实诚实输出。
- **G17.2 M-a 双端复测与暖态重标定波**：同会话同协议四轮复跑；UE 暖态基线程序产新阈入 g17_budget；新旧环境差异如实分解。
- **G17.3 M-b NGX 310.6.0+ 对齐波**：SDK 演进 diff、PaddedWindowNetwork 形态核验、in-stream/提交税源重测分解；收益为零或负如实登记。
- **G17.4 M-c D3D12 宿主车道波**：RFC-0032 Approved 后实现（或 no-go/defer 留档后跳过实现，走兜底）；触 unsafe/FFI ABI 面按 unsafe 纪律。
- **G17.5 M-d t100 档优化与终判复测波**：scene ≈1.02ms 面有界优化（禁碰 NGX 税源物理地板冒充收益）；终判双端复测 18 格全协议；ratio 终值必须来自 evidence JSON 命令输出。
- **G17.6 M-e 旧门零降级波**：G13/G14/G15/G16 受影响门 `--verify-latest` 全绿；`g17_` 前缀不抢 latest。
- **G17.7a P2 穷举 + soak**：十五行 + 新增候选穷举零空行；soak ≥1800s 零失败 + `budget_eval --strict` 零 estimated。
- **G17.7b close-out**：终审八 facts VERDICT=READY → `status: active → closed`（独立洁净 commit）→ tag `g17-closed` + guardrail 基准链切换。

每波退出跑 `ci/g17_wave{N}_exit_check.py --gate g17.wave.{N}.exit`（只读汇总，不代绿）。波次内可并行、波次间不越级。

## 3. 治理波交付物（D-G17-1~5）

见 front matter deliverables / acceptance_gates 逐字判据；本波零 src/spec/conformance 语义实现（治理期）。RFC-0032 于本波起草并对抗评审（G17.1 允许面），实现推迟到 G17.4（且仅当 Approved）。

## 4. P0 独立断言表

### 4.1 统一纪律

接入/落盘 + 冻结面 0-byte（G13 锁定双差距登记表终态 / G15 处置与预算与读图记录终态 / G16 处置与预算终态 / G12 锁定 PT 差距登记表终态 / RXS-0386~0393 锁定度量口径 / g14_fps_gap_registry 判据面）+ measured 面程序产阈禁手写（P-09）+ 不降级既有门绿面 + 不改坐标尺度 + 旧门禁 `--gate` + UpscaleBackend trait 签名/temporal 底座/RXS-0357 面触碰须独立 Full RFC。

### 4.2 五行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | G14 M-d 门同口径协议双端复测（复测窗内同会话四轮全协议复跑，三轮进程级独立 50×3 trimmed mean 跨轮中位数零缩短，Stage A digest 锚守护）+ UE 参照臂暖态基线程序产重标定（复测窗 UE 逐格帧时包络程序产入 `g17_budget` 新条目，禁手写 P-09；`g14/g15/g16_budget` 既有条目 0-byte）+ 新旧环境差异如实分解（UE 侧暖态事件与 Rurix 侧变化分列登记，禁混淆归因） | G17.2 |
| **M-b** | NGX 版本演进面对齐评估：nvngx_dlss.dll 310.5.2→310.6.0+ 换版评估走新缓存目录 + G17 新 provenance 登记面 `milestones/g17/g17_vendor_sdk_registry.json`（g13 登记表 0-byte）+ PaddedWindowNetwork 实例化形态核验（SL verbose 日志逐字）+ in-stream/提交税源 X2 边际探针重测分解（对照 1.90+0.10ms 基线，新鲜命令输出）+ 画质守护双门禁（Stage A digest 锚零漂移 + 画质锚带复核带内，超带即拒绝换版）+ A/B measured 结论如实登记（采纳/拒绝/零收益均合法） | G17.3 |
| **M-c** | RFC-0032（D3D12 宿主 NGX 车道：跨 device 同步面/单 device 化评估）终态兑现——经 D-409 对抗评审（评审 provenance ≠ 起草 provenance，findings 逐条 disposition）后 approved/no-go/defer 三态均合法终态；approved → 实现（unsafe 纪律：`// SAFETY:` + unsafe-audit 注册条目 + 单块单操作）；no-go/defer → 可机器核验评估证据留档 + 兜底字面维持（RFC 终态字面入 evidence） | G17.4 |
| **M-d** | t100 档优化与终判复测：scene 面有界优化（L0 位级探针漂移即弃，禁碰 NGX 税源物理地板冒充收益）+ 终判双端 18 格全协议复测（G14 M-d 同口径，ratio 终值必须来自 evidence JSON 命令输出）+ 终判判定如实登记（达标 18/18 或维持未达标登记不冒充，二者均合法收口，兜底字面与 G15 同源） | G17.5 |
| **M-e** | G13/G14/G15/G16 受影响门 `--verify-latest` 全绿零降级；`g17_` 前缀不抢旧门 latest；禁对旧脚本发 `--gate` | G17.6 |

### 4.3 治理三门（本波即落盘真脚本真步骤）

| 门 | symbolic gate key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射核验 | `g17.wave.1.acceptance_map` | `ci/g17_acceptance_map_check.py` | 293（落盘前实测 CI_step.next_free=293 顺位领取） |
| 候选决策核验 | `g17.wave.1.candidate_decisions` | `ci/g17_candidate_decisions_check.py` | 294（同批顺位领取） |
| 实现互锁 | `g17.gov.implementation_interlock` | `ci/g17_interlock_check.py` | 295（同批顺位领取） |

## 5. Guardrails

见 front matter guardrails 十一条逐字。

## 6. Deferred 处置

RD-034/039/040/041/042/043/044/045 八条 open 维持（条目级 status 0-byte，history 只追加）。本波零新 RD（max=RD-045 维持，实测 next_free=46）。未竟事项在战后阶段按实测 next_free 追加 RD + `// STUB(RD-###)` 双侧标注。

## 7. 修订与开工裁决

- **立项裁决 1**：现在立项 G17；G17.0 不可变 ref = `8fc1fdaa0a9c23c53d5a31b419ad5c9759aeaff8`（G16 close-out flip commit，tag `g16-closed`）。
- **立项裁决 2**：用户 G17.2 开工指令留痕——2026-08-24 用户指令「帮我一次性完成G17」字面 + 十阶段推进程序全期授权面（同指令 §3 波次内可并行、波次间不越级）。实现互锁未过前 `implementation_status=blocked`。
- **立项裁决 3**：本期只承接 G15-MD-F1（① 双端同协议复测与暖态重标定 ② NGX 版本演进面对齐 ③ D3D12 宿主 NGX 车道 Full RFC）。G16 `defer-to-G17+` 十四行本期重评窗结论逐行入候选决策表；G18 方向零实现。RFC-0032 本波领取（实测 RFC next_free=32）。
- **立项裁决 4**：治理三门步骤 293/294/295 = 落盘前实测 `CI_step.next_free=293` 顺位领取。P0 实现门 numeric_step 一律 `post-interlock actual-next-free allocation`。
- **立项裁决 5**：终判两态合法收口——达标 18/18 或维持未达标如实登记均允许 close（G-G17-9 字面）；M-d 门内「协议完整性/证据链」与「ratio 达标判定」双断言分离，后者红时如实登记不遮蔽（沿 G15 §8.5 诚实红先例）。

## 8. Implementation activation / Close-out（只追加区）

<!-- 本区只追加。开工时为空；禁止预填 PASS；每波验收记录按五块模板追加（独立断言清单 / 波聚合门实测输出 / 验收命令逐字输出 / 门序与登记面摘要 / 签署块）。 -->

### §8.1 G-G17-2 implementation_status 解锁记录（2026-08-24）

- **事实门①~④全绿**：G16 closed + §8.4 签署块在位 + G17.0 不可变 ref `8fc1fdaa0a9c23c53d5a31b419ad5c9759aeaff8`（实测 `git rev-parse g16-closed`）；候选表 19 行零空行 + deferred 只追加（vs G17.0 base 四字段 0-byte）+ MAP 五行 P0；用户指令「帮我一次性完成G17」字面 + workflow 末号 295 == ledger on_tree_max 295 且 next_free=296（v1.162 校准）；治理两门独立 PASS（`g17_acceptance_map_check_20260824T061203Z.json` 12 facts + `g17_candidate_decisions_check_20260824T061204Z.json` 32 facts）。
- **机器事实**：`py -3 ci/g17_interlock_check.py --gate g17.gov.implementation_interlock` VERDICT=READY（evidence/g17_interlock_check_20260824T061205Z.json，八 facts 全 PASS）；三门 selftest 全 PASS（9 RED+GREEN / 11 RED+双 GREEN / 17 RED+GREEN+TREE）。
- **G17.0 measured baseline 定盘**：G14 M-d 门同口径一轮真跑（evidence/g14_m_d_dual_end_fps_parity_20260824T054145Z.json，VERDICT=FAIL 17/18 诚实红）——**bistro-interior/t100/dlss_sr ratio=0.9810**（UE=3.723ms / Rurix=3.795ms）。环境新事实如实登记：UE 参照臂本会话回升至 3.723ms（G15 期暖态定盘 2.96~3.00ms → +24%），**暖态包络双向波动实证**（G15plus「暖态为新环境基线面」的基线本身跨会话非单调——M-a 重标定必须以复测窗内包络为准，禁用单点历史值冒充基线）；Rurix 臂 3.795ms（G15 期带 3.482~3.657ms 微升，同会话环境面）。ratio 距 ×1.00 差 1.9%（G15 期四轮 0.833~0.857 → 本 baseline 0.981）。g17_budget.json 双条目程序产（threshold = measured × 2.0 宽上界守护，budget_eval --strict 263 pass 零 skip 含新条目）；g14_fps_gap_registry.json gap_id `51a150cb4523e8b6` 门产刷新行（门自身登记面，判据面 0-byte）。
- **RFC-0032**：`rfcs/0032-d3d12-host-ngx-lane.md` v0.2 Agent Approved（决策程序 + 实现语义；D-409 对抗评审 6 findings 全 disposition，评审 `milestones/g17/design/rfc0032_adversarial_review.md`；终态 disposition 待 G17.4 M-c 按 §5 决策树以 M-a/M-b 实测为输入程序产出）。
- **解锁**：`implementation_status: blocked → unlocked`。G17.2+ 实现波（M-a~M-e）现可开工。P0 数字步骤按当时实测 `CI_step.next_free`（快照 296）顺位领取。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v3.0 agent 完全自主签署）。`Assisted-by: Cursor Claude Fable 5（G17.1 治理波 + G17.2 开工）`（影响范围：milestones/g17/ 全新目录〔契约四件套 + 候选表 + MAP + 治理三门 schema 三件 + design/rfc0032_adversarial_review.md〕+ rfcs/0032 + ci/g17_{acceptance_map,candidate_decisions,interlock}_check.py 三真脚本 + ci/check_schemas.py 九处纯追加 + ci/budget_eval.py eval_g17_dual_end_cell 判读路由纯追加 + .github/workflows/pr-smoke.yml 步骤 293~295 + registry/number_ledger.json v1.162〔CI_step 292→295/RFC 31→32〕+ evidence 本批真跑件；src/spec/conformance 0-byte；G13/G15/G16 冻结面 0-byte；deferred.json 0-byte；验证方式：治理三门 --gate 全 PASS + 三 selftest 全 PASS + check_structure/check_schemas/check_number_ledger PASS + budget_eval --strict 263 pass + pytest 142 passed + baseline 真跑件 17/18 诚实红如实登记）。
