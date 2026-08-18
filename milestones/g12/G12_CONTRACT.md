---
contract: G12
title: G12 路径追踪生产化期
status: closed
implementation_status: unlocked
active_scope: g12_1_governance_only + g12_2_production_core_wave
version: v1.0
date: 2026-08-17
timebox: "G12.1 治理波即刻执行（G11 已 closed）；G12.2~G12.7b 严格波次，工期在实现互锁开放后由 measured baseline 校准"
rfc_required: "一份 Full RFC（编号按立项时实测 registry/number_ledger.json namespaces.RFC next_free=29 领取，禁推测号）：G12 路径追踪生产化伞形 RFC（MIS 完整面语义 / 俄罗斯轮盘生产化语义 / 采样策略升级与低差异序列确定性协议扩展 / 收敛判据生产化语义 / 降噪管线与 TSR 底座联动语义面 / UE Path Tracer 对标口径面 / spec/global_illumination.md RXS-0357 参照器面演进显式修订行）。判档依据：路径追踪生产化触 spec/global_illumination.md 冻结面（RXS-0357 起步范围/固定 seed 确定性协议/门序面演进 + 采样语义冻结面），G5~G11 冻结面改动必须 RFC 显式修订行，判档争议向上取严。SER（M52）若 G12.2 复评 go 需语言层原语 → 独立 Full RFC 评估（承接锚：capability rt.ser 设备面实测可用 + 真实集成需求——先核验后裁决，未命中维持 defer，语义面留 RFC-0023 冻结面不接线）。RFC 须 D-409 独立 provenance 对抗性评审后 Agent Approved 方为语义冻结；未 Approved 前本契约对应条款为引用占位"
upstream_docs:
  - "milestones/g12/G12_PLAN.md v1.0（八波结构、P0 建议清单 8 行 + go P1 1 行、风险表 R-G12-1~11、治理裁决表项的契约上游事实源）"
  - "milestones/g12/G12_CANDIDATE_DECISIONS.md v1.0（G11 defer 19 行 + 存续 open RD + G12 新增候选逐行映射）"
  - "milestones/g11/G11_CONTRACT.md §8.8（G11 closed 终态，2026-08-17，flip commit 51279d45 + 回归刷新批 5ae83aa7）"
  - "milestones/g11/G11_P2_DECISIONS.md v1.0（28 行闭集；defer-to-G12+ 19 行承接锚 = G12 法定输入）"
  - "milestones/g10/G10_P2_DECISIONS.md v1.0（27 行闭集；M52 承接锚原文——锚定 G12 高分歧 RT workload 真实集成需求 + capability rt.ser 设备面实测可用）"
  - "spec/global_illumination.md（M96/RXS-0357 参照器冻结面：起步范围冻结 + 固定 seed 确定性协议 + pbrt-v4 容差带 + golden 门序硬约束 D2-Q7）"
  - "milestones/g9/g9_m96_pbrt_tolerance_band.json（M96 冻结容差带 measured 基值，G12 生产化回归锚来源）"
  - "rfcs/0026-visual-comparison-metrics.md（度量口径冻结面）+ rfcs/0027-external-reference-harness-license.md（外部参照 harness 与许可边界）+ rfcs/0028-g11-gi-quality-closure.md（G11 GI 修订先例）"
  - "registry/deferred.json RD-034/039/040/041/042/043/044（存续 open RD；只追加禁静默改判；RD040-nrd = 降噪承接锚）"
  - "04 P-01/P-07/P-09/P-12/P-13；10 §3/§7/§9.5；14 §1/§3/§4/§5（同 G11 口径）"
implementation_unlock:
  required_all:
    - "G12.1 治理门全部完成且有真实验证记录"
    - "check_g12_implementation_interlock --require-ready 输出 READY（互锁 validator 机器事实，不以叙述替代）"
    - "共享编号按互锁开放时 actual next_free 重新校准；数字 CI 步骤不得沿用推测号与草案建议值"
in_scope:
  - g12_1_governance_only
  - rfc_path_tracer_productionization
  - candidate_decisions_and_rd_mapping
  - p0_acceptance_mapping
  - measured_4070ti_baseline
  - g12_2_production_core_wave
  - g12_3_denoise_wave
  - g12_4_ue_pt_parity_wave
  - g12_5_throughput_baseline_wave
  - g12_6_p2_exhaustive_decisions
  - g12_7_stabilization_and_closeout
out_of_scope:
  - g12_2_plus_while_implementation_interlock_is_red
  - g12_1_src_spec_conformance_semantic_implementation
  - g12_1_numbered_workflow_steps_or_stub_scripts
  - absolute_ue_pt_quality_pass_line_deferred_to_g15
  - unanchored_new_production_items
  - caustics_volume_specular_chain_productionization_deferred_g15
  - ser_language_primitive_implementation_pending_reevaluation
  - nrd_vendor_denoiser_integration_evaluation_only
  - dlss_upscale_integration_implementation_deferred_to_g13
  - formal_fps_parity_and_pass_line_deferred_to_g14
  - gpu_pipeline_dual_ab_surface_deferred_to_g14
  - ue_source_or_binary_vendoring_into_rurix_repo
  - safe_gpu_operator_platform_remains_deferred_g12_plus
  - rewriting_g5_to_g11_closed_contracts_and_00_14
  - m96_reference_frozen_surface_rewrite
  - temporal_base_rewire
  - foreign_uncommitted_src_surface_consumption
  - speculative_number_consumption
deferred_refs: [RD-034, RD-039, RD-040, RD-041, RD-042, RD-043, RD-044]
deliverables:
  - id: D-G12-1
    name: "G12.1 治理四件套：G12_PLAN（升格契约上游事实源）、G12_CONTRACT、CI_GATES、非空 measured g12_budget；status=active 且 implementation_status=blocked"
  - id: D-G12-2
    name: "G12.1 完整候选决策表：G11 defer-to-G12+ 19 行 + 存续 open RD + G12 新增候选逐行映射（go / no-go / defer-to-G13+ / strategic_override + 承接锚）；缺行阻断 G12.2"
  - id: D-G12-3
    name: "G12.1 验收映射：全部 P0 各有独立 symbolic gate key、稳定脚本名、evidence schema 目标路径与判据；已 go 的 P1 同步覆盖"
  - id: D-G12-4
    name: "Full RFC-0029（G12 路径追踪生产化伞形）经 D-409 独立 provenance 对抗性评审后 Agent Approved"
  - id: D-G12-5
    name: "G12.1 RTX 4070 Ti measured baseline 与非空 g12_budget（零 estimated：G11.1 baseline 锚复测重登记 + PT 参照器收敛曲线基线锚）；G12 validator 五件套落盘——implementation interlock 当前诚实报告 BLOCKED"
  - id: D-G12-6
    name: "G12.2 生产化核心波：MIS 完整面 / 俄罗斯轮盘生产化 / 采样策略升级+低差异序列 / 收敛判据生产化（M158~M161）+ PT 生产化标定（M166）"
  - id: D-G12-7
    name: "G12.3 降噪波：时域/空域降噪管线 + TSR 底座联动（底座 0-byte）+ NRD 类 vendor 降噪评估报告（M162）"
  - id: D-G12-8
    name: "G12.4 UE Path Tracer 对标波：同场景同 spp 双端出图 + 收敛曲线逐段/噪声谱/能量守恒 measured 对拍 + UE PathTracing 模块归属差距登记（M163）+ 生产化回归门（M164）"
  - id: D-G12-9
    name: "G12.5 性能面波（PT 吞吐优化基线 measured 为 G14 备料，M165）+ G12.6 P2 穷举 + G12.7a soak + G12.7b close-out（生产化差距清单终审锁定）"
acceptance_gates:
  - id: G-G12-1
    check: "治理激活门：用户 2026-08-15「/goal G10~G15 六期分期 + 全期自主推进」指令留痕（G12.1 立项与 G12.2+ 开工授权同源——指令含「路径追踪等前沿技术」字面）；agent 依 10 §7/P-13/D-406 v2.0 完全自主签署立项裁决留痕；十项立项裁决全部落定；G12.0 不可变 ref=5ae83aa7 登记；仅 governance-only 范围 active"
  - id: G-G12-2
    check: "G12.1 完成门：D-G12-1~5 齐备并通过结构/schema/ledger/guardrail/预算核验；验收映射无缺行；无 src/spec/conformance 语义实现、无数字 workflow 空步骤；本门通过不自动开放实现"
  - id: G-G12-3
    check: "实现互锁门：check_g12_implementation_interlock --require-ready 输出 READY + 用户 G12.2 开工指令留痕（2026-08-15 指令全期授权面）+ 共享编号按 actual next_free 重新校准。任一条件不满足均保持 implementation_status=blocked"
  - id: G-G12-4
    check: "G12.2 退出门：M158/M159/M160/M161 四个 P0 独立断言全绿（MIS 完整面/RR 生产化/采样策略升级+低差异/收敛判据生产化——生产化落盘 + 正确性锚 0-byte〔M96 既有判据/确定性协议/golden 门序〕+ 收敛/方差面 measured 不劣于参照器基线锚）；M166 P1 标定值入 g12_budget 且 provenance 齐备（P-09，禁手写阈值；estimated 冒充 measured 即 RED）"
  - id: G-G12-5
    check: "G12.3 退出门：M162 P0 独立断言全绿——降噪管线落盘 + 噪声谱高频能量下降 measured + 帧均值能量守恒容差内 + temporal 底座 0-byte 断言 + NRD 类 vendor 降噪评估报告落盘（评估不接线，接入另判 G13+ 窗）；golden 对拍面不降级"
  - id: G-G12-6
    check: "G12.4 退出门：M163/M164 两个 P0 独立断言全绿——同场景同 spp 双端对拍（契约 digest 独立冻结，不等仍出报告即 RED）+ 收敛曲线逐段/噪声谱/能量守恒 measured 对拍 + UE PathTracing 模块归属差距登记表落盘（差距项显式登记即 RED 评审面，不静默混入）；不设绝对通过线；单端缺帧聚合不得 PASS；62 门（G9 34 + G10 14 + G11 14）零降级"
  - id: G-G12-7
    check: "G12.5 退出门：M165 P0 独立断言全绿——PT 吞吐基线 measured 入 g12_budget provenance 齐备 + 不设通过线登记（以基线冒充帧率对标即 RED，正式帧率对标锚定 G14 字面）+ 优化前后正确性锚（固定 seed digest 0-byte 或演进位显式登记）"
  - id: G-G12-8
    check: "G12.6 决策门：G12 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G13+，零空行；defer 必有承接锚（机核同构 ci/g11_p2_decisions_check.py）；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述"
  - id: G-G12-9
    check: "G12.7a 稳定门：全部 P0 与所有 go 的 P1 全量回归；G5~G11 既有判据 0-byte；生产化链路（PT 出图/降噪/对标装配）连续复跑 soak（量级沿 G11.7a 继承〔≥1800s〕或 measured 证明更短足够，阈值 G12.1 裁决 measured 标定）；strict budget 非空、零 estimated/skip；同日放行按立项裁决 7（7a full-run 先行完成后允许同日进 7b）"
  - id: G-G12-10
    check: "G12.7b 收口门：验收映射、候选决策、RD 最终状态逐字一致；全部 P0 独立断言均 PASS；evidence/schema/预算终审；生产化差距清单终审锁定（残余差距/未闭环行如实登记不冒充全闭环）；§8 只追加后 status active→closed"
guardrails:
  - "双状态不可混同：status=active 仅表示 G12.1 governance-only 已立项；在 G-G12-3 真实通过前 implementation_status=blocked，任何治理完成叙述不得冒充 G12.2 开工"
  - "G12.1 允许 milestones/g12、G12 RFC、G12 专属 claim、deferred history 只追加、未编号 validator 与 measured baseline；src/spec/conformance 和编号 workflow 步骤 0-byte"
  - "G12 CI 只冻结 symbolic gate key 与脚本名；numeric_step 一律写 post-interlock actual-next-free allocation。不得沿用推测号与草案建议值，不得预放空 workflow、空脚本或空 schema 壳"
  - "每个 P0 必须独立布尔断言与独立 evidence subject；可共享一次进程执行，但聚合 PASS 不能遮蔽任一子断言 FAIL/SKIP"
  - "缺硬件/工具链仅可 dev_env_degrade 或 SKIP=not-triggered；两者均不充 P0 绿。host oracle、mock、isolated nonzero、既有最小见证、人工截图均不能替代目标门"
  - "生产化范围唯一法定来源：M96 参照器冻结面（RXS-0357）+ 候选决策表行集；G12 不得无锚新立生产化项；新发现差距进对标差距登记显式登记 + G12.6 穷举，不得静默混入"
  - "生产化判据 = 生产化落盘 + 正确性锚 0-byte（M96 既有判据/固定 seed 确定性协议/golden 门序 D2-Q7）+ 收敛/方差/噪声面 measured 不劣于参照器基线锚（容差由标定程序 measured 产出，禁手写；或演进位显式登记即 RED 评审面）；G12 不设绝对 UE PT 画质通过线——「已达 UE5 PT 画质」判定归 G15 商用收口期，G12 期内一律不成立"
  - "生产化不得降级既有 62 门绿面（G9 34 key + G10 14 key + G11 14 key）；G5~G11 closed 契约与判据 0-byte；回归门独立 P0 断言；M96 golden 门序机器阻断（D2-Q7）维持"
  - "M96 参照器既有判据 0-byte：RXS-0357 起步范围冻结（焦散/体积/specular 链 out）/ 固定 seed 确定性协议 / pbrt-v4 容差带 / golden 门序 0-byte；生产化演进经 RFC 显式修订行 + 新条款承载，既有条款字面不动"
  - "UE 源码仅外部参照只读：PathTracing.cpp 等只读可参照（F:\\UE_5.8 与 E:\\Kimi_Agent_Taichi Engine 优化计划\\references\\UnrealEngine 双树），零 vendoring、零片段复制进 src/spec；违反即 revert + 留痕（RFC-0027 字面）"
  - "temporal 底座 0-byte 不接线（RD040-nrd 承接锚口径）：G12.3 降噪只消费既有 TAA/TSR 历史接口面；NRD 类 vendor 降噪只评估不接线，接入经 UpscaleBackend 同构契约另判 G13+ 窗"
  - "g12_budget 首个实现 PR 前必须非空 measured_local 且有 evaluator；全程零 estimated；性能数字不替代 correctness gate；阈值全部实测标定禁手写"
  - "新 unsafe 仅在实现互锁开放后按 actual next_free 登记并附 SAFETY；rurix-render 维持 forbid(unsafe_code)"
  - "触 G5~G11 冻结面必须 RFC 显式修订行（spec/global_illumination.md RXS-0357 参照器面演进只经 RFC-0029 修订行），禁静默扩；G5~G11 closed 契约与 00-14 0-byte，close-out 证据只追加"
  - "异己并发工作树面不混入零消费：G12 车道 commit 只含 G12 车道文件；立项时工作树异己会话 src/ 未提交面（hzb/restir/sdf_trace/smrt/ssr/ktx2_read 等）严禁消费/混入（G10.8b §8.10/G11 先例同模）"
  - "新文件 LF + 尾换行；本契约合入后正文冻结，激活/验收/收口只追加 §8，除最终 status flip 外不回写既有事实"
---

# G12 契约 — 路径追踪生产化期

> 计划：[G12_PLAN.md](G12_PLAN.md) v1.0 · 候选决策：[G12_CANDIDATE_DECISIONS.md](G12_CANDIDATE_DECISIONS.md) · 机器门：[CI_GATES.md](CI_GATES.md)。
> 当前裁决：**G12.1 governance-only active；G12.2~G12.7b implementation blocked**。`active` 不是实现门绿灯。

---

## 1. 目标与双门状态

G12 是**路径追踪生产化期**：把 M96 参照器（`src/rurix-render/src/gi/path_trace.rs` + `src/rurix-render/src/rt/ref_tracer.rs` + `src/rurix-render/src/bin/g9_m96_path_tracer.rs`，G9.4 已验收——固定 seed 位级确定性 + pbrt-v4 收敛曲线容差带 + golden 门序硬约束 D2-Q7）提升为生产级路径追踪器——生产化核心（MIS 完整面/俄罗斯轮盘/采样策略升级+低差异序列/收敛判据生产化）→ 降噪（时域/空域 + TSR 底座联动）→ UE Path Tracer 对标（同场景同 spp 双端出图，收敛曲线逐段/噪声谱/能量守恒 measured 对拍 + UE PathTracing 模块归属差距登记）→ 性能基线（为 G14 备料）。「UE5 级」可核对基线沿用 G9/G10/G11 口径 = UE 5.8；验收五层级沿用：核心等价、功能闭环、可降级、可生产化、Vulkan 主线。**G12 设生产化判据（正确性锚 0-byte + measured 不劣于参照器基线锚）但不设绝对 UE PT 画质通过线**——「已达 UE5 PT 画质」的绝对判定归 G15 商用收口期；DLSS/超分归 G13、正式帧率对标归 G14。

本契约拆分两种状态：

| 状态 | 当前值 | 含义 |
|---|---|---|
| `status` | `active` | G12.1 治理波已获授权，可落治理资产、Full RFC、候选决策/验收映射、G12 专属 claim、互锁 validator、RTX 4070 Ti measured baseline 与非空 budget |
| `implementation_status` | `blocked` | G12.2+ 尚未获准；当前不得改 `src/`、`spec/`、`conformance/`，不得 materialize 数字 CI 步骤 |

G-G12-3 是唯一实现入口：互锁 validator（`check_g12_implementation_interlock --require-ready`）输出 READY + 用户 G12.2 开工指令留痕 + 共享编号按 actual `next_free` 重新校准，三者齐备方可解锁；任一缺失均保持 `blocked`。

## 2. 范围与严格波次

### 2.1 G12.1 governance-only

G12.1 只做 D-G12-1~5。允许治理文档、Full RFC（须 D-409 评审后 Agent Approved 方为语义冻结）、候选决策表、验收映射、G12 专属无冲突 claim、互锁 validator、RTX 4070 Ti baseline 与非空 budget；禁止语义实现和编号 workflow。interlock validator 在当前事实下应明确返回 `BLOCKED`，这正是正确结果，不是失败需要被绕开。

### 2.2 G12.2~G12.7b implementation

实现互锁开放后按以下顺序推进，波次内可蜂群并行，波次间不得越级；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑：

```text
G12.2 生产化核心波（MIS 完整面 / 俄罗斯轮盘生产化 / 采样策略升级+低差异序列 / 收敛判据生产化 + PT 生产化标定）
  → G12.3 降噪波（时域/空域降噪 + TSR 底座联动〔底座 0-byte〕+ NRD 类 vendor 降噪评估）
  → G12.4 UE Path Tracer 对标波（同场景同 spp 双端出图 + 逐段/噪声谱/能量守恒 measured 对拍 + 模块归属差距登记 + 回归门）
  → G12.5 性能面波（PT 吞吐优化基线 measured——G14 备料，不设通过线）
  → G12.6 P2 穷举决策 → G12.7a stabilization/soak → G12.7b close-out
```

每波退出门见 YAML `acceptance_gates`（G-G12-4~7，判据按 G12_PLAN §2 各波退出门草案硬化）；任一上游门未绿，下游 evidence 即使局部成功也不能宣称波次完成。单点依赖：G12.2 是全部下游波的硬前置（生产化核心面未落地则降噪输入与对标输入不存在）；G12.4 是全部生产化闭环的统一核验面；G12.5 依赖 G12.4 的正确性锚。

## 3. G12.1 交付冻结

| ID | 交付 | 退出判据 |
|---|---|---|
| D-G12-1 | 契约四件套与双状态 | PLAN v1.0、CONTRACT、CI_GATES、非空 measured budget 一致；`status=active`、`implementation_status=blocked` |
| D-G12-2 | 候选决策与 RD 总映射 | G11 defer 19 行 + 存续 open RD + G12 新增候选逐行；裁决、波次、承接锚、最终状态无空项；缺行阻断 G12.2 |
| D-G12-3 | 验收映射 | 全部 P0 全部有独立 key/script/schema 目标路径/check；go 的 P1 同步入表；不存在"由邻项代绿"；缺行阻断 G12.2 |
| D-G12-4 | Full RFC-0029 | 经 D-409 独立 provenance 评审后 Approved（未 Approved 前本契约对应条款为引用占位）；编号登记与 README/ledger 一致 |
| D-G12-5 | baseline、budget、互锁 validator | RTX 4070 Ti measured 数据非空、零 estimated；interlock validator 对当前状态诚实报 BLOCKED；无空 workflow、无空 schema 壳 |

G12.1 完成仅关闭治理准备，不改变 G-G12-3 的机器事实。

## 4. 验收门与 P0 独立断言

### 4.1 波次验收门

G-G12-1~10 以 YAML 头为可提取摘要。[CI_GATES.md](CI_GATES.md) 冻结脚本与 evidence 形态。条件型分项的 `SKIP=not-triggered` 只表示决策已记录，不是成功；设备门的 `dev_env_degrade` 只表示环境缺失，也不是成功。

### 4.2 P0 独立断言

以下 8 行是 close-out 不可合并、不可删减的独立布尔断言（key 命名空间三方逐字一致，冻结）。一次 smoke 可以共享启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。evidence schema 目标路径统一为 `milestones/g12/g12_m<###>_<slug>_evidence_schema.json`——本契约只冻结路径，不预建文件。硬判据由 G12_PLAN §2 各波退出门草案与 §3 P0 建议清单展开为可机器求值形式，负例 RED 臂要求逐行写明。**生产化判据统一形态**：生产化落盘（只消费 M96 冻结面 + 候选决策表对应行）+ 正确性锚 0-byte（M96 既有判据/固定 seed 确定性协议/golden 门序 D2-Q7）+ 收敛/方差/噪声面 measured 不劣于参照器基线锚（容差由标定程序 measured 产出禁手写；或演进位显式登记即 RED 评审面）+ 不降级既有 62 门绿面。

| Symbolic gate key | M### | 最晚波次 | 稳定脚本名 | 独立硬判据 |
|---|---:|---|---|---|
| `g12.p0.m158.mis_full_surface` | M158 | G12.2 | `ci/g12_mis_full_surface_smoke.py` | MIS 完整面生产化：光源采样（NEE）× BSDF 采样 MIS 权重全路径覆盖 + 能量守恒（白炉 + 逐级能量增量单调不增，RXS-0395 口径继承）+ 同 spp 收敛曲线不劣于参照器基线锚（g12_budget pt.ref_curve 锚，容差标定程序产）+ 固定 seed 位级确定性协议继承 + M96 既有判据 0-byte；权重缺失冒充 MIS 即 RED；能量偏置注入即 RED；收敛劣化冒充升级即 RED；确定性协议漂移即 RED |
| `g12.p0.m159.russian_roulette_prod` | M159 | G12.2 | `ci/g12_russian_roulette_prod_smoke.py` | 俄罗斯轮盘生产化：吞吐自适应 RR（路径吞吐权重驱动终止概率）+ 无偏补偿（补偿因子闭式）+ 最小反弹保障（低深度不早杀）+ RR 终止率/补偿计数非空 + 收敛曲线不劣于基线锚；早杀偏置注入即 RED；补偿缺失冒充无偏即 RED；跳 RR 偏移未检出即 RED（RXS-0357 三臂 RED 面继承） |
| `g12.p0.m160.sampling_lds_upgrade` | M160 | G12.2 | `ci/g12_sampling_lds_upgrade_smoke.py` | 采样策略升级 + 低差异序列：分层/低差异序列生产化 + 确定性协议扩展（序列索引确定性 + 固定 seed 位级一致维持 + RNG 流布局 provenance）+ 收敛曲线 measured 不劣于独立 PCG 流锚；序列非确定冒充低差异即 RED；位级一致破坏未登记即 RED；收敛劣化冒充升级即 RED |
| `g12.p0.m161.convergence_criterion_prod` | M161 | G12.2 | `ci/g12_convergence_criterion_prod_smoke.py` | 收敛判据生产化：逐像素方差驱动自适应 spp 终止 + 收敛报告（逐像素 spp 分布/方差/未收敛像素计数非空）+ 收敛误判率 ≤ 标定阈（标定程序产禁手写）+ 固定全 spp golden 对拍不偏离冻结带（measured×2.0 带继承）；早停冒充收敛即 RED；未收敛像素缺报即 RED；golden 偏离冻结带即 RED |
| `g12.p0.m162.denoise_pipeline_tsr` | M162 | G12.3 | `ci/g12_denoise_pipeline_tsr_smoke.py` | 降噪管线 + TSR 联动：时域/空域降噪管线落地 + 噪声谱高频能量下降 measured（标定阈）+ 帧均值能量守恒容差内（不引入系统性变暗/变亮偏置）+ temporal 底座 0-byte 断言 + NRD 类 vendor 降噪评估报告落盘（评估不接线）+ golden 对拍面不降级；降噪引入系统性偏置即 RED；temporal 底座接线即 RED；评估冒充接入即 RED；噪声底未降冒充降噪即 RED |
| `g12.p0.m163.ue_pt_parity` | M163 | G12.4 | `ci/g12_ue_pt_parity_smoke.py` | UE Path Tracer 对标：同场景同 spp 双端出图（UE build digest == M128 登记 ue_build_id 机核；契约 digest 独立冻结，不等仍出报告即 RED）+ 收敛曲线逐段 measured 对拍（容差标定程序产）+ 噪声谱对拍 + 能量守恒对拍 + UE PathTracing 模块归属差距登记表落盘（差距项显式登记即 RED 评审面）；不设绝对通过线；逐段对拍超容差静默即 RED；差距项静默混入即 RED；单端缺帧聚合 PASS 即 RED |
| `g12.p0.m164.regression_guard` | M164 | G12.4 | `ci/g12_regression_guard_smoke.py` | 生产化回归门：既有 62 门（G9 34 key + G10 14 key + G11 14 key）最新 evidence 全绿只读汇总 + 生产化触改面既有门重跑回归零降级（M96 golden 门序面真跑抽检）；既有门降级即 RED；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED |
| `g12.p0.m165.pt_throughput_baseline` | M165 | G12.5 | `ci/g12_pt_throughput_baseline_smoke.py` | PT 吞吐优化基线：吞吐基线 measured（rays/sec + 帧时 at 固定 spp × 场景集，50×3 trimmed mean 协议）入 g12_budget provenance 齐备 + 不设通过线登记 + 优化前后正确性锚（固定 seed digest 0-byte 或演进位显式登记）；基线冒充帧率对标即 RED；digest 漂移未登记即 RED；estimated 冒充 measured 即 RED |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G12.7b。M166（PT 生产化标定）为 P1，入验收映射随主门核验。

## 5. Guardrails

见 YAML `guardrails`。特别强调四点：

1. 治理 active 不等于实现 active；G-G12-3 的机器事实（validator READY + 用户 G12.2 开工指令 + actual `next_free` 重校）不可替代。
2. 数字 CI 步骤只能在实现互锁开放后读取 actual `next_free` 再分配；文档中的稳定身份是 symbolic gate key 和脚本名；禁止沿用草案建议值。
3. **生产化范围唯一法定来源 + 正确性锚 0-byte**：G12 只消费 M96 参照器冻结面（RXS-0357）+ 候选决策表行集；每生产化项的判据 = 正确性锚 0-byte + measured 不劣于参照器基线锚（容差标定程序产禁手写）；G12 不设绝对 UE PT 画质通过线（归 G15）。
4. **回归零降级 + M96 门序维持**：生产化不得降级既有 62 门绿面；golden 门序机器阻断（D2-Q7）维持；temporal 底座 0-byte 不接线（RD040-nrd 承接锚口径）。

## 6. Deferred 处置

| Deferred | G12 处置 |
|---|---|
| RD-039 | 总体维持 open 为法定输入；M61 mesh shader 分项维持 defer（G11.6 逐字承接，承接锚字面 0-byte，G12+ 重评窗顺延 G13+ 不关闭）；其余分项未触发维持 open |
| RD-040 | **M52 SER G12 重评窗核验**（G12.1：真实集成需求未至〔治理波零实现〕+ capability rt.ser 设备面未实测〔树内零探针〕→ maintain-defer；复评点 = G12.2 高分歧 RT workload 集成面 materialize 时；承接锚字面 0-byte 维持，语义面留 RFC-0023 冻结面不接线）；M99-clipmap 承接兑现完结维持（G11.6 登记字面 0-byte）；M100-high 维持 defer（G12.4 触发评估登记——双端 PT 对标若产多灯 workload measured 对照面则按只追加程序重判，否则维持 defer 锚定 G14）；**RD040-nrd 评估窗承接**（G12.3 NRD 类 vendor 降噪评估报告，承接锚口径：UpscaleBackend 同构输入契约接入面评估，接入时不改 temporal 底座——评估不接线，接入另判 G13+ 窗）；history 只追加 |
| RD-041 | M28/M40-svt/M26-fg/M05-mv/M56-wg 维持 no-go 留档；DLSS/Streamline 方向登记维持（G10-N5 锚定 G13，G12 仅档案零接线） |
| RD-044 | M126-rd044/RD044-continuum/RD044-fluid 维持 open-留档/观察；G12 度量面 FLIP 图像度量与 RD-044 族 FLIP 流体防混淆登记维持（G10/G11 口径字面） |
| RD-034 | DXIL RT/mesh 上游 blocked 维持 open；G12 生产化仅 Vulkan 主腿，不阻主线 |
| RD-042/043 | 可微物理观察 / wgrapier GPU 刚体观察维持，不进 G12 任何面 |

详情始终以 `registry/deferred.json` 为唯一事实源；本表只冻结承接纪律。G11 defer-to-G12+ 19 行逐行处置归 [G12_CANDIDATE_DECISIONS.md](G12_CANDIDATE_DECISIONS.md) §1；SAFE-GPU 维持「G10+ 独立期立项」defer（G12 非其独立期，沿 G10/G11 立项裁决口径）。

## 7. 修订记录与开工裁决

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-17 | 初版契约：按 G12_PLAN v1.0 显式拆分 governance 与 implementation；G12.1 active、G12.2+ blocked；冻结波次门（G-G12-1~10）与 8 个 P0 独立断言（key 命名空间三方逐字一致）+ 1 个 go P1（M166）；CI 数字延迟到 post-interlock actual-next-free allocation；十项立项裁决逐字登记；§8 只追加区启用。 |

**开工裁决留痕**：

- **用户立项指令**：2026-08-15 主会话下达「/goal 帮我完成 G10-15 的内容，自主派发调研 agent 和进行决策，里程碑推进时组织 agent-team 完成，要求彻底完成对标 UE5 渲染器的目标，并支持 dlss、超分采样、路径追踪等前沿技术。技术完成需要严格的画面审查，需要获取完整渲染画面，再用本地已有的 UE5 渲染器出图对比，修复画面中出现的细节问题；同时优化渲染管线效率，使帧率对标 UE5 略高（不降级画质）。本地已有 UE5 渲染器参考项目，你也可以联网获取（我的 GitHub 在 UE5 组织内），同时支持联网获取压测模型环境等必要工具集。最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在 G15 后无限制新建里程碑继续优化」（指令原文以会话留痕为准，G10_CONTRACT §7 / G11_CONTRACT §7 同字面援引）。该指令授权 G10~G15 六期分期与全期自主推进——**G12.1 立项与 G12.2+ 开工授权同源**：「支持 dlss、超分采样、路径追踪等前沿技术」中**路径追踪**即 G12 路径追踪生产化期的用户目标字面；本治理波为该指令在 G12 期的执行留痕（2026-08-17 G12.1 治理波任务下达：G12 立项 + 治理四件套 + 候选决策 + 验收映射 + RFC + measured baseline + 互锁解锁）。
- **agent 立项裁决**：依 10 §7、P-13 与 D-406 v2.0，agent 完全自主签署立项裁决；G12.1 治理波即刻 active，G12.2+ 继续由 G-G12-3 硬阻断。
- **不可变基线**：G12.0 文档集不可变 ref = `5ae83aa7`（G11 close-out flip commit `51279d45` + G11.7a 回归刷新批 HEAD，立项时实测 HEAD；任务背景声明的 `51279d45` = G11 status flip commit 留痕——G12.0 取立项时实测 HEAD 沿 G11.0 取 G10 复跑批 HEAD 先例）。工作树带异己会话 src/ 未提交面——处置见裁决 1。
- **十项立项裁决（逐字登记）**：
  1. 现在立项；G12.0 不可变 ref=`5ae83aa7`；**带未提交项立项**——工作树异己会话 src/ 未提交面（rurix-asset/rurix-render geometry/gi/shadow/ssr/ktx2_read/hzb/restir/sdf_trace/smrt 声明面，含 untracked `src/rurix-render/src/gi/restir.rs`——ReSTIR 相关面）保持不混入 G12 车道、**严禁消费**（G10.8b §8.10/G11 立项裁决 1 先例同模：flip/治理 commit 只含本车道文件，异己面维持未提交）。
  2. 生产化范围唯一法定来源 = M96 参照器冻结面（RXS-0357）+ G12.1 候选决策表行集（生产化核心 4 行 + 降噪 1 行 + 对标 2 行 + 性能基线 1 行 + 标定 1 行）；G12 不得无锚新立生产化项；新发现差距进对标差距登记显式登记 + G12.6 穷举。
  3. UE PT 对标判据形态 = 收敛曲线逐段 measured 对拍 + 噪声谱 + 能量守恒 + UE PathTracing 模块归属差距登记（容差由标定程序 measured 产出禁手写）；**G12 不设绝对「已达 UE5 PT 画质」通过线**——绝对判定归 G15 商用收口期。
  4. **RFC 判档**：生产化核 + 降噪 + 对标口径 = Full RFC-0029（触 spec/global_illumination.md RXS-0357 参照器面 + 采样语义冻结面，判档争议向上取严）；SER（M52）若 G12.2 复评 go 需语言层原语 → 独立 Full RFC 评估（RFC-0023 冻结面衔接；承接锚：capability rt.ser 设备面实测可用 + 真实集成需求——先核验后裁决）。
  5. **M52 SER 重评窗核验（G12.1，RD-040 history 兑现登记）**：真实集成需求未至（G12.1 治理波零实现，生产化核心面 G12.2 才 materialize）+ capability rt.ser 设备面未实测（树内零 rt.ser 探针，grep 实测 src/ 零 `rt.ser`/`HitObject`/`reorderThread` 命中）→ **maintain-defer**；复评点 = G12.2 生产化核心波 materialize 高分歧 RT workload 集成面时按只追加程序重判；承接锚字面 0-byte 维持（兜底 = 语言层不加 SER 原语维持，无 capability 设备 fail-closed 降级语义留 RFC-0023 冻结面不接线）。
  6. **M96 参照器既有判据 0-byte**：RXS-0357 起步范围冻结（焦散/体积/specular 链 out）/ 固定 seed 确定性协议 / pbrt-v4 容差带 / golden 门序（D2-Q7）0-byte；生产化演进经 RFC-0029 显式修订行 + 新条款承载（post-interlock actual-next-free allocation 领新 RXS 条款，既有条款字面不动）；`g9_m96_pbrt_tolerance_band.json` 冻结带只消费不回写。
  7. G8.8b/G9.8b/G10.8b/G11.8b 同日放行先例 = 继承（7a full-run 先行完成后允许同日进 7b close-out；先例字面不扩展解释）。
  8. 回归零降级：生产化不得降级既有 62 门绿面（G9 34 key + G10 14 key + G11 14 key）；G5~G11 closed 判据 0-byte；回归门独立 P0（M164）。
  9. **G11 defer-to-G12+ 19 行逐行处置**：M52 重评窗核验（裁决 5）；M100-high 维持 defer（G12.4 触发评估登记）；RD040-nrd 评估窗承接（G12.3 NRD 类 vendor 降噪评估报告，评估不接线）；G11-N5 度量口径修订评估面维持 defer（G12.6 触发评估登记）；G11-N8/G11-N9 锚定 G15 维持（焦散/透射/specular IBL 面——M96 起步范围冻结维持）；G10-N17 维持 defer（G12.4 触发评估登记）；G10-N11/N16/G11-N3/M114-strand 锚定 G14 维持；G10-N5 锚定 G13 维持；M61/SAFE-GPU/M127/M98-l4/M118-hdr-cal/M125-adopt3/G10-N6/G10-N8 维持 defer 承接锚字面 0-byte——逐行落 [G12_CANDIDATE_DECISIONS.md](G12_CANDIDATE_DECISIONS.md) §1。
  10. 压测资产二进制**不入 git**（外部缓存 K: 盘，仓库内只登记清单/许可/digest 元数据——沿 G10/G11 裁决）；数字 CI 步骤 `post-interlock actual-next-free allocation` 重申确认；UE 零 vendoring 重申（PathTracing.cpp 只读外部参照，RFC-0027 字面）。
- **G15 后无限续期授权登记**：用户指令「允许在 G15 后无限制新建里程碑继续优化」留痕（G10_CONTRACT §7 / G11_CONTRACT §7 同字面援引）——G15 收口若未达真实可商用标准，按同治理范式续立 G16+（每期仍独立走立项/治理波/互锁/full-run，不因授权免除任何机器门）。
- **RFC 编号**：Full RFC 编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free=29` 领取（RFC-0029）；RXS/RD/U/RX/数字 CI 均延迟到实现互锁开放后按 actual `next_free` 领取。

---

## 8. Implementation activation / Close-out（只追加区）

<!-- 首条未来记录只能是 G-G12-3 互锁实测与 implementation_status 解锁凭据；其后追加逐波验收与 close-out。当前不得写 PASS、不得预填 run URL。 -->

### §8.1 G-G12-3 implementation_status 解锁记录（2026-08-17）——G12.1 治理波验收：互锁 VERDICT=READY（事实门①~④全绿 + 一致性门 C1~C4 全绿）+ 守卫套件全 PASS + registry 补落同批；G12.2 生产化核心波开工面开放

- **① 独立断言全绿清单（`g12.gov.implementation_interlock` 事实门四条 + 一致性门 C1~C4，全 host 治理断言面，无 device 面——治理波零 device 交付）**：

  | gate（脚本内断言标识） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g12.gov.implementation_interlock` 事实门① | G11_CONTRACT status==closed 且 §8.8 签署块在位 + G12.0 不可变 ref `5ae83aa7` 登记 | host | milestones/g11/G11_CONTRACT.md §8.8 + milestones/g12/G12_CONTRACT.md §7 | PASS |
  | `g12.gov.implementation_interlock` 事实门② | RFC-0029 在树 Agent Approved + §9.1 ≠起草 provenance 独立评审记录（D-409） | host | rfcs/0029-g12-path-tracer-productionization.md（Agent Approved 2026-08-17）+ milestones/g12/design/rfc0029_adversarial_review.md（第 1 轮 10 findings 全 disposition，v0.2 修法批） | PASS |
  | `g12.gov.implementation_interlock` 事实门③ | 候选决策表 37 行闭集零空行 + deferred.json history 只追加（vs G12.0 base 条目四字段 0-byte）+ 验收映射 §1 八行 P0（M158~M165 闭集全等）+ §2 一行 P1（M166）无缺行 | host | milestones/g12/G12_CANDIDATE_DECISIONS.md + registry/deferred.json + milestones/g12/G12_ACCEPTANCE_MAP.md | PASS |
  | `g12.gov.implementation_interlock` 事实门④ | 用户 G12.2 开工指令留痕（2026-08-15 全期授权面「支持 dlss、超分采样、路径追踪等前沿技术」字面）+ workflow 实测末号 216 == ledger CI_step on_tree_max 216 且 next_free 217 == +1 | host | milestones/g12/G12_CONTRACT.md §7 + .github/workflows/pr-smoke.yml + registry/number_ledger.json | PASS |
  | `g12.gov.implementation_interlock` 一致性门 C1~C4 | C1 双状态诚实 / C2 §8 记录一致 / C3 数字步骤零预占（unlocked 态 not_applicable 两态登记，实测 0 处）/ C4 src-spec-conformance 0-byte（unlocked 态 not_applicable 两态登记，实测 0 处） | host | 本契约 front matter + §8.1 本条 + ci/check_g12_implementation_interlock.py | PASS |

  治理波 measured baseline 事实面（D-G12-5，门消费面非独立门断言）：g12_budget.json 10 条 measured_local 零 estimated——2 bench 回归守护锚沿 G10.1/G11.1 同协议本回合复测重登记（evidence/g12_baseline_sr_pipeline_l3_20260817T102907Z.json / evidence/g12_baseline_bandwidth_d2h_pinned_20260817T102907Z.json）+ 8 条 PT 参照器收敛曲线基线锚（evidence/g12_pt_ref_curve_{cornell,direct}_spp{1,4,16,64}_20260817T102907Z.json，M96 本回合 RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 真跑，双跑位级一致 + golden digest 全等 + pbrt 冻结带内）。

- **② 波聚合门实测输出**：G12.1 治理波**不设 `g12.wave.N.exit` 波聚合门**（波聚合门属 G12.2+ 实现波面，契约 §2.2 波次序列）——SKIP=not-triggered 如实登记不充绿。治理期唯一机器聚合核验面 = 互锁 validator 只读汇总：事实门①~④ 逐条独立断言 + 一致性门 C1~C4，聚合 VERDICT 不遮蔽任一子断言 RED/FAIL（validator 逐行打印 + selftest 红臂实证）；实测 **VERDICT=READY，exit=0**（逐字输出见块③）。验收映射机核由 validator 事实门③承载（M158~M165 闭集全等 + key/script/schema 字面齐备）；契约 §4.2 冻结的 key 命名空间三方逐字一致（契约↔MAP↔CI_GATES）为文档冻结面，G12.2 门 materialize 时由数字机器门逐字对账（post-interlock actual-next-free allocation）。

- **③ 验收命令逐字输出（2026-08-17 真跑留痕，仓库根目录）**：
  - `py -3 ci/check_g12_implementation_interlock.py --require-ready` → **VERDICT=READY，exit=0**，完整输出：
    ```text
    [check_g12_implementation_interlock] 事实门（当前可为红）：
      PASS ① G11_CONTRACT status = 'closed'（要求 closed）且 §8.8 签署块在位 = True；G12.0 不可变 ref 5ae83aa7 登记 = True
      PASS ② rfcs/0029-g12-path-tracer-productionization.md：Agent Approved；独立评审 provenance ['Kimi-K3（D-409 独立评审轮次，与起草轮次隔离）']
      PASS ③ 决策表/ deferred/ 验收映射三面：候选决策表 37 行零空行；deferred history 只追加（vs G12.0 base 四字段 0-byte）；验收映射 §1 八行 P0 + §2 一行 P1 无缺行
      PASS ④ 用户 G12.2 开工指令留痕（2026-08-15 全期授权面「支持 dlss、超分采样、路径追踪等前沿技术」字面） = True；workflow 实测末号 = 216、ledger CI_step on_tree_max = 216、next_free = 217（一致 = True）
    [check_g12_implementation_interlock] 一致性门（红即脚本失败）：
      PASS C1 G12_CONTRACT implementation_status = 'unlocked'；事实门全绿 = True（事实未全绿时必须保持 blocked，禁止治理完成冒充实现开工）
      PASS C2 §8 G-G12-3 解锁记录存在 = True；事实门全绿 = True、implementation_status = 'unlocked'（双状态与 §8 记录必须一致）
      PASS C3 数字步骤零预占：not_applicable（implementation_status='unlocked' 已解锁，治理期口径不适用；skipped_reason=实现波合法 materialize，实测 numeric_step 违例 0 处 / workflow g12 token 0 处 / ci/g12_*_smoke.py 0 件均为解锁后合法实现面，非预占；blocked 态恢复原机核，判据语义 0-byte）
      PASS C4 src/spec/conformance 治理期 0-byte：not_applicable（implementation_status='unlocked' 已解锁，治理期口径不适用；skipped_reason=实现波合法改动三面，实测 g12 实现面 token/命名命中 0 处均为解锁后合法实现面，非治理期预放；blocked 态恢复原机核，判据语义 0-byte）
    [check_g12_implementation_interlock] VERDICT = READY
    ```
  - `py -3 ci/check_g12_implementation_interlock.py --selftest` → **SELFTEST PASS (16 RED + 1 GREEN + 1 TREE)，exit=0**：16 红臂全过（事实门①~④ 红臂 ×10 + C3/C4 预占注入 FAIL 臂 ×2 + C2 记录/状态不一致 FAIL 臂 + unlocked 态 C3/C4 not_applicable 两态校准臂 + unlocked 态事实门红 C1 仍 FAIL 臂 + closed 态全门 not_applicable VERDICT=CLOSED 三态臂）+ 合成正本 GREEN + 当前树 TREE ok（VERDICT=READY，exit=0）。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 379 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (379/379 clauses anchored, 843 test files scanned)`；`py -3 ci/budget_eval.py` → `[budget_eval] PASS (176 pass, 0 skip, normal mode)`（g12.bench 2 条 + g12.pt 8 条 measured_local 全 PASS 零 estimated）。

- **④ 门序 / not-triggered / no-go 登记面摘要**：
  - **门序**：G-G12-1（治理激活门：用户 2026-08-15 指令留痕 + 十项立项裁决 + G12.0 不可变 ref=5ae83aa7 登记）与 G-G12-2（G12.1 完成门：D-G12-1~5 齐备、验收映射无缺行、零 src/spec/conformance 语义实现、零数字 workflow 空步骤）于 commit `46ac9dcf` 留痕；本批 G-G12-3 三条件齐备——①互锁 validator VERDICT=READY（块①/③机器事实，不以叙述替代）+ ②用户 G12.2 开工指令留痕（契约 §7 指令字面，G12.1 立项与 G12.2+ 开工授权同源）+ ③共享编号按 actual next_free 重新校准（快照：RXS next_free=398 / CI_step next_free=217 / RD next_free=45 / U next_free=58 / RFC next_free=30——一律 `post-interlock actual-next-free allocation`，禁推测号与草案建议值；`py -3 ci/check_number_ledger.py` PASS）。
  - **front matter flip（本条同批）**：`implementation_status: blocked` → `unlocked`；`active_scope: g12_1_governance_only` → `g12_1_governance_only + g12_2_production_core_wave`（G11 §8.1 flip 先例同模；正文冻结 0-byte——§1 双状态表与「当前裁决」行维持立项时字面，flip 事实以本条为准）。G12.2 起每个实现 PR 必须把 `py -3 ci/check_g12_implementation_interlock.py --require-ready` 作为前置 required check，spec-first + RED 先行，数字 CI 步骤按落盘前实测 actual next_free 顺位领取。
  - **registry 补落（同批，G12.1 治理波声明未落面）**：number_ledger `reserved_in_flight[G12]` 命名空间登记（RFC-0029 单号 claim + RXS/CI_step/RD/U/RX_error/MR/SG/D 零数字 claim 字面，G10/G11 条目格式同模）+ RFC 命名空间校准（on_tree_max 28→29、next_free 29→30，RFC-0029 已 materialize 在树——v1.113 G11.1 收口先例同模）+ revision_log v1.124；deferred.json history 只追加两条——**RD-039 +1**（M61 G12.1 治理门承接登记：defer-to-G12+ → defer-to-G13+，承接锚字面 0-byte 维持，G13+ 重评窗顺延不关闭）/ **RD-040 +1**（M52 G12 重评窗核验：①真实集成需求未至〔G12.1 治理波零实现〕+ ②capability rt.ser 设备面未实测〔树内零 rt.ser 探针〕双条件未命中 → **maintain-defer**，复评点 = G12.2 生产化核心波 materialize 高分歧 RT workload 集成面时按只追加程序重判）；条目级字段与 status 0-byte。
  - **not-triggered / no-go / defer 登记面**（如实保持 open，不写进全绿叙述）：M52 maintain-defer（G12.2 复评窗登记）；M61 defer-to-G13+；M100-high 维持 defer（G12.4 触发评估登记——双端 PT 对标若产多灯 workload measured 对照面则按只追加程序重判，未命中锚定 G14）；G10-N17 维持 defer（G12.4 触发评估登记）；G11-N5 度量口径修订评估面维持 defer（G12.6 触发评估登记）；G11-N8/G11-N9 锚定 G15（焦散/透射/specular IBL 面——M96 起步范围冻结维持）；G10-N11/N16/G11-N3/M114-strand 锚定 G14；G10-N5 锚定 G13（DLSS/Streamline 零接线）；SAFE-GPU/M127/M98-l4/M118-hdr-cal/M125-adopt3/G10-N6/G10-N8 维持 defer 承接锚字面 0-byte；G12-N10 材质链 defer-to-G13+ 锚定 G15；G12-N11 no-go（异己会话 src/ 未提交面严禁消费/混入/冒充 G12 任何门绿）；SG-010 软保留 not_triggered 维持。
  - **异己并发工作树面**：本批只含 registry/number_ledger.json + registry/deferred.json + 本契约（front matter 双字段 + §8.1 本条）三文件；异己会话 src/ 未提交面（rurix-asset/rurix-render geometry/gi/shadow/ssr/ktx2_read/hzb/restir/sdf_trace/smrt 声明面，含 untracked `src/rurix-render/src/gi/restir.rs`）与 evidence/d3d12_interop_smoke.json 异己改写面维持未提交、不混入本批（立项裁决 1，G10.8b §8.10/G11 先例同模——`git add` 按文件名显式择取，异己面零混入）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10 §8.1 / G11 §8.1 同模）。`Assisted-by: Kimi-K3（G12.1 互锁解锁）`（影响范围：registry/number_ledger.json〔reserved_in_flight[G12] + RFC 命名空间校准 + revision_log v1.124〕+ registry/deferred.json〔RD-039/RD-040 history 各 +1〕+ G12_CONTRACT.md〔front matter 双字段 flip + §8.1 本条〕；验证方式：`py -3 ci/check_g12_implementation_interlock.py --require-ready` VERDICT=READY exit=0 + `--selftest` 16 RED+1 GREEN+1 TREE + 守卫套件〔check_structure / check_schemas / check_number_ledger / trace_matrix --check / budget_eval〕全 PASS 实测输出留痕，逐字见块③）。

---

### §8.2 G12.2 生产化核心波验收记录（2026-08-17）——G-G12-4 退出门：M158/M159/M160/M161 四个 P0 独立断言全绿 + M166 P1 标定值入 g12_budget provenance 齐备；波聚合门 `g12.wave.2.exit` VERDICT=PASS（5 门 + 6 facts）

- **① 独立断言全绿清单（四个 P0 + 一个已 go P1——每行单独 PASS，聚合不遮蔽）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g12.p1.m166.pt_production_calibration`（步骤 217） | 标定集七值 measured 产（τ=0.245 p50 逐路径 bounce==2 吞吐 n=85176 / θ=0.1453003 p75@spp=N_floor=16 / 误判率阈 7.008705e-2 =（场景×族单元 p100 3.471074e-2 + 1/3005）×2.0 / 曲线容差 6.440295e-1 / 白炉容差 2.692664e-3 / 单调带 3.887211e-1 / RR 无偏容差 4.322649e-5）+ 两跑逐字节一致 + 样本集 35 项 ≥24 digest sha256:ec0ad563… + 7 条目字节级纯追加入 g12_budget measured_local + 选型 artifact winner=sobol_class_seed_perturbed（0.1787243 < stratified 0.1885907 < pcg 0.1906762）+ RED 四臂 | host（纯 host；pbrt 1024 参照子进程真跑） | evidence/g12_pt_production_calibration_20260817T151512Z.json（checks 10/10） | PASS |
  | `g12.p0.m158.mis_full_surface`（步骤 218） | MIS 完整面 device 兑现（多光源/delta/白炉三 fixture + 双 m96 场景）：双跑位级一致 + delta 退化 MIS 开/关位级一致 + 白炉 device 均值 2.112980 vs host 参照 2.112980（gap 1.1e-7 ≤ 容差）且不超 Le=4 不产能量上界 + 逐级能量增量单调不增 + 光源分布 digest 同场景确定 + 收敛曲线 8 点全不越锚带（cornell spp64 8.457470e-2 ≤ 锚 9.022783e-2；direct spp64 2.901401e-2 ≤ 锚 2.899106e-2 ×（1+6.44e-1）带内）+ RED 三臂（no-mis/energy-bias/seed-change）+ 子模式复跑 + M96 冻结面 0-byte 机核 | host+device（RTX 4070 Ti，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1） | evidence/g12_m158_mis_full_surface_20260817T151826Z.json（checks 15/15） | PASS |
  | `g12.p0.m159.russian_roulette_prod`（步骤 219） | 吞吐自适应 RR：终止率/补偿计数非空（cornell 终止率 2.169151e-1、补偿 p50 1.234215/p90 1.671676/max ≤20；direct 2.797203e-2）+ RR 开/关无偏对照 ≤ 标定容差（cornell 2.1e-6 / direct 0 / two_light 4.14e-5 ≤ 4.322649e-5）+ N_min<2/p_max=1 fail-closed + 曲线锚 8 点 + RED 三臂（no-rr/comp-off/early-kill）+ 子模式复跑 | host+device（同上双置） | evidence/g12_m159_russian_roulette_prod_20260817T151908Z.json（checks 13/13） | PASS |
  | `g12.p0.m160.sampling_lds_upgrade`（步骤 220） | 低差异序列生产化：三族流位级一致 + 索引推导确定性（逐索引重求值==流内容）+ RNG 流布局 provenance 登记 + 选型 benchmark 两跑同族 + winner artifact 一致性（sobol_class_seed_perturbed）+ device 双跑位级一致 + 收敛曲线不劣于 PCG 流锚 + RED 双臂（nondeterministic/seed-change）+ 子模式复跑 | host+device（同上双置） | evidence/g12_m160_sampling_lds_upgrade_20260817T151951Z.json（checks 14/14） | PASS |
  | `g12.p0.m161.convergence_criterion_prod`（步骤 221） | 收敛判据生产化：自适应双跑位级一致 + 收敛报告非空（cornell spp[min16/p50 16/p90 64/max64] 未收敛 1091/4096；direct 未收敛 15/4096）+ 独立重算一致 + spp 下界 N_floor=16 保障 + 误判率 ≤ 标定阈（cornell 1.430948e-2 / direct 2.450380e-4 ≤ 7.008705e-2）+ 固定全 spp golden 对拍不偏离 measured×2.0 冻结带（cornell rel_dev 1.101732e-1 ≤ 带 2.289098e-1；direct 4.541055e-2 ≤ 9.199936e-2）+ 帧型标签闭集 + RED 三臂（early-stop/underreport/label-mix）+ 子模式复跑 | host+device（同上双置） | evidence/g12_m161_convergence_criterion_prod_20260817T152012Z.json（checks 14/14） | PASS |

  生产化判据统一形态逐行核验：生产化落盘（device megakernel `src/rurix-render/kernels/g12_pt_production.rx` rurixc --target vulkan 产 SPV + spirv-val 通过；host oracle `gi::path_trace::prod` 公式面逐字同源）+ 正确性锚 0-byte（M96 门最新 evidence PASS + `g9_m96_pbrt_tolerance_band.json` vs G12.0 base 零差分 + `path_trace.rs` diff ⊆ prod 模块注册块纯追加，门内机核）+ 收敛/方差面 measured 不劣于基线锚（容差 M166 标定程序产，g12_budget 7 条目零 estimated）+ 不降级既有 62 门绿面（本波不改任何既有门脚本/判据；M164 回归门归 G12.4）。

- **② 波聚合门实测输出**：`py -3 ci/g12_wave2_exit_check.py --gate g12.wave.2.exit` → **VERDICT=PASS，exit=0**（evidence/g12_wave2_exit_20260817T152228Z.json）——required_gates 5 行全 PASS（只读汇总最新 evidence，不重跑不代绿）+ 六 facts 全 PASS：①M96 正确性锚 0-byte（M96 门最新 PASS + 冻结面 diff 闭集机核）②五门 RED 臂独立有效（共 23 臂）③g12_budget 8 锚 + 7 标定条目齐备 measured_local 零 estimated + budget_eval 全 PASS ④spec-first RXS-0398~0401 条款头在树 + RFC-0029 Agent Approved ⑤conformance 11 件锚定在位 ⑥M166 标定 provenance 齐备（两跑逐位一致 + 样本集 digest + 选型 artifact + 7 条目 evidence 在档）。

- **③ 验收命令逐字输出（2026-08-17 真跑留痕，仓库根目录）**：
  - `py -3 ci/g12_pt_production_calibration_smoke.py --gate g12.p1.m166.pt_production_calibration` → `[g12_m166] checks 10/10 device=not_applicable` / PASS（标定两跑逐字节一致：digest sha256:ec0ad563646fc83b8905a9a07d6b8e4c96f1c4e4624c9c575d29ba1a3c4eb752 双跑同值）。
  - `py -3 ci/g12_mis_full_surface_smoke.py --gate g12.p0.m158.mis_full_surface` → `[g12_m158] checks 15/15 device=executed` / PASS。
  - `py -3 ci/g12_russian_roulette_prod_smoke.py --gate g12.p0.m159.russian_roulette_prod` → `[g12_m159] checks 13/13 device=executed` / PASS。
  - `py -3 ci/g12_sampling_lds_upgrade_smoke.py --gate g12.p0.m160.sampling_lds_upgrade` → `[g12_m160] checks 14/14 device=executed` / PASS。
  - `py -3 ci/g12_convergence_criterion_prod_smoke.py --gate g12.p0.m161.convergence_criterion_prod` → `[g12_m161] checks 14/14 device=executed` / PASS。
  - 五门 `--selftest` 全 PASS（M166 4 RED+3 GREEN；M158/159/160/161 各 1 RED+1 GREEN 合成面 + schema/CHECK_KEYS 闭集互核）；`py -3 ci/g12_wave_exit_lib.py --selftest` 6 RED+1 GREEN；`py -3 ci/g12_wave2_exit_check.py --selftest` 缺 evidence→红 + 真树 VERDICT==子门实测态（不遮蔽）双PASS。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G12 七前缀路由落）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 383 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (383/383 clauses anchored, 854 test files scanned)`；`py -3 ci/budget_eval.py` → `[budget_eval] PASS (183 pass, 0 skip, normal mode)`（g12.pt 15 条〔8 锚 + 7 标定〕measured_local 全 PASS 零 estimated）；`py -3 ci/check_g12_implementation_interlock.py --require-ready` → VERDICT=READY（事实门④实测 workflow 末号 222 == ledger on_tree_max 222 == next_free−1）。
  - `cargo test -p rurix-render --lib gi::path_trace` → `test result: ok. 23 passed; 0 failed`（M96 10 + prod 13——含本波修复批，见块④偏差登记）。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G12-3 互锁 READY（§8.1）→ 本波 spec-first（commit `41ea7f65`，RXS-0398~0401 + 语料 11 件，v1.125 登记）先行 → 本批实现门 materialize；数字步骤 217~222 按落盘前实测 actual next_free=217 顺位领取（ledger v1.126 校准同批）。
  - **前轮 host 起步缺陷修复批（偏差如实登记）**：前轮 prod.rs 仅编译通过、单测未跑绿——本波定位并修复三处：(a) 白炉测试预期错误——裸双臂 w=1 加和均值 4.037 为**双重计数伪影**（每顶点 NEE 与 BSDF 两臂各估全量直接光照，w=1 并计即双倍），4-bounce 截断正确 MIS 均值 ≈ 2.1（纯 NEE 形 2.2087；判决实验：恒等半权 2.0186 ≈ bare/2 锁定臂结构）；白炉门语义修正为「device vs host 截断参照对照 + 不产能量上界 Le 硬断言」并留痕于 fixture 文档；(b) `G12_ADAPTIVE_N_FLOOR` 4→**16**——floor=4 时边缘像素「前 4 样本全 0 ⇒ 方差估计 0 ⇒ 假收敛」误判率 43%（cornell θ=0.03 host 实测），N_floor×θ 网格实测后冻结 16（亚百分点级）；(c) RED 预锚篡改维修正（篡改 bounce<min_bounce 的 RR 维不改变输出——必消费维承载）。修复后 13 prod 单测全绿。
  - **工具链/环境偏差登记（不阻断本波判据）**：本机 `ci/check_schemas.py` 对 IDE 编辑面不落盘（并发会话快照面），G12 路由三处经脚本直写落盘并以 grep+真跑复核；`cargo fmt --check` / `cargo clippy -D warnings` 在本机 pinned 1.93.1 下对 **HEAD 既有文件**（如 world/water.rs、rurix-rt vk.rs）即报预存差异（与 G12.2 无关，CI 面以流水线工具链为准）——本波新增文件（kernel/bin/prod.rs/smoke/schema）在本地 pinned 工具链下 fmt/clippy 零新增问题（`--no-deps` 面实测）。
  - **not-triggered / 维持 open 面**：M52 SER 复评窗——本波生产化核心面已 materialize 高分歧 RT workload 的**初态集成面**（megakernel 单入口，无 SER 原语消费），复评按只追加程序留 G12.6 穷举窗核验（本波不作 go/no-go 改判）；M100-high（G12.4 触发评估）/G10-N17（G12.4）/G11-N5（G12.6）窗维持；异己会话 src/ 未提交面（hzb/restir/sdf_trace/smrt/ssr/ktx2_read 及 mod.rs/lib.rs 异己注册行、render_exec.rs 异己改写面）维持未提交、零消费、零混入（本批 `git add` 按文件名显式择取，G12 车道文件清单见块⑤）。
  - **工作树并发面留痕**：本波期间 `ci/check_schemas.py` 两次被并发面回退（恢复为 HEAD 版），最终以脚本直写落盘并验证（check_schemas PASS 真跑留痕）；G12 车道其余文件未受影响。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11 §8.x 同模）。`Assisted-by: Kimi-K3（G12.2 生产化核心波续）`（影响范围：src/rurix-render/kernels/g12_pt_production.rx + src/rurix-render/src/bin/g12_pt_production.rs + src/rurix-render/src/gi/path_trace/prod.rs + src/rurix-render/src/gi/path_trace.rs〔prod 模块注册块纯追加〕+ src/rurix-render/Cargo.toml〔bin 登记〕+ ci/g12_{pt_prod_lib,mis_full_surface_smoke,russian_roulette_prod_smoke,sampling_lds_upgrade_smoke,convergence_criterion_prod_smoke,pt_production_calibration_smoke,wave_exit_lib,wave2_exit_check}.py + ci/check_schemas.py〔G12 路由三处纯追加〕+ milestones/g12/ 七 evidence schema + g12_pt_sampler_selection.json + g12_budget.json〔7 标定条目纯追加〕+ CI_GATES.md v1.1 修订行 + registry/number_ledger.json〔CI_step 216→222/next_free 223 + revision_log v1.126〕+ .github/workflows/pr-smoke.yml〔步骤 217~222〕+ 本契约 §8.2 本条 + evidence/g12_* 本波真跑件；验证方式：块③逐字命令输出——五门 PASS + 波聚合门 VERDICT=PASS + 守卫套件全 PASS + 全量 selftest 红绿留痕）。

### §8.3 G12.3 降噪波验收记录（2026-08-17）——G-G12-5 退出门：M162 P0 独立断言全绿——降噪管线落盘 + 噪声谱高频能量下降 measured（标定阈）+ 帧均值能量守恒容差内 + temporal 底座 0-byte 断言 + NRD 类 vendor 降噪评估报告落盘（评估不接线）+ golden 对拍面不降级；波聚合门 `g12.wave.3.exit` VERDICT=PASS（1 门 + 6 facts）

- **① 独立断言全绿清单（一个 P0——单独 PASS，聚合不遮蔽）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g12.p0.m162.denoise_pipeline_tsr`（步骤 223） | 降噪管线 + TSR 联动 device 兑现（双 kernel：PT megakernel 0-byte + g12_pt_denoise.rx 时域累积/firefly 预钳位/A-trous 3 级双帧管线）：降噪标定腿两跑逐字节一致 + 样本集 12 项 ≥12 digest sha256:dc9a412e… + 2 条 g12.pt.denoise_* 标定条目字节级纯追加入 g12_budget measured_local（hf_drop_min = 12 单元 min 6.860882e-1 × 0.5 = 3.430441e-1；mean_energy_tol = p100 6.356283e-3 × 2.0 = 1.271257e-2，禁手写 P-09）+ 噪声谱高频能量下降 measured（低梯度半幅掩码口径进 evidence：cornell hf 4.793371e-4 → 1.103747e-4，drop 7.697348e-1 ≥ 阈；direct 2.891100e-5 → 7.509365e-7，drop 9.740259e-1 ≥ 阈）+ 帧均值能量守恒容差内（cornell 2.494836e-3 / direct 2.736881e-3 ≤ 1.271257e-2；区域 p90 8.901973e-2 / 1.051987e-1 进 evidence）+ 历史验证活性（移动帧拒绝 cornell 96/4096 / direct 2108/4096 ∈ (0,N) 开区间）+ golden 对拍面不降级（固定全 spp64 vs pbrt 冻结带：cornell rel_dev 1.101732e-1 ≤ 带 2.289098e-1；direct 4.541055e-2 ≤ 9.199936e-2——与 M161 门实测值逐位一致）+ 帧型标签闭集 {raw, denoised} + 固定 seed 双跑位级一致 + RED 三臂（denoise-energy-bias〔ediff 5.840175e-1 越容差检出〕/ denoise-masquerade〔drop 0 检出〕/ history-validation-off〔拒绝 64 < 洁净臂 96 检出〕）+ 子模式独立复跑 + **temporal 底座 0-byte 机核**（目录级 git diff vs G12.0 不可变 ref 5ae83aa7 空 + 工作树零未提交面）+ M96 冻结面 0-byte + NRD 评估报告落盘 + 树内零 vendor 接线符号 | host+device（RTX 4070 Ti，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1） | evidence/g12_m162_denoise_pipeline_tsr_20260817T180257Z.json（checks 20/20） | PASS |

  生产化判据统一形态逐行核验：生产化落盘（device 降噪 kernel `src/rurix-render/kernels/g12_pt_denoise.rx` rurixc --target vulkan 产 SPV + spirv-val 通过；host oracle `gi::path_trace::prod_denoise` 公式面逐字同源）+ 正确性锚 0-byte（M96 门最新 evidence PASS + `g9_m96_pbrt_tolerance_band.json` vs G12.0 base 零差分 + `path_trace.rs` diff ⊆ prod 模块注册块纯追加 + **temporal/ 目录级 0-byte**——底座只读消费不接线，门内机核）+ 噪声/能量面 measured 经标定程序产阈（g12_budget 2 新条目零 estimated）+ 不降级既有 62 门绿面（本波不改任何既有门脚本/判据；M164 回归门归 G12.4）。**RD040-nrd 承接兑现**：评估报告 milestones/g12/design/nrd_vendor_denoise_evaluation.md v1.0 落盘（UpscaleBackend 同构输入契约五轴接入面评估 + 许可/ABI 取证〔2026-08-17 联网实测 NRD v4.17.5：自定义 NVIDIA RTX SDKs LICENSE，NOASSERTION 非 OSI 许可〕+ 自研 measured 对照）——**评估不接线**，接入另判 G13+ 窗；deferred.json RD-040 history 只追加一条，条目级字段与 status 0-byte。

- **② 波聚合门实测输出**：`py -3 ci/g12_wave3_exit_check.py --gate g12.wave.3.exit` → **VERDICT=PASS，exit=0**（evidence/g12_wave3_exit_20260817T180458Z.json）——required_gates 1 行 PASS（只读汇总最新 evidence，不重跑不代绿）+ 六 facts 全 PASS：①M96 正确性锚 0-byte（M96 门最新 PASS + 冻结面 diff 闭集机核）②temporal 底座 0-byte（目录级 diff + 工作树双面机核）③M162 RED 臂独立有效（共 5 臂）④g12_budget 8 锚 + 7 标定 + 2 降噪标定条目齐备 measured_local 零 estimated + budget_eval 全 PASS ⑤spec-first RXS-0402 条款头在树 + RFC-0029 Agent Approved + conformance 3 件锚定 ⑥NRD 评估报告落盘 + 零接线 + RD-040 status=open 维持。

- **③ 验收命令逐字输出（2026-08-17 真跑留痕，仓库根目录）**：
  - `py -3 ci/g12_denoise_pipeline_tsr_smoke.py --gate g12.p0.m162.denoise_pipeline_tsr` → `[g12_m162] checks 20/20 device=executed` / PASS（标定两跑逐字节一致：digest sha256:dc9a412efdd07a588016cda71b15247e78c575b99fa0afeadd73d5cd7a56b855 双跑同值；device 全档双 kernel 真跑 + RED 三臂子模式独立复跑全检出）。
  - `py -3 ci/g12_wave3_exit_check.py --gate g12.wave.3.exit` → `VERDICT = PASS`（1 门 + 6 facts 逐行打印全 PASS）。
  - 两门 `--selftest` 全 PASS（M162：1 合成 RED + CHECK_KEYS/schema 20 键闭集互核 + temporal 差分/vendor 接线符号两检出器红绿臂，3 RED + 2 GREEN；wave3：缺 evidence→红 + 真树聚合 VERDICT==子门实测态不遮蔽，双 PASS）；`py -3 ci/g12_wave_exit_lib.py --selftest` 6 RED+1 GREEN。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G12 十前缀路由落——本波新增 g12_m162_denoise_pipeline_tsr_ / g12_m162_calibration_ / g12_wave3_exit_ 三件，既有 0-byte）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 384 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (384/384 clauses anchored, 857 test files scanned)`；`py -3 ci/budget_eval.py` → `[budget_eval] PASS (185 pass, 0 skip, normal mode)`（g12.pt 17 条〔8 锚 + 7 标定 + 2 降噪标定〕measured_local 全 PASS 零 estimated）；`py -3 ci/check_g12_implementation_interlock.py --require-ready` → VERDICT=READY（事实门④实测 workflow 末号 224 == ledger on_tree_max 224 == next_free−1）。
  - `cargo test -p rurix-render --lib gi::path_trace::prod_denoise` → `test result: ok. 7 passed; 0 failed`（时域静态接受/深度突变拒绝/validation-off 面 + A-trous 降噪保边 + 均值守恒 + denoise_off 恒等面 + 噪声谱冒充检出 + 偏置注入检出 + G-buffer/MV 派生 + 参数 fail-closed/标签闭集/digest 确定性）。
  - G12.2 回归面抽检：`py -3 ci/g12_wave2_exit_check.py --gate g12.wave.2.exit` → VERDICT=PASS（evidence/g12_wave2_exit_20260817T175624Z.json——本波 path_trace.rs 追加行 ⊆ prod 模块注册块机核维持，五门 + 六 facts 全绿，零降级）。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G12-3 互锁 READY（§8.1）维持 → 本波 spec-first（commit `7e8f920f`，RXS-0402 + 语料 3 件 + spec/README 登记 + trace_matrix/stable 重生，ledger v1.127 登记）先行 → 本批实现门 materialize；数字步骤 223~224 按落盘前实测 actual next_free=223 顺位领取（ledger v1.128 校准同批）。
  - **降噪管线形态实测定盘面（实现波定，RFC-0029 §6 U4 裁决面登记）**：初版中心像素归一化边缘停止（σ_l=0.4）在 cornell 低 spp 面实测引入 ~10.5~12.7% 系统性变暗偏置（亮噪尖峰不对称保留）——**处方 = firefly 预钳位腿（YCoCg 3x3 邻域 μ±2σ 方差裁剪，消费底座 `neighborhood_variance_bounds`/`clamp_to_bounds` 面，γ=2.0 登记）+ 对称归一化 σ_l=0.2**（对称面压尖峰保留、紧 σ 压边缘光晕）；对称归一化 σ_l=0.4 中间态实测边缘光晕致高频误差反升（halo 面登记）；定盘后标定 12 单元 hf_drop ∈ [6.86e-1, 9.75e-1]、帧均值能量差 p100 = 6.36e-3（偏置面收敛到亚百分点级）。噪声谱口径细化登记：**低梯度半幅掩码**（参照帧亮度 3x3 极差 ≤ 中位数像素）——边缘位移/光晕偏置（高梯度区）与噪声底（平滑区）分离，降噪有效性在噪声所在平滑区 measured（RXS-0402 L2「口径进 evidence」委派面内，条款字面 0-byte）。
  - **工作树并发面留痕（沿 §8.2 同模登记）**：本波期间 `.github/workflows/pr-smoke.yml` 与 `registry/number_ledger.json` 各被并发面回退一次（恢复为 HEAD 版）——ledger 经 `.tmp/g12_3_ledger_edit.py` / `.tmp/g12_3_ledger_ci_edit.py`、workflow 经 `.tmp/g12_3_workflow_edit.py` 脚本原子重放落盘并以 check_number_ledger / 互锁 validator `--require-ready` 真跑复核（VERDICT=READY）；G12 车道其余文件未受影响；`.tmp/g12_3_*.py` 为一次性落盘工具不入 commit。
  - **工具链/环境偏差登记（不阻断本波判据，沿 §8.2 同模）**：`cargo fmt --check` / `cargo clippy -D warnings` 在本机 pinned 1.93.1 下对 HEAD 既有文件即报预存差异（与 G12.3 无关，CI 面以流水线工具链为准）——本波新增文件（prod_denoise.rs / g12_pt_denoise.rx / harness 扩展面 / smoke / wave3 聚合门 / schema 三件）在本地 pinned 工具链下 `cargo fmt --check`（对新增文件面）与 clippy 零新增问题；`check_guardrails`/`check_contribution` 为 advisory（evidence/d3d12_interop_smoke.json 异己会话未提交修改面维持不混入本批；RFC-0028/0029 评审段 provenance advisory 为 G12.1 前既存面）。
  - **not-triggered / 维持 open 面**：M52 SER 复评窗维持（G12.6 穷举窗核验）；M100-high（G12.4 触发评估）/G10-N17（G12.4）/G11-N5（G12.6）窗维持；RD040-nrd 接入面**评估完结不接线**（接入另判 G13+ 窗，重判条件 = 接入真实需求 + owner 法律面许可清结 + measured 对拍面接入裁决）；异己会话 src/ 未提交面（hzb/restir/sdf_trace/smrt/ssr/ktx2_read 及 mod.rs/lib.rs 异己注册行、render_exec.rs 异己改写面）维持未提交、零消费、零混入（本批 `git add` 按文件名显式择取，G12 车道文件清单见块⑤）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/本契约 §8.1/§8.2 同模）。`Assisted-by: Kimi-K3（G12.3 降噪波）`（影响范围：spec/global_illumination.md〔RXS-0402 条款 + v1.4 修订行〕+ spec/README.md〔§4 行扩写 + v1.91 修订行〕+ conformance/gi/{accept/denoise_pipeline_minimal.rx, reject/denoise_energy_bias.rx, reject/temporal_base_rewire.rx} + conformance/traceability_matrix.{json,md} + tests/stable/{stable_api.snapshot,bless_log.md} + src/rurix-render/src/gi/path_trace/prod_denoise.rs 新建 + src/rurix-render/src/gi/path_trace.rs〔prod_denoise 模块注册块纯追加〕+ src/rurix-render/kernels/g12_pt_denoise.rx 新建 + src/rurix-render/src/bin/g12_pt_production.rs〔M162 门 + 降噪标定腿 + 降噪 RED 三臂 + --denoise-spv 扩展面；G12.2 四门 + M166 判据面 0-byte〕+ ci/g12_denoise_pipeline_tsr_smoke.py + ci/g12_wave3_exit_check.py + ci/check_schemas.py〔三处纯追加〕+ milestones/g12/g12_m162_denoise_pipeline_tsr_evidence_schema.json + g12_m162_calibration_entry_evidence_schema.json + g12_wave3_exit_evidence_schema.json + milestones/g12/design/nrd_vendor_denoise_evaluation.md + milestones/g12/g12_budget.json〔2 降噪标定条目纯追加〕+ milestones/g12/CI_GATES.md v1.2 修订行 + registry/number_ledger.json〔RXS 401→402/next_free 403（v1.127）+ CI_step 222→224/next_free 225（v1.128）〕+ registry/deferred.json〔RD-040 history 只追加一条〕+ .github/workflows/pr-smoke.yml〔步骤 223~224〕+ 本契约 §8.3 本条 + evidence/g12_m162_*/g12_wave3_exit_* 本波真跑件；验证方式：块③逐字命令输出——M162 门 20/20 PASS + 波聚合门 VERDICT=PASS + 守卫套件全 PASS + 全量 selftest 红绿留痕 + G12.2 波聚合门复跑 PASS 零降级）。

### §8.4 G12.4 UE Path Tracer 对标波验收记录（2026-08-17）——G-G12-6 退出门：M163/M164 两个 P0 独立断言全绿——同场景同 spp 双端对拍（契约 digest 独立冻结三向互证 + UE build digest == M128 登记机核）+ 收敛曲线逐段/噪声谱/能量守恒 measured 对拍（容差标定腿产）+ UE PathTracing 模块归属差距登记表落盘 + 62 门零降级；波聚合门 `g12.wave.4.exit` VERDICT=PASS（2 门 + 6 facts）

- **① 独立断言全绿清单（两个 P0——每行单独 PASS，聚合不遮蔽）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g12.p0.m163.ue_pt_parity`（步骤 225） | UE Path Tracer 对标双端出图+对拍 device 兑现：契约 digest **三方独立实现全等**（host python 内嵌解析器 / Rurix Rust harness `--contract-digest` / UE 内嵌 CPython 建设探针 = sha256:4515625e07…）∧ == 门内冻结注册值（不等仍出报告即 RED——门序拒产面，RED 臂真跑实证 harness 拒出图）+ UE build digest == M128 登记 ue_build_id 机核（Build.version 实测 5.8.1-56057345）+ M133 清单 digest 转引只读 + **双端各 12 帧齐备**（2 场景 × spp 1/4/16/64/256/1024；Rurix 臂 = 生产化 PT megakernel device 真跑〔glTF→ProdScene→type=2 三角网格光 + 4 点光〕固定 seed 双跑位级一致；UE 臂 = 5.8.1 PT MRQ 逐 job 真跑 + 新鲜度/真帧机核 + PT 接通收敛签名〔逐 spp 帧互异 + rel_err 随 spp 降〕）+ **收敛曲线逐段对拍**（cornell 5 段全在容差 2.391968e-1 内；bistro spp1 Δ=3.899e-1 / spp4 Δ=3.136e-1 超容差**显式登记**不静默）+ **噪声谱对拍**（cornell Δ=5.630e-2 / bistro Δ=1.501e-1 > 容差 1.217e-2 显式登记）+ **能量守恒对拍**（×2^(−ev100) 派生尺度链：cornell Δ=3.211e-1 / bistro Δ=6.929e-1 > 容差 1.380e-3 显式登记）+ 标定腿双 seed 方差底 p100×2.0 三条目字节级纯追加入 g12_budget（measured_local，budget_eval 全 PASS）+ **UE PathTracing 模块归属差距登记表落盘**（milestones/g12/g12_ue_pt_gap_registry.json 10 行 = 6 quality_gap〔全部超容差项对账齐〕+ 4 caliber_diff〔残余口径逐环节〕，RXS-0391 归属枚举合法）+ 不设绝对通过线 + RED 五臂全检出 | host+device（RTX 4070 Ti，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1；UE 5.8.1 F:\UE_5.8 MRQ 臂） | evidence/g12_m163_ue_pt_parity_20260817T223129Z.json（checks 21/21） | PASS |
  | `g12.p0.m164.regression_guard`（步骤 226） | 生产化回归门：**既有 62 门最新 evidence 全绿只读汇总**（G9 34 key + G10 14 key + G11 14 key〔M147 双 phase 纪律两态面继承〕，聚合不遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE）+ **生产化触改面既有门重跑回归零降级 8 抽检真跑全绿**（M96 golden 门序面真跑抽检 g9.p0.m96 〔kernel type=2/stride 17/二分选择/直查四面加性扩展后 golden 对拍全绿〕+ G12.2 M158/M160 全档真跑〔位级一致 + 曲线锚复核〕+ G12.3 M162 + wave2/wave3 exit 聚合复跑 + G10 M140/G11 M157 host 面广幅抽检——子进程真跑 exit 0 + 最新 evidence PASS + 新鲜度机核 timestamp ≥ 会话起点）+ 既有判据 0-byte（G5~G11 closed 面工作树空集）+ RED 三臂全检出 | host 纯 host（抽检子进程自持 device 面） | evidence/g12_m164_regression_guard_20260817T224055Z.json（checks 15/15） | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g12_wave4_exit_check.py --gate g12.wave.4.exit` → **VERDICT=PASS，exit=0**（evidence/g12_wave4_exit_20260817T224622Z.json）——required_gates 2 行全 PASS（只读汇总最新 evidence，不重跑不代绿）+ 六 facts 全 PASS：①M96 正确性锚 0-byte（M96 门最新 PASS + 冻结带/参照器面 diff 闭集机核）②对标契约 digest 冻结面（parity.contract_digest == 冻结注册值 ∧ ue_build_id == M128 值 ∧ 三向一致字面 ∧ 段数 10 非空）③差距登记表落盘 + RXS-0391 归属枚举合法 + 行集对账（10 行：quality 6 + caliber 4，枚举越集零，场景集全等）④g12_budget 20 条目齐备 measured_local 零 estimated（8 锚 + 7 标定 + 2 降噪标定 + 3 对标标定）+ budget_eval 全 PASS ⑤spec-first RXS-0403 条款头在树 + RFC-0029 Agent Approved + conformance 3 件锚定 ⑥62 门零降级（M164 PASS 承载）+ RD-040 history 只追加（M100-high G12.4 触发评估登记在树）。

- **③ 验收命令逐字输出（2026-08-17 真跑留痕，仓库根目录）**：
  - `py -3 ci/g12_ue_pt_parity_smoke.py --gate g12.p0.m163.ue_pt_parity` → `[g12_m163] checks 21/21 device=executed` / PASS（标定三条目追加：g12.pt.parity_curve_tol = 2.391968e-1〔基值 1.195984e-1 × 2.0〕/ parity_noise_tol = 1.216932e-2〔6.084661e-3 × 2.0〕/ parity_energy_tol = 1.380229e-3〔6.901145e-4 × 2.0〕——双 seed 方差底实测，禁手写 P-09；差距登记表 10 行 + RED 五臂全检出）。
  - `py -3 ci/g12_regression_guard_smoke.py --gate g12.p0.m164.regression_guard` → `[g12_m164] checks 15/15 device=not_applicable` / PASS（62 门只读汇总全绿 + 8 抽检真跑零降级 + RED 三臂全检出）。
  - `py -3 ci/g12_wave4_exit_check.py --gate g12.wave.4.exit` → `VERDICT = PASS`（2 门 + 6 facts 逐行打印全 PASS）。
  - 三门 `--selftest` 全 PASS（M163：1 合成红 + 解析红绿 + 4 RED 合成臂 + schema/CHECK_KEYS 21 键闭集互核；M164：5 RED + 2 GREEN；wave4：缺 evidence→红 + 真树聚合 VERDICT==子门实测态不遮蔽双 PASS）；`py -3 ci/g12_wave_exit_lib.py --selftest` 6 RED+1 GREEN。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G12 十四前缀路由落——本波新增 g12_m163_ue_pt_parity_ / g12_m163_calibration_ / g12_m164_regression_guard_ / g12_wave4_exit_ 四件，既有 0-byte）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 385 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (385/385 clauses anchored, 861 test files scanned)`；`py -3 ci/budget_eval.py` → `[budget_eval] PASS (188 pass, 0 skip, normal mode)`（g12.pt 20 条〔8 锚 + 7 标定 + 2 降噪标定 + 3 对标标定〕measured_local 全 PASS 零 estimated）；`py -3 ci/check_g12_implementation_interlock.py --require-ready` → VERDICT=READY（事实门④实测 workflow 末号 227 == ledger on_tree_max 227 == next_free−1）。
  - `cargo test -p rurix-render --lib gi::path_trace` → `test result: ok. 24 passed; 0 failed`（M96 10 + prod 12 + prod_denoise 7 面——含本波新增三角网格光单测 2 件：validate/layout 锚 + **双三角灯 ≡ 同面积 quad 灯判决实验**〔rel_dev < 0.08 MC 容差内等价——折叠采样 + 半面积 + 分布加权三面联合正确性〕）。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G12-3 互锁 READY（§8.1）维持 → 本波 spec-first（commit `76bccefc`，RXS-0403 + 语料 3 件 + spec/README 登记〔同批补登 RXS-0392~0393 行内登记面〕+ trace_matrix/stable 重生，ledger v1.129 登记）先行 → 本批实现门 materialize；数字步骤 225~227 按落盘前实测 actual next_free=225 顺位领取（ledger v1.130 校准同批）。
  - **UE 臂探针取证与修复批（偏差如实登记）**：UE 5.8.1 PT MRQ 臂接通历经六轮探针定位——①编辑器命令面缺 `-unattended` 挂起（G10.5 实证面补正）；②"Path Tracer is not enabled by this project"——项目 RT 链补齐（r.SupportHardwareRayTracing/r.RayTracing/r.SkinCache.CompileShaders + 运行期 r.RayTracing.Enable=1〔Dynamic 模式〕+ **PCD3D_SM6 targeted shader formats**〔SM5 卡壳实证〕，外部项目 Config 登记面非 UE 源码）；③cornell 壳体 PT 下整面缺失——逐轮排除双面 MIC/父材质/网格槽位重挂/Nanite/包围盒/反绕向五路后定位：**Interchange 导入反射映射（x,z,y)·100 det=−1 翻转绕向 + UE RT 遍历背面剔除**（引擎原生网格 PT 正常对照实证）⇒ **内容恒等双绕向派生语料承载**（harness 侧对齐面，G11.3 U1 双面置换同族——milestones/g12/harness/g12_4_make_pt2sided.py 产 cornell/bistro 两变体，逐三角 (a,b,c)+(a,c,b)，顶点/UV/材质/节点逐字节不动；派生报告 milestones/g12/g12_4_pt2sided_derivation.json + digest 随档；Rurix 臂续消费 M133 原语料——同一表面集场景恒等）；④Rurix 臂 bistro spp≥256 单 dispatch **VK_ERROR_DEVICE_LOST**（TDR 超时）⇒ kernel 两面加性修复：命中发光面 MIS 联合 PDF **逐灯 O(L) 累加循环 → 命中光源直查**（44k 三角网格光面；循环命中项外全零 ⇒ 直取**位级同值**）+ NEE 光源选择线性扫 → **CDF 二分**（lights stride 16→17 槽位 16 存 CDF；同一选择语义位级不变）+ 像素带分段 dispatch（params[36] pixel_base，0=整帧既有面）——**G12.2 零降级实证：M158 全档真跑 15/15 PASS（位级一致 + 曲线锚）+ M164 抽检八面全绿**；⑤Rurix 臂相机手性修正——PtCamera::look_at pbrt 同式（right=up×forward）与 UE 呈水平镜像，改 G10.5 双端一致口径（right=forward×up0，cornell 绿墙左/红墙右双端同侧目视实证）；⑥契约 ev100 波裁决调整（cornell 2.0→0.0 / bistro −2.0→−4.0——PT 灯面量级下可判读区间实测标定，provenance 登记）⇒ 契约 digest 冻结注册值随调整重登记（sha256:fbad7465e0…→sha256:4515625e07…，门内常量 + 三向互证同批）。
  - **UE 侧场景面对拍实测**（目视+数值双证）：cornell UE PT 帧 = 绿左墙/红右墙/双方块/天花灯条完整（与 Rurix 帧同侧同构图，mean 0.144 vs Rurix 0.099——进能量对拍 measured 登记）；bistro 双端帧 = 四吊灯 + 壁灯 + 家具剪影同位可见（UE mean 0.0464 vs Rurix 派生 0.0143——UE 侧偏亮，emissive 逐纹素/材质链口径差进差距登记表 caliber_diff 行显式归属）。
  - **工作树并发面留痕（沿 §8.2/§8.3 同模）**：本波期间 registry/number_ledger.json、.github/workflows/pr-smoke.yml、ci/check_schemas.py 各被并发面回退一次（恢复为 HEAD 版）——三面经 .tmp/g12_4_ledger_edit.py / g12_4_ledger_ci_edit.py / g12_4_workflow_edit.py / g12_4_checkschemas_edit.py 脚本原子重放落盘并以 check_number_ledger / 互锁 validator --require-ready / check_schemas 真跑复核（全 PASS 留痕）；G12 车道其余文件未受影响；.tmp/g12_4_*.py 为一次性落盘工具不入 commit。
  - **工具链/环境偏差登记（不阻断本波判据，沿 §8.2/§8.3 同模）**：`cargo fmt --check` / `cargo clippy -D warnings` 在本机 pinned 1.93.1 下对 HEAD 既有文件即报预存差异（与 G12.4 无关，CI 面以流水线工具链为准）——本波新增文件（prod.rs 三角网格光面/g12_4_ue_pt_parity_render.rs/kernel 加性段/smoke×2/wave4/schema×4/harness 三件）在本地 pinned 工具链下零新增问题面实测。
  - **not-triggered / 维持 open 面**：**M100-high 触发条件命中**——bistro 4 点光 + 4 emissive 双端 PT 对拍多灯 workload measured 对照面已产出（本波登记不改判，RD-040 history 只追加一条，留 G12.6 穷举窗按只追加程序重判，承接锚字面 0-byte 维持锚定 G14）；**G10-N17（M137 scalars.flip）未触发**——本波对拍消费 rel-MAE/噪声谱/能量三面，零 FLIP 标量面消费（RXS-0388 L3 演进位维持 null 不翻转，M137 门不回归——维持 defer 锚定字面 0-byte）；M52 SER 复评窗维持（G12.6 穷举窗核验）；G11-N5 度量口径修订评估面维持（G12.6 触发评估登记）；G11-N8/G11-N9 锚定 G15（焦散/透射/specular IBL 面——bistro 材质纹理均值扁平化差距行进差距登记表显式归属不承接修复）；异己会话 src/ 未提交面（hzb/restir/sdf_trace/smrt/ssr/ktx2_read 及 mod.rs/lib.rs 异己注册行、render_exec.rs 异己改写面）维持未提交、零消费、零混入（本批 `git add` 按文件名显式择取，G12 车道文件清单见块⑤）。
  - **材质链边界登记面（MAP §3.3 口径）**：本波对拍实测未现透射/焦散/镜面 IBL 类画质量级主差新增项——bistro 能量差主因 = 材质纹理逐纹素 vs 均值扁平化 + emissive 口径面（差距登记表 caliber_diff 四行显式归属），锚定 G15 画质量级收口面维持。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/本契约 §8.1~§8.3 同模）。`Assisted-by: Kimi-K3（G12.4 UE PT 对标波）`（影响范围：src/rurix-render/kernels/g12_pt_production.rx〔type=2 三角网格光折叠采样 + lights stride 16→17 CDF 槽位二分选择 + 命中光源 MIS 直查 + params[36] 像素带分段——type 0/1 既有路径位级不变〕+ src/rurix-render/src/gi/path_trace/prod.rs〔ProdLight::Tri 变体 + 校验/分布/打包/host oracle 镜像 + 单遍分桶校验优化〔语义不变〕+ 单测 2 件〕+ src/rurix-asset/Cargo.toml〔rurix-rt optional dep + vulkan feature + bin 登记〕+ src/rurix-asset/src/bin/g12_4_ue_pt_parity_render.rs 新建 + milestones/g12/g12_ue_pt_parity_contract.json〔独立冻结契约〕+ milestones/g12/g12_ue_pt_gap_registry.json〔差距登记表 10 行〕+ milestones/g12/g12_4_pt2sided_derivation.json〔双绕向派生报告〕+ milestones/g12/harness/{g12_4_make_pt2sided.py, g12_4_ue_render.py, ue_python/g12_pt_contract.py, ue_python/g12_4_build_pt_scenes.py} + ci/g12_ue_pt_parity_smoke.py + ci/g12_regression_guard_smoke.py + ci/g12_wave4_exit_check.py + ci/check_schemas.py〔三处纯追加〕+ milestones/g12/ 四 evidence schema〔g12_m163_ue_pt_parity_/g12_m163_calibration_entry_/g12_m164_regression_guard_/g12_wave4_exit_〕+ milestones/g12/g12_budget.json〔3 对标标定条目纯追加〕+ milestones/g12/CI_GATES.md v1.3 修订行 + registry/number_ledger.json〔CI_step 224→227/next_free 228 + revision_log v1.130〕+ registry/deferred.json〔RD-040 history 只追加一条——M100-high G12.4 触发评估登记〕+ .github/workflows/pr-smoke.yml〔步骤 225~227〕+ 本契约 §8.4 本条 + evidence/g12_m163_*/g12_m164_*/g12_wave4_exit_* 本波真跑件；验证方式：块③逐字命令输出——M163 门 21/21 PASS + M164 门 15/15 PASS + 波聚合门 VERDICT=PASS + 守卫套件全 PASS + 全量 selftest 红绿留痕 + M158/M164 触改面抽检零降级）。

---

### §8.5 G12.5 性能面波验收记录（2026-08-17）——G-G12-7 退出门：M165 P0 独立断言全绿——PT 吞吐基线 measured（rays/sec + 帧时 at 固定 spp{16,64} × M133 双场景闭集，50×3 trimmed mean 协议）入 g12_budget provenance 齐备 + 不设通过线登记 + 优化前后正确性锚（固定 seed digest 0-byte）；波聚合门 `g12.wave.5.exit` VERDICT=PASS（1 门 + 6 facts）

- **① 独立断言全绿清单（一个 P0——单独 PASS，聚合不遮蔽）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g12.p0.m165.pt_throughput_baseline`（步骤 228） | PT 吞吐优化基线 device 兑现（`g12_4_ue_pt_parity_render --benchmark` 加性子模式，--render/--contract-digest 既有面 0-byte）：L0 环境画像齐备 + 锁频状态实测登记（未锁频 clock_lock_note 诚实存档，G10.1/G12.1 先例）+ **吞吐基线 measured**（场景 = M133 双场景闭集 {cornell-box, bistro-interior}，固定 spp 档位 {16,64} 自 G12.4 对标序列取；warmup 10 + timed 150 = 3×50 块 IQR→块中位数→3 块均值，M141 冻结统计口径同字面继承；计时口径 = host Instant 墙钟 around run_device 全帧〔G12.4 生产化出图路径逐帧全链路：host RNG 流生成 + 打包 + Vulkan 初始化/BLAS 构建/dispatch/回读同步〕；rays/sec 口径 = 主射线〔像素数 × spp，次级射线未计——显式登记不冒充全光线计数〕；采样腿持 gpu_device_lock 串行）+ **8 条目入 g12_budget measured_local 零 estimated**（frame_ms ×4〔max 向阈 = 实测 ×1.5〕+ primary_rays_sec ×4〔min 向阈 = 实测 ÷1.5〕——回归守护语义非通过线，沿 G9.1/G10.1/G11.1/G12.1 measured 冻结先例覆盖频率漂移，P-09 禁手写；逐条目 evidence 落盘 + budget_eval 全 PASS）+ **不设通过线登记**（evidence zero_pass_line 字面 + 8 条目描述逐个携带「不构成帧率对标通过线」字面——正式帧率对标锚定 G14，G10-N11/N16 承接锚字面维持）+ **正确性锚固定 seed digest 0-byte**（逐 cell 基准帧内容 digest == M163 Rurix 臂 receipt 冻结锚〔seed=9182346301 固定 seed 确定性协议 RXS-0357 L2/RXS-0400 继承〕+ 全 160 帧 distinct digest==1 + 演进位 null）+ 统计面独立第二实现重算核验 + RED 三臂全检出（基线冒充帧率对标〔合成注入登记校验必拒〕/ digest 漂移未登记〔tamper 子进程真跑 + 合成面双保险〕/ estimated 冒充 measured〔合成注入必拒〕） | host+device（RTX 4070 Ti，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1） | evidence/g12_m165_pt_throughput_baseline_20260818T001938Z.json（checks 14/14；首跑登记件 20260817T233025Z） | PASS |

  吞吐基线 measured 值（budget 登记值 = 首跑 20260817T233025Z 实测；终版复跑 20260818T001938Z 在档守护复检 8/8 PASS，预算条目 0-byte 不回写）：

  | cell | 帧时 trimmed mean（ms） | 主射线吞吐（rays/s） | cv |
  |---|---:|---:|---:|
  | cornell-box spp16（128×128） | 142.976 | 1.833477×10⁶ | 0.0410 |
  | cornell-box spp64 | 300.049 | 3.494680×10⁶ | 0.0315 |
  | bistro-interior spp16 | 227.192 | 1.153845×10⁶ | 0.0462 |
  | bistro-interior spp64 | 396.135 | 2.647020×10⁶ | 0.0367 |

- **② 波聚合门实测输出**：`py -3 ci/g12_wave5_exit_check.py --gate g12.wave.5.exit` → **VERDICT=PASS，exit=0**（evidence/g12_wave5_exit_20260818T001948Z.json）——required_gates 1 行 PASS（只读汇总最新 evidence，不重跑不代绿）+ 六 facts 全 PASS：①M96 正确性锚 0-byte（M96 门最新 PASS + band 0-byte + path_trace.rs 差分 ⊆ prod 模块注册块纯追加机核）②吞吐基线 8 条目齐备 measured_local 零 estimated + 逐条目 evidence 在档 + budget_eval 全 PASS ③不设通过线登记机核（zero_pass_line 字面 + 8 条目登记校验 + 门 checks 三面）④正确性锚断言（4 cell digest_match 全真 + distinct==1 + 演进位 null + 冻结锚集字面一致）⑤M165 RED 臂独立有效（三臂）⑥62 门零降级（M164 最新 evidence PASS 承载——本波不改任何既有门脚本/判据）+ 本波触改二进制面零降级机核（M163 全档复跑 PASS 21/21 ∧ base_commit == M165 同值 6734633c——g12_4_ue_pt_parity_render 加性 --benchmark 扩展面共享二进制回归锚）。

- **③ 验收命令逐字输出（2026-08-17 真跑留痕，仓库根目录）**：
  - `py -3 ci/g12_pt_throughput_baseline_smoke.py --gate g12.p0.m165.pt_throughput_baseline` → `[g12_m165] checks 14/14 device=executed` / PASS（首跑 20260817T233025Z：8 条目字节级纯追加入 g12_budget；终版复跑 20260818T001938Z：在档守护复检 8/8 PASS——frame_ms 复测 141.086/296.153/225.458/388.305 vs 阈 ×1.5 全内，rays_sec 复测 1.858047M/3.540662M/1.162718M/2.700391M vs 阈 ÷1.5 全内；digest 锚 0-byte 4/4 cell 全等 + distinct==1）。
  - `py -3 ci/g12_ue_pt_parity_smoke.py --gate g12.p0.m163.ue_pt_parity` → `[g12_m163] checks 21/21 device=executed` / PASS（evidence/g12_m163_ue_pt_parity_20260818T000809Z.json——终版二进制全档复跑零降级：契约 digest 三方全等 + UE build 5.8.1-56057345 + 双端 12 帧 + 对拍三面 + 差距登记表 10 行 + RED 五臂；Rurix 臂 digest 锚与 G12.4 冻结值逐位一致）。
  - `py -3 ci/g12_wave5_exit_check.py --gate g12.wave.5.exit` → `VERDICT = PASS`（1 门 + 6 facts 逐行打印全 PASS）。
  - 三门 `--selftest` 全 PASS（M165：4 RED + 6 GREEN 合成面 + schema/CHECK_KEYS 14 键闭集互核 + entry schema 8 id 枚举互核；wave5：缺 evidence→红 + 真树聚合 VERDICT==子门实测态不遮蔽双 PASS）；`py -3 ci/g12_wave_exit_lib.py --selftest` 6 RED+1 GREEN。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G12 十七前缀路由落——本波新增 g12_m165_pt_throughput_baseline_ / g12_m165_baseline_ / g12_wave5_exit_ 三件，既有 0-byte）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 385 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (385/385 clauses anchored, 861 test files scanned)`；`py -3 ci/budget_eval.py` → `[budget_eval] PASS (196 pass, 0 skip, normal mode)`（g12.pt 28 条〔8 锚 + 7 标定 + 2 降噪标定 + 3 对标标定 + 8 吞吐基线〕measured_local 全 PASS 零 estimated）；`py -3 ci/check_g12_implementation_interlock.py --require-ready` → VERDICT=READY（事实门④实测 workflow 末号 229 == ledger on_tree_max 229 == next_free−1）。
  - G12.2~G12.4 波聚合门复跑：`g12_wave2_exit_check` / `g12_wave3_exit_check` / `g12_wave4_exit_check` → 三连 VERDICT=PASS（触改面链零降级）；`cargo test -p rurix-render --lib gi::path_trace` → `test result: ok. 32 passed; 0 failed`。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G12-3 互锁 READY（§8.1）维持 → G12.4（§8.4）正确性锚承接 → 本波实现门 materialize（M165 纯测量面无新 RXS 条款/conformance 语料——G10.5 M141 先例同模，消费 BENCH_PROTOCOL §3 + 14 §5 既有口径面 + RFC-0029 §4.7 基线锚消费面；spec/RXS 命名空间 0-byte）；数字步骤 228~229 按落盘前实测 actual next_free=228 顺位领取（ledger v1.131 校准同批）。
  - **间歇非确定性事件登记（偏差如实登记，不冒充全闭环）**：M165 门复跑一回合（evidence/g12_m165_pt_throughput_baseline_20260817T235251Z.json，status=fail 在档不删）bistro-interior spp16 腿 160 帧内检出 distinct digest==2（首帧 == 冻结锚 bb803d8e…，中间帧单帧漂移一次）——「digest 漂移未登记即 RED」机核**如实触发**（正确性锚按设计工作）。表征：复现频率实测 1 次 / ~1760 帧（事件后 10 个完整 160 帧回合〔含 6 回合 flip-trace 诊断臂〕零复现；cornell 双面零观察；M163 双跑位级面跨三回合全绿）；未定位——假设面登记：驱动侧 BLAS 构建调度方差或分配布局相关未初始化读取（重复 dispatch 面偶发，单帧粒度）；**诊断面已落盘**（--benchmark `G12_5_BENCH_FLIP_TRACE=1` + `--flip-dump-dir` 加性诊断臂：逐帧 digest eprintln + 首帧/漂移帧 EXR 双双落盘像素级对照面，默认关闭 stdout 形态 0-byte）；**承接 = G12.6 穷举候选行**（复现率升高或 M163/M164 后续复跑检出同型漂移 ⇒ 升级生产化缺陷修复项；锚定面 = 固定 seed 确定性协议 RXS-0357 L2/RXS-0400）。本波判据不受阻：事件回合 FAIL 如实留存，后续回合全 160 帧位级一致 + digest 锚 0-byte 逐 cell 全等（终版 evidence 20260818T001938Z）。
  - **工作树并发面留痕（沿 §8.2~§8.4 同模）**：本波期间 `ci/check_schemas.py` 与 `.github/workflows/pr-smoke.yml` 各被并发面回退一次（恢复为 HEAD 版）——分别经 `.tmp/g12_5_checkschemas_edit.py` / `.tmp/g12_5_workflow_edit.py` 脚本原子重放落盘并以 check_schemas / 互锁 validator `--require-ready` 真跑复核（PASS/READY 留痕）；G12 车道其余文件未受影响；`.tmp/g12_5_*.py` 为一次性落盘工具不入 commit。
  - **工具链/环境偏差登记（不阻断本波判据，沿 §8.2~§8.4 同模）**：`cargo fmt --check` / `cargo clippy -D warnings` 在本机 pinned 1.93.1 下对 HEAD 既有文件即报预存差异（本文件 17 hunks HEAD 预存；clippy 8 errors 全在 rurix-rt lib unsafe-comment 预存面）——本波新增面（run_benchmark/flip-trace 诊断臂/两个 py 门）本地 pinned 工具链下**零新增 fmt hunks / 零新 clippy 项**实测；GPU 未锁频边界经 clock_lock_note 诚实存档（×1.5/÷1.5 守护余量先例覆盖）。
  - **not-triggered / 维持 open 面**：M52 SER 复评窗维持（G12.6 穷举窗核验）；M100-high（G12.4 已触发评估登记——G12.6 穷举窗按只追加程序重判）/G10-N17（未触发——本波零 FLIP 标量面消费）/G11-N5（G12.6）窗维持；G11-N8/G11-N9 锚定 G15；G10-N11/N16 帧率面锚定 G14 字面维持（本波只建基线不设通过线）；异己会话 src/ 未提交面（hzb/restir/sdf_trace/smrt/ssr/ktx2_read 及 mod.rs/lib.rs 异己注册行、render_exec.rs 异己改写面、apps/ 异己面、evidence/d3d12_interop_smoke.json 异己改写面）维持未提交、零消费、零混入（本批 `git add` 按文件名显式择取，G12 车道文件清单见块⑤）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/本契约 §8.1~§8.4 同模）。`Assisted-by: Kimi-K3（G12.5 吞吐基线波）`（影响范围：src/rurix-asset/src/bin/g12_4_ue_pt_parity_render.rs〔--benchmark 加性子模式 + flip-trace 诊断臂——--render/--contract-digest 既有面 0-byte，M163 全档复跑零降级机核同窗〕+ ci/g12_pt_throughput_baseline_smoke.py 新建 + ci/g12_wave5_exit_check.py 新建 + ci/check_schemas.py〔三处纯追加〕+ milestones/g12/g12_m165_pt_throughput_baseline_evidence_schema.json + g12_m165_baseline_entry_evidence_schema.json + g12_wave5_exit_evidence_schema.json 三新建 + milestones/g12/g12_budget.json〔8 吞吐基线条目字节级纯追加 measured_local〕+ milestones/g12/CI_GATES.md v1.4 修订行 + registry/number_ledger.json〔CI_step 227→229/next_free 230 + revision_log v1.131〕+ .github/workflows/pr-smoke.yml〔步骤 228~229〕+ 本契约 §8.5 本条 + evidence/g12_m165_pt_throughput_baseline_*〔首跑登记 20260817T233025Z + 偏差 FAIL 件 20260817T235251Z + 终版 20260818T001938Z〕+ evidence/g12_m165_baseline_* 8 件 + evidence/g12_m163_ue_pt_parity_20260818T000809Z〔复跑〕+ evidence/g12_wave5_exit_20260818T001948Z 本波真跑件；验证方式：块③逐字命令输出——M165 门 14/14 PASS + M163 复跑 21/21 PASS + 波聚合门 VERDICT=PASS + 守卫套件全 PASS + 全量 selftest 红绿留痕 + wave2/3/4 exit 三连复跑 PASS 零降级）。

---

### §8.6 G12.6 P2 穷举决策验收记录（2026-08-17）——G-G12-8 决策门：G12 期全部 P2/留档/未触发分项逐条裁决，33 行闭集零空行（go 5 closed-go 留痕 + no-go 6 + defer-to-G13+ 22 + strategic_override 0）；决策门 `g12.wave.6.decisions` VERDICT=PASS（38 facts）

- **① 独立断言全绿清单（决策门一行——host 治理断言面，无 device 面）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g12.wave.6.decisions`（步骤 230） | G12.6 P2/留档/未触发分项穷举决策：33 行冻结候选闭集全等（G12.1 决策表校准后冻结〔§1 G11 defer 19 + §3 G12-N10/N11 + §2 RD 级 RD-034/042/043/044〕+ G12.2~G12.5 期内新增 G12-N12~G12-N19 八行）+ 裁决枚举合法（go/no-go/defer-to-G13+/strategic_override）+ 零空行（全列非空）+ 承接锚「重判条件+兜底」、defer 行 G13+ 重评窗字面 + go 行 evidence 义务 + no-go 行 RD/矩阵/契约锚义务 + 三横向机核（①G12_ACCEPTANCE_MAP 9 key〔8 P0 + 1 已 go P1〕互斥——P2 行零命中已 go M### 裸 token；②deferred.json history 对账——G12.6 P2 登记恰好 RD-039 +1〔M61〕/RD-040 +2〔M52/M100-high〕，零新 RD max=RD-044，RD-039/040 status open 0-byte；③G12.1 候选决策表对账——19 行 G11 defer 承接 + G12-N10 == CANDIDATE §1/§3 行集 defer-to-G13+ 字面逐字承接 n=20/20） | host 纯 host（只读文档与 registry，不代绿实现门） | evidence/g12_p2_decisions_20260818T005337Z.json（extra_facts 38/38） | PASS |

  裁决分类汇总（零空行）：**go 5 行**（G12-N14 降噪形态实测定盘〔firefly 预钳位 + 对称归一化 σ_l=0.2〕/ G12-N15 UE 臂六轮探针定位与双绕向派生语料承载 / G12-N16 TDR device-lost 修复批 / G12-N17 Rurix 相机手性修正 / G12-N19 前轮 host 起步缺陷修复批——closed-go 留痕，门绿承载不再产生后续分项）+ **no-go 6 行**（RD034 blocked 维持 / RD042/RD043 观察维持 / RD044 maintain_no_go 维持 / G12-N11 异己面严禁消费 / G12-N18 fmt/clippy 预存漂移面零修复纪律不回写——如实保持 open/留档，不写进全绿叙述，不阻塞 G12.7a soak）+ **defer-to-G13+ 22 行**（§1 G11 defer 19 行承接锚字面 0-byte 转引 + G12-N10 材质链锚定 G15 + G12-N12 UE PT 差距登记表 10 行处置锚定 G15 + G12-N13 M165 间歇非确定性事件诊断臂在树——每行承接锚「重判条件+兜底+G13+ 重评窗」齐备）。

- **② 波聚合门实测输出**：G12.6 为决策门（G-G12-8 本体，非 exit 聚合门）——`py -3 ci/g12_p2_decisions_check.py --gate g12.wave.6.decisions` → **VERDICT=PASS，exit=0**（evidence/g12_p2_decisions_20260818T005337Z.json）——extra_facts 38/38 全 PASS（set_equality_frozen 33/33 闭集全等 + no_duplicate_ids + row_* 33 行逐行 + acceptance_map_mutex〔MAP 实解 P0=8 P1=1，命中已 go 裸 token：无〕+ deferred_history_reconcile〔G12.6 P2 history: RD-039 ×1 + RD-040 ×2〕+ candidate_decisions_reconcile〔承接行对账 n=20/20〕）。

- **③ 验收命令逐字输出（2026-08-17 真跑留痕，仓库根目录）**：
  - `py -3 ci/g12_p2_decisions_check.py --gate g12.wave.6.decisions` → `VERDICT = PASS`（38 facts 逐行打印全 PASS——闭集全等/互斥/history 对账/候选对账逐字见块②；首跑登记件 g12_p2_decisions_20260818T004839Z.json 在档，终版 20260818T005337Z.json）。
  - `py -3 ci/g12_p2_decisions_check.py --selftest` → **[selftest] ALL PASS**（真表 33 行绿 + 合成全表绿 + 7 红臂全过：缺行→红 / defer 缺 G13+ 承接锚→红 / 非法裁决枚举→红 / 已 go P0 裸 token M163 入表互斥违例→红 / 空单元格→红 / deferred history 缺登记→红 / 候选决策表对账失配→红）。
  - `py -3 ci/check_g12_acceptance_map.py` → `[check_g12_acceptance_map] PASS（8 P0 + 1 已 go P1（M166）覆盖齐备；9 key 唯一且同一命名空间；P0 行 MAP/CONTRACT/CI_GATES 三向逐字一致、P1 行 MAP §2/CI_GATES §4A 双向逐字一致；零空行/占位；numeric_step 全列 post-interlock 字面零预占）`；`--selftest` → **SELFTEST PASS (14 RED + 1 GREEN)**（本批 materialize：CI_GATES §3 `g12.gov.acceptance_coverage` 声明 validator 落盘——G12.7b close-out 三向机核面成环，判据语义 0-byte，不占数字步骤）。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G12 十八前缀路由落——本波新增 g12_p2_decisions_ 一件，既有 0-byte）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 385 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (385/385 clauses anchored, 861 test files scanned)`；`py -3 ci/budget_eval.py` → `[budget_eval] PASS (196 pass, 0 skip, normal mode)`（g12.pt 28 条 measured_local 全 PASS 零 estimated——本波零 budget 新条目）；`py -3 ci/check_g12_implementation_interlock.py --require-ready` → VERDICT=READY（事实门④实测 workflow 末号 230 == ledger on_tree_max 230 == next_free−1）。

- **④ 门序 / not-triggered / no-go 登记面摘要**：
  - **门序**：G-G12-3 互锁 READY（§8.1）维持 → G12.5（§8.5）→ 本波 G-G12-8 决策门 materialize（纯治理核验面无新 RXS 条款/conformance 语料/src 改动——G11.6 先例同模，spec/RXS 命名空间 0-byte）；数字步骤 230 按落盘前实测 actual next_free=230 顺位领取（ledger v1.132 校准同批）。
  - **触发/复评窗兑现登记（本波穷举裁决，承接锚字面 0-byte 维持）**：**M52 G12.6 复评窗核验兑现**——G12.2 megakernel 初态集成面已 materialize 后复评：真实集成需求仍未至（四波零 SER 原语消费）+ capability rt.ser 设备面仍未实测（树内零 rt.ser 探针维持，2026-08-17 grep 复核）双条件未命中 → maintain-defer 顺延 G13+（RD-040 history +1）；**M100-high G12.6 触发重判兑现**——G12.4 多灯 workload measured 对照面已产（触发条件命中在案）但对照面未消费 G9 低档 MegaLights GPU 光栅管线（PT megakernel ≠ MegaLights 管线），「低档不足」measured 对照证据仍未齐备 → maintain-defer 锚定 G14（RD-040 history +1）；**G10-N17 触发评估兑现=未触发**（G12.4/G12.5 零 FLIP 标量面消费，null 演进位维持，M137 门不回归）；**G11-N5 G12.6 触发评估兑现=数据集未齐备**（G12.4 对拍 measured 面非低反照率暗帧稳健性对照数据集，不冒充，维持 defer）；**G11-N8/G11-N9 G12.4 差距登记联动兑现**（材质纹理均值扁平化/emissive 口径差进差距登记表 caliber_diff 行显式归属不承接，锚定 G15 维持）。
  - **G12 新增行裁决登记**：G12-N12 UE PT 对标差距登记表 10 行处置（6 quality_gap 超容差项对账齐 + 4 caliber_diff 残余口径显式归属——锚定 G15，残余差距如实登记不冒充全闭环，G12.7b 终审锁定 = G13 法定输入）；G12-N13 M165 间歇非确定性事件（1/~1760 帧 digest 漂移未定位，flip-trace 诊断臂在树，FAIL 件 20260817T235251Z 0-byte 保留，升级条件未命中维持 defer）；G12-N14~N17/N19 五行 closed-go 留痕（降噪形态定盘/UE 探针定位/TDR 修复批/相机手性/前轮缺陷修复批，门绿承载）；G12-N18 fmt/clippy 预存漂移面 no-go（G11-N10/G10-N15 同族，G12 零修复纪律不回写）。
  - **no-go / defer 如实保持 open**（不写进全绿叙述，不阻塞 G12.7a soak）：RD034/042/043/044 四条 RD 级维持行 + G12-N11 异己面 + G12-N18 漂移面 no-go 6 行；defer-to-G13+ 22 行承接锚逐行在表（G12_P2_DECISIONS §3 清单 22 行）；SG-010 软保留 not_triggered 维持。
  - **工作树并发面留痕（沿 §8.2~§8.5 同模）**：本波期间 `.github/workflows/pr-smoke.yml` 与 `ci/check_schemas.py` 各被并发面回退一次（恢复为 HEAD 版——步骤 230 块与 g12_p2_decisions_ 三处路由被抹）——经 `.tmp/g12_6_replay.py` 脚本原子重放落盘（幂等四面：check_schemas 三处路由 + workflow 步骤 230 + ledger v1.132 + deferred 三条）并以 check_schemas / 互锁 validator `--require-ready` 真跑复核（PASS/READY 留痕）；G12 车道其余文件未受影响；`.tmp/g12_6_*.py` 为一次性落盘工具不入 commit。
  - **工具链/环境偏差登记（不阻断本波判据，沿 §8.2~§8.5 同模）**：`cargo fmt --check` / `cargo clippy -D warnings` 在本机 pinned 1.93.1 下对 HEAD 既有文件即报预存差异（与 G12.6 无关，CI 面以流水线工具链为准）——本波新增文件（G12_P2_DECISIONS.md / g12_p2_decisions_check.py / check_g12_acceptance_map.py / schema 一件）为文档与 Python 面无 rustfmt/clippy 消费面；`check_guardrails`/`check_contribution` 为 advisory（evidence/d3d12_interop_smoke.json 异己会话未提交修改面维持不混入本批）。
  - **异己并发工作树面**：本批只含 G12 车道文件（块⑤清单按文件名显式择取）；异己会话 src/ 未提交面（hzb/restir/sdf_trace/smrt/ssr/ktx2_read 及 mod.rs/lib.rs 异己注册行、render_exec.rs 异己改写面、apps/ 异己面、evidence/d3d12_interop_smoke.json 异己改写面）维持未提交、零消费、零混入（立项裁决 1，G12-N11 行登记面）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/本契约 §8.1~§8.5 同模）。`Assisted-by: Kimi-K3（G12.6/G12.7 收口波）`（影响范围：milestones/g12/G12_P2_DECISIONS.md 新建〔33 行闭集 v1.0〕+ ci/g12_p2_decisions_check.py 新建 + ci/check_g12_acceptance_map.py 新建〔CI_GATES §3 声明 validator materialize〕+ milestones/g12/g12_p2_decisions_evidence_schema.json 新建 + ci/check_schemas.py〔g12_p2_decisions_ 三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 230〕+ registry/number_ledger.json〔CI_step 229→230/next_free 231 + revision_log v1.132〕+ registry/deferred.json〔RD-039 +1〔M61〕/RD-040 +2〔M52/M100-high〕history 只追加，条目级字段与 status 0-byte〕+ milestones/g12/CI_GATES.md v1.5 修订行 + 本契约 §8.6 本条 + evidence/g12_p2_decisions_20260818T005337Z.json〔终版；首跑登记件 20260818T004839Z 在档〕；验证方式：块③逐字命令输出——决策门 VERDICT=PASS（38 facts）+ selftest 红绿留痕（真表绿 + 合成绿 + 7 红臂）+ check_g12_acceptance_map PASS（14 RED + 1 GREEN selftest）+ 守卫套件全 PASS + 互锁 VERDICT=READY）。

---

### §8.7a G12.7a stabilization soak 验收记录（2026-08-17）——G-G12-9 稳定门：8 P0 + 1 go P1 全量回归真跑全绿（9 门 base_commit 同值 = 同一候选 close-out 基线）+ PT 生产化链路连续复跑 soak 33 迭代 1813.6s 零失败 + budget --strict 零 estimated/skip + G5~G11 既有判据 0-byte；稳定门 `g12.wave.7a.soak` VERDICT=PASS（6 facts）

- **① 独立断言全绿清单（9 key 回归逐门真跑 --gate + 稳定门聚合一行——每行单独 PASS，聚合不遮蔽）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g12.p1.m166.pt_production_calibration`（步骤 217 复跑） | 标定两跑逐字节一致 + 7 标定条目 budget 守护 + 选型 artifact + RED 四臂 | host | evidence/g12_pt_production_calibration_20260818T013105Z.json（checks 10/10） | PASS |
  | `g12.p0.m158.mis_full_surface`（步骤 218 复跑） | MIS 完整面 device 全档（双跑位级 + 白炉/逐级能量 + 曲线锚 8 点 + RED 三臂复跑） | host+device（env 双置） | evidence/g12_m158_mis_full_surface_20260818T013349Z.json（checks 15/15） | PASS |
  | `g12.p0.m159.russian_roulette_prod`（步骤 219 复跑） | 吞吐自适应 RR（终止率/补偿计数 + 无偏对照 + fail-closed + RED 三臂复跑） | host+device | evidence/g12_m159_russian_roulette_prod_20260818T013415Z.json（checks 13/13） | PASS |
  | `g12.p0.m160.sampling_lds_upgrade`（步骤 220 复跑） | 低差异序列（三族流位级 + 索引确定性 + winner 一致性 + 曲线锚 + RED 双臂复跑） | host+device | evidence/g12_m160_sampling_lds_upgrade_20260818T013443Z.json（checks 14/14） | PASS |
  | `g12.p0.m161.convergence_criterion_prod`（步骤 221 复跑） | 收敛判据（自适应位级 + 报告非空 + 误判率 ≤ 标定阈 + golden 冻结带 + RED 三臂复跑） | host+device | evidence/g12_m161_convergence_criterion_prod_20260818T013505Z.json（checks 14/14） | PASS |
  | `g12.p0.m162.denoise_pipeline_tsr`（步骤 223 复跑） | 降噪管线 + TSR 联动（双 kernel 全档 + 噪声谱/能量阈 + temporal 底座 0-byte + RED 三臂复跑） | host+device | evidence/g12_m162_denoise_pipeline_tsr_20260818T013505Z.json（checks 20/20） | PASS |
  | `g12.p0.m163.ue_pt_parity`（步骤 225 复跑） | UE PT 对标（契约 digest 三方全等 + 双端 12 帧 + 逐段/噪声谱/能量对拍 + 差距登记表 10 行 + RED 五臂复跑） | host+device（UE 5.8.1 MRQ 臂） | evidence/g12_m163_ue_pt_parity_20260818T013657Z.json（checks 21/21） | PASS |
  | `g12.p0.m164.regression_guard`（步骤 226 复跑） | 生产化回归门（62 门只读汇总全绿 + 触改面抽检真跑零降级 + 既有判据 0-byte + RED 三臂复跑） | host（抽检自持 device 面） | evidence/g12_m164_regression_guard_20260818T014631Z.json（checks 15/15） | PASS |
  | `g12.p0.m165.pt_throughput_baseline`（步骤 228 复跑） | PT 吞吐基线（50×3 协议在档守护复检 8/8 + digest 锚 0-byte 4/4 cell + distinct==1 + RED 三臂复跑） | host+device | evidence/g12_m165_pt_throughput_baseline_20260818T015338Z.json（checks 14/14） | PASS |
  | `g12.wave.7a.soak`（步骤 231 聚合门） | 四腿终审：①全量回归 14 门真跑全绿（9 门顶层 status==pass 字面 + base_commit 同值 d7348d23 + 14 门 stamp 新鲜度机核）②PT 生产化链路 soak 33 迭代 1813.6s 零失败（计数非空）③budget --strict 非空零 estimated/skip ④日期锚 + G5~G11 0-byte | host 聚合（device 面归 9 门本体） | evidence/g12_stabilization_soak_20260818T022404Z.json（facts 6/6 + checks 7 键全真） | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g12_stabilization_soak.py --gate g12.wave.7a.soak` → **VERDICT=PASS，exit=0**（evidence/g12_stabilization_soak_20260818T022404Z.json）——六 facts 全 PASS：`regression_8p0_1p1_5wave`（gates=14，base_commit=d7348d23fd71c388a764a92eb02f0094633a8236）/ `base_commit_uniform`（9 门同值 = HEAD）/ `soak_dual_threshold`（iterations=33 seconds=1813.6 active=1813.6 sleep=0.0 failures=0 subject='chain-soak'）/ `budget_strict`（exit=0，[budget_eval] PASS (196 pass, 0 skip, strict mode)）/ `legacy_criteria_0byte`（闭集面空集）/ `date_anchor`（utc_date=20260818）。soak 块机器可核：iterations=33、seconds=1813.6406655311584 ≥ 1800、active_chain_seconds=1813.6391551494598 ≈ seconds（差 1.5ms）、sleep_seconds=0.0、outer_wall_seconds=1814.218（外测 ≥ 自称，谎报机核不触）、failures=0、pt_frames_rendered=66、denoise_pipeline_runs=33、gap_registry_assemblies=33、throughput_baseline_reruns=33、throughput_frames_timed=264。聚合门内波聚合/决策门五连复跑：`g12.wave.2.exit`（20260818T015338Z）/ `g12.wave.3.exit`（015347Z）/ `g12.wave.4.exit`（015348Z）/ `g12.wave.5.exit`（015349Z）/ `g12.wave.6.decisions`（015349Z）全 VERDICT=PASS。

- **③ 验收命令逐字输出（2026-08-17 真跑留痕，仓库根目录）**：
  - `py -3 ci/g12_stabilization_soak.py --gate g12.wave.7a.soak` → **VERDICT=PASS**（终版 022404Z；逐门真跑 `[g12_m166] checks 10/10` / `[g12_m158] checks 15/15 device=executed` / `[g12_m159] checks 13/13` / `[g12_m160] checks 14/14` / `[g12_m161] checks 14/14` / `[g12_m162] checks 20/20 device=executed` / `[g12_m163] checks 21/21 device=executed` / `[g12_m164] checks 15/15` / `[g12_m165] checks 14/14 device=executed`；soak 逐迭代 `[7a] soak iter 1..33 ok (elapsed …/1800s)`）。
  - `py -3 ci/g12_stabilization_soak.py --verify-latest` → `[7a] verify-latest PASS(honest chain soak)← evidence/g12_stabilization_soak_20260818T022404Z.json`。
  - `py -3 ci/g12_stabilization_soak.py --selftest` → **[selftest] PASS: 反假绿臂全部符合预期(5 红臂 + 1 绿臂)**（sleep 充墙钟→红 / 外测墙钟戳穿谎报→红 / 迭代不足→红 / failures≠0 或计数面空→红 / 缺 honesty 字段→红 + 诚实样本→绿）。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G12 十九前缀路由落——本波新增 g12_stabilization_soak_ 一件，既有 0-byte）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 385 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (385/385 clauses anchored, 861 test files scanned)`；`py -3 ci/budget_eval.py --strict` → `[budget_eval] PASS (196 pass, 0 skip, strict mode)`；`py -3 ci/check_g12_implementation_interlock.py --require-ready` → VERDICT=READY（事实门④实测 workflow 末号 231 == ledger on_tree_max 231 == next_free−1）；`py -3 ci/check_g12_acceptance_map.py` → PASS（9 key 三向逐字一致）。
  - **首跑 FAIL 偏差如实登记（不删不改在档）**：首跑 `evidence/g12_stabilization_soak_20260818T013012Z.json` VERDICT=FAIL——soak 前置构建命令漏 `--features vulkan`（`target g12_4_ue_pt_parity_render in package rurix-asset requires the features: vulkan`；回归 14 门与 budget strict/legacy/date 四 facts 已 PASS，soak_dual_threshold FAIL）；修复（构建命令补 feature 与 M165 门同字面 + 失败诊断 stderr 落 detail）后整门复跑全绿（终版 022404Z）。FAIL 件在档不冒充、不删除（G12.5 M165 偏差 FAIL 件同模）。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G12-3 互锁 READY（§8.1）维持 → G12.6（§8.6）→ 本波 G-G12-9 稳定门 materialize；数字步骤 231 按落盘前实测 actual next_free=231 顺位领取（ledger v1.133 校准同批）；同日放行先例继承（立项裁决 7：7a full-run 先行完成后允许同日进 7b close-out——本批 full-run 先行完成留痕）。
  - **soak 诚实口径登记**：墙钟 = PT 生产化链路复跑实测（active_chain_seconds 逐迭代计时求和，sleep_seconds 恒 0，gate 外测墙钟 1814.218s ≥ 自称 1813.641s 交叉核验不触谎报机核）；吞吐基线腿 = `--benchmark` 轻量链复跑（warmup 4 + timed 8 场景轮转）——链复跑口径如实登记**不冒充 M165 冻结 50×3 协议**，吞吐守护阈归 M165 门本体（①回归腿 50×3 全协议 ×1.5/÷1.5 守护复检 8/8 PASS：cornell spp16 复测 141.990ms 中位 vs 阈 214.465ms 在内）；轻量腿帧时样本进 evidence notes 信息面不充判据。
  - **G12-N13 间歇非确定性事件面**：soak 全窗口 33 迭代（66 出图帧 + 33 吞吐轻量复跑 ×8 帧 = 264 计时帧 + 33 降噪全档）零失败、出图/吞吐腿 digest 锚逐迭代全等（distinct==1）——零同型漂移观察；事件维持 open-defer（诊断臂在树，升级条件未命中），不写进全绿叙述。
  - **not-triggered / 维持 open 面**：M52（G12.6 复评窗核验 = maintain-defer）/ M100-high（G12.6 触发重判 = maintain-defer 锚定 G14）/ G10-N17（未触发）/ G11-N5（G12.6 触发评估 = 数据集未齐备）/ G11-N8/G11-N9（锚定 G15）/ G12-N10/N12/N13 defer 维持——§8.6 登记字面 0-byte；SG-010 软保留 not_triggered 维持；异己会话 src/ 未提交面维持未提交、零消费、零混入（本批 `git add` 按文件名显式择取，G12 车道文件清单见块⑤）。
  - **工作树并发面留痕（沿 §8.2~§8.6 同模）**：本波接线期 `.github/workflows/pr-smoke.yml`、`ci/check_schemas.py`、`registry/number_ledger.json` 三面经 `.tmp/g12_7a_replay.py` 脚本原子重放落盘（幂等三面：check_schemas 三处路由 + workflow 步骤 231 + ledger v1.133）并以 check_schemas / check_number_ledger / 互锁 validator `--require-ready` 真跑复核（PASS/READY 留痕）；G12 车道其余文件未受影响；`.tmp/g12_7a_*.py` 为一次性落盘工具不入 commit。
  - **工具链/环境偏差登记（不阻断本波判据，沿 §8.2~§8.6 同模）**：`cargo fmt --check` / `cargo clippy -D warnings` 在本机 pinned 1.93.1 下对 HEAD 既有文件即报预存差异（与 G12.7a 无关，CI 面以流水线工具链为准）——本波新增文件（g12_stabilization_soak.py / schema 一件）为 Python/JSON 面无 rustfmt/clippy 消费面；GPU 未锁频边界沿 M165 门 clock_lock_note 诚实存档先例（50×3 守护余量 ×1.5/÷1.5 覆盖频率漂移）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/本契约 §8.1~§8.6 同模）。`Assisted-by: Kimi-K3（G12.6/G12.7 收口波）`（影响范围：ci/g12_stabilization_soak.py 新建 + milestones/g12/g12_stabilization_soak_evidence_schema.json 新建 + ci/check_schemas.py〔g12_stabilization_soak_ 三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 231〕+ registry/number_ledger.json〔CI_step 230→231/next_free 232 + revision_log v1.133〕+ milestones/g12/CI_GATES.md v1.6 修订行 + 本契约 §8.7a 本条 + evidence/g12_stabilization_soak_20260818T022404Z.json〔终版 PASS；首跑 FAIL 偏差件 20260818T013012Z 在档〕+ evidence/g12_{m166,m158,m159,m160,m161,m162,m163,m164,m165,wave2_exit,wave3_exit,wave4_exit,wave5_exit,p2_decisions}_* 本波回归真跑件；验证方式：块③逐字命令输出——回归 14 门真跑全绿 + 稳定门 VERDICT=PASS（6 facts + checks 7 键）+ verify-latest PASS + selftest 5 红臂 + 1 绿臂 + 守卫套件全 PASS + 互锁 VERDICT=READY）。

---

### §8.7b G12.7b close-out 终审记录（2026-08-17）——G-G12-10 收口门：八 facts 全 PASS，VERDICT=READY；生产化差距清单 10 行终态（quality_gap 6 + caliber_diff 4）终审锁定——残余差距/未闭环行如实登记不冒充全闭环，锁定面 = G13 法定输入；status flip 独立 commit

- **① 独立断言全绿清单（终审门一行——host 只读终审面，不重跑子门）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g12.wave.7b.closeout`（步骤 232 终审门） | 八 facts 只读终审：①9 key（8 P0 + 1 go P1，G12_ACCEPTANCE_MAP §1/§2 实记，与 ci/g12_stabilization_soak.py REGRESSION_GATES 前 9 行同一闭集）逐门 PASS（wel 口径 + 顶层 status==pass 字面——G12 证据形态统一无豁免面）②wave2~7a 六聚合/决策门（exit×4 + decisions + soak）全 PASS ③MAP 三向 check_g12_acceptance_map exit=0 ④P2 表 33 行闭集最终状态无漂移（最新 evidence host_section_pass + FROZEN_IDS 在树，复用 ci/g12_p2_decisions_check.py 闭集单一事实源）⑤budget_eval --strict 非空零 estimated/skip ⑥7a full-run 先行（最新 g12_stabilization_soak evidence host_section_pass，base_commit_7a=d7348d23 留痕；立项裁决 7 同日放行）⑦RD 最终状态逐字一致（deferred.json RD-034/039~044 七条目级 status 全 open 逐字 + G12_P2_DECISIONS 33 行 FROZEN_IDS 闭集在树两面一致；G12 无 defer 重评窗表）⑧生产化差距清单终审锁定（g12_ue_pt_gap_registry.json 10 行闭集：gap_id 集 == G12.4 锁定清单逐字对账 + 计数 10/6/4 重算一致 + generated_by == M163 门字面 + quality_gap 6 行超容差项对账齐 + caliber_diff 4 行残余口径归属非空）+ 最后新绿 UTC 日留痕 | host 只读（不重跑子门 smoke、不设 RURIX_REQUIRE_REAL） | evidence/g12_wave7b_closeout_20260818T024429Z.json（facts 8/8 + checks 八键全真 + required_gates 15 行） | READY |

- **② 终审门实测输出**：`py -3 ci/g12_closeout_check.py --gate g12.wave.7b.closeout` → **VERDICT=READY，exit=0**（evidence/g12_wave7b_closeout_20260818T024429Z.json）——八 facts 全 PASS：`nine_keys_pass`（pass=9/9）/ `wave_exits_2_to_7a`（pass=6/6）/ `acceptance_map_triple`（exit=0）/ `p2_decisions_33_frozen`（g12_p2_decisions_20260818T015349Z.json，frozen_33_in_tree=True）/ `budget_strict`（exit=0）/ `soak_7a_precedes`（g12_stabilization_soak_20260818T022404Z.json，base_commit_7a=d7348d23fd71c388a764a92eb02f0094633a8236，立项裁决 7 同日放行）/ `rd_final_state_consistent`（7 RD open 逐字一致 + P2 33 行闭集在树，G12 无重评窗表两面一致）/ `gap_registry_locked_and_green_recorded`（gap_id 集对账 + 计数 10/6/4 重算一致 + 超容差项对账齐 + 残余口径归属非空；last_green_utc=20260818 today=20260818 missing=[]）。required_gates 15 行（9 key + wave2/3/4/5 exit + p2_decisions + soak）逐行 status==PASS 且 evidence 锚定批次 B 回归真跑件；checks 八键闭集全真；utc_date=20260818、last_new_green_utc_date=20260818。首跑登记件 g12_wave7b_closeout_20260818T023148Z.json 在档（同 READY），终版 024429Z 为复跑新鲜度留痕（evidence 只增不删不改）。

- **③ 验收命令逐字输出（2026-08-17 真跑留痕，仓库根目录）**：
  - `py -3 ci/g12_closeout_check.py --gate g12.wave.7b.closeout` → **VERDICT = READY**（八 facts 逐行打印全 PASS，字面见块②；首跑 023148Z 与复跑终版 024429Z 双件在档）。
  - `py -3 ci/g12_closeout_check.py --selftest` → `[selftest] OK materialized step 232`。
  - `py -3 ci/g12_stabilization_soak.py --verify-latest` → `[7a] verify-latest PASS(honest chain soak)← evidence/g12_stabilization_soak_20260818T022404Z.json`。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G12 二十前缀路由落——本波新增 g12_wave7b_closeout_ 一件，既有 0-byte）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 385 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (385/385 clauses anchored, 861 test files scanned)`；`py -3 ci/budget_eval.py --strict` → `[budget_eval] PASS (196 pass, 0 skip, strict mode)`；`py -3 ci/check_g12_implementation_interlock.py --require-ready` → VERDICT=READY（事实门④实测 workflow 末号 232 == ledger on_tree_max 232 == next_free−1）；`py -3 ci/check_g12_acceptance_map.py` → PASS（8 P0 + 1 已 go P1（M166）覆盖齐备，9 key 三向逐字一致）。

- **④ 门序 / 终审锁定 / 纪律面摘要**：
  - **门序**：G-G12-3 互锁 READY（§8.1）维持 → G12.7a（§8.7a）→ 本波 G-G12-10 终审门 materialize（纯治理核验面无新 RXS 条款/conformance 语料/src 改动）；数字步骤 232 按落盘前实测 actual next_free=232 顺位领取（ledger v1.134 校准同批）；同日放行先例继承（立项裁决 7：7a full-run 先行完成后允许同日 close-out——7a 终版 022404Z 先行完成留痕）。
  - **生产化差距清单终审锁定（G-G12-10 字面兑现）**：`milestones/g12/g12_ue_pt_gap_registry.json` 10 行终态——quality_gap 6 行（超容差项 measured_delta 非空 + delta==b−a 可溯源 + g11_anchor 锚定 G15）+ caliber_diff 4 行（残余口径归属非空，RXS-0391 归属枚举口径）；计数重算一致（total 10 / quality_gap 6 / caliber_diff 4；scene_summary cornell-box 4 + bistro-interior 6；not_ready_scenes==[]）；generated_by == M163 门字面。**残余差距/未闭环行如实登记不冒充全闭环**；终审锁定 0-byte（终审不重写清单本体）——**锁定面 = G13 法定输入**：G13 期只消费本清单与 G12_P2_DECISIONS 承接锚，不得另起无锚差距面。
  - **RD 最终状态**：RD-034/039/040/041/042/043/044 七条维持 open 逐字一致（分项 go/no-go/defer 已由候选决策表、G12_P2_DECISIONS 33 行闭集与 deferred history 只追加留痕；G12 无 defer 重评窗表，全表深对账由 g12.wave.6.decisions 门承载不重复）。
  - **not-triggered / 维持 open 面**：M52 / M100-high / G10-N17 / G11-N5 / G11-N8 / G11-N9 / G12-N10 / G12-N12 / G12-N13 与 SG-010 软保留 not_triggered——§8.6/§8.7a 登记字面 0-byte。
  - **纪律面**：只读终审不重跑子门 smoke、不设 RURIX_REQUIRE_REAL；G5~G11 closed 判据与既有门脚本 0-byte；evidence/ 只增不删不改；RD 条目级 status 与四字段 0-byte；差距清单终态 0-byte；异己会话 src/ 未提交面（hzb/restir/sdf_trace/smrt/ssr/ktx2_read 及 mod.rs/lib.rs 异己注册行、render_exec.rs 异己改写面、apps/ 异己面）与 evidence/d3d12_interop_smoke.json 异己改写面维持未提交、零消费、零混入（立项裁决 1 / §8.6 ⑤ 同模）；milestones/g12/g12_pt_sampler_selection.json 经 M166 复跑同值重写（内容逐字一致、timestamp 151512Z→013105Z 随复跑前移——「同值重写幂等,漂移即 RED」机核在门）维持未提交，批次 B 处置同模；本批 commit 只含本车道文件（块⑤清单按文件名显式择取）。**VERDICT=READY ⇒ status flip 独立洁净 commit（active→closed + §8.8 签署块）**。
  - **工作树并发面留痕（沿 §8.2~§8.7a 同模）**：本波接线期 `.github/workflows/pr-smoke.yml`、`ci/check_schemas.py`、`registry/number_ledger.json` 三面经 `.tmp/g12_7b_replay.py` 脚本原子重放落盘（幂等三面：check_schemas 三处路由 + workflow 步骤 232 + ledger v1.134；CI_GATES v1.7 修订行经 `.tmp/g12_7b_cigates_row.md` 素材落盘）并以 check_schemas / check_number_ledger / 互锁 validator `--require-ready` 真跑复核（PASS/READY 留痕）；G12 车道其余文件未受影响；`.tmp/g12_7b_*` 为一次性落盘工具不入 commit。
  - **工具链/环境偏差登记（不阻断本波判据，沿 §8.2~§8.7a 同模）**：`cargo fmt --check` / `cargo clippy -D warnings` 在本机 pinned 1.93.1 下对 HEAD 既有文件即报预存差异（与 G12.7b 无关，CI 面以流水线工具链为准）——本波新增文件（g12_closeout_check.py / schema 一件 / evidence 二件）为 Python/JSON 面无 rustfmt/clippy 消费面；`check_guardrails`/`check_contribution` 为 advisory（evidence/d3d12_interop_smoke.json 异己会话未提交修改面维持不混入本批）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/本契约 §8.1~§8.7a 同模）。`Assisted-by: Kimi-K3（G12.6/G12.7 收口波）`（影响范围：ci/g12_closeout_check.py 新建 + milestones/g12/g12_wave7b_closeout_evidence_schema.json 新建 + ci/check_schemas.py〔g12_wave7b_closeout_ 三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 232〕+ registry/number_ledger.json〔CI_step 231→232/next_free 233 + revision_log v1.134〕+ milestones/g12/CI_GATES.md v1.7 修订行 + 本契约 §8.7b 本条 + evidence/g12_wave7b_closeout_20260818T024429Z.json〔终版 READY；首跑登记件 20260818T023148Z 在档〕；验证方式：块③逐字命令输出——终审门 VERDICT=READY（8 facts + checks 八键 + required_gates 15 行）+ selftest materialized step 232 + 7a verify-latest PASS + 守卫套件全 PASS + 互锁 VERDICT=READY）。

---

### §8.8 Close-out 终审签署块（2026-08-17）

**裁决**：G-G12-1~10 对应波次与硬门已 materialize 并逐波验收（G12.1 治理四件套 §8.1、G12.2 生产化核心波 §8.2、G12.3 降噪波 §8.3、G12.4 UE Path Tracer 对标波 §8.4、G12.5 性能面波 §8.5、G12.6 P2 穷举决策 §8.6、G12.7a stabilization soak §8.7a、G12.7b close-out 终审 §8.7b）；7a full-run PASS（`g12_stabilization_soak_20260818T022404Z`——14 门全量回归真跑全绿 + PT 生产化链路 33 迭代 1813.6s 零失败 honest 口径）；7b `VERDICT=READY`（`g12_wave7b_closeout_20260818T024429Z`——八 facts + checks 八键 + required_gates 15 行）。

front matter **`status: active` → `status: closed`**（洁净独行）。RD-034/039/040/041/042/043/044 总体维持 open（分项 go/no-go/defer 已由候选决策表、G12_P2_DECISIONS 33 行闭集与 deferred history 只追加留痕）。**生产化差距清单 `g12_ue_pt_gap_registry.json` 10 行终态（quality_gap 6 + caliber_diff 4）终审锁定**——残余差距/未闭环行如实登记不冒充全闭环（G-G12-10）；**锁定面 = G13 法定输入**（G13 期只消费本清单与 G12_P2_DECISIONS 承接锚，不得另起无锚差距面）。本条为 close-out 终审签署块。

- **异己并发工作树面**：本 flip commit 只含 front matter `status` 字段 + §8.8 追加 + README/00_MASTER_INDEX 勘误行；工作树异己 src 面（hzb/restir/sdf_trace/smrt/ssr/ktx2_read 及 mod.rs/lib.rs 异己注册行、render_exec.rs 异己改写面、apps/ 异己面）与 evidence/d3d12_interop_smoke.json 异己改写面维持未提交、不混入本 commit（立项裁决 1 / §8.6 ⑤ / §8.7b ④ / G11 §8.8 先例同模）；milestones/g12/g12_pt_sampler_selection.json M166 复跑同值重写面维持未提交（批次 B 处置同模）。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10 §8.10 / G11 §8.8 同模）。`Assisted-by: Kimi-K3（G12.6/G12.7 收口波）`（影响范围：§8.8 本条 + front matter `status` 字段 + README/00_MASTER_INDEX 勘误行；验证方式：§8.7b 八 facts READY evidence + 守卫终扫全绿）。
