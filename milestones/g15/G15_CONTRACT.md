---
contract: G15
title: G15 画质量级收口与商用终审期
status: closed
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

### §8.4 G15.4 绝对画质终审波验收记录（2026-08-23）——G-G15-5 字面兑现：M-c 绝对画质终审门（g15.p0.m_c.absolute_quality_final_review，步骤 273，18/18 checks 全绿 VERDICT=PASS）+ 波聚合门 g15.wave.4.exit（步骤 274）VERDICT=PASS 六 facts；商用收口判定 = **未达标 0/18 如实定盘不冒充**

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g15.p0.m_c.absolute_quality_final_review`（步骤 273） | 契约 §4.2 M-c 逐字 + RXS-0407 L1~L6：**绝对通过线程序产标定**（UE 参照 deficit 双 seed〔契约 seed vs calibration_seed〕方差底场景内 p100×2.0 程序产，禁手写 P-09——四条目：cornell_box ssim p100=3.773167e-03 阈 7.546333e-03 / cornell_box flip p100=1.228874e-03 阈 2.457747e-03 / bistro_interior ssim p100=1.987428e-03 阈 3.974856e-03 / bistro_interior flip p100=6.707719e-04 阈 1.341544e-03，标定链路全要素〔双 seed 帧 digest/逐格 deficit/方差样本集/参数面〕入 evidence + 四条目入 g15_budget measured_local + 同 seed 双跑位级探针双场景一致 + 标定值自在档帧面重算 f64 精确核验〔复跑两批标定值位级一致〕）+ **18 格逐格判定**（Rurix 生产管线 g14_3_pipeline_perf --render 真跑 36 格〔18 格 × 双 seed，GPU 锁纪律，RURIX_REQUIRE_REAL=1 + validation〕vs UE 同场景同档参照帧〔G15.2 M-a 复跑产出面——receipt started_epoch ≥ M-a 启动锚 20260823T084242Z + 抽帧 canonical digest 重算 == receipt 登记全绿〕，display-referred LDR 臂双端同一派生链单源〔双端派生尺度均 1.0——UE 帧管线内 ev100 曝光已施 / Rurix 生产出图全后端管线内 ×2^(−ev100) 已施 receipt exposure==2^(−ev100) 机核；scene-linear 域面 = G15-MA-F1 caliber 已登记面零混入〕SSIM/FLIP 双度量逐格比对，逐格 verdict 逐字入 evidence）+ **逐格 AI 读图严格画面审查记录**（18 格 PNG 导出 .tmp/g15_m_c_preview/ + 结构代理 18 格全绿 + 读图记录 milestones/g15/g15_m_c_ai_reading_records.json 18 格闭集零空行 + PNG digest 逐格绑定 + UE 参照读图面双格登记）+ **商用收口判定**（达标格数 0/18 如实定盘 + 未达格 18 格逐格归因 + G16+ 承接锚字面〔用户 2026-08-19 授权面逐字〕）+ **UE 参照内容有效性机核**（HDR 亮度 max ≤ 1e-3 死黑失败模式字面编码——cornell 三档参照退化检出显式登记 G15-MC-F1，bistro 三档有效）+ RED 五臂独立有效（handwritten-threshold/reading-record-missing/verdict-masquerade/calibration-single-run/stale-ue-reference） | host+device（Rurix 臂生产二进制真跑面；UE 臂 = M-a 复跑产出只读消费面） | evidence/g15_m_c_absolute_quality_final_review_20260823T145400Z.json（18/18） | PASS |
  | `g15.wave.4.exit`（步骤 274） | 波聚合门只读汇总六 facts 全绿：① M-c 最新 evidence PASS + RED 臂独立有效（5 臂）② 18 格判定矩阵 + 读图记录重算绿（M-c 门 crosscheck_verdicts/validate_reading_records 函数面消费——逐格 verdict 重算 == 存储标签 + met_count/verdict 字面一致 + PNG digest 逐格绑定重算绿）③ 标定链程序产机核（g15_budget 九条目齐备〔五既有 + 本门四条目〕measured_local + threshold==measured×2.0 f64 精确重算 + budget_eval 全 PASS，P-09）④ 商用收口判定定盘字面（未达标 0/18 + 未达格逐格归因非空 + g16_anchor 承接锚字面 + findings G15-MC-F1 登记——未达标如实登记不冒充为合法定盘字面）⑤ 三契约三冻结表 0-byte + RXS-0407 spec 锚定维持（trace_matrix/stable_snapshot --check 全 PASS）⑥ G5~G14 closed 面 0-byte（vs G15.0 ref f061487e committed 闭集 = 归档授权双面，工作树闭集空）+ 零 src 变更机核（G14 M-d 复跑义务 not-triggered 如实登记）+ RFC next_free=31 / RXS next_free=408 机核 | host 只读（不重跑子门） | evidence/g15_wave4_exit_20260823T150418Z.json（六 facts 全绿） | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g15_wave4_exit_check.py --gate g15.wave.4.exit` → **VERDICT = PASS，exit=0**（required_gates M-c PASS + 六 facts 逐行打印不遮蔽）；`py -3 ci/g15_wave4_exit_check.py --selftest` → ALL PASS（负样本空目录红 + 真树聚合 VERDICT==子门实测态不遮蔽机核双臂；真树臂 PASS 件 150418Z + 负样本 FAIL 件 141206Z/141208Z 在档；提交后复验件归档另批（chore 留痕沿 G15.2/G15.3 同模））。
- **③ 验收命令逐字输出（2026-08-23 真跑留痕，仓库根目录）**：
  - spec-first 条款批（**条款 PR 先于门 PR**，commit `b2c58b7f`）：spec/visual_comparison.md **RXS-0407** 单号 materialize（落盘前实测 RXS.next_free=407 顺位领取；判档 = 加性 spec 条款——rfc_required 触发面逐条未命中，零冻结面消费零 RFC）+ conformance 锚定语料三件 + trace_matrix 重生成 388→389 全锚定 + stable 快照 388→389 重 bless（bless_log 2026-08-23 行）+ ledger v1.155（RXS on_tree_max 406→407/next_free 408）。
  - `py -3 ci/g15_absolute_quality_review_smoke.py --selftest` → selftest PASS checks=18（schema 闭集 + RED/GREEN 函数面臂）；首跑 `--gate`（20260823T141249Z，16/18——读图记录未落盘诚实红，起草期 FAIL 轨迹留档不删）→ 18 格逐张真读 + 读图记录落盘后复跑 `--gate` → **VERDICT=PASS checks=18/18**（evidence 145400Z；帧面复用 36/36 = digest 复算 == receipt 登记位级确定性复用〔同 seed 双跑探针双场景位级一致 + main/calibration 微差成立旁证〕，标定值两批位级一致）；`--verify-latest` → PASS（145400Z，18 键全绿）。
  - `py -3 ci/g15_wave4_exit_check.py --gate g15.wave.4.exit` → VERDICT=PASS（150418Z）；wave2/wave3 聚合复跑 PASS（150508Z/150511Z——wave3 budget 条目数快照 5→9 加性校准面绿）；互锁 `--require-ready` → READY 维持（workflow 实测末号 274 == ledger on_tree_max 274）。
  - 守卫套件全 PASS：`py -3 ci/check_structure.py` → PASS（11 dirs, 6 files）；`py -3 ci/check_schemas.py` → PASS（本批新增 g15_m_c_absolute_quality_final_review_/g15_m_c_calibration_/g15_wave4_exit_ 三前缀 × load/validator/路由九处纯追加，io.open 补丁法 + count==1 断言 + Select-String 核实——G15.1/G15.3 两起 Edit 静默失效教训字面执行）；`py -3 ci/check_number_ledger.py` → PASS（CI_step on_tree_max 274/next_free 275 校准后实测）；`py -3 ci/budget_eval.py --strict` → PASS（253 pass/0 skip——g15 九条目含本门标定四条目 measured_local 零 estimated）；`py -3 ci/trace_matrix.py --check` → PASS（389/389）；`py -3 ci/stable_snapshot.py --check` → PASS（spec_clauses=389）；`py -3 -m pytest tests/ -q` → 136 passed 零回归；check_guardrails advisory 一条（spec 修订行档位标记「加性条款」沿 G13.4 v1.7 同型先例，不阻断）。
  - **G14 M-e 回归门复跑（触改共享面零降级机核）**：`py -3 ci/g14_regression_drift_guard_smoke.py --gate g14.p0.m_e.regression_drift_guard` → VERDICT=PASS checks=9/9（evidence/g14_m_e_regression_drift_guard_20260823T150723Z.json——76 门最新 evidence 全绿只读汇总 + 触改面真跑抽检零降级 + M165 漂移零检出字面）；本波零 src 变更 → **G14 M-d 复跑义务 not-triggered 如实登记**（g14_dual_end_fps_parity_smoke.py 复跑触发面 = src 改动，契约 guardrails 性能零降级守护字面——本波 = 测量与判定面，出图链路零 src 改动，18 格 ×1.00 定盘面无机面触改；wave4 fact⑥ 机核面把守）。
  - 起草期 FAIL 轨迹（诚实留档不删）：① 门首跑 141249Z（16/18——ai_reading_records_18_cells_valid + red_arm_reading_record_missing_detected 双键红，读图记录未落盘预期面，AI 读图强制门字面兑现）+ 二跑 145400Z 全绿；② ledger revision_log 前插补丁两起 `{` 吞行（v1.155/v1.156 各一起，io.open 补丁锚点含下行开括号所致——即修即核 JSON 校验 + check_number_ledger 复绿，补丁法纪律内诚实留痕）。
- **④ 门序 / 偏差 / not-triggered / no-go 登记面摘要**：
  - **门序**：G-G15-4 兑现（§8.3）→ 本波 G-G15-5 兑现；数字步骤 273/274 按落盘前实测 actual next_free 顺位领取（ledger v1.156 校准同批；RXS-0407 = v1.155 条款批先行）；G15.5 性能零降级波（M-d）开工面开放。
  - **绝对通过线标定值（逐度量逐场景，程序产禁手写 P-09）**：cornell-box ssim 阈 7.546333e-03（p100 3.773167e-03）/ cornell-box flip 阈 2.457747e-03（p100 1.228874e-03）/ bistro-interior ssim 阈 3.974856e-03（p100 1.987428e-03）/ bistro-interior flip 阈 1.341544e-03（p100 6.707719e-04）——双 seed 方差底场景内 p100×2.0（沿 G13.4 标定三条目范式；样本 = 场景内 3 档 × 3 后端九格逐格 |deficit_main − deficit_calibration|）；标定 evidence 四件（g15_m_c_calibration_{ssim,flip}_{cornell_box,bistro_interior}_20260823T141249Z）+ g15_budget 四条目 measured_local（既有五条目 0-byte，纯追加 diff 44 insertions/0 deletions 机核）。
  - **18 格判定矩阵（SSIM/FLIP/读图/verdict 逐格，measured 面定盘）**：cornell-box 九格——ssim_deficit 0.460574~0.462832 / flip_deficit 0.174001~0.175752（对死黑参照的退化大数值面），reference_state=degenerate_black，读图 PASS 9/9（盒体/左绿墙/右红墙/白后墙/顶部面光/双箱全在位，无乱序/错位/斑块伪影，三后端互一致；箱体背光面深黑 = 直接光口径固有态如实登记），verdict 全 fail（归因 = ue_reference_degenerate〔G15-MC-F1〕）；bistro-interior 九格——ssim_deficit 0.057792~0.062832 / flip_deficit 0.016176~0.017934 vs 阈 3.974856e-03/1.341544e-03 **双度量全超阈**（超阈量级 ssim ≈ 15× / flip ≈ 12×——跨端 GI/曝光链残余口径差 + 真实重建差 measured 面），reference_state=ok，读图 PASS 9/9（吊灯群前厅 4 盏 + 右上 2 盏明亮清晰位置逐格对齐，红色墙板/壁柱依稀可辨，吧台/桌椅剪影中后景可辨，地面砖反射微光可辨；暗部深黑 = 夜景 ev100=−4 固有 + 无 GI 直接光口径边界态双因子叠加〔M-a §8.2 已登记〕——「暗但结构在」非死黑无内容，诚实区分字面兑现；无乱序/错位/伪影，三后端 × 三档互一致），verdict 全 fail（归因 = ssim/flip deficit 双超阈逐格数值面）。
  - **商用收口判定结论：未达标 0/18（如实登记不冒充）**——达标 = deficit 双度量均进绝对阈 ∧ AI 读图 PASS；measured 面零格全立。未达格逐格归因双族：① cornell 九格 = UE 参照臂死黑退化（G15-MC-F1——测量链缺陷如实登记，判定不冒充达标亦不静默消费）；② bistro 九格 = deficit 双超程序产绝对阈（跨端口径残余〔Rurix 无 GI 直接光 vs UE Lumen——lumen 登记行/材质链评估 not-triggered 在案〕+ 重建差真实面，UE 参照读图旁证：UE 帧墙面纹理/挂画/吧台器物/桌椅排/地面砖反射显著充盈于 Rurix 帧）。**G16+ 承接锚字面**（用户 2026-08-19 授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」逐字承接）入 evidence commercial_closure.g16_anchor——重判条件 = G16+ 立项窗逐项重评；兜底 = 维持未达标登记不冒充。判定结论以 measured 面为准未受期望影响（0/18 定盘）。
  - **新发现显式登记（法定来源唯一纪律「新发现差距显式登记不静默混入」）**：**G15-MC-F1 `ue_reference_arm_black_frames@cornell-box`**（measurement_chain_defect）——UE 参照臂 cornell-box 全三档 32 帧 RGB ≈ 0（alpha=1.0；HDR 亮度全帧扫描 max = 0.000e+00 ≤ 1e-3 死黑失败模式字面命中；bistro 三档同法实测 max ≈ 92~100 内容正常旁证）；G13.4 期最早 evidence（20260818T212204Z）起 ssim_ue=1.0/flip_ue=0.0 精确值在档——端内参照黑对黑退化完美值吸收潜伏，M-a 处置面 cornell 行 a_value=1.0/0.0 同形态佐证；**本门参照内容有效性机核 + AI 读图参照面首次检出**（G14.10f 教训字面兑现——该参照臂 receipt exit=0、DLSS engagement 日志齐备、digest 登记齐全、freshness 机核绿，唯内容死黑：digest/流程面全绿 ≠ 内容正确）。处置：disposition_hint = open-defer-G16+（UE 项目侧 cornell 出图链诊断/修复归 G16+ 面；本波 = 测量与判定面；G13.4/G15.2 既存端内参照面 evidence 不因本 finding 失效〔退化面如实登记，不 retroactive 改写 0-byte 纪律维持〕）；finding 入 M-c evidence parity.findings 在档 + reference_readings degenerate_black 读图直证在档。
  - **AI 读图 18 格逐格审查记录（本门强制门面，digest 面不替代内容面）**：逐格记录全文入 milestones/g15/g15_m_c_ai_reading_records.json（18 格闭集 + 结构完整性/乱序/错位/全黑/关键结构可见/暗部态/三后端互一致性逐格字段 + PNG digest 逐格绑定 + 审查者/时刻字面）；UE 参照双格读图（cornell 死黑直证 / bistro 内容正常且 GI 充盈旁证）同档。
  - **RD-045/M165 漂移监控登记（G15 复跑面）**：本波生产管线 36 格渲染 + 双跑位级探针双场景一致（cornell t67 tsr + bistro t67 tsr 同 seed 双跑 converged_digest 位级一致）+ 两批门跑标定值位级一致 + G14 M-e 复跑 150723Z 件漂移零检出字面——同型 digest 漂移**零检出**（维持 open-defer 不写进全绿叙述，M-e 门面承接）。
  - **not-triggered / 维持 open 面**：M-d/M-e 未跑（后续波次面）；RD-034/039/040/041/042/043/044/045 八条 open 维持 0-byte；G14 defer 29 行承接锚 0-byte；G14 M-d 复跑义务 not-triggered（零 src 变更机核字面）；G15.6a 穷举面承接本波 G15-MC-F1 与 16 行 open-defer-G16+ 终态（承接锚字面在档）。
  - **异己并发工作树面**：本批只含 G15 车道文件（按文件名显式择取）；异己会话 src/ 未提交面（ktx2_read/hzb/restir/sdf_trace/smrt/ssr 等 untracked 面）维持未提交、零消费、零混入（立项裁决 3——wave4 fact⑥ 机核面把守：src/ tracked diff 空 + untracked ⊆ 异己登记六件闭集，越界即 FAIL）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模）。`Assisted-by: Kimi-K3（G15.4 绝对画质终审波）`（影响范围：spec-first 条款批 commit b2c58b7f〔spec/visual_comparison.md RXS-0407 + v1.8 修订行 + conformance 锚定三件 + traceability_matrix 双产物 + stable_api.snapshot + bless_log + ledger v1.155〕+ 门批〔ci/g15_absolute_quality_review_smoke.py + ci/g15_wave4_exit_check.py 新建 + milestones/g15 三 evidence schema 新建〔g15_m_c_absolute_quality_final_review_/g15_m_c_measured_entry_/g15_wave4_exit_〕+ milestones/g15/g15_m_c_ai_reading_records.json 首建 18 格 + g15_budget.json 四条目纯追加〔既有五条目 0-byte，44+/0−〕+ ci/check_schemas.py 九处纯追加 + ci/budget_eval.py g15.m_c.absolute_pass_line_ 分派支纯追加 + ci/g15_wave3_exit_check.py budget 条目数快照 5→9 加性校准〔同里程碑车道门期内修订沿 G14.6 先例〕+ pr-smoke.yml 步骤 273/274〔步骤 272 块后追加，273 = device 真跑门 --verify-latest 沿 M-h/soak 重门先例〕+ registry/number_ledger.json v1.156〔CI_step 272→274/next_free 275〕+ 本契约 §8.4 本条 + evidence 本批真跑件〔M-c 141249Z 起草 FAIL 留痕 + 145400Z PASS + 标定四条目件 + wave4 150418Z PASS + 自检负样本 FAIL 件 141206Z/141208Z + wave2/wave3 复跑 150508Z/150511Z + G14 M-e 复跑 150723Z PASS〕+ .tmp/g15_m_c_preview/ 20 格 PNG〔一次性预览面不入 commit〕；验证方式：块③逐字命令输出——M-c 门 18/18 checks 全绿 + 波聚合门六 facts PASS + 双 selftest 红绿留痕 + 守卫套件全 PASS + budget_eval --strict 253 pass + pytest 136 passed 零回归 + G14 M-e 回归门复跑 9/9 零降级 + 互锁 READY 维持）。
### §8.5 G15.5 性能零降级波验收记录（2026-08-23）——G-G15-6 **未通过如实定盘**：M-d 性能零降级守护门（g15.p0.m_d.perf_parity_zero_regression，步骤 275，6/12 checks VERDICT=FAIL）+ 波聚合门 g15.wave.5.exit（步骤 276）VERDICT=FAIL——G14 M-d 同口径复跑 17/18（bistro-interior/t100/dlss_sr ratio ≈0.83 跨两轮全协议复跑一致未达 ×1.00）诚实红不冒充

- **① 独立断言清单（未全绿如实登记，红面不遮蔽）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g15.p0.m_d.perf_parity_zero_regression`（步骤 275） | 契约 §4.2 M-d 逐字：**G14 M-d 门同口径复跑 fresh PASS**（g14_m_d_dual_end_fps_parity 本波复跑件 timestamp ≥ 本波启动锚 20260823T153347Z + base_commit==HEAD 同树机核 + device executed——**复跑件本体 status=fail（17/18）故 freshness 链 PASS 面不成立，诚实红**）+ **逐格 ratio ≥ ×1.00 维持**（fps_ratio f64 精确重算 == 存储值 + 三轮守护带 + 跨轮中位数口径 + 生产口径不变量——复跑件红面下逐格重算跳过不充绿）+ **digest 锚漂移守护**（复跑件本体面 Stage A digest 守护 18 格 × 3 轮 == 冻结锚全等零漂移〔双跑机核绿在档〕，本门消费面因复跑件红而同红不遮蔽）+ **G14 M-c 画质锚带复核 = 绿**（SSIM deficit 重算 0.005389118830 ≤ 0.010779849285388998 带内，g14_m_c 最新件 044803Z PASS + 锚定条目 measured×2.0==threshold f64 对账绿）+ **G14 门产 budget 零 estimated 维持 = 绿**（32 条目全 measured_local 零 skip）+ RED 四臂独立有效 = 绿（ratio 篡改/旧 evidence 冒充 fresh/缺轮两轮冒充三轮/锚漂移静默——函数面注入全检出） | host+device（G14 M-d 复跑 = 本波子进程双臂真跑面消费，GPU 锁纪律沿脚本本体面） | evidence/g15_m_d_perf_parity_zero_regression_20260823T165103Z.json（6/12） | **FAIL** |
  | `g15.wave.5.exit`（步骤 276） | 波聚合门只读汇总六 facts 不遮蔽：**①红**（M-d 最新 evidence 非 PASS）**②红**（复跑真跑面——双轮复跑件均 status=fail；18 格全达不成立）**③红**（M-d evidence 锚覆盖面 0/18——复跑件红面消费跳过如实红；复跑件本体 digest 守护双跑绿在案不冒充）**④绿**（画质锚带复核重算 0.005389118830 ≤ 带，M-d 存储面与聚合侧重算位级一致）**⑤绿**（g14_budget 32 条目零 estimated + g15_budget 九条目 measured_local + budget_eval 全 PASS）**⑥红**（工作树闭集越界 = milestones/g14/g14_fps_gap_registry.json 门产登记行——未达格如实登记面，见 ④） | host 只读（不重跑子门） | evidence/g15_wave5_exit_20260823T165117Z.json（facts 2/6 PASS 不遮蔽） | **FAIL** |

- **② 波聚合门实测输出**：`py -3 ci/g15_wave5_exit_check.py --gate g15.wave.5.exit` → **VERDICT = FAIL，exit=1**（required_gates M-d FAIL + 六 facts 逐行打印不遮蔽：④⑤ 绿、①②③⑥ 红）；`py -3 ci/g15_wave5_exit_check.py --selftest` → ALL PASS（负样本空目录红 165139Z + 真树聚合 VERDICT==FAIL==子门实测态不遮蔽机核双臂 165142Z——红树下面聚合不充绿字面兑现）。
- **③ 验收命令逐字输出（2026-08-23 真跑留痕，仓库根目录）**：
  - **G14 M-d 门同口径复跑（本波两轮全协议真跑，GPU 锁纪律沿脚本既有面——UE 臂 harness 子进程自持锁 / Rurix 臂门侧逐格持锁，无嵌套持锁；三轮进程级独立运行 160 帧协议，零缩短）**：
    - 复跑① `py -3 ci/g14_dual_end_fps_parity_smoke.py --gate g14.p0.m_d.dual_end_fps_parity` → VERDICT=FAIL checks=10/10 pass_line=未达标（evidence 153359Z——达标 17/18；bistro-interior/t100/dlss_sr UE=2.962ms Rurix=3.540ms ratio=0.8368 未达标；生产口径 v2 机核=True + Stage A digest 守护 18 格 × 3 轮 == 冻结锚=True）；
    - 复跑②（确认跑——单格翻线 + UE 臂跨会话摆幅假设鉴别，同全协议不缩短）→ VERDICT=FAIL checks=10/10（evidence 161302Z——达标 17/18；同格 UE=2.968ms Rurix=3.562ms ratio=0.8332 未达标；digest 守护=True；**两轮一致 = 非单次抖动，当前环境下定盘面**）。
  - `py -3 ci/g15_perf_parity_guard_smoke.py --selftest` → selftest PASS checks=12（schema 闭集 + 4 RED + 4 GREEN 函数面臂）；`--gate g15.p0.m_d.perf_parity_zero_regression --wave-start 20260823T153347Z` → **VERDICT=FAIL checks=6/12**（evidence 165103Z——绿键：quality_anchor_band_recheck / g14_budget_zero_estimated / RED 四臂；红键：g14_m_d_rerun_fresh_pass + 逐格四面 + comparison_vs_g14_12_rerun，诚实红不充绿）；`--verify-latest` → FAIL（165103Z 六红键逐字列出，诚实面）。
  - 守卫套件全 PASS：`py -3 ci/check_structure.py` → PASS（11 dirs, 6 files）；`py -3 ci/check_schemas.py` → PASS（本批新增 g15_m_d_perf_parity_zero_regression_ / g15_wave5_exit_ 双前缀 × load/validator/路由六处纯追加，io.open 补丁法 + count==1 断言 + Select-String 核实——G15.1/G15.3 两起 Edit 静默失效教训字面执行；含本批红件 schema 校验全过）；`py -3 ci/check_number_ledger.py` → PASS（CI_step on_tree_max 276/next_free 277 校准后实测）；`py -3 ci/budget_eval.py --strict` → PASS（253 pass/0 skip——g14 画质锚 0.005 vs 带 0.010779849285388998 + g15 九条目维持，本波零追加）；`py -3 -m pytest tests/ -q` → 136 passed 零回归；`py -3 ci/g15_interlock_check.py --require-ready` → READY 维持（workflow 实测末号 276 == ledger on_tree_max 276 一致面）。
  - 起草期 FAIL 轨迹（诚实留档不删）：ledger v1.157 补丁首跑 notes 锚点撞车（「下一个可用 CI_step=269」短语在 CI_step notes 与 revision_log 双处命中，count==2 断言拒写——即修锚点加长后复跑绿，写盘前断言零污染在案）；G14 M-d 复跑双 FAIL 件 153359Z/161302Z + M-d 门 FAIL 件 165103Z + wave5 FAIL 件 165117Z/自检双件 165139Z/165142Z 全量在档不删（evidence 只增不删不改纪律）。
- **④ 门序 / 偏差 / not-triggered / no-go 登记面摘要**：
  - **门序**：G-G15-5 兑现（§8.4）→ 本波 **G-G15-6 未通过如实定盘**（M-d 判据「逐格 ratio ≥ ×1.00 维持」于 bistro-interior/t100/dlss_sr 格当前测量面不成立——红 = 诚实面，不阻塞 G15.6a 穷举面对本格的承接处置登记；数字步骤 275/276 按落盘前实测 actual next_free 顺位领取，ledger v1.157 校准同批）。
  - **本波复跑与 G14.12 soak 复跑 ratio 对照表（18 格逐格，跨轮中位数口径）**：

    | 格 | G14.12 soak 复跑（051754Z） | 本波复跑①（153359Z） | 本波复跑②（161302Z） |
    |---|---|---|---|
    | cornell-box/t50/tsr_device | 8.2261 | 7.7906 | 8.2380 |
    | cornell-box/t50/dlss_sr | 3.4031 | 3.0658 | 3.1446 |
    | cornell-box/t50/fsr_3_1_5 | 3.4059 | 3.2724 | 3.4659 |
    | cornell-box/t67/tsr_device | 6.3505 | 6.4812 | 6.7475 |
    | cornell-box/t67/dlss_sr | 2.7493 | 2.5000 | 2.9230 |
    | cornell-box/t67/fsr_3_1_5 | 2.9440 | 3.0157 | 3.2051 |
    | cornell-box/t100/tsr_device | 4.0629 | 4.3671 | 4.3520 |
    | cornell-box/t100/dlss_sr | 2.0701 | 2.1469 | 2.2925 |
    | cornell-box/t100/fsr_3_1_5 | 2.5562 | 2.4958 | 2.5971 |
    | bistro-interior/t50/tsr_device | 2.3750 | 2.2068 | 2.4289 |
    | bistro-interior/t50/dlss_sr | 1.9771 | 1.7999 | 1.8164 |
    | bistro-interior/t50/fsr_3_1_5 | 2.7920 | 2.4713 | 2.4104 |
    | bistro-interior/t67/tsr_device | 2.3226 | 1.9496 | 1.9740 |
    | bistro-interior/t67/dlss_sr | 1.5723 | 1.3432 | 1.3031 |
    | bistro-interior/t67/fsr_3_1_5 | 2.2909 | 1.9794 | 1.9824 |
    | bistro-interior/t100/tsr_device | 1.7987 | 1.4242 | 1.4017 |
    | bistro-interior/t100/dlss_sr | **1.0831** | **0.8368 未达** | **0.8332 未达** |
    | bistro-interior/t100/fsr_3_1_5 | 1.6135 | 1.2451 | 1.2539 |

    汇总：G14.12 = 18/18 达标；本波复跑①/② = 17/18（同格双轮一致未达）。
  - **未达格归因（measured 面，不拟合 RXS-0392）**：bistro-interior/t100/dlss_sr——**Rurix 臂零降级证据面**：跨四会话 3.482（G14plus 波0 定盘）→ 3.657（G14.12）→ 3.540 → 3.562ms 稳定带内，G15 全期零 src 变更机核（src/ tracked diff 空 + Stage A digest 锚 18 格 × 3 轮位级全等双跑零漂移 = 生产管线码面位级同 G14.12）；**UE 参照臂跨会话摆幅**：同格 UE median 4.212 → 3.961 → 2.962/2.968ms（今日午后两轮一致 −25% vs G14.12；cornell 三格反向 +3~+10% 旁证双臂环境面非单向）——通过线棒（UE 臂）环境位移致边界格跨线；同格同档 vendor/自研对照：tsr_device 1.4017 / fsr_3_1_5 1.2539 双达，dlss_sr 0.8332 未达 = Rurix DLSS 车道 t100 档当前真实性能特征（逐轮比值双轮 0.786~0.848 一致，非噪声）。UE 臂 receipt 三面机核绿（command_digest 三轮位级同值 = 同命令面同口径 + DLSS-SR engagement 日志令牌在档 + CSV 新鲜度机核过）——测量链口径内事件，非测量缺陷。
  - **处置与承接锚**：未达格已经 G14 M-d 门自身登记面写入 `milestones/g14/g14_fps_gap_registry.json`（gap_id `51a150cb4523e8b6` fps_parity_deficit@bistro-interior/t100/dlss_sr，gaplib 正典形校验绿，只登记不拟合 RXS-0392——门产如实登记不静默，本批随文提交显式登记）+ 本波 M-d 门 evidence 定盘红 + **G16+ 承接锚**：商用收口未达标定盘面引用 §8.4 字面「**商用收口判定结论：未达标 0/18（如实登记不冒充）**」同律——本波性能守护面叠加一例未达格（bistro-interior/t100/dlss_sr ratio 0.8332 < ×1.00），重判条件 = G16+ 立项窗本格双端复测 + Rurix DLSS 车道 t100 档优化面评估；兜底 = 维持未达标登记不冒充（用户 2026-08-19 授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」逐字承接——G15.6b 终审面双未达标登记在案：画质 0/18 + 性能 17/18）。
  - **RD-045/M165 漂移监控登记（G15 复跑面）**：本波 G14 M-d 双轮复跑面 Stage A digest 守护 18 格 × 3 轮 == 冻结锚位级全等（双跑机核绿在档）——同型 digest 漂移**零检出**（维持 open-defer 不写进全绿叙述，M-e 门面承接）。
  - **画质锚带复核（M-d 判据第二分项 = 绿）**：G14 M-c 最新 evidence 044803Z PASS + 在树 converged.exr 双件 SSIM deficit 重算 **0.005389118830 ≤ 0.010779849285388998** 带内（g14_budget 锚定条目 threshold f64 精确 == 契约字面 + measured×2.0==threshold 程序产对账绿）。
  - **budget 零 estimated 维持（M-d 判据第三分项 = 绿）**：g14_budget 32 条目全 measured_local 零 skip + g15_budget 九条目维持 measured_local（本波零追加）+ budget_eval --strict 253 pass/0 skip。
  - **not-triggered / 维持 open 面**：M-e 未跑（G15.6a 面）；RD-034/039/040/041/042/043/044/045 八条 open 维持 0-byte（本波零追加零新 RD——未达格走 g14_fps_gap_registry 门产登记面 + G15.6a 穷举承接，不私设 RD）；G14 defer 29 行承接锚 0-byte；G14 M-d 门脚本与其 G14.12 定盘 evidence 0-byte（本波只消费复跑件不改门）。
  - **异己并发工作树面**：本批只含 G15 车道文件 + G14 M-d 门产登记面刷新（g14_fps_gap_registry.json 1 行——门自身登记面如实落盘显式提交，非判据/契约面；wave5 fact⑥ 红 = 该登记面在树越界字面，与 M-d 红同源不遮蔽）+ evidence 本批真跑件；异己会话 src/ 未提交面（ktx2_read/hzb/restir/sdf_trace/smrt/ssr 等 untracked 面）维持未提交、零消费、零混入（立项裁决 3——wave5 fact⑥ src/ 零变更机核绿）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模；本波 = 未通过如实定盘签署——红面签署不充绿）。`Assisted-by: Kimi-K3（G15.5 性能零降级波）`（影响范围：ci/g15_perf_parity_guard_smoke.py + ci/g15_wave5_exit_check.py 新建 + milestones/g15 双 evidence schema 新建〔g15_m_d_perf_parity_zero_regression_/g15_wave5_exit_〕+ ci/check_schemas.py〔双前缀 × load/validator/路由六处纯追加，io.open 补丁法〕+ .github/workflows/pr-smoke.yml〔步骤 275/276，步骤 274 块后追加，io.open 补丁法；275 = device 真跑门消费面 --verify-latest 沿 M-c/M-h/soak 重门先例〕+ registry/number_ledger.json〔CI_step 274→276/next_free 277 + revision_log v1.157〕+ milestones/g14/g14_fps_gap_registry.json〔G14 M-d 门产未达格登记 1 行，gaplib 正典形〕+ 本契约 §8.5 本条 + evidence 本批真跑件〔G14 M-d 复跑双 FAIL 件 153359Z/161302Z + M-d 门 FAIL 件 165103Z + wave5 FAIL 件 165117Z + 自检负样本/真树一致性双件 165139Z/165142Z 在档不删〕；g15_budget.json/budget_eval.py 0-byte〔本波零追加〕；G14 M-d 门脚本与 G14.12 定盘 evidence 0-byte〔本波只消费复跑件不改门〕；验证方式：块③逐字命令输出——M-d 门 6/12 诚实红 + 波聚合门 facts 2/6 不遮蔽红 + 双 selftest 红绿留痕（含红树下聚合不充绿字面兑现）+ 守卫套件全 PASS + pytest 136 passed 零回归 + 互锁 READY 维持）。

### §8.6 G15plus 延续波（诊断 + 有界攻坚）记录（2026-08-23 UTC）——M-d 红面格 bistro-interior/t100/dlss_sr 攻坚：诊断定论 = **UE 参照臂缓存暖态跨会话位移**（测量链口径内事件，非测量缺陷）+ Rurix 臂 DLSS 车道 t100 档有界优化**物理不可达**定论；第三轮全协议复跑 17/18（本格 ratio 0.8571）维持 → 诚实定论登记 **G15-MD-F1（open-defer-G16+）**；本波零新门/零脚本/零编号消费（诊断 + 复跑件 + 契约记录面）

- **① 独立断言清单（本波零新门——消费既有门三面，未全绿如实登记，红面不遮蔽）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g14.p0.m_d.dual_end_fps_parity`（G14 closed 门同口径复跑，第三轮） | G14 契约 §4.2 M-d 逐字：checks 10/10 机器键全绿（dual_end_measurement_fresh / three_run_independence / sampling_protocol_50x3 / production_caliber_v2 / stage_a_digest_drift_guard〔18 格 × 3 轮 == 冻结锚零漂移〕/ pass_line_evaluated / quality_guard_green / gap_registry_written / budget_eval_all_pass / red_arms_effective）+ **通过线判定 = 达标 17/18**（bistro-interior/t100/dlss_sr UE=3.001ms Rurix=3.502ms ratio=0.8571 < ×1.00 未达，诚实红不冒充；base_commit==HEAD fcfac944 同树机核 + device executed + GPU 锁纪律沿脚本既有面〔UE 臂 harness 子进程自持锁 / Rurix 臂门侧逐格持锁〕+ 三轮进程级独立 160 帧协议零缩短） | host+device | evidence/g14_m_d_dual_end_fps_parity_20260823T172719Z.json（status=fail，met 17/18） | **FAIL（17/18）** |
  | `g15.p0.m_d.perf_parity_zero_regression`（步骤 275，--wave-start 20260823T172500Z 消费本波复跑件） | 契约 §4.2 M-d 逐字六键绿维持：quality_anchor_band_recheck（SSIM deficit 重算 0.005389118830 ≤ 0.010779849285388998 带内）+ g14_budget_zero_estimated（32 条目）+ RED 四臂独立有效（ratio 篡改/旧件冒充 fresh/缺轮冒充三轮/锚漂移静默——函数面注入全检出）；六键红维持：g14_m_d_rerun_fresh_pass（复跑件本体 status=fail，freshness 链 PASS 面不成立诚实红）+ 逐格四面 + comparison_vs_g14_12_rerun（诚实红不充绿） | host+device（G14 M-d 复跑 = 本波子进程双臂真跑面消费） | evidence/g15_m_d_perf_parity_zero_regression_20260823T180618Z.json（6/12） | **FAIL（6/12）** |
  | `g15.wave.5.exit`（步骤 276，波聚合门只读汇总复跑） | 六 facts 不遮蔽：④绿（画质锚带复核重算绿）⑤绿（g14_budget 32 + g15_budget 九条目 measured_local 零 estimated + budget_eval 全 PASS）①②③⑥ 红（M-d 最新 evidence 非 PASS / 复跑真跑面 18 格全达不成立 / 锚覆盖面红面消费跳过〔复跑件本体 digest 守护绿在档不冒充〕/ g14_fps_gap_registry.json 门产登记面在树越界字面——与 M-d 红同源不遮蔽） | host 只读（不重跑子门） | evidence/g15_wave5_exit_20260823T180709Z.json（facts 2/6） | **FAIL（facts 2/6）** |

- **② 波聚合门实测输出**：`py -3 ci/g15_wave5_exit_check.py --gate g15.wave.5.exit` → **VERDICT = FAIL，exit=1**（required_gates M-d FAIL + 六 facts 逐行打印不遮蔽：④⑤ 绿、①②③⑥ 红——红树下聚合不充绿字面维持）；`py -3 ci/g15_perf_parity_guard_smoke.py --gate g15.p0.m_d.perf_parity_zero_regression --wave-start 20260823T172500Z` → **VERDICT=FAIL checks=6/12**（红绿键分布与 §8.5 定盘面逐字一致）。两门脚本本波 0-byte（selftest 红绿留痕在 §8.5 档，本波只消费不改门）。
- **③ 验收命令逐字输出（2026-08-23 UTC 真跑留痕，仓库根目录）**：
  - **诊断臂（阶段 1，host 测量面——零门消费零编号）**：
    - UE 臂跨会话包络重建（`evidence/g14_m_b_ue_benchmark_arm_measurement_*.json` 13 件 + `evidence/g14_m_d_dual_end_fps_parity_*.json` 12 件逐格 ue_median_ms 提取）：bistro-interior/t100 UE median 逐会话 = 4.243/4.227/4.264（08-19 三会话）→ 4.851/4.875/4.889（08-20）→ 5.015/5.119（08-21 午/昏）→ 4.161（08-21 夜）→ 4.195/4.154（08-22）→ 4.196（08-23 09:17 本地）→ 3.960（08-23 12:41 本地，G14.12 soak 定盘件 044132Z/051754Z）→ **2.962/2.968（本日午后再轮，破历史包络下限 −25%）→ 3.001（本波第三轮，仍 −24%）**；bistro t50/t67 同向破下限（t50 历史 min 2.974→2.623/2.643；t67 min 3.100→2.676/2.715）；cornell 三档全在历史带内（2.05~2.27 vs 带 1.95~3.73）。**场景选择性破包络**（重内容场景全档破下限，轻量场景带内）。
    - 会话内形态对拍：G14.12 soak 件 bistro t100 UE 三轮 = 3.960/3.961/3.923（紧致，无冷轮）；本日三轮会话 = 2.951~2.986 同紧致（per_run_ratios × Rurix 逐轮值反解 + 今日 CSV 实测 trimmed 2.9508/2.9678/2.9855 互证）——位移为**跨会话持久态**，非会话内冷缓存税残留。
    - 环境画像七元组/receipt 全字段对拍：command_digest 三轮位级同值（`sha256:b24553a5…`，G14 期与今日同值——同命令面同口径）+ CSV 元数据 engineversion=5.8.1-56057345（M128 登记面）+ gpudriver=620.02（今日 CSV 元数据 == M-b 登记值 == `nvidia-smi` 实测）+ OS build 10.0.28120 六件 evidence environment 块位级同值 + LastBootUpTime=2026-08-22 22:06 本地（G14.12 首判/soak/本日三轮四会话同 boot）+ DLSS engagement NGX 令牌在档（SrcRect=DestRect=1920×1080、NGXPerfQuality=DLAA(5)、NGXDLSSPreset=Default(0)→title default Preset K、Container0/1/2 Tensor 分配令牌齐备）——**驱动/OS/UE build/命令面/engagement 全同**。
    - 今日 CSV 逐帧分布（`g14_bench_bistro-interior_t100_r{1..3}.csv`，480 帧弃首 300 取末 150 协议窗）：三块均值 3.03/3.03/3.00 均匀无 warmup 残留；GPUTime median 2.266 vs FrameTime 2.960（render-thread/GPU 绑定态）；cv 0.09~0.10 正常抖动态。
    - 缓存写活跃机核：`%LOCALAPPDATA%\NVIDIA\DXCache` 08-23 12:00 本地后 18 件 .nvph 写（本日门跑窗口内）+ `%LOCALAPPDATA%\UnrealEngine\Common\DerivedDataCache` 同窗 7 件——持久缓存面今日仍在填充。
    - 内容面排除：`G13_BistroInterior.umap`/`G13_CornellBox.umap` mtime 刷新 = 门跑内部例程（G13/G12 门 UE 步顺城存盘），G15.2 复跑三门契约 digest 机核全绿（20/20+19/19+21/21）——场景内容零变化；uproject/Config 0-byte（08-18 起未动）。
  - **Rurix 臂残余解剖（阶段 1，`RURIX_VENDOR_TIMING=1` 六段遥测，bistro t100 dlss 生产驻留车道，160 帧弃 warmup 20）**：staging=0.000（G14.11 攻坚①消 copy 面维持）/ sl_book=0.009 / record=0.011 / evaluate=0.118（CPU 簿记）/ **submit_wait=2.169（NGX 网络 GPU 执行+同步 = 残余主项）** / DLSS 侧 total=2.307ms + Vulkan 侧 scene_gpu=1.024ms（G14.11 登记 1.08 带内）——与 G14.11 末格攻坚登记（DLSS 网络 ~2.1ms + 三条跨设备 copy 0.6ms 已消面）逐字对账一致，**残余 headroom = NGX 网络黑盒 ~2.1ms + 场景 GPU ~1.0ms + 两侧调度/同步 ~0.4ms**。
  - **攻坚复跑（阶段 2，A 支合法动作 = 当前暖态环境下双端同会话重赛，全协议第三轮）**：`py -3 ci/g14_dual_end_fps_parity_smoke.py --gate g14.p0.m_d.dual_end_fps_parity` → **VERDICT=FAIL checks=10/10 pass_line=未达标（达标 17/18）**（evidence 172719Z；逐格输出见 ④ 矩阵；bistro-interior/t100/dlss_sr UE=3.001ms Rurix=3.502ms **ratio=0.8571 未达标**；生产口径 v2 机核=True + Stage A digest 守护 18 格 × 3 轮 == 冻结锚=True）。
  - 守卫套件全 PASS：`py -3 ci/check_structure.py` → PASS（11 dirs, 6 files）；`py -3 ci/check_schemas.py` → PASS（本波零追加）；`py -3 ci/check_number_ledger.py` → PASS（off_tree grx advisory 一条沿既有不阻断）；`py -3 ci/budget_eval.py --strict` → PASS（253 pass/0 skip）；`py -3 -m pytest tests/ -q` → **136 passed** 零回归；`py -3 ci/g15_interlock_check.py --require-ready` → **READY 维持**（workflow 实测末号 276 == ledger on_tree_max 276、next_free 277——本波零编号消费一致面）。
- **④ 门序 / 偏差 / not-triggered / no-go 登记面摘要**：
  - **门序**：本波 = §8.5 G15.5 红面的延续波（诊断 + 有界攻坚），不改波次状态机——**G-G15-6 维持未通过如实定盘**（第三轮复跑仍 17/18）；G15.6a 穷举面承接本波 G15-MD-F1 终态（承接锚字面在下）。零新门/零脚本/零 schema/零 workflow 步 → **零编号消费**（CI_step next_free=277 维持，ledger 0-byte；RFC next_free=31 / RXS next_free=408 / RD 八条 0-byte 维持）。
  - **诊断定论（UE 臂 −25% 归因三选一）= b) 缓存暖态（UE shader cache / NVIDIA DXCache / NGX 模型缓存 / UE DDC 跨会话积累）**，证据链八面：① 场景选择性——bistro（重材质/重 PSO 场景）全三档破历史包络下限而 cornell（36 三角轻量场景）三档全在带内，环境均匀位移（热态/时钟/负载）应双臂同向，实测 cornell 反向 +3~+10%；② 持久单调——UE bistro t100 跨 13+ 会话阶梯下行（4.24→4.87→5.1→4.16→3.96→2.96），跨 boot 不回弹（磁盘持久态——08-22 双 boot 后 4.19/4.15/4.20 续行前值）；③ 会话内紧致——G14.12 与本日三轮会话各自三轮值紧致无冷轮，位移非协议窗内冷税而是窗态整体迁移；④ 测量链口径内——receipt 三面机核绿（command_digest 位级同值 + DLSS engagement 令牌 + CSV 新鲜度），驱动/OS build/UE build/命令面全同，**非测量面异常（候选 c 排除）**；⑤ 缓存写活跃——DXCache/DDC 今日窗口写入机核；⑥ 内容面排除——契约 digest 复验全绿；⑦ 热态假说反证——同 boot 内 12:41（午后热态）3.96 已低于 09:17（晨凉）4.20，且 −25% ≈ GPU 有效吞吐 +33% 超 4070 Ti 温度 boost 物理窗（候选 a 真环境位移为主因排除，残余次级贡献不排除但不构成主因）；⑧ Rurix 臂反向旁证——同环境同会话 Rurix 臂跨四会话 3.482~3.657ms 稳定带内零位移（digest 锚位级零漂移），位移为 UE 侧独有事件。**「G14 期 UE 臂含冷缓存税」论证成立面**：G14 期（尤其 08-19~08-21 早期样本 4.2~5.1ms）承载缓存未暖税，G14.12 定盘 3.96 为当时暖态包络下限，今日 2.96~3.00 为充分暖态新基线——G14 closed 判据 0-byte 不改、G15 M-d 门消费同口径复跑 0-byte，新基线下复跑判定 = 当前真实环境面。
  - **攻坚支判定（判档向上取严）**：A 支合法动作已执行（环境事件登记 = 本条 + 当前暖态双端同会话全协议复跑第三轮）→ **未恢复 18/18**（三轮 ratio 0.8368/0.8332/0.8571 带内一致，非单次抖动）；B 支（Rurix 侧有界优化）**物理不可达定论**：通过线要求本格 prod ≤ UE 2.96~3.00ms，而地板算术 = NGX DLSS 网络黑盒 submit_wait ~2.17ms（vendor SDK 黑盒不可工程化介入；preset 对称机核 = 双端同 Default(0)→Preset K 同网络，无 preset 非对称面）+ 场景 GPU ~1.02ms（digest 锚受护面，光栅结构手术 = G14.11 已评估未立项面 + 漂移即弃/锚重收割同型程序代价）+ 不可消调度/同步 ~0.3ms ⇒ 有界优化理论下限 ~3.1~3.3ms > 2.96ms；本车道史上最好样本 3.545ms（G14.11 末格攻坚 7 样本最好）仍高于通过线需求 19.7%——有界面内无可达 ×1.00 路径，**落 C 支诚实定论**。
  - **诚实定论登记（G15-MD-F1，open-defer-G16+，承接锚字面）**：**G15-MD-F1 `fps_parity_deficit@bistro-interior/t100/dlss_sr`（perf_parity_gap）**——重判条件 = G16+ 立项窗本格双端复测（G14 M-d 同口径协议）+ UE 参照臂暖态基线程序产重标定（缓存暖态为新环境基线面，复测窗内双端同会话同协议）+ Rurix DLSS 车道 t100 档优化面重估（NGX 网络成本若经 vendor SDK 演进/驱动更新变化，或车道架构面〔跨 device 同步面/单 device 化评估〕立项则重评，触冻结面独立 Full RFC）；兜底 = 维持未达标登记不冒充（用户 2026-08-19 授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」逐字承接——G15.6b 终审面维持双未达标登记在案：画质 0/18 + 性能 17/18，本波不新增第三面）。门产登记面维持 `milestones/g14/g14_fps_gap_registry.json` gap_id `51a150cb4523e8b6`（本波第三轮复跑门自身刷新 a=333.178/b=285.566/δ=−47.612 fps + evidence_digest c9568689——只登记不拟合 RXS-0392，0-byte 判据面）。
  - **本波复跑 18 格矩阵（172719Z，跨轮中位数口径）与 ratio 对照（G14.12 soak + 本日三轮）**：

    | 格 | ratio 本波③（172719Z） | ①153359Z | ②161302Z | G14.12 soak（051754Z） |
    |---|---|---|---|---|
    | cornell-box/t50/tsr_device | 7.5685 | 7.7906 | 8.2380 | 8.2261 |
    | cornell-box/t50/dlss_sr | 3.1826 | 3.0658 | 3.1446 | 3.4031 |
    | cornell-box/t50/fsr_3_1_5 | 3.1804 | 3.2724 | 3.4659 | 3.4059 |
    | cornell-box/t67/tsr_device | 5.9920 | 6.4812 | 6.7475 | 6.3505 |
    | cornell-box/t67/dlss_sr | 2.2468 | 2.5000 | 2.9230 | 2.7493 |
    | cornell-box/t67/fsr_3_1_5 | 2.7787 | 3.0157 | 3.2051 | 2.9440 |
    | cornell-box/t100/tsr_device | 3.5759 | 4.3671 | 4.3520 | 4.0629 |
    | cornell-box/t100/dlss_sr | 2.1224 | 2.1469 | 2.2925 | 2.0701 |
    | cornell-box/t100/fsr_3_1_5 | 2.4564 | 2.4958 | 2.5971 | 2.5562 |
    | bistro-interior/t50/tsr_device | 2.2242 | 2.2068 | 2.4289 | 2.3750 |
    | bistro-interior/t50/dlss_sr | 1.6819 | 1.7999 | 1.8164 | 1.9771 |
    | bistro-interior/t50/fsr_3_1_5 | 2.4819 | 2.4713 | 2.4104 | 2.7920 |
    | bistro-interior/t67/tsr_device | 1.8508 | 1.9496 | 1.9740 | 2.3226 |
    | bistro-interior/t67/dlss_sr | 1.3602 | 1.3432 | 1.3031 | 1.5723 |
    | bistro-interior/t67/fsr_3_1_5 | 1.9775 | 1.9794 | 1.9824 | 2.2909 |
    | bistro-interior/t100/tsr_device | 1.3997 | 1.4242 | 1.4017 | 1.7987 |
    | bistro-interior/t100/dlss_sr | **0.8571 未达** | **0.8368 未达** | **0.8332 未达** | **1.0831** |
    | bistro-interior/t100/fsr_3_1_5 | 1.2666 | 1.2451 | 1.2539 | 1.6135 |

    汇总：本波③ = 17/18；本格 ratio 终值 = **0.8571**（UE=3.0014ms / Rurix=3.5018ms；逐轮比值 0.862/0.856/0.846，三轮跨波 0.786~0.862 带内一致——非噪声，当前环境下定盘面）。
  - **Rurix 臂零降级证据面（维持）**：本格 Rurix 跨会话 3.482（G14plus 波0）→ 3.657（G14.12）→ 3.540/3.562（①②）→ 3.502（③）稳定带内；G15 全期零 src 变更机核（src/ tracked diff 空 + 异己 untracked 闭集）+ Stage A digest 守护 18 格 × 3 轮 == 冻结锚位级全等（本格三轮 digest `sha256:55ea0c2b…` 位级同）——生产管线码面位级同 G14.12 定盘态。
  - **RD-045/M165 漂移监控登记（G15 复跑面）**：本波 G14 M-d 第三轮复跑面 Stage A digest 守护 18 格 × 3 轮 == 冻结锚位级全等（双跑机核绿在档）——同型 digest 漂移**零检出**（维持 open-defer 不写进全绿叙述，M-e 门面承接）。
  - **画质锚带复核 + budget 零 estimated（M-d 判据二、三分项 = 绿维持）**：G14 M-c 最新 evidence 044803Z PASS + SSIM deficit 重算 0.005389118830 ≤ 0.010779849285388998 带内；g14_budget 32 条目 + g15_budget 九条目全 measured_local 零 skip + budget_eval --strict 253 pass（本波零追加——UE 臂新基线落入既有 max 阈内〔g14.ue_benchmark.frame_ms.bistro-interior_t100 3.001 vs max 5.9406〕，预算面 = 性能上界守护，UE 臂变快不破阈，0-byte 不回写）。
  - **not-triggered / 维持 open 面**：M-e 未跑（G15.6a 面）；RD-034/039/040/041/042/043/044/045 八条 open 维持 0-byte（本波零追加零新 RD——G15-MD-F1 走本契约登记面 + G15.6a 穷举承接，不私设 RD）；G14 defer 29 行承接锚 0-byte；G14 closed 门判据/锚/协议 0-byte（门脚本与 G14.12 定盘 evidence 0-byte，本波只消费复跑件）；g14_ue_variance_samples.json 0-byte（该面 = 画质度量样本带，无 FPS 条目族——UE FPS 包络自 M-b/M-d evidence 链重建入 ③，不回写）；src/spec/conformance 0-byte；优化触改面零发生 → G14 M-c/M-e 复跑义务 not-triggered（本波零 src 变更机核）。
  - **异己并发工作树面**：本批只含 G15 车道文件（契约 §8.6 本条 + G14 M-d 门产登记行刷新 + evidence 本批三件真跑件，按文件名显式择取）；异己会话 src/ 未提交面（ktx2_read/hzb/restir/sdf_trace/smrt/ssr 等 untracked 面 + .cursor/）维持未提交、零消费、零混入（立项裁决 3）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模；本波 = 诊断 + 有界攻坚未恢复签署——诚实定论红面签署不充绿）。`Assisted-by: Kimi-K3（G15plus 延续波）`（影响范围：本契约 §8.6 本条 + milestones/g14/g14_fps_gap_registry.json〔G14 M-d 门产未达格登记行第三轮刷新，gaplib 正典形，判据面 0-byte〕+ evidence 本批真跑件〔G14 M-d 复跑第三轮 FAIL 件 172719Z + M-d 门 FAIL 件 180618Z + wave5 FAIL 件 180709Z 在档不删〕；零新门/零脚本/零 schema/零 workflow 步/零编号消费〔CI_step next_free=277 / RFC 31 / RXS 408 / RD 八条维持〕；src/spec/conformance 0-byte；G14 closed 门判据/锚/协议 0-byte；验证方式：块③逐字命令输出——G14 M-d 全协议复跑第三轮 17/18 诚实红 + M-d 门 6/12 诚实红 + 波聚合门 facts 2/6 不遮蔽红 + 诊断证据链八面（UE 包络破下限场景选择性 + 会话内紧致 + receipt 全字段同口径 + 缓存写活跃机核 + 内容面排除 + 热态反证 + Rurix 臂反向旁证）+ Rurix 臂六段遥测残余解剖 + B 支地板算术物理不可达定论 + 守卫套件全 PASS + pytest 136 passed 零回归 + 互锁 READY 维持）。

### §8.7 G15plus-II 延续波（NGX 执行路径深潜 + 有界攻坚）记录（2026-08-23 UTC）——M-d 红面格 bistro-interior/t100/dlss_sr NGX 执行路径取证定论 = **NGX Vulkan 执行 = CUDA cubin kernels 宿主面（NGXCubinVulkan；纯 Vulkan〔非 CUDA〕DLSS 执行面在 NGX 内不存在，候选 a 原范围不可行）** + 税源分解 = NGX 网络 in-stream GPU ≈1.90ms（X2 边际探针）+ 提交边界固定 ~0.10ms——**GPU-only 地板 3.02ms > 通过线 2.96ms 物理不可达终版**；有界候选 b（reactive 按需化）落地（L0 位级同一 + A/B 微幅收益 −0.048ms）；第四轮全协议复跑 17/18（本格 ratio 0.8483）维持 → **G15-MD-F1 维持 open-defer-G16+（承接锚 0-byte）**；本波零新门/零脚本/零编号消费

- **① 独立断言清单（本波零新门——消费既有门三面，未全绿如实登记，红面不遮蔽）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g14.p0.m_d.dual_end_fps_parity`（G14 closed 门同口径复跑，第四轮） | G14 契约 §4.2 M-d 逐字：checks 10/10 机器键全绿（dual_end_measurement_fresh / three_run_independence / sampling_protocol_50x3 / production_caliber_v2 / stage_a_digest_drift_guard〔18 格 × 3 轮 == 冻结锚零漂移——candidate-b 落地态下全矩阵位级复证〕/ pass_line_evaluated / quality_guard_green / gap_registry_written / budget_eval_all_pass / red_arms_effective）+ **通过线判定 = 达标 17/18**（bistro-interior/t100/dlss_sr UE=3.002ms Rurix=3.539ms ratio=0.8483 < ×1.00 未达，诚实红不冒充；逐轮比值 0.826/0.850/0.845；base_commit==HEAD e0a0676a + 本波 candidate-b 工作树触改面同窗机核 + device executed + GPU 锁纪律沿脚本既有面〔UE 臂 harness 子进程自持锁 / Rurix 臂门侧逐格持锁〕+ 三轮进程级独立 160 帧协议零缩短） | host+device | evidence/g14_m_d_dual_end_fps_parity_20260823T192244Z.json（status=fail，met 17/18） | **FAIL（17/18）** |
  | `g15.p0.m_d.perf_parity_zero_regression`（步骤 275，--wave-start 20260823T183000Z 消费本波复跑件） | 契约 §4.2 M-d 逐字六键绿维持：quality_anchor_band_recheck（SSIM deficit 重算 0.005389118830 ≤ 0.010779849285388998 带内）+ g14_budget_zero_estimated（32 条目）+ RED 四臂独立有效（ratio 篡改/旧件冒充 fresh/缺轮冒充三轮/锚漂移静默——函数面注入全检出）；六键红维持：g14_m_d_rerun_fresh_pass（复跑件本体 status=fail，freshness 链 PASS 面不成立诚实红）+ 逐格四面 + comparison_vs_g14_12_rerun（诚实红不充绿） | host+device（G14 M-d 复跑 = 本波子进程双臂真跑面消费） | evidence/g15_m_d_perf_parity_zero_regression_20260823T195859Z.json（6/12） | **FAIL（6/12）** |
  | `g15.wave.5.exit`（步骤 276，波聚合门只读汇总复跑） | 六 facts 不遮蔽：④绿（画质锚带复核重算绿）⑤绿（g14_budget 32 + g15_budget 九条目 measured_local 零 estimated + budget_eval 全 PASS）①②③⑥ 红（M-d 最新 evidence 非 PASS / 复跑真跑面 18 格全达不成立 / 锚覆盖面红面消费跳过〔复跑件本体 digest 守护绿在档不冒充〕/ 工作树闭集越界 = g14_fps_gap_registry.json 门产登记行 + **本波 candidate-b src 触改面〔src/rurix-rt/src/vendor_upscale.rs〕**——与 M-d 红同源不遮蔽 + 触改面如实登记） | host 只读（不重跑子门） | evidence/g15_wave5_exit_20260823T195904Z.json（facts 2/6） | **FAIL（facts 2/6）** |

- **② 波聚合门实测输出**：`py -3 ci/g15_wave5_exit_check.py --gate g15.wave.5.exit` → **VERDICT = FAIL，exit=1**（required_gates M-d FAIL + 六 facts 逐行打印不遮蔽：④⑤ 绿、①②③⑥ 红——红树下聚合不充绿字面维持）；`py -3 ci/g15_perf_parity_guard_smoke.py --gate g15.p0.m_d.perf_parity_zero_regression --wave-start 20260823T183000Z` → **VERDICT=FAIL checks=6/12**（红绿键分布与 §8.5/§8.6 定盘面逐字一致）。两门脚本本波 0-byte（selftest 红绿留痕在 §8.5 档，本波只消费不改门）。
- **③ 验收命令逐字输出（2026-08-23 UTC 真跑留痕，仓库根目录）**：
  - **诊断臂（阶段 1，NGX 执行路径取证——零门消费零编号）**：
    - **执行后端定论 = NGXCubinVulkan**（`RURIX_VENDOR_TIMING=1` 六段 + SL verbose log 回调全量捕获真跑留痕 = `.tmp/g15plus2/ngx_diag_rurix_bistro_t100.log` 141372 字节）：[sl] 日志逐字 `NGXCubinVulkan::Init:191 Enabling texmode_raw` + `NGXCubinKernelMap::InitCubins:45 Loading NGXCubin kernels` + `DLSSCubinKernelMap::InitCubins:311 Setting DLAA Cubins / DLTSS Engine Cubins / DLTSS NW Cubins / DLTSS NW E5M3_SKIP Cubins`——NGX 在 Vulkan 臂的 DLSS 执行 = **CUDA cubin kernels 经 VK_NVX_binary_import 注入命令流**（`vkCreateCuModuleNVX` 双证在案：G14.11 陷阱登记「validation × NVX 首帧崩于 vkCreateCuModuleNVX」+ validation 豁免白名单 VUID-VkCuModuleCreateInfoNVX-pNext-pNext 结构性归属 NGX 内部——本模块从不调该符号）。**纯 Vulkan（非 CUDA）DLSS 执行面在 NGX 内不存在——候选 a 原范围（SDK 参数/特性位/资源形态转纯 Vulkan 执行面）不可行定论**。
    - **网络/preset/模型库对称面**：NgxDltss::FillCreationParams DLAA→title default **Preset K**（Quality/Balanced 同 K、Perf→M、UltraPerf→L——与 UE 310.6.0 日志映射逐字一致）；`NGXDLAA::DLSS_GetOptimalSettings` + `NGXDLAA::CreateDlssInstance` InDynamicMaxSetDims 1920×1080 / Out 1920×1080（in==out DLAA 实例 = UE NGXPerfQuality=DLAA(5) SrcRect=DestRect=1920×1080 同路径）；模型库共享 `C:\ProgramData\NVIDIA\NGX\models`（OTA 同一）。
    - **UE 侧对拍**（`G10RefRender.log` + `bench_receipt_r1.json` dlss_log_tokens）：**`NGXCubinD3D12::Init:133` Enabling texmode_raw** + 同族 cubin kernel map（DLAA/DLTSS NW/E5M3_SKIP 逐字同名）+ 同 Preset K 映射 + `PaddedWindowNetwork::CreatePaddedWindowNetwork` Container0Tensor 10 MiB allocated / Container1Tensor 12 MiB aliased / Container2Tensor 8 MiB aliased——**UE 臂同为 cubin（CUDA）执行，宿主 API = D3D12；差异面 = cubin 宿主 API（NGXCubinD3D12 vs NGXCubinVulkan）+ NGX minor 版本（UE Build v310.6.0 CL 37642667 vs Rurix nvngx_dlss.dll 310.5.2——网络实例化形态差：PaddedWindowNetwork 容器族 vs encoder 族 + InternalHistoryA/B 35.6MB×2 全分辨率历史 + OutputHistRes 17.8MB）**。版本差 = vendor SDK 演进面（G15-MD-F1 承接锚已命名重判触发面）；DLL provenance 登记面（g13_vendor_sdk_registry + DllProvenance sha256 逐件重算比对机核）使本波不可静默换版——换版 = G16+ 程序面。
    - **税源分解（X2 边际探针——同 cmd 内第二次 slEvaluateFeature，env `RURIX_G15_DLSS_EVAL_X2` 探针轮专用，诊断完成即撤除 0-byte）**：submit_wait 单 evaluate 中位 2.002 / 均值 2.223（n=150 弃 warmup 20）→ 双 evaluate 中位 3.903 / 均值 4.006——**边际 ≈1.90（中位）/ 1.78（均值）ms = NGX 网络 in-stream GPU 执行成本（无第二 submit/waitIdle 边界）**；**提交边界固定 ≈0.10ms**。六段稳态：staging 0.000 / sl_book 0.012 / record 0.016 / evaluate 0.184（CPU 录制）/ submit_wait 2.298（mean）。
    - **分辨率缩放互证**：cornell-box/t100（512×512 = 像素 ÷7.91）同协议真跑 submit_wait 中位 0.419 + scene_gpu 0.258 + digest==冻结锚——NGX 成本 ~像素缩放 + launch 密度下限税一致面，排除「1080p 独有固定税」假说。
    - **候选 c 核实（输入形态隐式转换税）= 无可动面**：RGBA16F color（pack SPV 硬件 RTE 直写，与 G14.10f host f16 位面逐位同）+ R32F depth + RG32F mv 经 OPAQUE_WIN32 image 共享 NGX 直采样（G14.10b 格式容忍探明 + G14.12 imageType 修正后零 copy 面维持——staging=0.000 实测）；NGX 日志内部分配均实例创建期一次性，无逐帧转换面证据——**接口面零隐式转换税**（NGX 内部面不可分解不可介入）。
    - **候选 d 核实（evaluate 参数面 DLAA 形态最优集）= 无可动面**：NGXDLAA 路由在案（MaxPerf(0) + in==out → NGXDLAA 实例）+ Do Sharpening 0 + use_auto_exposure 0（UE 臂 bUseAutoExposure=1——我方参数面已省其 auto-exposure 计算）+ alpha upscaling 0 + HDR 1 + pre_exposure 供给 + preset 对称——参数集已最优。
  - **候选 b 落地（阶段 2，有界优化唯一落地项 = reactive 按需化，单变量隔离）**：
    - 改动面：`src/rurix-rt/src/vendor_upscale.rs::upscale_resident_external` 单函数——`reactive=None`（生产驻留车道恒 None）不再上传零 mask、不再附带 reactive tag（SL required tag 注册面仅 kBufferTypeDepth/MotionVectors/ScalingInputColor/ScalingOutputColor 四项，SL verbose 日志逐字在案；NGX 缺省 = 零 mask 语义）；`Some(..)` 帧维持原 R8 pack staging + 上传 + tag 全链（该形态语义 0-byte）；tag 集由定长 6 指针改 Vec 按计数装配（4 或 5 + viewport）；unsafe 块 SAFETY 注释维持 + unsafe-audit U58 G15plus-II 扩注登记；UpscaleBackend trait 签名/temporal 底座/RXS-0357 面 0-byte。
    - **L0 位级探针（门禁，漂移即弃）**：bistro-interior t100/t50 + cornell-box t67 末帧 digest == G14.12 冻结锚**三格全 HIT** + cornell t100 同窗 HIT（探针驱动 `.tmp/g15plus2/probe_bench.py` + `probe_results.jsonl` 逐轮留痕）——位级同一实证，无漂移不弃。
    - **A/B measured（同窗 no-timing 三轮单变量）**：baseline 3.831/3.801/3.747（中位 3.801）→ candidate 3.730/3.753/3.787（中位 3.753）——**中位 −0.048ms（≈1.3%，方向一致微幅收益；判档向上取严 = 收益存在但不构成跨线面，地板算术主导面不变）**。
    - 诊断探针撤除登记：X2 env 探针完成分解后即撤（src 面恢复），不留探针脚手架。
  - **攻坚复跑（阶段 3，第四轮全协议复跑，GPU 锁纪律沿脚本既有面）**：`py -3 ci/g14_dual_end_fps_parity_smoke.py --gate g14.p0.m_d.dual_end_fps_parity` → **VERDICT=FAIL checks=10/10 pass_line=未达标（达标 17/18）**（evidence 192244Z；bistro-interior/t100/dlss_sr UE=3.002ms Rurix=3.539ms **ratio=0.8483 未达标**；生产口径 v2 机核=True + Stage A digest 守护 18 格 × 3 轮 == 冻结锚=True——candidate-b 落地态下全矩阵位级零漂移复证；逐格输出见 ④ 矩阵）；
    - `py -3 ci/g15_perf_parity_guard_smoke.py --gate g15.p0.m_d.perf_parity_zero_regression --wave-start 20260823T183000Z` → **VERDICT=FAIL checks=6/12**（evidence 195859Z）；
    - `py -3 ci/g15_wave5_exit_check.py --gate g15.wave.5.exit` → **VERDICT=FAIL exit=1**（evidence 195904Z，facts 2/6 不遮蔽）。
  - 守卫套件全 PASS：`py -3 ci/check_structure.py` → PASS（11 dirs, 6 files）；`py -3 ci/check_schemas.py` → PASS（本波零追加）；`py -3 ci/check_number_ledger.py` → PASS（off_tree grx advisory 一条沿既有不阻断）；`py -3 ci/budget_eval.py --strict` → PASS（253 pass/0 skip——candidate-b 微幅收益落入既有 max 阈内〔g14.pipeline_perf.frame_ms.bistro-interior_t100_dlss_sr 3.539 vs max 6.293〕，预算面 0-byte 不回写）；`py -3 -m pytest tests/ -q` → **136 passed** 零回归；`py -3 ci/g15_interlock_check.py --require-ready` → **READY 维持**（workflow 实测末号 276 == ledger on_tree_max 276、next_free 277——本波零编号消费一致面）。
  - cargo test 面（HEAD 基线核实先行 + candidate-b 落地态复跑，同为 `cargo test -p rurix-rt --features vendor-upscale --no-fail-fast`）：**214 passed / 3 failed**——2 败 = HEAD 基线既有留痕面（m103_descriptor_buffer_ffi_layout_anchors 常量锚〔left 1000316002 vs right 1000316012〕+ binding_supply_chain_no_external_vulkan_crate〔`vulkan = []` 空依赖集断言 vs 现状 `vulkan = ["dep:rurix-pkg"]`〕，G14.9 登记同两面）+ 1 败 = command_build zero_readback_full_chain 进程级计数器并行污染（delta 32 == 同文件 L486 兄弟测试 `readback_counter_record(32)` 逐字——测试设计既有交互面，非本波引入，与本波触改面无因果关系〔不同模块、零计数器消费〕）；candidate-b 落地态复跑同 214 passed / 同 3 failed——vendor_upscale 测试面零新败。
- **④ 门序 / 偏差 / not-triggered / no-go 登记面摘要**：
  - **门序**：本波 = §8.5/§8.6 红面的第二延续波（NGX 执行路径深潜 + 有界攻坚），不改波次状态机——**G-G15-6 维持未通过如实定盘**（第四轮复跑仍 17/18）；G15.6a 穷举面承接本波终态（G15-MD-F1 维持）。零新门/零脚本/零 schema/零 workflow 步 → **零编号消费**（CI_step next_free=277 维持，ledger 0-byte；RFC next_free=31 / RXS next_free=408 / RD 八条 0-byte 维持）。
  - **税源判定（候选 a 求证结论，假说修正如实登记）**：「Vulkan↔CUDA interop 同步税」假说经实测修正——税的主项**不是**提交边界同步常量（实测仅 ~0.10ms），而是 **NGX 网络 in-stream GPU 执行本身 ≈1.90ms**（X2 边际探针分解面）；CUDA interop 是 NGX Vulkan 执行的**唯一形态**（非回退——UE 臂同为 cubin 执行，宿主 D3D12）。双臂黑盒差（UE 全帧 GPUTime 2.266 含场景 vs 我方 NGX 单项 in-stream 1.90）的归因三面：① 宿主 API 差（D3D12 vs Vulkan 的 cubin 发射/同步路径——G14.11 FSR 臂 D3D12 反向共享驻留先例 = 工程可行形态，但 = 车道架构面）；② NGX 版本差（310.6.0 PaddedWindowNetwork vs 310.5.2 encoder/InternalHistory 实例化形态——vendor SDK 演进面）；③ UE CSV GPUTime 对 CUDA 引擎工作的口径面（测量链口径内事件沿 §8.6 定论）。**三面均在本波有界界面外**——①② = G15-MD-F1 承接锚已命名 G16+ 重评触发面（车道架构面触冻结面独立 Full RFC；vendor SDK 演进面 provenance 登记同步）。
  - **地板算术终版**：NGX in-stream 1.90 + 提交固定 ~0.10 + scene_gpu 1.02（digest 锚受护面，光栅结构手术 = 漂移即弃/锚重收割同型程序代价面）= **GPU-only 地板 3.02ms > 通过线 2.96ms**——CPU 侧残余（evaluate 录制 0.15 + record/book 0.03 + 帧循环 ~0.35）即便全归零亦不跨线；候选 b 落地 −0.048ms 不改变主导面。**物理不可达终版定论维持**（较 §8.6 地板算术 sharpen：黑盒内部 1.90+0.10 分解替代原 2.17 笼统面）。
  - **有界候选台账（逐候选 L0 探针门禁已执行，漂移即弃本轮零触发）**：a) NGX 纯 Vulkan 执行路径切换 = **不可行**（NGX Vulkan = cubin interop by design，SDK 面无开关在案）；b) reactive 按需化 = **落地**（位级同一 + 微幅收益）；c) DLSS 输入资源形态 = **无可动面**（零隐式转换实测核实）；d) evaluate 调用参数面 = **无可动面**（DLAA 形态最优集实测核实）。
  - **诚实定论登记（G15-MD-F1 维持 open-defer-G16+，承接锚 0-byte）**：**G15-MD-F1 `fps_parity_deficit@bistro-interior/t100/dlss_sr`（perf_parity_gap）**——重判条件 = G16+ 立项窗本格双端复测（G14 M-d 同口径协议）+ UE 参照臂暖态基线程序产重标定（缓存暖态为新环境基线面，复测窗内双端同会话同协议）+ Rurix DLSS 车道 t100 档优化面重估（NGX 网络成本若经 vendor SDK 演进/驱动更新变化，或车道架构面〔跨 device 同步面/单 device 化评估〕立项则重评，触冻结面独立 Full RFC——**本波 sharpen 补注（承接锚字面 0-byte 不动，本注为新增登记面）**：① NGX 版本演进面 = nvngx_dlss.dll 310.5.2→310.6.0+ 的 PaddedWindowNetwork 实例化形态对齐评估〔DllProvenance/g13_vendor_sdk_registry provenance 机核同步换版程序面〕；② 车道架构面 = D3D12 宿主 NGX（NGXCubinD3D12，UE 同款宿主；G14.11 FSR 臂 D3D12 反向共享驻留为工程先例）；③ 本波实测约束 = NGX in-stream 1.90ms + 提交固定 0.10ms 分解字面，G16+ 重评以本分解为输入面）；兜底 = 维持未达标登记不冒充（用户 2026-08-19 授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」逐字承接——G15.6b 终审面维持双未达标登记在案：画质 0/18 + 性能 17/18，本波不新增第三面）。门产登记面维持 `milestones/g14/g14_fps_gap_registry.json` gap_id `51a150cb4523e8b6`（第四轮复跑门自身刷新 a=333.084/b=282.561/δ=−50.523 fps + evidence_digest da23abda——只登记不拟合 RXS-0392，0-byte 判据面）。
  - **本波复跑 18 格矩阵（192244Z，跨轮中位数口径）与 ratio 对照（G14.12 soak + 本日四轮）**：

    | 格 | ratio 本波④（192244Z） | ③172719Z | ①153359Z | ②161302Z | G14.12 soak（051754Z） |
    |---|---|---|---|---|---|
    | cornell-box/t50/tsr_device | 7.9330 | 7.5685 | 7.7906 | 8.2380 | 8.2261 |
    | cornell-box/t50/dlss_sr | 3.0509 | 3.1826 | 3.0658 | 3.1446 | 3.4031 |
    | cornell-box/t50/fsr_3_1_5 | 3.3421 | 3.1804 | 3.2724 | 3.4659 | 3.4059 |
    | cornell-box/t67/tsr_device | 6.1634 | 5.9920 | 6.4812 | 6.7475 | 6.3505 |
    | cornell-box/t67/dlss_sr | 2.7130 | 2.2468 | 2.5000 | 2.9230 | 2.7493 |
    | cornell-box/t67/fsr_3_1_5 | 2.8512 | 2.7787 | 3.0157 | 3.2051 | 2.9440 |
    | cornell-box/t100/tsr_device | 4.2862 | 3.5759 | 4.3671 | 4.3520 | 4.0629 |
    | cornell-box/t100/dlss_sr | 2.0661 | 2.1224 | 2.1469 | 2.2925 | 2.0701 |
    | cornell-box/t100/fsr_3_1_5 | 2.5260 | 2.4564 | 2.4958 | 2.5971 | 2.5562 |
    | bistro-interior/t50/tsr_device | 2.2564 | 2.2242 | 2.2068 | 2.4289 | 2.3750 |
    | bistro-interior/t50/dlss_sr | 1.7905 | 1.6819 | 1.7999 | 1.8164 | 1.9771 |
    | bistro-interior/t50/fsr_3_1_5 | 2.4495 | 2.4819 | 2.4713 | 2.4104 | 2.7920 |
    | bistro-interior/t67/tsr_device | 1.8473 | 1.8508 | 1.9496 | 1.9740 | 2.3226 |
    | bistro-interior/t67/dlss_sr | 1.3518 | 1.3602 | 1.3432 | 1.3031 | 1.5723 |
    | bistro-interior/t67/fsr_3_1_5 | 1.9652 | 1.9775 | 1.9794 | 1.9824 | 2.2909 |
    | bistro-interior/t100/tsr_device | 1.4323 | 1.3997 | 1.4242 | 1.4017 | 1.7987 |
    | bistro-interior/t100/dlss_sr | **0.8483 未达** | **0.8571 未达** | **0.8368 未达** | **0.8332 未达** | **1.0831** |
    | bistro-interior/t100/fsr_3_1_5 | 1.2712 | 1.2666 | 1.2451 | 1.2539 | 1.6135 |

    汇总：本波④ = 17/18；本格 ratio 终值 = **0.8483**（UE=3.0023ms / Rurix=3.5391ms；逐轮比值 0.826/0.850/0.845，四轮跨波 0.786~0.862 带内一致——非噪声，当前环境下定盘面；candidate-b −0.048ms 微幅收益在跨会话噪声带内，不改变判定面）。
  - **Rurix 臂零降级证据面（candidate-b 落地态措辞）**：本格 Rurix 跨会话 3.482（G14plus 波0 定盘）→ 3.657（G14.12）→ 3.540/3.562（①②）→ 3.502（③）→ 3.539（④ candidate-b 落地态）稳定带内；Stage A digest 守护 18 格 × 3 轮 == 冻结锚位级全等（本格四轮 digest `sha256:55ea0c2b…` 位级同）——**内容面位级同 G14.12 定盘态**；本波 src 触改面 = candidate-b 单函数面（vendor_upscale.rs 非冻结面加性演进，触改面登记 G15.6a M-e 触改面真跑抽检承接；UpscaleBackend trait 签名/temporal 底座/RXS-0357 面 0-byte）。
  - **RD-045/M165 漂移监控登记（G15 复跑面）**：本波 G14 M-d 第四轮复跑面 Stage A digest 守护 18 格 × 3 轮 == 冻结锚位级全等（双跑机核绿在档）——同型 digest 漂移**零检出**（维持 open-defer 不写进全绿叙述，M-e 门面承接）。
  - **画质锚带复核 + budget 零 estimated（M-d 判据二、三分项 = 绿维持）**：G14 M-c 最新 evidence 044803Z PASS + 在树 converged.exr 双件 SSIM deficit 重算 **0.005389118830 ≤ 0.010779849285388998** 带内；g14_budget 32 条目 + g15_budget 九条目全 measured_local 零 skip + budget_eval --strict 253 pass（本波零追加——candidate-b 收益落入既有 max 阈内，0-byte 不回写）。
  - **not-triggered / 维持 open 面**：M-e 未跑（G15.6a 面——本波 candidate-b 触改面入其「触改面真跑抽检零降级」承接登记）；RD-034/039/040/041/042/043/044/045 八条 open 维持 0-byte（本波零追加零新 RD——G15-MD-F1 走本契约登记面 + G15.6a 穷举承接，不私设 RD）；G14 defer 29 行承接锚 0-byte；G14 closed 门判据/锚/协议 0-byte（门脚本与 G14.12 定盘 evidence 0-byte，本波只消费复跑件）；g14_ue_variance_samples.json 0-byte；spec/conformance 0-byte；UpscaleBackend trait 签名/temporal 底座/RXS-0357 面 0-byte。
  - **异己并发工作树面**：本批只含 G15 车道文件（契约 §8.7 本条 + src/rurix-rt/src/vendor_upscale.rs candidate-b 触改 + unsafe-audit/rurix-rt.md U58 扩注 + G14 M-d 门产登记行第四轮刷新 + evidence 本批三件真跑件，按文件名显式择取）；异己会话 src/ 未提交面（ktx2_read/hzb/restir/sdf_trace/smrt/ssr 等 untracked 面 + .cursor/）维持未提交、零消费、零混入（立项裁决 3）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模；本波 = NGX 执行路径深潜 + 有界攻坚未恢复签署——诚实定论红面签署不充绿）。`Assisted-by: Kimi-K3（G15plus-II NGX 执行路径深潜延续波）`（影响范围：本契约 §8.7 本条 + src/rurix-rt/src/vendor_upscale.rs〔candidate-b reactive 按需化单函数面——非冻结面加性演进，unsafe 块 SAFETY 注释维持〕+ unsafe-audit/rurix-rt.md〔U58 G15plus-II 扩注登记〕+ milestones/g14/g14_fps_gap_registry.json〔G14 M-d 门产未达格登记行第四轮刷新，gaplib 正典形，判据面 0-byte〕+ evidence 本批真跑件〔G14 M-d 复跑第四轮 FAIL 件 192244Z + M-d 门 FAIL 件 195859Z + wave5 FAIL 件 195904Z 在档不删〕；零新门/零脚本/零 schema/零 workflow 步/零编号消费〔CI_step next_free=277 / RFC 31 / RXS 408 / RD 八条维持〕；spec/conformance 0-byte；UpscaleBackend trait 签名/temporal 底座/RXS-0357 面 0-byte；G14 closed 门判据/锚/协议 0-byte；验证方式：块③逐字命令输出——G14 M-d 全协议复跑第四轮 17/18 诚实红 + M-d 门 6/12 诚实红 + 波聚合门 facts 2/6 不遮蔽红 + NGX 执行路径取证证据链七面（NGXCubinVulkan 日志面 + vkCreateCuModuleNVX 双证在案 + UE NGXCubinD3D12 对拍同族 cubin + X2 边际分解 1.90+0.10 + 分辨率缩放互证 + 双臂 preset/网络/模型库对称 + NGX 版本差实例化形态差）+ 候选 b L0 位级探针三格 HIT + A/B 三轮 −0.048ms + 地板算术终版 3.02>2.96 物理不可达 + 候选 c/d 无可动面核实 + 守卫套件全 PASS + pytest 136 passed 零回归 + cargo test 214 passed（基线 2 败 + 1 既有并行污染面除外）+ 互锁 READY 维持）。

### §8.8 G15.6a P2 穷举 + M-e 回归门 + stabilization soak 验收记录（2026-08-23）——G-G15-7/G-G15-8 字面兑现：P2 穷举决策门（g15.wave.6a.decisions，步骤 277，46 facts 全绿 VERDICT=PASS）+ M-e 回归门 + 漂移监控（g15.p0.m_e.regression_drift_guard，步骤 278，10/10 checks 全绿 VERDICT=PASS）+ 稳定门 soak（g15.wave.6a.soak，步骤 279，六 facts + 七 checks 全绿 VERDICT=PASS——59 迭代 1852.5s ≥1800s 零失败）

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g15.wave.6a.decisions`（步骤 277） | 契约 G-G15-7 逐字：**G15 期全部 P2/留档/未触发分项逐条裁决零空行**（`milestones/g15/G15_P2_DECISIONS.md` v1.0 首建——40 行冻结闭集全等：§1 G15 候选表 35 行终态裁决〔go 行兑现完结核验引 evidence 真跑件转 closed-go 留痕沿 G15.1 §5.5 范式——G13-N8/N9+G12-N12 → M-a/M-b 兑现、G11-N3 → M-a/M-c 兑现、G11-N8/N9+G12-N10 → M-b 材质链未命中登记兑现、G12-N13 → M-e 监控臂兑现、G15-N1~N6 → 各门兑现；14 行 defer-to-G16+ 维持字面承接锚 0-byte〕+ §2 期内新增 5 行〔G15-MA-F1 closed-caliber-registered 终态 / G15-MC-F1 UE 参照臂黑帧 open-defer-G16+ / G15-MD-F1 DLSS t100 格 open-defer-G16+ 承接锚字面 + G15plus 双延续波留痕行〕）+ 裁决枚举合法（go/closed-go/no-go/defer-to-G16+/strategic_override）+ 零空行（全列非空）+ 承接锚「重判条件+兜底」字面 + defer 行 G16+ 重评窗 + closed-go/go 行 evidence 义务 + 四横向机核（MAP §1 五 P0 互斥 / deferred 八条 open 0-byte+零新 RD+vs G15.0 base 只追加〔RD-045 零检出维持 open 不关闭字面〕/ G15.1 候选表 35 行迁移对账 / 差距登记表对账〔G15 处置/闭环双表 20 行 tally 0/4/16 + g14 帧率表 1 行 == 最新 G14 M-d unmet_count + g13/g12 三表终态 0-byte + findings 三行登记字面〕）+ RED 四臂独立有效（缺行/defer 缺 G16+ 锚/非法枚举/closed-go 缺 evidence） | host 只读（文档与 registry 核验，不代绿实现门） | evidence/g15_p2_decisions_20260823T204316Z.json（46/46） | PASS |
  | `g15.p0.m_e.regression_drift_guard`（步骤 278） | 契约 §4.2 M-e 逐字：**既有 84 门最新 evidence 全绿只读汇总不遮蔽**（G9 34 + G10 14 + G11 14 + G12 9 + G13 5 + G14 8——84/84 合格；**G14 M-d 诚实红门面特判** = checks 全绿 ∧ status==fail ∧ unmet_count == g14_fps_gap_registry 行数〔1 行 gap_id 51a150cb4523e8b6〕，红面维持红登记不遮蔽不代绿——回归门判据 = 零降级机核）+ **触改面真跑抽检零降级**（G15 期 src 触改面 = G15plus-II vendor_upscale.rs candidate-b 单文件：`cargo test -p rurix-rt --features vendor-upscale --no-fail-fast` 子进程真跑 233 passed / 2 failed 全基线〔m103 常量锚 + binding_supply_chain 空依赖集断言——G14.9 登记双面；zero_readback_full_chain 并行污染面本轮未现不强制〕零新败 + G14 M-c 最新件复核画质锚带重算 0.005389118830 ≤ 0.010779849285388998 带内 + src 触改闭集机核〔committed ⊆ candidate-b 单文件，工作树 tracked 空 + untracked ⊆ 异己六件〕）+ **RD-045/M165 漂移监控登记**（G15 全期复跑面确定性键族 25 键全真——M-a 上游三门同口径复跑 + G14 生产面 M-c/M-d/M-f/M-g + M-c 生产管线 36 格 + M-d 四轮复跑 Stage A digest 守护 18 格 × 3 轮 == 冻结锚，同型 digest 漂移**零检出**字面入 evidence + FAIL 件 0-byte 在档 + flip-trace 诊断臂在树 + RD-045 open 维持）+ **G5~G14 closed 判据 0-byte**（vs G15.0 ref f061487e committed 闭集 = {g14_budget.json, g14_ue_variance_samples.json, g14_fps_gap_registry.json} 授权三面，工作树闭集空）+ temporal 底座/UpscaleBackend trait 面 0-byte + RED 四臂独立有效（degraded-gate/aggregate-masking/drift-unregistered/honest-red-masquerade） | host+device（cargo test 子进程真跑面消费） | evidence/g15_m_e_regression_drift_guard_20260823T204851Z.json（10/10） | PASS |
  | `g15.wave.6a.soak`（步骤 279） | 契约 G-G15-8 逐字：四腿——**①全量回归**（G15 5 P0 = M-a~M-e 逐门最新 evidence 核验〔wel 口径 + 顶层 status==pass 字面〕+ wave2/3/4/5 exit + wave6a decisions 五聚合/决策门核验 10/10 合格——**M-d/wave5 诚实红门面特判同 G14 closeout 先例字面**：G15 M-d status==fail ∧ 绿键 6 全真 ∧ 红键 6 全假闭集 ∧ 消费的 G14 M-d 复跑件 checks 全绿 ∧ status==fail ∧ unmet==帧率表行数 ∧ G15-MD-F1 承接锚在案；wave5 聚合 FAIL 红面 = M-d 行 FAIL 镜像 + facts ④⑤ 绿 + 红 facts ⊆ M-d 红同源闭集——红面如实登记不充绿不充降级，soak 门 VERDICT 语义 = 回归腿零降级 + 红面登记完备）+ **②画质/帧率链路连续复跑 soak ≥1800s**（g14_3_pipeline_perf --bench cornell-box/bistro-interior t67 × 三后端六组合轮转 canonical 协议 160 帧 warmup 10——**59 迭代 1852.5s 零失败零 sleep**，frames=9440，active==seconds==outer 1852.5s 交叉核验；receipt last_frame_digest == g14_3_stage_a_digest_anchor 冻结锚逐迭代位级复现 59/59——RD-045 同型漂移**零检出**；登记表装配腿 59 次幂等复核）+ **③budget_eval --strict 非空零 estimated/skip**（253 pass/0 skip）+ **④日期锚 + G5~G14 既有判据 0-byte**（同 M-e 闭集字面）+ RED 判读臂独立有效（selftest 1 GREEN + 8 RED 判读面 + 2 诚实红评定面臂） | host+device（bench 腿 device 真跑 gpu_device_lock 串行） | evidence/g15_stabilization_soak_20260823T205354Z.json（六 facts + 七 checks 全绿） | PASS |

- **② 验收命令逐字输出（2026-08-23 真跑留痕，仓库根目录）**：
  - `py -3 ci/g15_p2_decisions_check.py --gate g15.wave.6a.decisions` → VERDICT=PASS（46 facts 全绿，evidence 204316Z）；`--selftest` → ALL PASS（真表 40 行绿 + 合成全表绿 + 4 RED 臂〔缺行/defer 缺 G16+ 锚/非法枚举/closed-go 缺 evidence〕全检出）。
  - `py -3 ci/g15_regression_drift_guard_smoke.py --gate g15.p0.m_e.regression_drift_guard` → VERDICT=PASS checks=10/10（evidence 204851Z——84/84 合格 + 诚实红面登记 + cargo test 233 passed/2 基线败零新败 + 画质锚带重算带内 + 漂移零检出 + closed 0-byte + RED 四臂）；`--selftest` → PASS（schema 闭集 + 4 RED + 2 GREEN 函数面臂）。
  - `py -3 ci/g15_stabilization_soak.py --gate g15.wave.6a.soak` → VERDICT=PASS（evidence 205354Z——回归腿 10/10 + soak 59 迭代 1852.5s 零失败 + budget 253 pass + 0-byte + 日期锚）；`--verify-latest` → PASS（checks 7 键全绿 + soak 判读面复核过）；`--selftest` → PASS（1 GREEN + 8 RED 判读面 + 2 诚实红评定面臂）。
  - 守卫套件全 PASS：`py -3 ci/check_structure.py` → PASS；`py -3 ci/check_schemas.py` → PASS（本批新增 g15_p2_decisions_ / g15_m_e_regression_drift_guard_ / g15_stabilization_soak_ / g15_wave6b_closeout_ 四前缀 × load/validator/路由十二处纯追加，io.open 补丁法 + count==1 断言 + Select-String 核实——与既有全族互不包含）；`py -3 ci/check_number_ledger.py` → PASS（CI_step on_tree_max 280/next_free 281 校准后实测，off_tree grx advisory 一条沿既有不阻断）；`py -3 ci/budget_eval.py --strict` → PASS（253 pass/0 skip——g15 九条目维持，本波零追加）；`py -3 -m pytest tests/ -q` → 136 passed 零回归；`py -3 ci/g15_interlock_check.py --require-ready` → READY 维持（workflow 实测末号 280 == ledger on_tree_max 280 一致面）。
  - 起草期 FAIL 轨迹：零（四门首跑全绿——G15.1~G15.5 五波范式继承面齐备，本波零修订留痕；接线补丁三锚一次命中）。
- **③ 门序 / 偏差 / not-triggered / no-go 登记面摘要**：
  - **门序**：G-G15-6 未通过如实定盘（§8.5~§8.7）→ 本波 G-G15-7（P2 穷举）+ G-G15-8（M-e 回归门 + soak）兑现；数字步骤 277/278/279/280 按落盘前实测 actual next_free 顺位领取（ledger v1.158 校准同批：CI_step 276→280/next_free 281）；G15.6b close-out 开工面开放。
  - **P2 行数与三态分布（G-G15-7 定盘字面）**：穷举闭集 **40 行**（§1 35 + §2 5）零空行——**closed-go 24 行**（§1 21 = 7 G14plus 留痕〔G14-N8~N14〕+ 8 G14 defer 承接兑现〔G13-N8/G13-N9/G12-N12 → M-a/M-b；G11-N3 → M-a/M-c；G11-N8/G11-N9/G12-N10 → M-b 材质链评估 not-triggered 未命中如实登记；G12-N13 → M-e 监控臂〕+ 6 G15 新增兑现〔G15-N1~N6〕+ §2 3 = G15-MA-F1 M-b 定论 + G15PLUS-W1/W2 双延续波留痕）+ **defer-to-G16+ 16 行**（§1 14 = M61/M52/M100-high/SAFE-GPU/M127/M98-l4/M114-strand/M118-hdr-cal/M125-adopt3/G10-N6/G10-N8/G10-N17/G11-N5/G13-N7 承接锚字面 0-byte 重评窗顺延 + §2 2 = G15-MC-F1/G15-MD-F1 承接锚字面）+ go 0 / no-go 0 / strategic_override 0 + 维持 open 8 行（§3 RD 映射不重复计入三值枚举——RD-045 零检出维持 open 不关闭字面）。no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述（G-G15-7 字面兑现）。
  - **M-e VERDICT（G-G15-8 第一分项）**：PASS 10/10——84 门汇总零降级（84/84 合格；登记红面恰 {G14 M-d} 诚实红维持红登记不遮蔽不代绿——回归门判据 = 零降级机核字面兑现）+ 触改面抽检零降级（cargo test 233 passed/2 基线败零新败 + 画质锚带重算 0.005389118830 带内 + src 触改闭集 candidate-b 单文件授权面）+ RD-045/M165 漂移监控零检出字面（25 键确定性键族全真）+ G5~G14 closed 判据 0-byte。
  - **soak 实测参数与 VERDICT（G-G15-8 第二分项）**：**59 迭代 / 1852.5s 墙钟（≥1800s）/ 9440 帧（59×160 canonical 协议）/ 59 次登记表装配**——active_chain_seconds=1852.5 == seconds == outer_wall 交叉核验，sleep_seconds 恒 0，failures=0；六组合轮转 last_frame digest == 冻结锚逐迭代位级复现 59/59（**RD-045 零检出**——candidate-b 落地态位级同一再证）；VERDICT=PASS（回归腿零降级 + 红面登记完备 + budget 253 pass/0 skip + 0-byte + 日期锚 20260823）。
  - **诚实红特判字面（G14 closeout 先例同型兑现面）**：M-d/wave5 红面 = 未达标如实登记——不充绿（G15 M-d 门自身 evidence status=fail 0-byte 维持、wave5 聚合 FAIL 0-byte 维持）亦不充降级（checks/结构面合格 = 判据机核面零劣化；soak/closeout 消费面特判合格行置 PASS 并 detail 承载诚实红字面）。本波零红面静默翻绿、零判据/锚/登记表 0-byte 面改写。
  - **双未达标维持登记（如实登记不冒充）**：商用收口 0/18（§8.4 定盘）+ 性能 17/18 单格环境事件面（§8.5~§8.7 定盘——bistro-interior/t100/dlss_sr ratio 0.8332~0.8571 跨四轮一致，UE 参照臂缓存暖态跨会话位移 + NGX 物理不可达终版）——本波不新增第三面；G16+ 承接锚三面齐备（① GI 表达面 + UE 参照臂修复〔g15_gap_fix_closure_registry lumen 2 行 + G15-MC-F1 锚〕② DLSS NGX 版本与宿主车道〔G15-MD-F1 锚——NGX 310.5.2→310.6.0+ 版本演进面/D3D12 宿主车道架构面/税源分解实测约束〕③ 绝对画质 deficit 收口〔16 行 open-defer 锚字面 + M-c g16_anchor + P2 §5 汇总〕），用户 2026-08-19 授权面逐字承接。
  - **RD-045/M165 漂移监控登记（G15 复跑面）**：本波 M-e 门 25 键确定性键族全真 + soak 59 迭代 digest 锚位级复现——同型 digest 漂移**零检出**（维持 open-defer 不写进全绿叙述；G12-N13 承接锚 M-e 监控臂逐波登记兑现完结，条目维持 open 不关闭）。
  - **not-triggered / 维持 open 面**：G15.6b close-out 未跑（下一波面）；RD-034/039/040/041/042/043/044/045 八条 open 维持 0-byte（本波零追加零新 RD）；G14 defer 29 行承接锚 0-byte（本表 14 行 defer-to-G16+ 窗结论维持）；RFC next_free=31 / RXS next_free=408 维持（本波零消费）；g15_budget/budget_eval 0-byte（本波零 measured 新面零追加）。
  - **异己并发工作树面**：本批只含 G15 车道文件（按文件名显式择取）；异己会话 src/ 未提交面（ktx2_read/hzb/restir/sdf_trace/smrt/ssr 等 untracked 面 + .cursor/）维持未提交、零消费、零混入（立项裁决 3——M-e 门 src 触改闭集机核面把守：committed ⊆ candidate-b 单文件，untracked ⊆ 异己登记六件闭集，越界即 FAIL）。
- **④ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模）。`Assisted-by: Kimi-K3（G15.6a P2 穷举 + M-e 回归门 + soak 波）`（影响范围：milestones/g15/G15_P2_DECISIONS.md v1.0 首建 40 行 + ci/g15_p2_decisions_check.py + ci/g15_regression_drift_guard_smoke.py + ci/g15_stabilization_soak.py + ci/g15_closeout_check.py 新建 + milestones/g15 四 evidence schema 新建〔g15_p2_decisions_/g15_m_e_regression_drift_guard_/g15_stabilization_soak_/g15_wave6b_closeout_〕+ ci/check_schemas.py〔四前缀 × load/validator/路由十二处纯追加，io.open 补丁法〕+ .github/workflows/pr-smoke.yml〔步骤 277~280，步骤 276 块后追加，io.open 补丁法；279 = device 真跑门消费面 --verify-latest 沿 M-c/M-h/soak 重门先例〕+ registry/number_ledger.json〔CI_step 276→280/next_free 281 + revision_log v1.158〕+ 本契约 §8.8 本条 + evidence 本批真跑件〔P2 204316Z PASS + M-e 204851Z PASS + soak 205354Z PASS〕；g15_budget.json/budget_eval.py 0-byte〔本波零追加〕；src/spec/conformance 判据面 0-byte〔candidate-b 落地态 = §8.7 在案面，本波零触改〕；G5~G14 closed 判据/锚/登记表 0-byte〔只消费不回写〕；验证方式：块②逐字命令输出——四门真跑全绿 + 四 selftest 红绿留痕 + 守卫套件全 PASS + budget_eval --strict 253 pass + pytest 136 passed 零回归 + 互锁 READY 维持）。

### §8.9 G15.6b close-out 终审签署块（2026-08-23）——G-G15-9 字面兑现：close-out 终审门（g15.wave.6b.closeout，步骤 280）八 facts 全绿 **VERDICT=READY** → status active→closed 独立洁净 commit + tag `g15-closed`

- **① 终审八 facts 逐条（evidence/g15_wave6b_closeout_20260823T212501Z.json）**：
  1. **five_p0_pass = PASS**（5/5——M-a 16/16 + M-b 19/19 + M-c 18/18 + M-e 10/10 全绿；**M-d 诚实红门面特判合格**：G15 M-d status==fail ∧ 绿键 6 全真〔画质锚带复核/budget 零 estimated/RED 四臂〕∧ 红键 6 全假闭集 ∧ 消费的 G14 M-d 复跑件 checks 全绿 ∧ status==fail ∧ unmet_count==1==g14_fps_gap_registry 行数〔gap_id 51a150cb4523e8b6〕∧ G15-MD-F1 承接锚在案 = 未达标如实登记不充绿亦不充降级，G14 closeout 先例同型字面）。
  2. **wave_gates_2_to_6a = PASS**（5/5——wave2/3/4 exit 全 PASS + wave6a decisions PASS；**wave5 红面特判同型合格**：聚合 FAIL 红面 = M-d 行 FAIL 镜像 + facts ④⑤ 绿 + 红 facts ⊆ M-d 红同源闭集——如实登记不充绿不充降级）。
  3. **acceptance_map_check = PASS**（`g15.wave.1.acceptance_map` 双向 exit=0——MAP §1 五行 P0 ↔ 契约 §4.2 逐字一致维持）。
  4. **p2_decisions_40_frozen = PASS**（最新 evidence 204316Z host_section_pass + FROZEN_IDS 40 行闭集在树——闭集最终状态无漂移）。
  5. **budget_strict = PASS**（budget_eval --strict 253 pass/0 skip——g14 32 + g15 九条目全 measured_local 零 estimated，P-09）。
  6. **soak_6a_precedes = PASS**（g15_stabilization_soak_20260823T205354Z host_section_pass + base_commit_6a=6a0b25ea 留痕——59 迭代 1852.5s ≥1800s 零失败先行完成；同日放行沿 G14 立项裁决先例：6a full-run 先行完成后允许同日 close-out）。
  7. **rd_final_state_consistent = PASS**（deferred.json RD-034/039/040/041/042/043/044/045 八条目级 status 全 open 逐字 + P2 40 行闭集在树——RD-045 零检出维持 open 不关闭字面，G14PLUS_RECORD §6.2 承接）。
  8. **dual_unmet_finalized = PASS**（**双未达标终审定盘——如实登记不冒充**：**商用收口 0/18 画质面**〔M-c 最新 evidence PASS ∧ commercial_closure verdict==未达标 ∧ met_count==0/18 ∧ 18 格 unmet_attribution 逐格归因非空〔cornell 九格 ue_reference_degenerate——G15-MC-F1 参照死黑退化面；bistro 九格 deficit 双超程序产绝对阈〕∧ g16_anchor 用户 2026-08-19 授权面字面〕+ **性能 17/18 单格环境事件面**〔G14 M-d 最新 evidence status==fail ∧ met 17/unmet 1 ∧ 帧率表 1 行 gap_id 51a150cb4523e8b6 ∧ G15-MD-F1 承接锚 §8.6/§8.7——UE 参照臂缓存暖态跨会话位移 + NGXCubinVulkan 取证 + GPU-only 地板 3.02ms > 通过线 2.96ms 物理不可达终版〕；**G16+ 承接锚三面齐备**〔① GI 表达面 + UE 参照臂修复 ② DLSS NGX 版本与宿主车道 ③ 绝对画质 deficit 收口〕——未达标按用户 2026-08-19 授权新建 G16+ 里程碑继续优化，性能零降级守护面终态锁定；last_new_green_utc=20260823 留痕）。
- **② 终审命令逐字输出（2026-08-23 真跑留痕，仓库根目录）**：`py -3 ci/g15_closeout_check.py --gate g15.wave.6b.closeout` → **VERDICT=READY，exit=0**（八 facts 逐行打印不遮蔽；`--selftest` → OK materialized step 280）；四门 selftest 全过（P2 2 GREEN+4 RED / M-e 4 RED+2 GREEN / soak 1 GREEN+8 RED+2 评定臂 / closeout materialized 面）；守卫套件全 PASS（structure/schemas〔十二处纯追加〕/number_ledger〔on_tree_max 280/next_free 281〕/budget_eval --strict 253 pass/trace_matrix/stable_snapshot）；pytest 136 passed 零回归；互锁 `--require-ready` READY 维持。
- **③ 收口裁决（G-G15-9 逐字兑现面）**：① 验收映射最终状态、② 候选决策最终状态、③ RD 最终状态三面逐字一致；④ 全部 P0 独立断言均 PASS（M-d 诚实红特判面 = 未达标如实登记不冒充——不充绿亦不充降级，G14 closeout 先例同型）；⑤ evidence 终审、⑥ schema 终审（check_schemas 全族 PASS）、⑦ 预算终审（strict budget 非空全 PASS 零 estimated/skip）；⑧ 商用收口终审定盘 = **未达标如实登记不冒充**（画质 0/18 + 性能 17/18 双面在案）——按用户 2026-08-19 授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」逐字承接：G16+ 里程碑继续优化面开放，G16+ 承接锚三面齐备在案，性能零降级守护面终态锁定（G14 M-d 18 格 ×1.00 定盘判据面 0-byte 不改、当前环境复跑 17/18 如实登记）。
- **④ status flip 与 tag**：§8 只追加区本块落盘后，`status: active → closed` 独立洁净 commit（沿 G9~G14 先例——flip commit 只含契约 status 翻转 + README/00_MASTER_INDEX 勘误行，不混入门产），随后 tag `g15-closed`；check_guardrails 基准链维持 g7-closed（G8~G14 closeout 均未切换先例，0-byte）；`implementation_status: unlocked` 字面 0-byte 不动（双状态机：status 翻closed 后互锁 CLOSED 三态口径自动生效，判据语义 0-byte）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模；本块 = close-out READY 签署——双未达标如实登记不冒充，红面签署不充绿）。`Assisted-by: Kimi-K3（G15.6b close-out 波）`（影响范围：ci/g15_closeout_check.py 新建 + milestones/g15/g15_wave6b_closeout_evidence_schema.json 新建 + evidence/g15_wave6b_closeout_20260823T212501Z.json READY 件 + 本契约 §8.9 本条 + status flip 独立 commit + tag g15-closed + README.md/00_MASTER_INDEX.md 勘误行〔随 flip 批〕；G5~G14 closed 判据/锚/登记表 0-byte；deferred.json 八条 open 0-byte〔history 只追加，本波零追加〕；验证方式：块②逐字命令输出——closeout 八 facts 全绿 VERDICT=READY + 四门真跑全绿 + 四 selftest 红绿留痕 + 守卫套件全 PASS + pytest 136 passed 零回归 + 互锁 READY 维持）。
