---
contract: G15
title: G15 画质量级收口与商用终审期
status: active
implementation_status: unlocked
active_scope: g15_1_governance_only
version: v1.0
date: 2026-08-23
timebox: "G15.1 治理波即刻执行（G14 已 closed，tag g14-closed）；G15.2~G15.6b 严格波次，工期在实现互锁开放后由 measured baseline 校准；用户 2026-08-23 指令全期授权面「一次性完成G15里程碑」字面"
rfc_required: "G15.1 治理波零 RFC 消费——本波只落治理资产，RFC 命名空间 0-byte（实测 next_free=31 维持）。G15.3 修复闭环波若触冻结面（RXS-0357 起步范围冻结面 / RXS-0357 L2 参照器面 / UpscaleBackend trait 签名面 / temporal 底座历史接口面 / G13 锁定双差距登记表终态 / G12 锁定 PT 差距登记表终态 / G12~G14 既有门判据语义 / spec 锁定度量口径 RXS-0386~0393 面）必须独立 Full RFC 经 D-409 对抗性评审后 Agent Approved，编号按起草时实测 registry/number_ledger.json namespaces.RFC next_free 领取，禁推测号；判档争议向上取严（10 §3）。M-a 门 20 行逐项重评 = 对拍链路同口径复跑与处置表落盘面（登记表终态本体 0-byte 不回写，处置表另立 milestones/g15/ 新文件）"
upstream_docs:
  - "milestones/g14/G14_CONTRACT.md §8.13（G14 closed 终态，2026-08-23，flip commit f061487e + tag g14-closed；M-d 18/18 定盘 = G15 性能零降级守护法定输入）"
  - "milestones/g14/G14PLUS_RECORD.md §6.3（G15 承接锚：绝对画质通过线归 G15 + G13 超分 8 行/Lumen 2 行/G12 PT 10 行逐项重评 + RD-045 观察窗——G15 法定输入逐字）"
  - "milestones/g14/G14_P2_DECISIONS.md v1.0 §5（defer-to-G15+ 29 行承接锚 = G15 法定输入，本契约 §7 候选决策逐行承接）"
  - "milestones/g13/g13_ue_upscale_gap_registry.json（8 行终态只消费不回写）+ milestones/g13/g13_ue_lumen_gap_registry.json（2 行终态只消费不回写）+ milestones/g12/g12_ue_pt_gap_registry.json（10 行终态只消费不回写）"
  - "milestones/g13/G13_P2_DECISIONS.md G13-N8/N9 行 + milestones/g12/G12_P2_DECISIONS.md G12-N10/N12 行 + milestones/g14/G14_CANDIDATE_DECISIONS.md §1 G11-N3/N8/N9 行（画质量级收口面锚定 G15 字面）"
  - "registry/deferred.json RD-034/039/040/041/042/043/044/045（存续 open RD 八条；只追加禁静默改判）"
  - "milestones/g14/g14_budget.json（G14 measured 帧时/画质锚带基线——G15 性能零降级守护对照基线输入）+ milestones/g13/g13_budget.json（标定三条目双 seed 方差底口径输入）"
  - "spec/visual_comparison.md RXS-0384~0393/RXS-0403/RXS-0405/RXS-0406（视觉对拍口径锁定面）+ spec/display_pipeline.md RXS-0357（固定 seed 位级确定性协议）"
  - "04 P-01/P-07/P-09/P-12/P-13；10 §3/§7/§9.5；14 §1/§3/§4/§5（同 G14 口径）"
implementation_unlock:
  required_all:
    - "G15.1 治理门全部完成且有真实验证记录"
    - "ci/g15_interlock_check.py --require-ready 输出 READY（互锁 validator 机器事实，不以叙述替代）"
    - "用户 G15.2 开工指令留痕（2026-08-23 指令全期授权面——「一次性完成G15里程碑，积极使用并行智能体和workflow减少工期」字面 + 2026-08-19 授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」字面）"
    - "共享编号按互锁开放时 actual next_free 重新校准；数字 CI 步骤不得沿用推测号与草案建议值"
in_scope:
  - g15_1_governance_only
  - candidate_decisions_and_rd_mapping
  - p0_acceptance_mapping
  - g15_governance_three_gates_materialize
  - g15_2_dual_end_quality_reharvest_wave
  - g15_3_gap_fix_closure_wave
  - g15_4_absolute_quality_final_review_wave
  - g15_5_perf_parity_zero_regression_wave
  - g15_6_p2_exhaustive_decisions_stabilization_and_closeout
out_of_scope:
  - g15_2_plus_while_implementation_interlock_is_red
  - frame_generation_fg_mfg_independent_layer（FG/MFG 独立层另判——G14 重评窗不立项在案（G13-N7 承接锚字面维持），G15 画质收口期不承接）
  - path_tracer_scope_extension（路径追踪生产化 = G12 closed 面 0-byte；G15 PT 面 = G12 锁定 10 行登记表逐项重评对拍复测与处置，非 PT 起步范围扩展）
  - mesh_shader_ser_restir_high_end_lanes（M61/M52/M100-high G13.4/G14 双窗未命中在案——G15 重评窗只登记不立项，承接锚字面 0-byte）
  - new_scene_set_expansion（G10-N6 BistroExterior 未入清单维持——G15 双场景闭集 cornell-box + bistro-interior 0-byte；M133 清单 digest 注册在树）
  - rewriting_g5_to_g14_closed_contracts_and_00_14
  - vendor_sdk_redistribution_or_vendoring
deferred_refs:
  - "registry/deferred.json RD-034/039/040/041/042/043/044/045（存续 open RD 八条；只追加禁静默改判）"
deliverables:
  - id: D-G15-1
    check: "G15.1 完成门：D-G15-1~4 齐备并通过结构/schema/ledger/guardrail 核验；验收映射无缺行；无 src/spec/conformance 语义实现、零 RFC 消费；本门通过不自动开放实现"
  - id: D-G15-2
    check: "G15_CANDIDATE_DECISIONS：G14 defer-to-G15+ 29 行承接锚逐行转引处置 + open RD 八条逐条映射 + G15 新增候选逐行裁决，零空行"
  - id: D-G15-3
    check: "G15_ACCEPTANCE_MAP：5 个 P0 独立 symbolic gate key / 稳定脚本名 / evidence schema 目标路径 / 逐字判据，与契约 §4.2 双向逐字一致"
  - id: D-G15-4
    check: "治理三门（acceptance_map / candidate_decisions / implementation_interlock）真脚本真步骤 materialize，互锁按事实诚实输出（BLOCKED/READY 均为正确结论字面，不充绿）"
acceptance_gates:
  - id: G-G15-1
    check: "G15.1 完成门：D-G15-1~4 齐备并通过结构/schema/ledger/guardrail 核验；验收映射无缺行；无 src/spec/conformance 语义实现、零 RFC 消费；本门通过不自动开放实现"
  - id: G-G15-2
    check: "实现互锁门：ci/g15_interlock_check.py --require-ready 输出 READY + 用户 G15.2 开工指令留痕（2026-08-23 指令全期授权面「一次性完成G15里程碑」字面）+ 共享编号按 actual next_free 重新校准。任一条件不满足均保持 implementation_status=blocked"
  - id: G-G15-3
    check: "G15.2 退出门：M-a P0 独立断言全绿——双端画质对拍链路全量复跑（G13 M-c/M-d + G12 M163 三门同口径复跑，契约 digest 0-byte 门序维持）+ 20 行登记表逐项重评 fresh measured_delta + G15 差距处置表落盘零空行 + UE 方差带程序产 + AI 读图基线臂结构完整性断言"
  - id: G-G15-4
    check: "G15.3 退出门：M-b P0 独立断言全绿——measured 主差修复闭环逐项处置（20 行逐行 closed-resolved / closed-caliber-registered / open-defer-G16+ 三态零空行）+ 修复项 RED 先行 + 触冻结面独立 Full RFC 留痕 + 材质链表达面立项评估结论登记（G11-N8/N9 + G12-N10 承接锚命中判定逐字）"
  - id: G-G15-5
    check: "G15.4 退出门：M-c P0 独立断言全绿——绝对画质通过线程序产标定（禁手写 P-09）+ 双场景 × 三档 × 三后端 18 格逐格 AI 读图严格画面审查记录 + 商用收口判定（达标/未达标如实登记不冒充）"
  - id: G-G15-6
    check: "G15.5 退出门：M-d P0 独立断言全绿——G14 M-d 门同口径复跑 18 格 ratio ≥ ×1.00 维持（逐轮守护带口径）+ G14 M-c 画质锚带复核带内 + 性能零降级守护（画质修复不得致帧率跌破通过线）"
  - id: G-G15-7
    check: "G15.6a 决策门：G15 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G16+，零空行；defer 必有承接锚；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述"
  - id: G-G15-8
    check: "G15.6a 稳定门：全部 P0 与所有 go 的 P1 全量回归；G5~G14 既有判据 0-byte；画质对拍与帧率对标链路连续复跑 soak（量级沿 G14.5a 继承〔≥1800s〕或 measured 证明更短足够）；strict budget 非空、零 estimated/skip；既有 84 门（G9 34 + G10 14 + G11 14 + G12 9 + G13 5 + G14 8）零降级"
  - id: G-G15-9
    check: "G15.6b 收口门：验收映射、候选决策、RD 最终状态逐字一致；全部 P0 独立断言均 PASS；evidence/schema/预算终审；商用收口终审定盘（达标/未达标如实登记不冒充；未达标按用户 2026-08-19 授权新建 G16+ 里程碑继续优化，性能零降级守护面终态锁定）；§8 只追加后 status active→closed"
guardrails:
  - "双状态不可混同：status=active 仅表示 G15.1 governance-only 已立项；在 G-G15-2 真实通过前 implementation_status=blocked，任何治理完成叙述不得冒充 G15.2 开工"
  - "G15.1 允许 milestones/g15、G15 专属治理三门（ci/g15_*_check.py + evidence schema + workflow 步骤按 actual next_free）、G15 专属 claim、deferred history 只追加；src/spec/conformance 0-byte、零 RFC 消费；G13/G12 三差距登记表终态 0-byte 不回写"
  - "G15 P0 实现门 CI 只冻结 symbolic gate key 与脚本名；numeric_step 一律写 post-interlock actual-next-free allocation。不得沿用推测号与草案建议值，不得预放空 workflow、空脚本或空 schema 壳（G15.1 治理三门为例外：本波即落盘真脚本真步骤）"
  - "每个 P0 必须独立布尔断言与独立 evidence subject；可共享一次进程执行，但聚合 PASS 不能遮蔽任一子断言 FAIL/SKIP"
  - "缺硬件/工具链仅可 dev_env_degrade 或 SKIP=not-triggered；两者均不充 P0 绿。host oracle、mock、isolated nonzero、既有最小见证、人工截图均不能替代目标门"
  - "三表 20 行只消费不回写：g13_ue_upscale_gap_registry.json 8 行 + g13_ue_lumen_gap_registry.json 2 行 + g12_ue_pt_gap_registry.json 10 行终态 0-byte；G15 逐项重评处置面另立 milestones/g15/g15_quality_gap_disposition.json（新文件，gap_id 逐字转引 + fresh measured_delta 可溯源），不回写 G12/G13 表"
  - "绝对画质通过线纪律：通过线判据程序产标定（UE 参照 deficit 双 seed 方差底 p100×2.0 程序产口径沿 G13 标定链，禁手写 P-09）；逐格判定逐字入 evidence；未达格如实登记不冒充；通过线设立不 retroactive 改写 G13/G14 已 closed 判据（0-byte）"
  - "AI 读图强制门纪律（G14.10f 教训字面兑现：digest 双跑一致 ≠ 内容正确——确定性的坏内容照样全绿）：G15 画质终审面每格出图必须经 AI 读图结构完整性审查（无乱序/无错位/无全黑/关键结构可见），读图记录入 evidence；digest 面不替代内容面"
  - "性能零降级守护纪律（G14 收口定盘承接）：G15 全期任何画质修复/终审复跑不得致 G14 M-d 18 格 ratio 跌破 ×1.00 通过线；M-d 门复跑核验 = 每波退出硬前置；优化致性能劣化静默即 RED"
  - "对标范围唯一法定来源：G14PLUS_RECORD §6.3 G15 承接锚 + G14_P2_DECISIONS §5 29 行 + G13/G12 三表 20 行终态 + RD-045 观察窗；G15 不得无锚新立项；新发现差距进 G15 处置表显式登记 + G15.6a 穷举，不得静默混入"
  - "M165/RD-045 漂移监控登记条款（G12-N13/RD-045 承接）：G15 复跑面检出同型 digest 漂移即如实登记并升级评估（升级 = 生产化缺陷修复项 + Full RFC 评估）；零检出维持 open-defer 不写进全绿叙述"
  - "既有 84 门零降级：G9 34 key + G10 14 key + G11 14 key + G12 9 key + G13 5 key + G14 8 key 绿面 0-byte；G5~G14 closed 契约与判据 0-byte；回归门独立 P0 断言（M-e）；M96 golden 门序机器阻断（D2-Q7）维持"
  - "UpscaleBackend/temporal 底座 0-byte 不接线（RD-041/RD-040 承接锚口径）：G15 画质面修复经既有接口面进行，trait 签名与 temporal 底座历史接口面 0-byte；确需演进必须独立 Full RFC 显式修订行"
  - "UE 源码仅外部参照只读（F:\\UE_5.8 与 E:\\Kimi_Agent_Taichi Engine 优化计划\\references\\UnrealEngine 双树），零 vendoring、零片段复制进 src/spec；违反即 revert + 留痕（RFC-0027 字面）"
  - "主腿 = Vulkan RayQuery（M96 device 面）；DXIL RT blocked 维持（RD-034）；benchmark 臂 = spec RXS-0380 L2 臂 B `-game -benchmark` 命令面闭集内形态，schema 外开关注入即 fail-closed"
---

# G15 画质量级收口与商用终审期 契约

> 本契约是 G15 里程碑唯一事实源。front matter 双状态机：`status`（治理激活）与 `implementation_status`（实现解锁）严格分离。

## 1. 目标与双门状态

**目标（用户 2026-08-23 指令字面兑现面）**：「帮我一次性完成G15里程碑，积极使用并行智能体和workflow减少工期」+ 2026-08-19 全期授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」——G15 = 画质量级收口与商用终审期：① G13 超分差距登记表 8 行 + Lumen 差距登记表 2 行 + G12 PT 差距登记表 10 行逐项重评（承接锚字面兑现，fresh measured_delta 可溯源）；② 绝对画质通过线设立（G14 out_of_scope 锚定 G15 面）；③ 严格画面审查（AI 读图强制门——G14.10f 教训字面兑现）；④ 商用收口判定（达标/未达标如实登记不冒充）；⑤ 性能零降级守护（G14 M-d 18/18 ×1.00 定盘面维持——画质收口不以性能回退为代价）。「UE5 级」可核对基线沿用 G8 口径 = UE 5.8（G9_CONTRACT §1 字面；本机 UE 5.8.1-56057345 == M128 登记机核继承）。

**双门状态**：`status: active`（G15.1 governance-only）+ `implementation_status: blocked`（G-G15-2 事实互锁未过前 G15.2+ 禁止开工）。

## 2. 范围与波次

- **G15.1 治理波**（本波）：契约三件套 + 候选决策表（G14 defer 29 行逐行承接 + open RD 八条映射 + G15 新增候选）+ 验收映射 5 P0 + 治理三门 materialize + 互锁按事实诚实输出。
- **G15.2 测量重收割波**（M-a）：双端画质对拍链路全量复跑（G13 M-c/M-d + G12 M163 三门同口径，契约 digest 0-byte 门序维持）+ 20 行登记表逐项重评 fresh measured_delta + G15 差距处置表落盘 + AI 读图基线臂。
- **G15.3 修复闭环波**（M-b）：measured 主差修复闭环逐项处置（三态穷举零空行）+ 材质链表达面立项评估结论登记（G11-N8/N9 + G12-N10 承接锚命中判定）。
- **G15.4 绝对画质终审波**（M-c）：绝对画质通过线程序产标定 + 18 格逐格 AI 读图严格画面审查 + 商用收口判定。
- **G15.5 性能零降级波**（M-d）：G14 M-d 同口径复跑 18 格 ×1.00 维持 + 画质锚带复核。
- **G15.6a 决策+稳定波**：P2 穷举决策 + M-e 回归门 + stabilization soak。
- **G15.6b close-out**：终审八 facts + status flip 独立 commit + g15-closed tag。

## 3. 治理波交付物（D-G15-1~4）

见 front matter deliverables / acceptance_gates 逐字判据；本波零 RFC 消费、零 src/spec/conformance 语义实现。

## 4. P0 独立断言表

### 4.1 统一纪律

接入/落盘 + 冻结面 0-byte（RXS-0357 起步范围与参照器面 / UpscaleBackend trait 签名面与 temporal 底座历史接口面 / G13 锁定双差距登记表终态 / G12 锁定 PT 差距登记表终态 / G11 GI 既有判据 / M96 golden 门序 D2-Q7 / RXS-0386~0393 锁定度量口径）+ measured 面标定程序产阈禁手写（P-09）+ 不降级既有 84 门绿面 + AI 读图强制门（digest 面不替代内容面）+ 性能零降级守护（G14 18/18 ×1.00 维持）。

### 4.2 五行 P0

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | 双端画质对拍链路全量复跑（G13 M-c ue_upscale_parity + G13 M-d ue_lumen_gi_parity + G12 M163 ue_pt_parity 三门同口径复跑，对拍契约 digest 0-byte 门序维持）+ 20 行登记表逐项重评（逐行 gap_id 逐字转引 + fresh measured_delta + 方向判定〔收敛/维持/劣化〕）+ G15 差距处置表 `milestones/g15/g15_quality_gap_disposition.json` 落盘零空行 + UE 方差带程序产（G14 M-a 双程序产面取严口径继承）+ AI 读图基线臂（双场景 × 三档 × 三后端出图结构完整性断言） | G15.2 |
| **M-b** | measured 主差修复闭环：处置表 20 行逐行终态处置 ∈ {closed-resolved（修复后 fresh delta 进容差带，RXS-0393 收敛判据两款）/ closed-caliber-registered（口径差显式登记不拟合，RXS-0392）/ open-defer-G16+（承接锚字面「重判条件 = …；兜底 = …」）} 零空行 + 修复项 RED 先行（失败测试先落 main 为 RED）+ 触冻结面独立 Full RFC 留痕（D-409 对抗评审）+ 材质链表达面立项评估结论登记（G11-N8/G11-N9/G12-N10 承接锚命中判定逐字：透射/焦散/镜面 IBL 类能量是否成为画质量级 measured 主差） | G15.3 |
| **M-c** | 绝对画质通过线设立 + 严格画面审查：绝对通过线程序产标定（UE 参照 deficit 双 seed 方差底 p100×2.0 程序产，禁手写 P-09，标定链路入 evidence）+ 双场景 × 三档（t50/t67/t100）× 三后端（tsr_device/dlss_sr/fsr_3_1_5）18 格逐格判定 + 逐格 AI 读图严格画面审查记录（无乱序/无错位/无全黑/关键结构可见——cornell 盒体结构、bistro 吊灯/吧台/桌椅）+ 商用收口判定（达标格数/18 + 未达标格如实登记不冒充） | G15.4 |
| **M-d** | 性能零降级守护：G14 M-d 门同口径复跑（双场景 × 三档 × 三后端 18 格，三轮进程级独立运行 50×3 trimmed mean 跨轮中位数 + 逐轮守护带）逐格 ratio ≥ ×1.00 维持 + G14 M-c 画质锚带复核（SSIM deficit ≤ 0.010779849285388998 带内）+ G14 门产 budget 条目零 estimated 维持 + 画质修复致性能劣化静默即 RED | G15.5 |
| **M-e** | 回归门 + 漂移监控：既有 84 门（G9 34 + G10 14 + G11 14 + G12 9 + G13 5 + G14 8）最新 evidence 全绿只读汇总不遮蔽 + 触改面真跑抽检零降级 + RD-045/M165 同型 digest 漂移监控登记（G15 复跑面检出计数/零检出字面入 evidence） | G15.6a |

### 4.3 治理三门（本波即落盘真脚本真步骤）

| 门 | symbolic gate key | 脚本 | 步骤 |
|---|---|---|---|
| 验收映射核验 | `g15.wave.1.acceptance_map` | `ci/g15_acceptance_map_check.py` | 266（落盘前实测 CI_step.next_free=266 顺位领取） |
| 候选决策核验 | `g15.wave.1.candidate_decisions` | `ci/g15_candidate_decisions_check.py` | 267（同批顺位领取） |
| 实现互锁 | `g15.gov.implementation_interlock` | `ci/g15_interlock_check.py` | 268（同批顺位领取） |

## 5. Guardrails

见 front matter guardrails 十六条逐字（双状态不可混同 / G15.1 边界 / 数字步骤延迟分配 / P0 独立断言 / 诚实降级三档 / 三表只消费不回写 / 绝对画质通过线纪律 / AI 读图强制门 / 性能零降级守护 / 法定来源唯一 / RD-045 漂移监控 / 既有 84 门零降级 / UpscaleBackend·temporal 0-byte / UE 源码只读 / 主腿与 benchmark 臂闭集）。

## 6. Deferred 处置

RD-034/039/040/041/042/043/044/045 八条 open 维持（条目级 status 0-byte，history 只追加）。RD-045（M165 同型族间歇 digest 漂移）= G15 M-e 门漂移监控臂承接面——G15 复跑面检出即升级评估，零检出维持 open 不关闭（G14PLUS_RECORD §6.2 字面）。

## 7. 修订与开工裁决

- **立项裁决 1（立项与不可变 ref）**：现在立项；G15.0 不可变 ref = `f061487efaf7816684de18a6ef86554e5c392a75`（G14 close-out flip commit，tag `g14-closed`；G15.0 前置 housekeeping = G14 战后清零留痕 commit `34f96ac3`，仅 G14 期门产归档，不改 G15.0 事实源面）。
- **立项裁决 2（用户开工指令留痕）**：2026-08-23 用户指令全期授权面——「/goal 帮我一次性完成G15里程碑，积极使用并行智能体和workflow减少工期」；并承 2026-08-19 授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」。双面字面即 G15.2+ 开工指令留痕（G14-N7 先例：用户明示即生效）。
- **立项裁决 3（异己面）**：当前工作树洁净面机核——G14 战后遗留面（soak 门产 budget/方差样本刷新 + 未提交 evidence）已经 `34f96ac3` 归档，零异己 src/ 未提交面；G15 全波 commit 按文件名显式择取，后续异己面出现即按 G14-N6 同律严禁消费/混入。
- **立项裁决 4（编号面）**：治理三门 CI 步骤 266/267/268 = 落盘前实测 `registry/number_ledger.json` namespaces.CI_step next_free=266 顺位领取（actual next_free 校准面，主会话提交批统一接线）；RD/RFC/RXS/D 命名空间本波 0-byte（next_free 46/31/407/410 维持）。
- **立项裁决 5（工期与并行授权面）**：用户指令「积极使用并行智能体和workflow减少工期」字面 = G15 全波允许并行子 agent 实施面（波内并行、波间不越级，波次生命周期沿 skill §4 十阶段）；workflow 接线面按 actual next_free 延迟分配。

## 8. Implementation activation / Close-out（只追加区）

<!-- 本区只追加；开工时为空且禁止预填 PASS。G-G15-2 解锁记录、逐波验收记录、close-out 终审签署块均追加于此。 -->

### §8.1 G-G15-2 implementation_status 解锁记录（2026-08-23）

1. **互锁 validator 机器事实**：`py -3 ci/g15_interlock_check.py --gate g15.gov.implementation_interlock` VERDICT=READY（事实门①~④全绿 + 一致性门 C1~C4 全绿，evidence `evidence/g15_interlock_check_20260823T080629Z.json`）——① G14 closed + §8.13 签署块 + G15.0 不可变 ref f061487efaf7816684de18a6ef86554e5c392a75 登记；② 候选决策表 35 行零空行 + deferred vs G15.0 base 只追加 + MAP §1 五行 P0 无缺行；③ 用户开工指令留痕 + workflow 实测末号 268 == ledger on_tree_max 268、next_free 269；④ 治理两门独立 PASS 实测。
2. **用户开工指令留痕**：2026-08-23 指令全期授权面「一次性完成G15里程碑，积极使用并行智能体和workflow减少工期」（§7 立项裁决 2 逐字登记）+ 2026-08-19 授权面「真实可商用……允许在G15后无限制新建里程碑继续优化」。
3. **编号校准**：治理三门步骤 266/267/268 落盘前实测 CI_step next_free=266 顺位领取；ledger v1.152 校准（on_tree_max 268 / next_free 269）；P0 实现门 numeric_step 维持 post-interlock actual-next-free allocation（零预占机核 C3 绿）。
4. **治理两门独立 PASS**：`g15.wave.1.acceptance_map` evidence `evidence/g15_acceptance_map_check_20260823T080628Z.json` VERDICT=PASS；`g15.wave.1.candidate_decisions` evidence `evidence/g15_candidate_decisions_check_20260823T080505Z.json` VERDICT=PASS。
5. **签署块**：`Assisted-by: Kimi-K3`（G15.1 治理波）；影响范围 = milestones/g15/ 治理资产 + ci/g15_*_check.py 三脚本 + 三 evidence schema + check_schemas.py/budget_eval.py 纯追加 + pr-smoke.yml 步骤 266~268 + registry/number_ledger.json v1.152；验证方式 = 治理三门 --gate 真跑输出 + 守卫套件（structure/schemas/ledger/budget_eval 全 PASS）+ 三脚本 --selftest 红绿全过。

G-G15-2 三条件全齐（互锁 READY + 开工指令留痕 + 编号校准）→ `implementation_status: blocked → unlocked`（本记录与 front matter 翻转同批落地）。
