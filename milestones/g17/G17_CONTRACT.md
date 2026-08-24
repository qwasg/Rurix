---
contract: G17
title: G17 DLSS 性能缺口收口期（G15-MD-F1 字面兑现）
status: closed
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

### §8.2 G17.2 M-a 双端复测与暖态重标定波验收记录（2026-08-24）——G-G17-3 字面兑现：M-a 门（g17.p0.m_a.dual_end_retest_warm_recalib，步骤 296，10/10 facts VERDICT=PASS）+ 波聚合门 g17.wave.2.exit（步骤 297）VERDICT=PASS

- **① 独立断言**：复测窗（stamp ≥ 20260824T054145Z）四轮 G14 M-d 同口径全协议复跑消费（054145Z/062859Z/070924Z/074948Z——四轮 50×3 三轮进程级独立 + 生产口径 v2 零缩短 + Stage A digest 18 格 × 3 轮 == 冻结锚位级全等四轮全绿〔RD-045 同型漂移零检出登记〕）+ 本格窗内逐轮 ratio = [0.9810, 0.8157, 0.7966, 0.8086]（登记面，达标判定归 M-d 终判）+ 6 UE 格暖态包络条目程序产入 g17_budget（threshold = 窗 max × 2.0，f64 精确重算 == 存储值，禁手写 P-09；g14/g15/g16_budget git 机核 0-byte）+ 新旧环境差异分解（UE 侧：G15 第四轮定盘 3.0023ms → 本窗 median 3.1922ms +6.3%，窗内 [3.723, 2.983, 2.985, 3.400]——**暖态包络双向波动实证**，会话初慢态 3.72 → 稳定快态 2.98 双态分布；Rurix 侧分列：3.5391 → 3.7711ms +6.6% 归环境面非代码收益，码面位级同 G14.12 定盘态）+ RED 四臂（三轮冒充四轮/窗外旧件/digest 漂移静默/手写阈——注入全检出）。
- **② 命令输出**：`py -3 ci/g17_dual_end_retest_warm_recalib_smoke.py --gate …` → VERDICT=PASS 10/10（evidence/g17_m_a_dual_end_retest_warm_recalib_20260824T083524Z.json）；首跑 083423Z 红件在档不删（budget_eval 对暖态条目缺判读路由 KeyError——即修 eval_g17_dual_end_cell 通用格解析〔id 解析 scene/tier/backend〕+ g17.m_a.warm_ue_frame_ms. 前缀路由纯追加后复跑绿，budget_eval --strict 269 pass 零 skip）；`py -3 ci/g17_wave_exit_check.py --gate g17.wave.2.exit` → VERDICT=PASS 四 facts（evidence/g17_wave2_exit_20260824T083537Z.json）。
- **③ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Claude Fable 5（G17.2 M-a 波）`（影响范围：ci/g17_dual_end_retest_warm_recalib_smoke.py + ci/g17_wave_exit_check.py〔参数化五 key〕+ 双 schema + g17_budget 六条目程序产追加 + budget_eval 判读路由修正；验证方式：M-a 门 10/10 + wave2 四 facts + selftest 全 PASS）。

### §8.3 G17.3 M-b NGX 版本演进面对齐评估波验收记录（2026-08-24）——G-G17-4 字面兑现：M-b 门（g17.p0.m_b.ngx_evolution_alignment，步骤 298，10/10 facts VERDICT=PASS）+ 波聚合门 g17.wave.3.exit（步骤 299）VERDICT=PASS；**评估结论 = reject_version_swap（310.6.0 在 SL 2.10.3 pin 下 DLSSContext 兼容性不可用，如实登记）**

- **① 评估定盘**：评估缓存 external/streamline-2.10.3-ngx310.6.0（sl 三件原件 sha256 == g13 登记逐字 + nvngx_dlss.dll 310.6.0〔UE 项目 DLSS 插件同款，sha256 099b3e1e…，版本资源 FileVersion 310,6,0,0 实测〕）+ g17_vendor_sdk_registry.json provenance 登记（四 DLL sha256 实测重算 == 登记；g13 表 git 机核 0-byte）。**B 臂（310.6.0）臂级不可用**：SL 2.10.3 sl.dlss.dll 加载 nvngx 310.6.0 报 `NGX indicates DLSSContext is not available`（dlssEntry.cpp:974 逐字 + slIsFeatureLoaded → eErrorFeatureMissing(31)，fail.log 留档 .tmp/g17_mb/arm_b_fail.log）——**vendor 栈耦合兼容性失败 = 换版在当前 Streamline 2.10.3 pin 下不可行**，拒绝换版如实登记（采纳/拒绝/零收益均合法字面）；SL 运行时升级面 = G18+ 换版程序前置。**A 臂（310.5.2 生产默认）**：X2 边际探针重测分解 = **in-stream 净成本 2.224ms**（submit_wait x1 中位 2.2205 / x2 中位 4.4445，n=130 弃 warmup 20；G15 §8.7 快态基线 1.90+0.10 对照——本会话慢态成比例放大，与 A 臂 prod 三轮 [3.905, 4.047, 4.165] vs M-a 窗 3.66~3.80 同向环境面）+ notiming digest == G14.12 冻结锚 HIT（画质守护第一门禁 A 臂绿）+ NGX tokens 六个在档（NGXCubinVulkan cubin 宿主面维持取证）。PaddedWindowNetwork 形态对齐评估结论 = 当前 SL pin 下不可评（形态核验的诚实产出）。
- **② 命令输出**：probe 首跑暴露三真缺陷（A 臂假 MISS = digest 集合误含 X2 探针轮〔双 evaluate 输出漂移是注入预期〕/ 失败轮 receipt 残留污染〔mtime 5s 容差不充分〕/ B 臂五轮重复失败无信息量）——修复（notiming-only digest 判定 + ok=False 不消费 receipt + fail-fast）后重跑定盘（milestones/g17/g17_mb_ngx_probe_results.json）；`py -3 ci/g17_ngx_evolution_alignment_smoke.py --gate …` → VERDICT=PASS 10/10（evidence/g17_m_b_ngx_evolution_alignment_20260824T091311Z.json；首跑 091245Z 红件在档不删——fact⑧ A/B 对照未适配不可用终态即修复跑）；wave3 → PASS（evidence/g17_wave3_exit_20260824T091313Z.json）。X2 探针 src 触改经分解完成后撤除（git diff src/ 空复核，G15plus-II 同型撤除纪律）+ release bin 重编。
- **③ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Claude Fable 5（G17.3 M-b 波）`（影响范围：probe driver + probe json + M-b 门 + 评估缓存〔external/ gitignored 登记面承载〕；src 最终 0-byte；验证方式：M-b 门 10/10 + wave3 四 facts + RED 五臂 selftest PASS）。

### §8.4 G17.4 M-c D3D12 宿主车道波验收记录（2026-08-24）——G-G17-5 字面兑现：M-c 门（g17.p0.m_c.d3d12_host_lane_disposition，步骤 300，10/10 facts VERDICT=PASS）+ 波聚合门 g17.wave.4.exit（步骤 301）VERDICT=PASS；**RFC-0032 终态 = defer（决策树分支③程序产出，三态均合法终态字面）**

- **① 终态兑现**：RFC-0032（Agent Approved 决策程序 + 实现语义，D-409 对抗评审 6 findings 全 disposition）§5 决策树以 M-a 复测窗四件 + M-b probe 为输入程序产出：F1 预估式 = 窗 Rurix 中位 3.771125ms − M-b 采纳差值 0（拒绝换版态 ab_delta 零混入）= est_rurix 3.771125 > ue_med 3.1922（Δ' = +0.5789ms 未达标预估）→ 分支② implement 条件核验：宿主差可分离收益上界估算不可紧化（UE 侧 NGX 份额 CSV GPUTime 口径不可分解 = G15 §8.7 归因三面之③字面；F2 口径限制）∧ 同步税预算下界 0.1ms 与 Δ' 同量级净收益判定不可得 → **分支③ defer + 测算式留档**（重判条件 = G18+ 宿主差可分离 measured 证据出现〔NGX 分解 profiling 或 UE 侧插桩〕；兜底 = Vulkan interop 车道生产默认维持）。§4.3 单 device 化结构性 no-go 留档维持。终态字面入 RFC v0.3 修订行 + evidence（F4 时序注：M-d 翻转构成新事实时按只追加程序留档 G18+ 承接锚不回翻）。**D3D12 宿主车道零实现 = 决策树合法终态**（unsafe/FFI 面零触改，U next_free=59 维持）。
- **② 命令输出**：`py -3 ci/g17_d3d12_host_lane_disposition_smoke.py --gate …` → VERDICT=PASS 10/10 terminal_disposition=defer（evidence/g17_m_c_d3d12_host_lane_disposition_20260824T091333Z.json）；wave4 → PASS（evidence/g17_wave4_exit_20260824T091412Z.json）；M-c selftest 首跑检出 RED 臂与 evaluate 相互递归爆栈真红（RecursionError 留痕 md:1）——修复（终判轮缺失断言内联不递归）后 selftest PASS。
- **③ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Claude Fable 5（G17.4 M-c 波）`（影响范围：M-c 门 + rfcs/0032 v0.3 修订行〔状态字段终态登记 + 修订表只追加〕；src/spec/conformance 0-byte；验证方式：M-c 门 10/10 + wave4 四 facts + RED 臂 selftest PASS）。

### §8.5 G17.5 M-d 终判 + G17.6 M-e 旧门零降级波验收记录（2026-08-24）——G-G17-6/G-G17-7 字面兑现：M-d 门（g17.p0.m_d.t100_final_verdict，步骤 302，11/11 facts VERDICT=PASS）+ M-e 门（g17.p0.m_e.closed_gate_no_regression，步骤 304，18/18 facts VERDICT=PASS）+ 波聚合门 wave5/wave6（步骤 303/305）双 VERDICT=PASS；**终判 = 维持未达标登记不冒充（17/18，本格 ratio 0.8563——合法收口态，兜底字面与 G15 同源）**

- **① M-d 终判定盘**：终判轮 = evidence/g14_m_d_dual_end_fps_parity_20260824T091444Z.json（波锚 20260824T091413Z 后最新，18 格全协议零缩短 + Stage A digest 18 格 × 3 轮 == 冻结锚位级全等〔RD-045 监控零检出〕）；**bistro-interior/t100/dlss_sr ratio 终值 = 0.856326（UE=3.4353ms / Rurix=4.0117ms，evidence JSON parity.cells 字段直取）**；verdict = unmet_honest_registered（达标 17/18，未达格单格如实登记——用户 2026-08-19 授权面逐字承接，两态均合法收口 G-G17-9 字面）。**G17 全窗五轮本格 ratio = 0.9810/0.8157/0.7966/0.8086/0.8563**（轮 1 = UE 会话初慢态 3.72ms 特例，轮 2~5 与 G15 期 0.786~0.862 带吻合——当前环境定盘面维持）。scene 面有界优化 = not-triggered 如实登记（G15plus-II 候选台账 a/b/c/d 已穷尽在案：b 已落地 / a·c·d 无可动面；Δ'≈0.58ms 超 CPU 侧残余上限 ~0.5ms，硬造优化触 digest 锚漂移即弃风险且不足跨线——终判以现状码面执行，digest 锚全等佐证零触改）；NGX 税源物理地板不冒充字面维持；G14 M-c 画质锚带复核绿（measured 0.005389924642694499 ≤ threshold 0.010779849285388998，×2.0 程序产对账）；budget --strict 269 pass 零 skip。
- **② M-e 旧门零降级**：G16 全套九门 `--verify-latest` 子进程全绿（M-a~M-g + soak + closeout）+ G13 双对拍/G13 closeout/G15 closeout/G15 M-e/G14 M-e 六 latest 只读全绿 + **G15 M-d 门诚实红终态维持面**（latest == 定盘件 g15_m_d_perf_parity_zero_regression_20260823T195859Z.json 字面 = 零降级——红终态 0-byte 不遮蔽不代绿；M-e 首跑将其误入子进程 rc==0 判定致假红，即修〔诚实红终态 ≠ 降级〕后复跑绿，首跑红件 095645Z 在档不删）+ g17_ 前缀零抢占 + 禁 --gate 声明。
- **③ 命令输出**：`py -3 ci/g17_t100_final_verdict_smoke.py --gate … --wave-start 20260824T091413Z` → VERDICT=PASS 11/11（evidence/g17_m_d_t100_final_verdict_20260824T095624Z.json）；`py -3 ci/g17_closed_gate_no_regression_smoke.py --gate …` → VERDICT=PASS 18/18（evidence/g17_m_e_closed_gate_no_regression_20260824T095834Z.json）；wave5/wave6 → 双 PASS（095626Z/095836Z）。
- **④ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Claude Fable 5（G17.5 M-d + G17.6 M-e 波）`（影响范围：M-d/M-e 双门真跑 + M-e 诚实红终态口径修正；src/spec/conformance 0-byte；G14 M-d 门脚本与判据 0-byte〔只消费复跑件〕；验证方式：块③逐字命令输出 + 双 selftest PASS）。

### §8.6 G17.7a P2 穷举 + stabilization soak 验收记录（2026-08-24）——G-G17-8 字面兑现：P2 穷举决策门（g17.wave.7a.decisions，步骤 306，25 facts 全绿 VERDICT=PASS）+ 稳定门 soak（g17.wave.7a.soak，步骤 307，8/8 facts VERDICT=PASS——49 迭代 1914.487s ≥1800s 零失败）

- **① P2 穷举定盘**：`G17_P2_DECISIONS.md` v1.0 穷举闭集 **21 行零空行**（§1 15 行终态：closed-go 1〔G15-MD-F1 承接锚三件套程序全要素兑现完结——M-a 复测重标定 + M-b 版本演进评估〔拒绝换版〕+ M-c 车道 RFC defer + M-d 终判登记 + M-e 零降级五门承载〕+ defer-to-G18+ 14〔十四行 G17 全期窗触发条件不交集/未命中/未齐备逐行核验，承接锚字面 0-byte 重评窗顺延〕；§3 期内行 6 行：closed-go 4〔G17-N1~N4 判据承载面真跑绿转留痕〕+ defer-to-G18+ 2〔**G17-MB-F1** ngx_310_6_0_sl_runtime_incompatibility 兼容性失败新 finding + **G17-MD-F1** fps_parity_deficit 期窗五轮定盘——承接锚三面字面：SL 运行时升级换版程序 / 宿主差可分离证据 / UE 暖态包络演进〕）；§2 open RD 八条映射全维持 open 0-byte（RD-045 = G17 复测面六轮 digest 守护零检出登记，零检出不判 closed）；汇总 closed-go 5 / defer-to-G18+ 16 / go 0 / no-go 0 / strategic_override 0。
- **② soak 定盘**：`py -3 ci/g17_stabilization_soak.py --gate g17.wave.7a.soak` → VERDICT=PASS 8/8（evidence/g17_stabilization_soak_20260824T103430Z.json）——M-d 终判在档前置绿（两态均合法字面）+ **wall=1914.487s ≥1800s + 49 迭代零失败（fails=0/49）+ active==wall drift=0.000（谎报秒数交叉核验）+ 零 sleep** + dlss_sr 车道迭代体（bistro/cornell t100 + tsr/fsr 轮换双场景默认臂 32 帧真跑）。
- **③ 命令输出**：P2 门 → VERDICT=PASS 25 facts（evidence/g17_p2_decisions_check_20260824T100216Z.json）；soak → VERDICT=PASS 8 facts；pytest 142 passed 零回归；budget_eval --strict 269 pass 零 skip。
- **④ 签署**：白栀（D-406 v3.0）。`Assisted-by: Cursor Claude Fable 5（G17.7a P2 + soak 波）`（影响范围：G17_P2_DECISIONS.md v1.0 落盘 + P2/soak 双门真跑；deferred.json 0-byte 零新 RD；验证方式：块③逐字命令输出）。

### §8.7 G17.7b close-out 终审签署块（2026-08-24）——G-G17-9 字面兑现：close-out 终审门（g17.wave.7b.closeout，步骤 308）八 facts 全绿 **VERDICT=READY** → status active→closed + tag `g17-closed`

- **① 终审八 facts 逐条（evidence/g17_wave7b_closeout_20260824T103606Z.json）**：
  1. **five_p0_evidence_green = PASS**（M-a 083524Z 10/10 + M-b 091311Z 10/10 + M-c 091333Z 10/10 + M-d 095624Z 11/11 + M-e 095834Z 18/18——五 P0 最新 evidence host_section_pass 全真）。
  2. **p2_exhaustive_zero_empty = PASS**（P2 门 100216Z 25 facts——穷举闭集 21 行零空行：closed-go 5 + defer-to-G18+ 16）。
  3. **final_verdict_chain_complete = PASS**（终判 verdict=unmet_honest_registered，**ratio 终值 0.856326** evidence JSON 字段直取——**维持未达标登记不冒充（17/18）**，兜底字面与 G15 同源，两态均合法收口 G-G17-9 字面兑现）。
  4. **rfc_0032_terminal_state_archived = PASS**（RFC-0032 Agent Approved 在树 + M-c 终态 = defer——approved-implement/no-go/defer 三态均合法终态字面兑现）。
  5. **old_gates_no_regression = PASS**（M-e 18/18——G16 九门子进程 + 六 latest + G15 M-d 诚实红终态维持面 + g17_ 前缀零抢占 + 禁 --gate）。
  6. **rd_eight_open = PASS**（RD-034/039/040/041/042/043/044/045 条目级 status 全 open，零新 RD max=RD-045）。
  7. **soak_ge_1800_zero_fail = PASS**（103430Z：49 迭代 wall=1914.487s ≥1800s 零失败，active==wall drift=0.000；budget_eval --strict 269 pass 零 skip 零 estimated）。
  8. **closeout_ready = PASS**（VERDICT=READY）。
- **② 终审命令逐字输出（2026-08-24 真跑留痕，仓库根目录）**：`py -3 ci/g17_closeout_check.py --gate g17.wave.7b.closeout` → **VERDICT=READY，exit=0**。守卫：check_structure PASS（11 dirs, 6 files）+ check_schemas PASS + check_number_ledger PASS（v1.163 CI_step on_tree_max=308/next_free=309）+ budget_eval --strict 269 pass 零 skip + pytest 142 passed 零回归；治理三门（acceptance_map 12 facts / candidate_decisions 32 facts / interlock READY）全绿在案。
- **③ 收口裁决（G-G17-9 逐字兑现面）**：G15-MD-F1 承接锚三件套字面全要素兑现完结（① 双端同协议复测与暖态重标定 = M-a；② NGX 版本演进面对齐评估 = M-b〔310.6.0 在 SL 2.10.3 pin 下 DLSSContext 兼容性不可用，拒绝换版如实登记，in-stream 分解 A 臂 2.224ms 新鲜留档〕；③ 车道架构面 D3D12 宿主 NGX = M-c〔RFC-0032 Full RFC 经 D-409 对抗评审 Agent Approved，§5 决策树终态 = defer 分支③留档，单 device 化结构性 no-go 维持〕）+ P2 穷举 21 行零空行 + 终判维持未达标登记（17/18，本格五轮 ratio 0.7966~0.9810 期窗定盘——G15 物理不可达定论在本窗环境下维持，兜底字面逐字兑现）。**两种结局均允许 close 的字面下，本期终局 = 维持未达标登记不冒充**；G18+ 承接锚三面齐备（SL 运行时升级换版程序〔G17-MB-F1〕/ 宿主差可分离证据〔RFC-0032 v0.3〕/ UE 暖态包络演进重标定〔M-a 包络条目〕）。G13/G15/G16 冻结面 0-byte；`--gi off` 默认臂 0-byte；坐标尺度 0-byte；G14 M-d 门判据/锚/协议 0-byte（全程只消费复跑件）；src 终态 0-byte（X2 探针撤除复核）。
- **④ status flip 与 tag**：§8 只追加区本块落盘后，`status: active → closed`；`implementation_status: unlocked` 字面不动。flip commit 独立洁净落盘，随后 tag `g17-closed`（沿 `g15-closed`/`g16-closed` 先例，guardrail 基准链切换）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v3.0 agent 完全自主签署）。`Assisted-by: Cursor Claude Fable 5（G17 一次性收口战役）`。

### §8.2 G17.2 M-a 双端复测与暖态重标定波验收记录（2026-08-24，不收口）

- **独立断言清单（M-a `g17.p0.m_a.dual_end_retest_warm_recalib` 步骤 296）**：十 facts 全 PASS（evidence/`g17_m_a_dual_end_retest_warm_recalib_20260824T083524Z.json`）——① 复测窗（stamp ≥ 20260824T054145Z）实测 4 轮 `['20260824T054145Z','20260824T062859Z','20260824T070924Z','20260824T074948Z']`；② 四轮 50×3 trimmed mean 三轮进程级独立 + 生产口径 v2 全 True 零缩短；③ Stage A digest 18 格 × 3 轮 == 冻结锚位级全等四轮全绿（RD-045 同型漂移零检出登记）；④ 焦点格 bistro-interior/t100/dlss_sr 窗内逐轮 ratio = `[0.981, 0.8157, 0.7966, 0.8086]`（登记面；达标判定归 M-d 终判门）；⑤ 6 UE 格暖态包络条目程序产（threshold = 窗 max × 2.0，f64 精确重算 == 存储值，禁手写 P-09）；⑥ g14/g15/g16_budget 既有条目 0-byte；⑦ 环境差异 UE 侧分解：G15 第四轮定盘 3.0023ms → 本窗 median 3.1922ms（+6.3%，窗 `[3.723, 2.983, 2.985, 3.4]`）——暖态包络双向波动实证，UE 侧事件 ≠ Rurix 侧收益（禁混淆归因）；⑧ Rurix 侧分列：3.5391ms → 3.7711ms（+6.6%，窗 `[3.795, 3.657, 3.747, 4.204]`），码面位级同 G14.12 定盘态（digest 锚佐证），变化归环境面非代码收益；⑨ budget_eval PASS（269 pass / 0 skip）；⑩ RED 四臂独立有效（三轮冒充/窗外旧件/digest 漂移静默/手写阈全检出）。**诚实红留痕**：首跑 `..._20260824T083423Z.json` fact `budget_eval_pass=FAIL`（暖态条目判读路由缺失真红）→ 修复 `ci/budget_eval.py`（`eval_g17_dual_end_cell` 格解析泛化〔scene/tier/backend 从条目 id 程序解析〕+ `g17.m_a.warm_ue_frame_ms.` 前缀路由纯追加）→ 复跑全绿；红件按 evidence 只增纪律保留不删。
- **波聚合门实测输出（`g17.wave.2.exit` 步骤 297）**：四 facts 全 PASS（evidence/`g17_wave2_exit_20260824T083537Z.json`）——required_m_gate_latest_pass（M-a 最新件 host_section_pass=True）/ guards_pass（check_structure/check_schemas/check_number_ledger exit 全 0）/ budget_eval_pass（exit 0）/ aggregate_read_only（零重跑零改写红树不充绿）。
- **验收命令逐字输出**：`py -3 ci/g17_dual_end_retest_warm_recalib_smoke.py --verify-latest` → `[g17_m_a] verify-latest PASS: g17_m_a_dual_end_retest_warm_recalib_20260824T083524Z.json`（exit 0，2026-08-24 本批复核）。
- **门序与登记面摘要**：步骤 296（M-a --gate）→ 297（wave2 聚合）；`g17_budget.json` 2→8 条目（+6 暖态包络，全 measured_local 零 estimated）；`g14_fps_gap_registry.json` gap_id `51a150cb4523e8b6` 门产刷新行（窗末轮 ratio 0.9810→0.8086，门自身登记面沿 §8.1 先例，判据面 0-byte）；G-G17-3 兑现（双端复测四轮全协议 + 暖态基线程序产重标定入 g17_budget + 新旧环境差异如实分解三要件齐）。G17.3+（M-b~M-e、P2/soak/close-out）未开工如实登记，本波不做 close-out，`status: active` 维持。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v3.0 agent 完全自主签署）。`Assisted-by: Cursor Claude Fable 5（G17.2 M-a 双端复测与暖态重标定波）`（影响范围：ci/budget_eval.py 判读面泛化 + milestones/g17/g17_budget.json +6 条目 + milestones/g14/g14_fps_gap_registry.json 门产刷新 + evidence 5 件〔g14 M-d 复测两轮 070924Z/074948Z + M-a 红绿两件 + wave2 绿件〕+ 本 §8.2 只追加；src/spec/conformance 0-byte；g14/g15/g16_budget 既有条目 0-byte；deferred.json 0-byte；验证方式：M-a --verify-latest PASS + wave2 四 facts 绿件在盘 + budget_eval 269 pass / 0 skip）。
