---
contract: G12
title: G12 路径追踪生产化期
status: active
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
