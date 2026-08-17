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
