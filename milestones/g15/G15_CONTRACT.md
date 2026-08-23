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

### §8.2 G15.2 测量重收割波验收记录（2026-08-23）——G-G15-3 字面兑现：M-a 双端画质重收割门（g15.p0.m_a.dual_end_quality_reharvest，步骤 269，16/16 checks 全绿 VERDICT=PASS）+ 波聚合门 g15.wave.2.exit（步骤 270）VERDICT=PASS 六 facts

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g15.p0.m_a.dual_end_quality_reharvest`（步骤 269） | 契约 §4.2 M-a 逐字：**上游三门同口径复跑 fresh 全 PASS**（G13 M-c `g13_m_c_ue_upscale_parity_20260823T084323Z` 20/20 + G13 M-d `g13_m_d_ue_lumen_gi_parity_20260823T102425Z` 19/19 + G12 M163 `g12_m163_ue_pt_parity_20260823T103107Z` 21/21，本波启动锚 20260823T084242Z 后真跑件——UE 5.8.1 MRQ + Rurix device 双臂真跑，锁纪律沿三门本体面〔UE 臂 harness 子进程自持锁 / Rurix 臂门侧持锁段，无嵌套持锁〕）+ **对拍契约 digest 0-byte 门序维持**（三 parity 契约 + 三冻结登记表在树 == HEAD 提交态逐字节 git 机核，8+2+10 行终态 0-byte 只消费不回写）+ **20 行逐项重评**（gap_id 逐字转引闭集全等 + fresh measured_delta 自当次复跑 evidence f64 精确提取〔delta==b−a 构造不变式维持〕+ 方向判定交叉核验重算绿）+ **G15 差距处置表落盘零空行**（`milestones/g15/g15_quality_gap_disposition.json` 20 行 gaplib 正典形同族 schema）+ **UE 方差带程序产**（upscale band_rel=0.00601379 / lumen band_rel=0.00101218 门内三样本 max 两两相对差 ×2.0 + gaplib 跨会话样本级联带取 max 双程序产面取严，fresh 带入 g15_budget 双条目 measured_local 零 estimated）+ **AI 读图基线臂 18 格**（PNG 导出 .tmp/g15_m_a_preview/ + 结构代理全绿 + manifest 入 evidence）+ RED 五臂独立有效（missing-row/gap-id-tamper/direction-lie/stale-evidence/fresh-delta-missing-field） | host+device（上游三门子进程真跑面消费） | evidence/g15_m_a_dual_end_quality_reharvest_20260823T110103Z.json（16/16） | PASS |
  | `g15.wave.2.exit`（步骤 270） | 波聚合门只读汇总六 facts 全绿：① 上游三门 fresh 全 PASS（timestamp ≥ 本波启动锚）② 三契约 + 三冻结表 0-byte（在树 == HEAD 逐字节）③ M-a RED 臂独立有效（5 臂）④ 处置表 20 行零空行 + gap_id 闭集逐字对账 + 方向交叉核验重算绿 ⑤ g15_budget 五条目齐备 measured_local 零 estimated + budget_eval 全 PASS（P-09）⑥ G5~G14 closed 面 0-byte（vs G15.0 ref f061487e committed diff 闭集 = {g14_budget.json, g14_ue_variance_samples.json} = 34f96ac3 归档授权双面；工作树闭集 = {g14_ue_variance_samples.json} 样本只追加面） | host 只读（不重跑子门） | evidence/g15_wave2_exit_20260823T110200Z.json（六 facts 全绿） | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g15_wave2_exit_check.py --gate g15.wave.2.exit` → **VERDICT = PASS，exit=0**（required_gates M-a PASS + 六 facts 逐行打印不遮蔽）；`py -3 ci/g15_wave2_exit_check.py --selftest` → ALL PASS（负样本空目录红 + 真树聚合 VERDICT==子门实测态不遮蔽机核双臂；真树臂 PASS 件 110223Z + 负样本 FAIL 件 110221Z 在档）。
- **③ 验收命令逐字输出（2026-08-23 真跑留痕，仓库根目录）**：
  - 上游三门同口径复跑（本波依次真跑，锁纪律 = 三门本体面，外层零持锁）：`py -3 ci/g13_ue_upscale_parity_smoke.py --gate g13.p0.m_c.ue_upscale_parity` → VERDICT=PASS checks=20/20（三方 digest 全等 == 冻结注册值 + UE 探针格带 0.00601379 程序产）；`py -3 ci/g13_ue_lumen_gi_parity_smoke.py --gate g13.p0.m_d.ue_lumen_gi_parity` → VERDICT=PASS checks=19/19（digest 三方全等 + 带 0.00101218）；`py -3 ci/g12_ue_pt_parity_smoke.py --gate g12.p0.m163.ue_pt_parity` → PASS checks=21/21（双端 12 帧齐备 + 登记表 10 行重产逐字节一致 + RED 五臂）。
  - `py -3 ci/g15_dual_end_quality_reharvest_smoke.py --selftest` → selftest PASS（schema 闭集 16 键 + 5 RED + 4 GREEN 函数面臂）；`--gate g15.p0.m_a.dual_end_quality_reharvest --wave-start 20260823T084242Z` → VERDICT=PASS checks=16/16（evidence 110103Z）；`--verify-latest` → PASS。
  - 守卫套件全 PASS：`py -3 ci/check_structure.py` → PASS（11 dirs, 6 files）；`py -3 ci/check_schemas.py` → PASS（本批新增 g15_m_a_dual_end_quality_reharvest_ / g15_m_a_band_ / g15_wave2_exit_ 三前缀九处纯追加，与既有全族互不包含）；`py -3 ci/check_number_ledger.py` → PASS（CI_step on_tree_max 270/next_free 271 校准后实测）；`py -3 ci/budget_eval.py --strict` → PASS（249 pass/0 skip——g15 五条目含 M-a 双 fresh 带 0.00601379/0.00101218 measured_local）；`py -3 -m pytest tests/ -q` → 136 passed 零回归；`py -3 ci/g15_interlock_check.py --require-ready` 面 = READY 维持（治理三门证据在档）。
  - 起草期 FAIL 轨迹（诚实留档不删）：M-a 首跑 104201Z（① `check(not bands_ok, …)` 调用反向——G14 M-a `not … is False` 惯写面清理时反向，修正为直判；② budget_eval 缺 `g15.m_a.ue_variance_band_` 分派支——共享面补丁先于门二跑落盘；③ 结构代理初版 `histogram_max_share<0.98` 对 bistro 夜景本真暗态误伤——见 ④ 修订留痕）+ 二跑 105450Z（逐后端输出域归一后 tsr 臂 canonical 曝光同暗态触发同谓词——谓词面重标定为失败模式字面编码〔全黑 max≤0.05 / 全白 min≥0.95 / 单值退化 std≤1e-4 / 直方图占用 <4 柱 / 全平块 ≥95%〕，三跑 110103Z 全绿；schema 侧 anyOf 双相沿 G14.6 M-d/M130 先例，首二跑 FAIL 件 0-byte 在档）。
- **④ 门序 / 偏差 / not-triggered / no-go 登记面摘要**：
  - **门序**：G-G15-2 解锁（§8.1）→ 本波 G-G15-3 兑现；数字步骤 269/270 按落盘前实测 actual next_free 顺位领取（ledger v1.153 校准同批）；G15.3 修复闭环波（M-b）开工面开放。
  - **20 行方向判定汇总（M-b 波消费面）**：**收敛 0 / 维持 20 / 劣化 0**——cornell 10 行全维持、bistro 10 行全维持；处置建议 = **open-defer-G16+ 16 行**（quality_gap 面：upscale deficit 2 行 + noise 6 行 + lumen 2 行 + PT 曲线 2/噪声 2/能量 2 行——M-b 修复评估面未决如实 open）+ **closed-caliber-registered 4 行**（PT caliber_diff 常驻行：材质纹理均值口径/emissive Le 同构/AA 滤波策略/EXR 位深——fresh == 登记位级一致）。fresh delta 与登记面位级/带内一致（bistro noise 三行 UE 侧 −0.490558→−0.488001 / −0.282365→−0.279802 / −0.413787→−0.411218 跨会话带内吸收；lumen indirect_ssim@bistro −0.993433→−0.993478 跨会话带内；PT 全行 f64 精确位级一致；G14.12 加性面 noise_hf 归约 Rurix 侧微抖动带〔标定 measured 程序产〕并合吸收 1e-7 级 vendor 臂归约抖动）。
  - **AI 读图 18 格逐格审查记录（本门强制门面，digest 面不替代内容面）**：cornell-box 九格（t50/t67/t100 × tsr_device/dlss_sr/fsr_3_1_5）——盒体结构完整：左绿墙/右红墙/白后墙/顶部长方面光/双箱（高箱左中、矮箱右下）全在位，墙面着色均匀，无乱序、无错位、无全黑全白、无大块纯色斑块、无伪影；三后端互一致、三档互一致；fsr 低档位轻微噪点颗粒（与 noise_hf 登记行形态一致）。bistro-interior 九格——吊灯群（前厅 4 盏 + 右上 2 盏）明亮清晰、位置逐格对齐无错位；红色墙板/壁柱左右依稀可辨；吧台区与桌椅剪影中后景隐约可见；无乱序、无错位、无伪影；三后端 × 三档互一致。**暗部深黑如实登记为双因子叠加面**：① 场景 = 夜景 bistro（ev100=−4 派生尺度补偿后固有暗态）② Rurix 臂直接光口径（无 GI/天光——契约 sun/sky=0.0，跨端口径差残余登记不拟合 RXS-0392，G13.5/G15 承接锚面）——暗部无间接光反弹故深黑；「关键结构可见」逐格判定 = 吊灯清晰可见 ✓、吧台/桌椅/墙面 canonical 曝光下仅依稀可辨（边界态如实登记不冒充清晰）。
  - **新发现显式登记（法定来源唯一纪律「新发现差距显式登记不静默混入」）**：**G15-MA-F1 `vendor_backend_output_domain_deviation@bistro-interior`**（quality_gap 候选）——vendor 双臂（dlss_sr/fsr_3_1_5）converged 输出停留 scene-linear HDR 域：bistro t67 原域均值 tsr_device=0.00977378 vs dlss_sr=0.00060290 vs fsr_3_1_5=0.00060379（比值 16.21/16.19 ≈ 2⁴ = bistro ev100=−4 派生尺度；cornell ev100=0 三臂同域旁证）；`UpscaleInputs.exposure` 语义 = backend 转显示域（tsr.rs「显示域图像（× exposure）」px_out=v×exposure 字面兑现，vendor pre_exposure 未达输出面）；端内参照 parity 面尺度消去故 G13.4 起潜伏不可见——**AI 读图基线臂首次检出（G14.10f 教训字面兑现面）**。处置：基线臂导出按逐后端输出域声明面归一显示域（vendor 臂 ×2^(−ev100)）后三后端显示域亮度互一致（九格 mean 0.00957~0.00983 lin）；disposition_hint = open-defer-G16+（G15.6a 穷举 + M-b 材质链/口径面评估输入）；本 finding 入 M-a evidence parity.findings 在档。
  - **RD-045/M165 漂移监控登记（G15 复跑面）**：本波三门复跑面确定性键族全位级——M-c rurix_double_run_bitexact + 帧 digest 重算对账 + M163 Rurix 双跑位级 + UE PT 帧 digest 全绿，同型 digest 漂移**零检出**（维持 open-defer 不写进全绿叙述，M-e 门面承接）。
  - **门脚本内部健壮性修订留痕（G14 M-a 先例同模，G13/G12 closed 门判据语义 0-byte）**：本波对上游三门**零修订**（三门脚本/契约/冻结表/判据语义全 0-byte——方差带面在 G14.2/G14.5a/G14.12 已程序产在树，本波复跑直接消费）；修订面全部在新门自身起草期（③ FAIL 轨迹字面）。
  - **not-triggered / 维持 open 面**：M-b~M-e 未跑（后续波次面）；RD-034/039/040/041/042/043/044/045 八条 open 维持 0-byte；G14 defer 29 行承接锚 0-byte（M61/M52/M100-high 等 14 行 defer-to-G16+ 窗结论维持）。
  - **异己并发工作树面**：本批只含 G15 车道文件（按文件名显式择取）；异己会话 src/ 未提交面（ktx2_read/hzb/restir/sdf_trace/smrt/ssr 等 untracked 面）维持未提交、零消费、零混入（立项裁决 3）；g14_ue_variance_samples.json 工作树修改 = G13 双门复跑门产样本只追加面（canonical 加性面，wave2 fact⑥ 允许面登记）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模）。`Assisted-by: Kimi-K3（G15.2 测量重收割波）`（影响范围：ci/g15_dual_end_quality_reharvest_smoke.py + ci/g15_wave2_exit_check.py 新建 + milestones/g15 三 evidence schema 新建〔g15_m_a_dual_end_quality_reharvest_/g15_m_a_measured_entry_/g15_wave2_exit_〕+ milestones/g15/g15_quality_gap_disposition.json 首建 20 行 + milestones/g15/g15_budget.json 双条目纯追加〔既有三条目 0-byte〕+ ci/check_schemas.py〔三前缀 × load/validator/路由九处纯追加〕+ ci/budget_eval.py〔g15.m_a.ue_variance_band_ 分派支纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 269/270，步骤 268 块后追加〕+ registry/number_ledger.json〔CI_step 268→270/next_free 271 + revision_log v1.153〕+ 本契约 §8.2 本条 + evidence 本批真跑件〔M-c 084323Z + M-d 102425Z + M163 103107Z 复跑件 + M-a 110103Z PASS + wave2 110200Z/110223Z + M-a band 双条目件 + 起草期 FAIL 轨迹三件 104201Z/105450Z/110221Z 在档〕+ .tmp/g15_m_a_preview/ 18 格 PNG〔一次性预览面不入 commit〕；验证方式：块③逐字命令输出——M-a 门 16/16 checks 全绿 + 波聚合门六 facts PASS + 上游三门复跑 20/19/21 全绿 + 双 selftest 红绿留痕 + 守卫套件全 PASS + pytest 136 passed 零回归）。

### §8.3 G15.3 修复闭环波验收记录（2026-08-23）——G-G15-4 字面兑现：M-b measured 主差修复闭环门（g15.p0.m_b.gap_fix_closure_loop，步骤 271，19/19 checks 全绿 VERDICT=PASS）+ 波聚合门 g15.wave.3.exit（步骤 272）VERDICT=PASS 六 facts

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g15.p0.m_b.gap_fix_closure_loop`（步骤 271） | 契约 §4.2 M-b 逐字：**处置表 20 行逐行终态处置三态零空行**（闭环登记表 `milestones/g15/g15_gap_fix_closure_registry.json` 首建——逐行 gap_id/kind/title/direction/suggestion 与 M-a 处置表标签级逐字对账全等 + final_disposition ∈ {closed-resolved/closed-caliber-registered/open-defer-G16+} 闭集 + open-defer 16 行承接锚「重判条件 = …；兜底 = …」字面全量 + closed-caliber-registered 4 行 RXS-0392 不拟合字面且 kind==caliber_diff 向上取严 + 逐行 fix_evaluation 修复面论证非空 + 汇总 tally 重算一致）+ **修复项 RED 先行**（本波评估结论 = 全部无可 bounded 修复面——零修复立项合法退出形态，RED 先行纪律 vacuous 成立如实登记不充绿；修复项无 RED 先行留痕注入臂必检出）+ **触冻结面独立 Full RFC 留痕面机核**（frozen_face_touched=false + rfc_consumed=0 + ledger RFC next_free=31 维持 + src/spec/conformance/milestones g5~g14/ci g5_*~g14_* tracked diff HEAD 空 + 异己 untracked 闭集机核绿——触冻结面零发生，零 RFC 消费）+ **材质链表达面立项评估结论登记**（G11-N8/G11-N9/G12-N10 承接锚命中判定逐字——verdict=not-triggered 未命中如实登记不充绿）+ **G15-MA-F1 评估定论登记**（closed-caliber-registered——契约语义内形态）+ M-a 最新 evidence PASS 只读核验 + 链锚对账（parity.wave_start == 处置表 wave_start==20260823T084242Z）+ M-a 处置表同族校验与方向交叉核验重算绿（20 行消费面 0-byte 不回写）+ 三 parity 契约与三冻结登记表 0-byte git 机核 + RED 五臂独立有效（missing-row/out-of-enum-disposition/open-defer-anchor-missing-literal/material-chain-verdict-missing/fix-project-without-red-first） | host（M-a device 真跑面只读消费，本波零修复零新 device 面） | evidence/g15_m_b_gap_fix_closure_loop_20260823T121607Z.json（19/19） | PASS |
  | `g15.wave.3.exit`（步骤 272） | 波聚合门只读汇总六 facts 全绿：① M-b 最新 evidence PASS + RED 臂独立有效（5 臂全真 ≥4）② 闭环登记表 20 行零空行重算绿（M-b 门同族校验器函数面消费）③ 三 parity 契约 + 三冻结登记表在树 == HEAD 逐字节 ④ 材质链评估 not-triggered 未命中 + G15-MA-F1 closed-caliber-registered 定论登记（triggered/fix-project 须 Full RFC Agent Approved 面——本波未触发如实登记）⑤ g15_budget 五条目齐备 measured_local 零 estimated + budget_eval 全 PASS（P-09；本波零修复零追加维持字面）⑥ G5~G14 closed 面 0-byte（vs G15.0 ref f061487e committed 闭集 = {g14_budget.json, g14_ue_variance_samples.json} = 34f96ac3 归档授权双面；工作树闭集空）+ RFC 命名空间 0-byte（next_free=31 维持） | host 只读（不重跑子门） | evidence/g15_wave3_exit_20260823T121609Z.json（六 facts 全绿） | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g15_wave3_exit_check.py --gate g15.wave.3.exit` → **VERDICT = PASS，exit=0**（required_gates M-b PASS + 六 facts 逐行打印不遮蔽）；`py -3 ci/g15_wave3_exit_check.py --selftest` → ALL PASS（负样本空目录红 + 真树聚合 VERDICT==子门实测态不遮蔽机核双臂；真树臂 PASS 件 120852Z/121609Z + 负样本 FAIL 件 120851Z 在档）。
- **③ 验收命令逐字输出（2026-08-23 真跑留痕，仓库根目录）**：
  - `py -3 ci/g15_gap_fix_closure_smoke.py --selftest` → selftest PASS checks=19（schema 闭集 + 5 RED + 2 GREEN 函数面臂）；`--gate g15.p0.m_b.gap_fix_closure_loop` → VERDICT=PASS checks=19/19（首跑 evidence 120834Z + 接线后复跑 121607Z 双 PASS 在档）；`--verify-latest` → PASS（121607Z，19 键全绿）。
  - `py -3 ci/g15_wave3_exit_check.py --gate g15.wave.3.exit` → VERDICT=PASS（evidence 121609Z）；`--selftest` → ALL PASS。
  - 守卫套件全 PASS：`py -3 ci/check_structure.py` → PASS（11 dirs, 6 files）；`py -3 ci/check_schemas.py` → PASS（本批新增 g15_m_b_gap_fix_closure_loop_ / g15_wave3_exit_ 双前缀 × load/validator/路由六处纯追加，与既有全族互不包含——本波零修复立项零 measured entry 面，故无双条目 budget 追加前缀，g15_budget/budget_eval 0-byte）；`py -3 ci/check_number_ledger.py` → PASS（CI_step on_tree_max 272/next_free 273 校准后实测）；`py -3 ci/budget_eval.py --strict` → PASS（249 pass/0 skip——g15 五条目维持 M-a 双 fresh 带 measured_local，本波零追加）；`py -3 -m pytest tests/ -q` → 136 passed 零回归；`py -3 ci/g15_interlock_check.py --require-ready` → READY 维持（workflow 实测末号 272 == ledger on_tree_max 272 一致面恢复——接线期 pr-smoke.yml Edit 静默失效致 ③ 短暂 RED，io.open 补丁法补回后 READY，诚实留痕不删）；治理两门（acceptance_map/candidate_decisions）复跑 PASS 维持。
  - **G14 M-e 回归门复跑（性能零降级守护 + 触改共享面零降级机核）**：`py -3 ci/g14_regression_drift_guard_smoke.py --gate g14.p0.m_e.regression_drift_guard` → VERDICT=PASS checks=9/9（evidence/g14_m_e_regression_drift_guard_20260823T121531Z.json——76 门最新 evidence 全绿只读汇总 + M96 golden 门序/M139/M140/wave 聚合族 live 抽检真跑零降级 fresh 机核 + G5~G13 closed 面 0-byte + M165 漂移零检出字面）；本波零 src 变更 → G14 M-d 复跑义务 not-triggered（g14_dual_end_fps_parity_smoke.py 复跑触发面 = src 改动，契约 guardrails 性能零降级守护字面——18 格 ×1.00 定盘面无机面触改）。
  - 起草期 FAIL 轨迹（诚实留档不删）：① M-b selftest 首跑 1 败——合成 open-defer 正例 m_a_suggestion 字段未随合成处置面同步（校验器正确拒答，修正合成面后全绿）；② pr-smoke.yml 步骤 271/272 首接 Edit 工具静默失效（snippet 回显在树实测无——G15.1 check_schemas.py 同族教训复现），io.open 补丁法 + 锚点 count==1 断言 + Select-String 核实补回，互锁 ③ 短暂 RED 留痕后 READY 恢复。
- **④ 门序 / 偏差 / not-triggered / no-go 登记面摘要**：
  - **门序**：G-G15-3 兑现（§8.2）→ 本波 G-G15-4 兑现；数字步骤 271/272 按落盘前实测 actual next_free 顺位领取（ledger v1.154 校准同批）；G15.4 绝对画质终审波（M-c）开工面开放。
  - **20 行终态三态分布（M-b 定盘字面）**：**closed-resolved 0 / closed-caliber-registered 4 / open-defer-G16+ 16**——caliber 闭合 4 行 = PT caliber_diff 常驻行（bistro_material_texture_mean_vs_per_texel / emissive_le_mean_vs_textured_emissive / aa_filter_policy_residual / exr_bit_depth_fp16_vs_f32——fresh == 登记位级一致跨会话零漂移，RXS-0392 不拟合显式登记维持）；open-defer 16 行 = lumen GI 2 行（间接光表达面真实差——修复 = GI 多级反弹/表面缓存表达面立项 = 触 RXS-0357 冻结面 + G11 GI 既有判据面 Full RFC + 大工程，非本波可闭合）+ upscale 8 行（deficit 2 行 = vendor DLSS 集成对齐面触 G13.4 契约 tier_note 冻结面 + 黑盒拟合 RXS-0392 RED 风险；noise_hf 6 行 = TSR/vendor 噪声谱面——跨场景符号互反，逐场景调参 = 拟合度量 RED 字面，vendor 黑盒不可工程化介入）+ PT quality_gap 6 行（采样/降噪/收敛/能量面 = G12 closed PT 起步范围面，契约 out_of_scope path_tracer_scope_extension 字面 + 位级锚重收割大工程——energy_conservation@bistro fresh delta 与材质 caliber 行位级全等 0.03216088163252785 归属证据入档，但 emissive/灯面残余面未逐分项对账结清，判档争议向上取严不以 caliber 登记冒充闭合）。逐行 fix_evaluation 论证全文入 `g15_gap_fix_closure_registry.json`（每行写清修复面在哪、为何触冻结面/为何收益风险不成立——零笼统 defer）。
  - **修复立项清单**：**零立项**（修复评估完结 + 零修复立项 + 20 行终态处置全量 = 合法退出形态，任务书字面兑现面）；RED 先行纪律 vacuous 成立如实登记不充绿；RED/GREEN evidence 面 not-triggered（无修复项故无修复 RED→GREEN 件——门内 RED 五臂自检留痕件在档）。
  - **材质链表达面立项评估结论（G11-N8/G11-N9/G12-N10 承接锚命中判定逐字）**：**未命中（not-triggered 如实登记不充绿）**——M-a 20 行 fresh measured_delta 逐族归因核对：lumen 2 行归因 = GI 能量/间接光表达面；upscale 8 行归因 = 重建质量/噪声谱表达面；PT 10 行归因 = 材质求值口径/收敛行为/噪声谱/能量守恒聚合面——**零行归因透射/焦散/镜面 IBL 类能量为画质量级 measured 主差**。G11-N8 太阳穿玻璃高光尾不追如实登记维持（C1 行残余归属字面 0-byte）；G11-N9 镜面 IBL 实测上界 0.031% 显式留档维持（漫反射链已对齐闭环在案）；G12-N10 起步范围冻结维持 0-byte（焦散/体积/specular 链 out，RXS-0357 L1）。Full RFC 立项义务未触发——零 RFC 消费（RFC next_free=31 维持机核）。
  - **G15-MA-F1 定论**：**closed-caliber-registered（契约语义内形态，RXS-0392 口径差显式登记不拟合）**——① 生产车道面（G14 M-d 消费面 = vendor 驻留输出）G14.10f 已修复完结在案（pack SPV rgb×exposure 转显示域；cornell ev100=0 位保持判据 digest 逐字同 + bistro 亮度对齐 tsr 0.00961 vs 0.00984 + 双场景读图 PASS；G14.11 fsr 臂同型）——生产缺陷假说已被修复面排除；② parity 锚定车道面（G13.4 M-a 门消费 = host 链 converged.exr）= G13.4 契约 exposure_note 锚定 scene 域语义 + 度量侧 ×2^(−ev100) 派生尺度链对齐消费（RXS-0392 口径继承；端内参照 deficit 面逐后端自洽尺度消去）+ G14.10f 字面「host pack_vendor_inputs 共享面零触碰保 M-a 锚」——刻意维持的锚定语义内形态；③ 修复排除论证：改 parity 车道输出域 = 触 G13 锁定门判据语义冻结面 + 登记表对账面 + digest 锚重收割同型程序——零收益不成立面排除。判档争议向上取严评估面：生产缺陷 vs 契约语义两假说逐面对账后定论契约语义内形态（修复面已在 G14.10f 兑现完结，残余 = parity 锚定语义面）。
  - **RD-045/M165 漂移监控登记（G15 复跑面）**：本波零复跑新面（M-b 只读消费 M-a evidence + G14 M-e 复跑抽检面漂移零检出字面在 121531Z 件）——同型 digest 漂移**零检出**维持（open-defer 不写进全绿叙述，M-e 门面承接）。
  - **not-triggered / 维持 open 面**：M-c~M-e 未跑（后续波次面）；RD-034/039/040/041/042/043/044/045 八条 open 维持 0-byte（本波零追加零新 RD）；G14 defer 29 行承接锚 0-byte；G15.6a 穷举面承接本波 16 行 open-defer-G16+ 终态（承接锚字面在登记表）。
  - **异己并发工作树面**：本批只含 G15 车道文件（按文件名显式择取）；异己会话 src/ 未提交面（ktx2_read/hzb/restir/sdf_trace/smrt/ssr 等 untracked 面）维持未提交、零消费、零混入（立项裁决 3——M-b 门 frozen_face_zero_rfc_zero 机核面把守：src/spec/conformance porcelain 闭集 = 异己登记六件，越界即 FAIL）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模）。`Assisted-by: Kimi-K3（G15.3 修复闭环波）`（影响范围：milestones/g15/g15_gap_fix_closure_registry.json 首建 20 行终态 + ci/g15_gap_fix_closure_smoke.py + ci/g15_wave3_exit_check.py 新建 + milestones/g15 双 evidence schema 新建〔g15_m_b_gap_fix_closure_loop_/g15_wave3_exit_〕+ ci/check_schemas.py〔双前缀 × load/validator/路由六处纯追加，io.open 补丁法〕+ .github/workflows/pr-smoke.yml〔步骤 271/272，步骤 270 块后追加，io.open 补丁法〕+ registry/number_ledger.json〔CI_step 270→272/next_free 273 + revision_log v1.154〕+ 本契约 §8.3 本条 + evidence 本批真跑件〔M-b 120834Z/121607Z 双 PASS + wave3 120852Z/121609Z PASS + 自检负样本 FAIL 件 120851Z + G14 M-e 复跑 121531Z PASS + 治理三门复跑件 121718Z〕；g15_budget.json/budget_eval.py 0-byte〔本波零修复 measured 面零追加〕；验证方式：块③逐字命令输出——M-b 门 19/19 checks 全绿 + 波聚合门六 facts PASS + 双 selftest 红绿留痕 + 守卫套件全 PASS + pytest 136 passed 零回归 + G14 M-e 回归门复跑 9/9 零降级）。
