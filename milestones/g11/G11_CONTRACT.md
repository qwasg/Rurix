---
contract: G11
title: G11 画质修复期
status: active
implementation_status: unblocked
active_scope: g11_1_governance_only + g11_2_plus_implementation_waves
version: v1.0
date: 2026-08-16
timebox: "G11.1 治理波即刻执行（G10 已 closed）；G11.2~G11.7b 严格波次，工期在实现互锁开放后由 measured baseline 校准"
rfc_required: "一份 Full RFC（编号按立项时实测 registry/number_ledger.json namespaces.RFC next_free=28 领取，禁推测号）：G11 GI 与光照画质闭环伞形 RFC（R4 多反弹 GI 修复语义 / M99-clipmap 世界辐射缓存世界级承接语义 / R3 灯种子集表达触光照语义面部分 / spec/global_illumination.md RXS-0360 世界级 not-triggered 登记翻转显式修订行 / C1 口径差对齐中 GI 与天光遮蔽语义面）。判档依据：M99-clipmap 世界级辐射缓存承接触 spec/global_illumination.md 冻结面（G5~G9 冻结面改动必须 RFC 显式修订行），判档争议向上取严。材质/纹理面（R1/U2/DDS）经评估不触 spec 语义冻结面 → Direct PR 面（触则升级 Full RFC 修订行，升级触发条件契约 §7 裁决 5/6 登记）。RFC 须 D-409 独立 provenance 对抗性评审后 Agent Approved 方为语义冻结；未 Approved 前本契约对应条款为引用占位"
upstream_docs:
  - "milestones/g11/G11_PLAN.md v1.0（八波结构、P0 建议清单 13 行 + go P1 1 行、风险表 R-G11-1~11、治理裁决表项的契约上游事实源）"
  - "milestones/g11/G11_CANDIDATE_DECISIONS.md v1.0（法定输入 11 差距行 + G10 defer 18 行 + 存续 open RD + 新增候选逐行映射）"
  - "milestones/g10/g10_gap_registry.json（G11 法定输入：11 行闭集终审锁定，每项带 UE5 模块归属 + measured delta + G11 承接锚）"
  - "milestones/g10/G10_CONTRACT.md §8.10（G10 closed 终态，2026-08-16，flip commit 27e3b07c + 幂等复跑批 53eb3a28）"
  - "milestones/g10/G10_P2_DECISIONS.md v1.0（27 行闭集；defer-to-G11+ 18 行承接锚）"
  - "milestones/g10/G10_DEFER_REEVALUATION.md v1.0（M99-clipmap rejudged-go 承接锚字面）"
  - "rfcs/0026-visual-comparison-metrics.md（度量口径冻结面——修复后复测的对拍基准）+ rfcs/0027-external-reference-harness-license.md（外部参照 harness 与许可边界）"
  - "registry/deferred.json RD-034/039/040/041/042/043/044（存续 open RD；只追加禁静默改判）"
  - "04 P-01/P-07/P-09/P-12/P-13；10 §3/§7/§9.5；14 §1/§3/§4/§5（同 G10 口径）"
implementation_unlock:
  required_all:
    - "G11.1 治理门全部完成且有真实验证记录"
    - "check_g11_implementation_interlock --require-ready 输出 READY（互锁 validator 机器事实，不以叙述替代）"
    - "共享编号按互锁开放时 actual next_free 重新校准；数字 CI 步骤不得沿用推测号与草案建议值"
in_scope:
  - g11_1_governance_only
  - rfc_gi_lighting_quality_closure
  - candidate_decisions_and_rd_mapping
  - p0_acceptance_mapping
  - measured_4070ti_baseline
  - g11_2_caliber_alignment_wave
  - g11_3_asset_scene_fix_wave
  - g11_4_lighting_gi_fix_wave
  - g11_5_ab_retest_closure_wave
  - g11_6_p2_exhaustive_decisions
  - g11_7_stabilization_and_closeout
out_of_scope:
  - g11_2_plus_while_implementation_interlock_is_red
  - g11_1_src_spec_conformance_semantic_implementation
  - g11_1_numbered_workflow_steps_or_stub_scripts
  - absolute_visual_quality_pass_line_deferred_to_g15
  - unanchored_new_fix_items_outside_g10_locked_gap_registry
  - path_tracer_productionization_implementation_deferred_to_g12
  - dlss_upscale_integration_implementation_deferred_to_g13
  - performance_optimization_implementation_deferred_to_g14
  - commercial_closeout_deferred_to_g15
  - gpu_pipeline_dual_ab_surface_deferred_to_g14
  - ue_source_or_binary_vendoring_into_rurix_repo
  - safe_gpu_operator_platform_remains_deferred_g11_plus
  - rewriting_g5_to_g10_closed_contracts_and_00_14
  - contract_param_or_g10_frame_library_rewrite
  - speculative_number_consumption
deferred_refs: [RD-034, RD-039, RD-040, RD-041, RD-042, RD-043, RD-044]
deliverables:
  - id: D-G11-1
    name: "G11.1 治理四件套：G11_PLAN（升格契约上游事实源）、G11_CONTRACT、CI_GATES、非空 measured g11_budget；status=active 且 implementation_status=blocked"
  - id: D-G11-2
    name: "G11.1 完整候选决策表：G10.8b 锁定差距清单 11 行 + G10 defer-to-G11+ 18 行 + 存续 open RD + G11 新增候选逐行映射（go / no-go / defer-to-G12+ / strategic_override + 承接锚）；缺行阻断 G11.2"
  - id: D-G11-3
    name: "G11.1 验收映射：全部 P0 各有独立 symbolic gate key、稳定脚本名、evidence schema 目标路径与判据；已 go 的 P1 同步覆盖"
  - id: D-G11-4
    name: "Full RFC-0028（G11 GI 与光照画质闭环伞形）经 D-409 独立 provenance 对抗性评审后 Agent Approved"
  - id: D-G11-5
    name: "G11.1 RTX 4070 Ti measured baseline 与非空 g11_budget（零 estimated：G10.1 baseline 锚复测重登记 + 11 行锁定差距 measured delta 闭环基线锚）；G11 validator 五件套落盘——implementation interlock 当前诚实报告 BLOCKED"
  - id: D-G11-6
    name: "G11.2 口径差对齐波：C1/C2/C3 逐行对齐闭环 + HDR-FLIP 独立标定（M144/M145/M146/M157）"
  - id: D-G11-7
    name: "G11.3 资产与场景面修复波：R1 材质子集 / R2 几何法线 / R5 i64 / U1 壳体 / U2 纹理〔DDS 面〕/ U3 动画逐行修复闭环（M147~M152）"
  - id: D-G11-8
    name: "G11.4 光照与 GI 修复波：R3 灯种子集 / R4 多反弹 GI + M99-clipmap 世界级辐射缓存承接（M153/M154）"
  - id: D-G11-9
    name: "G11.5 A/B 复测波（同契约双端复跑 + 复测差距清单 11 行逐项闭环核验 + 回归门，M155/M156）+ G11.6 P2 穷举 + G11.7a soak + G11.7b close-out（复测差距清单终审锁定）"
acceptance_gates:
  - id: G-G11-1
    check: "治理激活门：用户 2026-08-15「/goal G10~G15 六期分期 + 全期自主推进」指令留痕（G11.1 立项与 G11.2+ 开工授权同源）；agent 依 10 §7/P-13/D-406 v2.0 完全自主签署立项裁决留痕；十项立项裁决全部落定；G11.0 不可变 ref=53eb3a28 登记；仅 governance-only 范围 active"
  - id: G-G11-2
    check: "G11.1 完成门：D-G11-1~5 齐备并通过结构/schema/ledger/guardrail/预算核验；验收映射无缺行；无 src/spec/conformance 语义实现、无数字 workflow 空步骤；本门通过不自动开放实现"
  - id: G-G11-3
    check: "实现互锁门：check_g11_implementation_interlock --require-ready 输出 READY + 用户 G11.2 开工指令留痕（2026-08-15 指令全期授权面）+ 共享编号按 actual next_free 重新校准。任一条件不满足均保持 implementation_status=blocked"
  - id: G-G11-4
    check: "G11.2 退出门：M144/M145/M146 三个 P0 独立断言全绿（口径差逐行对齐闭环，残余口径差显式登记）；M157 P1 标定值入 g11_budget 且 provenance 齐备（P-09，禁手写阈值；estimated 冒充 measured 即 RED）；未对齐口径消费复测 delta 即 RED"
  - id: G-G11-5
    check: "G11.3 退出门：M147~M152 六个 P0 独立断言全绿（修复落盘 + 修复前后局部度量 delta 收敛 measured）；契约参数 digest 0-byte（相机/光照/seed 锁定值）；语料修订走 M133 只追加修订程序；未登记资产混入即 RED；不以局部绿色冒充 G11.5 复测闭环"
  - id: G-G11-6
    check: "G11.4 退出门：M153/M154 两个 P0 独立断言全绿；RFC-0028 语义面 spec-first 条款落地（RXS-0360 世界级登记翻转显式修订行）；不以 g9.p1.m99 屏幕级绿色冒充世界级验收；HDR 域 delta 收敛 measured（HDR-FLIP 标定值消费面）"
  - id: G-G11-7
    check: "G11.5 退出门：M155/M156 两个 P0 独立断言全绿——复测差距清单 11 行闭集逐项闭环核验（行集 == G10.8b 锁定清单逐字对账；新差距项显式登记即 RED 评审面）；契约 digest 不等仍出报告即 RED（门序硬约束继承）；单端缺帧聚合不得 PASS；回归门既有 48 门（G9 34 + G10 14）零降级"
  - id: G-G11-8
    check: "G11.6 决策门：G11 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G12+，零空行；defer 必有承接锚（机核同构 ci/g10_p2_decisions_check.py）；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述"
  - id: G-G11-9
    check: "G11.7a 稳定门：全部 P0 与所有 go 的 P1 全量回归；G5~G10 既有判据 0-byte；修复链路（复测出图/度量/差距清单装配）连续复跑 soak（量级沿 G10.8a 继承〔≥1800s〕或 measured 证明更短足够，阈值 G11.1 裁决 measured 标定）；strict budget 非空、零 estimated/skip；同日放行按立项裁决 7（7a full-run 先行完成后允许同日进 7b）"
  - id: G-G11-10
    check: "G11.7b 收口门：验收映射、候选决策、RD 最终状态逐字一致；全部 P0 独立断言均 PASS；evidence/schema/预算终审；复测差距清单终审锁定（残余差距/未闭环行如实登记不冒充全闭环）；§8 只追加后 status active→closed"
guardrails:
  - "双状态不可混同：status=active 仅表示 G11.1 governance-only 已立项；在 G-G11-3 真实通过前 implementation_status=blocked，任何治理完成叙述不得冒充 G11.2 开工"
  - "G11.1 允许 milestones/g11、G11 RFC、G11 专属 claim、deferred history 只追加、未编号 validator 与 measured baseline；src/spec/conformance 和编号 workflow 步骤 0-byte"
  - "G11 CI 只冻结 symbolic gate key 与脚本名；numeric_step 一律写 post-interlock actual-next-free allocation。不得沿用推测号与草案建议值，不得预放空 workflow、空脚本或空 schema 壳"
  - "每个 P0 必须独立布尔断言与独立 evidence subject；可共享一次进程执行，但聚合 PASS 不能遮蔽任一子断言 FAIL/SKIP"
  - "缺硬件/工具链仅可 dev_env_degrade 或 SKIP=not-triggered；两者均不充 P0 绿。host oracle、mock、isolated nonzero、既有最小见证、人工截图均不能替代目标门"
  - "修复范围唯一法定来源：G10.8b 锁定差距清单 11 行 + 每项承接锚字面；G11 不得无锚新立修复项；新发现差距进复测清单显式登记 + G11.6 穷举，不得静默混入"
  - "修复闭环判据 = 修复前后度量 delta 收敛 measured（复测 delta 相对锁定基线 delta 收敛，收敛阈值由 G11.2/G11.5 标定程序 measured 产出，禁手写）；G11 不设绝对画质通过线——「已达 UE5 画质」判定归 G15 商用收口期，G11 期内一律不成立"
  - "修复不得降级既有 48 门绿面（G9 34 key + G10 14 key）；G5~G10 closed 契约与判据 0-byte；回归门独立 P0 断言"
  - "复测对照口径：契约参数（相机/光照/seed/post）digest == G10.5 锁定值 0-byte；G10 帧库只读消费；语料修订走 M133 只追加修订程序（清单 digest 注册 + 修订行）"
  - "UE 源码仅外部参照只读：零 vendoring、零片段复制进 src/spec；违反即 revert + 留痕（RFC-0027 字面）"
  - "g11_budget 首个实现 PR 前必须非空 measured_local 且有 evaluator；全程零 estimated；性能数字不替代 correctness gate；阈值全部实测标定禁手写"
  - "新 unsafe 仅在实现互锁开放后按 actual next_free 登记并附 SAFETY；rurix-render 维持 forbid(unsafe_code)"
  - "触 G5~G10 冻结面必须 RFC 显式修订行（spec/global_illumination.md RXS-0360 世界级登记翻转只经 RFC-0028 修订行），禁静默扩；G5~G10 closed 契约与 00-14 0-byte，close-out 证据只追加"
  - "异己并发工作树面不混入：G11 车道 commit 只含 G11 车道文件；立项时工作树异己会话 src/ 未提交面保持不混入（G10.8b §8.10 先例同模）"
  - "新文件 LF + 尾换行；本契约合入后正文冻结，激活/验收/收口只追加 §8，除最终 status flip 外不回写既有事实"
---

# G11 契约 — 画质修复期

> 计划：[G11_PLAN.md](G11_PLAN.md) v1.0 · 候选决策：[G11_CANDIDATE_DECISIONS.md](G11_CANDIDATE_DECISIONS.md) · 机器门：[CI_GATES.md](CI_GATES.md)。
> 当前裁决：**G11.1 governance-only active；G11.2~G11.7b implementation blocked**。`active` 不是实现门绿灯。

---

## 1. 目标与双门状态

G11 是**画质修复期**：消费 G10.8b 终审锁定的 measured 差距清单 11 行（R1~R5/U1~U3/C1~C3，每项带 UE5 模块归属 + measured delta + G11 承接锚），先对齐口径差（C1~C3），再逐波修复资产/场景面（R1/R2/R5/U1/U2/U3）与光照/GI 面（R3/R4 + M99-clipmap 世界级辐射缓存承接），最终同契约双端复测并逐项闭环核验。「UE5 级」可核对基线沿用 G9/G10 口径 = UE 5.8；验收五层级沿用：核心等价、功能闭环、可降级、可生产化、Vulkan 主线。**G11 设修复闭环判据（修复前后度量 delta 收敛 measured，收敛阈值由标定程序 measured 产出）但不设绝对画质通过线**——「已达 UE5 画质」的绝对判定归 G15 商用收口期；路径追踪生产化归 G12、DLSS/超分归 G13、性能优化归 G14。

本契约拆分两种状态：

| 状态 | 当前值 | 含义 |
|---|---|---|
| `status` | `active` | G11.1 治理波已获授权，可落治理资产、Full RFC、候选决策/验收映射、G11 专属 claim、互锁 validator、RTX 4070 Ti measured baseline 与非空 budget |
| `implementation_status` | `blocked` | G11.2+ 尚未获准；当前不得改 `src/`、`spec/`、`conformance/`，不得 materialize 数字 CI 步骤 |

G-G11-3 是唯一实现入口：互锁 validator（`check_g11_implementation_interlock --require-ready`）输出 READY + 用户 G11.2 开工指令留痕 + 共享编号按 actual `next_free` 重新校准，三者齐备方可解锁；任一缺失均保持 `blocked`。

## 2. 范围与严格波次

### 2.1 G11.1 governance-only

G11.1 只做 D-G11-1~5。允许治理文档、Full RFC（须 D-409 评审后 Agent Approved 方为语义冻结）、候选决策表、验收映射、G11 专属无冲突 claim、互锁 validator、RTX 4070 Ti baseline 与非空 budget；禁止语义实现和编号 workflow。interlock validator 在当前事实下应明确返回 `BLOCKED`，这正是正确结果，不是失败需要被绕开。

### 2.2 G11.2~G11.7b implementation

实现互锁开放后按以下顺序推进，波次内可蜂群并行，波次间不得越级；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑：

```text
G11.2 口径差对齐波（C1/C2/C3 逐行对齐闭环 + HDR-FLIP 独立标定——先对齐口径否则修复无法被度量验证）
  → G11.3 资产与场景面修复波（R1 材质子集 / R2 几何法线 / R5 i64 / U1 壳体 / U2 纹理〔DDS 面〕/ U3 动画）
  → G11.4 光照与 GI 修复波（R3 灯种子集 / R4 多反弹 GI + M99-clipmap 世界级辐射缓存承接）
  → G11.5 A/B 复测波（同契约双端复跑 + 复测度量报告 + 复测差距清单 + 11 行逐项闭环核验）
  → G11.6 P2 穷举决策 → G11.7a stabilization/soak → G11.7b close-out
```

每波退出门见 YAML `acceptance_gates`（G-G11-4~7，判据按 G11_PLAN §2 各波退出门草案硬化）；任一上游门未绿，下游 evidence 即使局部成功也不能宣称波次完成。单点依赖：G11.2 是全部修复波的硬前置（口径差不对齐则修复闭环断言被口径噪声淹没）；G11.5 是全部修复闭环的统一核验面。

## 3. G11.1 交付冻结

| ID | 交付 | 退出判据 |
|---|---|---|
| D-G11-1 | 契约四件套与双状态 | PLAN v1.0、CONTRACT、CI_GATES、非空 measured budget 一致；`status=active`、`implementation_status=blocked` |
| D-G11-2 | 候选决策与 RD 总映射 | 法定输入 11 差距行 + G10 defer 18 行 + 存续 open RD + G11 新增候选逐行；裁决、波次、承接锚、最终状态无空项；缺行阻断 G11.2 |
| D-G11-3 | 验收映射 | 全部 P0 全部有独立 key/script/schema 目标路径/check；go 的 P1 同步入表；不存在"由邻项代绿"；缺行阻断 G11.2 |
| D-G11-4 | Full RFC-0028 | 经 D-409 独立 provenance 评审后 Approved（未 Approved 前本契约对应条款为引用占位）；编号登记与 README/ledger 一致 |
| D-G11-5 | baseline、budget、互锁 validator | RTX 4070 Ti measured 数据非空、零 estimated；interlock validator 对当前状态诚实报 BLOCKED；无空 workflow、无空 schema 壳 |

G11.1 完成仅关闭治理准备，不改变 G-G11-3 的机器事实。

## 4. 验收门与 P0 独立断言

### 4.1 波次验收门

G-G11-1~10 以 YAML 头为可提取摘要。[CI_GATES.md](CI_GATES.md) 冻结脚本与 evidence 形态。条件型分项的 `SKIP=not-triggered` 只表示决策已记录，不是成功；设备门的 `dev_env_degrade` 只表示环境缺失，也不是成功。

### 4.2 P0 独立断言

以下 13 行是 close-out 不可合并、不可删减的独立布尔断言（key 命名空间三方逐字一致，冻结）。一次 smoke 可以共享启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。evidence schema 目标路径统一为 `milestones/g11/g11_m<###>_<slug>_evidence_schema.json`——本契约只冻结路径，不预建文件。硬判据由 G11_PLAN §2 各波退出门草案与 §3 P0 建议清单展开为可机器求值形式，负例 RED 臂要求逐行写明。**修复闭环判据统一形态**：修复落盘（只消费 G10.8b 锁定清单对应行 + 承接锚字面）+ 修复前后度量 delta 收敛 measured（复测 delta 相对锁定基线 delta 收敛，收敛阈值由 G11.2/G11.5 标定程序 measured 产出，禁手写）+ 契约参数 digest 0-byte + 不降级既有 48 门绿面；各行的锁定基线 delta 字面见 [G11_ACCEPTANCE_MAP.md](G11_ACCEPTANCE_MAP.md) §1 与 [`g10_gap_registry.json`](../g10/g10_gap_registry.json) measured_delta 行。

| Symbolic gate key | M### | 最晚波次 | 稳定脚本名 | 独立硬判据 |
|---|---:|---|---|---|
| `g11.p0.m144.caliber_c1_indoor_luminance` | M144 | G11.2 | `ci/g11_caliber_c1_indoor_luminance_smoke.py` | GI/天光遮蔽口径差 + 太阳 lux→辐射度链差逐行对齐（对齐后残余口径差显式登记）+ 对齐前后口径参数 provenance 齐备；未对齐口径消费复测 delta 即 RED；拟合冒充对齐即 RED；残余口径差未登记即 RED |
| `g11.p0.m145.caliber_c2_exposure_chain` | M145 | G11.2 | `ci/g11_caliber_c2_exposure_chain_smoke.py` | 双端 EV100 同字面下派生尺度对齐（Rurix 臂 2^(−EV100) vs UE 臂 pipe 内手动曝光已施 ×1.0——统一或显式互证登记）+ 派生链元数据互证回归；派生尺度未对齐出 LDR 度量即 RED；互证链断裂即 RED |
| `g11.p0.m146.caliber_c3_exr_bit_depth` | M146 | G11.2 | `ci/g11_caliber_c3_exr_bit_depth_smoke.py` | UE EXR fp16→f32 提升口径（RXS-0385 strip-and-log）与 Rurix 原生 f32 度量域对齐登记 + 位深元数据闭集回归；位深截断注入即 RED；元数据缺字段即 RED |
| `g11.p0.m147.fix_r1_material_subset` | M147 | G11.3 | `ci/g11_fix_r1_material_subset_smoke.py` | R1 修复闭环：baseColorTexture/法线/metallic-roughness 采样接入（承接锚字面消费）+ 修复前后 LDR 臂度量 delta 收敛 measured（锁定基线 = bistro LDR SSIM delta 0.8328980787837229，收敛阈由标定程序产）+ 契约 digest 0-byte；未采样冒充修复即 RED；delta 未收敛冒充闭环即 RED；契约参数漂移即 RED |
| `g11.p0.m148.fix_r2_geometry_normals` | M148 | G11.3 | `ci/g11_fix_r2_geometry_normals_smoke.py` | R2 修复闭环：winding 朝向 + 双面翻转消费（平滑法线面承接锚字面）+ 修复前后 cornell HDR 覆盖 delta 收敛 measured（锁定基线 −0.7451210021972656）+ 与 U1 同面对账；法线未消费冒充修复即 RED；delta 未收敛冒充闭环即 RED |
| `g11.p0.m149.fix_r5_json_u64_seed` | M149 | G11.3 | `ci/g11_fix_r5_json_u64_seed_smoke.py` | R5 修复闭环：u64 顶格 seed 合法消费（i64 域 fail-closed 解除）+ 既有 seed=42 契约 digest 不变回归 + u64 边界语料锚定；顶格 seed 仍拒绝即 RED；既有 digest 漂移即 RED |
| `g11.p0.m150.fix_u1_cornell_shell_radiance` | M150 | G11.3 | `ci/g11_fix_u1_cornell_shell_radiance_smoke.py` | U1 修复闭环：cornell 壳体（墙/顶/地板）零辐射修复（语料派生面走 M133 只追加修订程序或双端着色口径对齐面）+ 修复后 UE 帧覆盖收敛 measured（锁定基线 = UE 覆盖 18.39% vs Rurix 92.90%，HDR nonzero 比 delta −0.7451210021972656）+ Rurix 侧覆盖面不降级；语料静默改写即 RED；覆盖未收敛冒充闭环即 RED；Rurix 侧降级即 RED |
| `g11.p0.m151.fix_u2_bistro_texture_dds` | M151 | G11.3 | `ci/g11_fix_u2_bistro_texture_dds_smoke.py` | U2 修复闭环：DDS 纹理解码面落地（G10-N7 承接锚兑现，Direct PR 面不触语义冻结面）+ 材质实例 texture_parameter_values 非空回归 + 修复前后 LDR 臂度量 delta 收敛 measured（锁定基线 = bistro LDR 亮度中位 delta 0.7698879749655723）；纹理仍全缺冒充修复即 RED；未登记资产混入即 RED；delta 未收敛冒充闭环即 RED |
| `g11.p0.m152.fix_u3_bistro_animation` | M152 | G11.3 | `ci/g11_fix_u3_bistro_animation_smoke.py` | U3 修复闭环：Bistro 动画 Take 001 / glTF 相机节点消费或显式静态契约登记闭环 + 包内动画通道计数对账（锁定基线 = 消费 0 vs 包内 2 通道）+ 相机位姿契约 0-byte；动画通道静默丢弃冒充闭环即 RED；相机契约漂移即 RED |
| `g11.p0.m153.fix_r3_light_subset` | M153 | G11.4 | `ci/g11_fix_r3_light_subset_smoke.py` | R3 修复闭环：点/面光源 + glTF emissive 表达（bistro 包内 4+ 盏实测消费）+ 修复前后 HDR 亮度中位 delta 收敛 measured（锁定基线 2.664779790997505）+ cornell 契约 sun+sky 灯面 0-byte；点光源未表达冒充修复即 RED；delta 未收敛冒充闭环即 RED；契约灯面漂移即 RED |
| `g11.p0.m154.fix_r4_gi_multibounce_world_cache` | M154 | G11.4 | `ci/g11_fix_r4_gi_multibounce_world_cache_smoke.py` | R4 + M99-clipmap 修复闭环：世界辐射缓存世界级 clipmap 级落地（G10.6 rejudged-go 承接锚字面 + RFC-0028 语义面 spec-first，RXS-0360 世界级登记翻转显式修订行）+ 修复前后 HDR 亮度 p90 delta 收敛 measured（锁定基线 4.697253086805343）+ 不以 g9.p1.m99 屏幕级绿色冒充世界级验收；世界级未落地冒充承接即 RED；屏幕级绿色冒充世界级即 RED；delta 未收敛冒充闭环即 RED |
| `g11.p0.m155.ab_retest_closure` | M155 | G11.5 | `ci/g11_ab_retest_closure_smoke.py` | A/B 复测闭环：同契约双端复跑（契约参数 digest == G10.5 锁定值，不等仍出报告即 RED）+ 复测度量报告 + 复测差距清单 11 行闭集落盘（行集逐字对账；新差距项显式登记即 RED 评审面）+ 逐项闭环状态机核（修复前后 delta 收敛 measured，收敛阈由标定程序产）；清单缺行即 RED；单端缺帧聚合 PASS 即 RED |
| `g11.p0.m156.regression_guard` | M156 | G11.5 | `ci/g11_regression_guard_smoke.py` | 修复回归门：既有 48 门（G9 34 key + G10 14 key）最新 evidence 全绿只读汇总 + 修复触改面既有门重跑回归零降级；既有门降级即 RED；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G11.7b。M157（HDR-FLIP 独立标定）为 P1，入验收映射随主门核验。

## 5. Guardrails

见 YAML `guardrails`。特别强调四点：

1. 治理 active 不等于实现 active；G-G11-3 的机器事实（validator READY + 用户 G11.2 开工指令 + actual `next_free` 重校）不可替代。
2. 数字 CI 步骤只能在实现互锁开放后读取 actual `next_free` 再分配；文档中的稳定身份是 symbolic gate key 和脚本名；禁止沿用草案建议值。
3. **修复范围唯一法定来源 + 闭环判据 measured**：G11 只消费 G10.8b 锁定清单 11 行 + 承接锚；每修复项的闭环判据 = 修复前后度量 delta 收敛（measured，收敛阈值由标定程序产出，禁手写）；G11 不设绝对画质通过线（归 G15）。
4. **回归零降级 + 复测对照口径**：修复不得降级既有 48 门绿面；契约参数 digest == G10.5 锁定值 0-byte，修复动契约参数则复测无对照意义（门序硬约束阻断）。

## 6. Deferred 处置

| Deferred | G11 处置 |
|---|---|
| RD-039 | 总体维持 open 为法定输入；M61 mesh shader 分项维持 defer（G10.6 重判 maintain-defer 逐字承接，承接锚字面 0-byte，G11+ 重评窗顺延 G12+ 不关闭）；M44-p4 超显存语料触发评估未实证维持 no-go 留档；其余分项未触发维持 open |
| RD-040 | M52 SER 维持 defer（锚定 G12 重评，语义面留 RFC-0023 冻结面不接线）；**M99-clipmap 承接兑现启动**（G10.6 rejudged-go 逐字承接 → G11.4 世界辐射缓存世界级落地，RFC-0028 语义冻结面）；M100-high 维持 defer（G10.6 重判 maintain-defer 逐字承接——R3 修复后多灯 workload measured 对照面若产出让 G11.6 穷举重评）；RD040-nrd 维持 no-go（G13 窗）；history 只追加 |
| RD-041 | M28/M40-svt/M26-fg/M05-mv/M56-wg 维持 no-go 留档；DLSS/Streamline 方向登记维持（G10-N5 锚定 G13，G11 仅档案零接线） |
| RD-044 | M126-rd044/RD044-continuum/RD044-fluid 维持 open-留档/观察；G11 度量面 FLIP 图像度量与 RD-044 族 FLIP 流体防混淆登记维持（G10 口径字面） |
| RD-034 | DXIL RT/mesh 上游 blocked 维持 open；G11 复测仅 Vulkan 主腿 + host 参考管线臂，不阻主线 |
| RD-042/043 | 可微物理观察 / wgrapier GPU 刚体观察维持，不进 G11 任何面 |

详情始终以 `registry/deferred.json` 为唯一事实源；本表只冻结承接纪律。G10 defer-to-G11+ 18 行逐行处置归 [G11_CANDIDATE_DECISIONS.md](G11_CANDIDATE_DECISIONS.md) §2；SAFE-GPU 维持「G10+ 独立期立项」defer（G11 非其独立期，沿 G10 立项裁决 7 口径）。

## 7. 修订记录与开工裁决

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-16 | 初版契约：按 G11_PLAN v1.0 显式拆分 governance 与 implementation；G11.1 active、G11.2+ blocked；冻结波次门（G-G11-1~10）与 13 个 P0 独立断言（key 命名空间三方逐字一致）+ 1 个 go P1（M157）；CI 数字延迟到 post-interlock actual-next-free allocation；十项立项裁决逐字登记；§8 只追加区启用。 |

**开工裁决留痕**：

- **用户立项指令**：2026-08-15 主会话下达「/goal 帮我完成 G10-15 的内容，自主派发调研 agent 和进行决策，里程碑推进时组织 agent-team 完成，要求彻底完成对标 UE5 渲染器的目标，并支持 dlss、超分采样、路径追踪等前沿技术。技术完成需要严格的画面审查，需要获取完整渲染画面，再用本地已有的 UE5 渲染器出图对比，修复画面中出现的细节问题；同时优化渲染管线效率，使帧率对标 UE5 略高（不降级画质）。本地已有 UE5 渲染器参考项目，你也可以联网获取（我的 GitHub 在 UE5 组织内），同时支持联网获取压测模型环境等必要工具集。最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在 G15 后无限制新建里程碑继续优化」（指令原文以会话留痕为准，G10_CONTRACT §7 同字面援引）。该指令授权 G10~G15 六期分期与全期自主推进——**G11.1 立项与 G11.2+ 开工授权同源**：「修复画面中出现的细节问题」即 G11 画质修复期的用户目标字面；本治理波为该指令在 G11 期的执行留痕（2026-08-16 G11.1 治理波任务下达：起草 G11 治理四件套 + 候选决策表 + 验收映射并完成立项留痕）。
- **agent 立项裁决**：依 10 §7、P-13 与 D-406 v2.0，agent 完全自主签署立项裁决；G11.1 治理波即刻 active，G11.2+ 继续由 G-G11-3 硬阻断。
- **不可变基线**：G11.0 文档集不可变 ref = `53eb3a28`（G10 close-out 幂等复跑批 HEAD；flip commit `27e3b07c`；工作树带异己会话 src/ 未提交面——处置见裁决 1）。
- **十项立项裁决（逐字登记）**：
  1. 现在立项；G11.0 不可变 ref=`53eb3a28`；**带未提交项立项**——工作树异己会话 src/ 未提交面（rurix-asset/rurix-render geometry/gi/shadow/ssr/ktx2/hzb/restir/sdf_trace/smrt 声明面）保持不混入 G11 车道（G10.8b §8.10 先例同模：flip/治理 commit 只含本车道文件，异己面维持未提交）。
  2. 修复范围唯一法定来源 = `milestones/g10/g10_gap_registry.json` 11 行闭集 + 每项 `g11_anchor` 承接锚字面；G11 不得无锚新立修复项；新发现差距进复测清单显式登记 + G11.6 穷举。
  3. 修复闭环判据 = 修复前后度量 delta 收敛 measured（复测 delta 相对锁定基线 delta 收敛，收敛阈值由 G11.2/G11.5 标定程序 measured 产出，禁手写）；**G11 不设绝对画质通过线**——「已达 UE5 画质」判定归 G15 商用收口期。
  4. **M99-clipmap 承接确认**：G10.6 重评窗 rejudged-go 逐字承接——G11.4 承接世界辐射缓存世界 clipmap 级（只消费 G10.8b 锁定清单 R4/C1 行 + 承接锚）；兜底 = 屏幕级 SPG + Radiance Cache（g9.p1.m99 门绿）维持，不以屏幕级绿色冒充世界级验收；语义面经 Full RFC-0028 冻结（spec/global_illumination.md RXS-0360 世界级 not-triggered 登记翻转走显式修订行）。
  5. **RFC 判档**：GI 面 = Full RFC-0028（M99-clipmap/R4 多反弹触 spec/global_illumination.md 冻结面，判档争议向上取严）；R1 材质/R2 法线/R5 i64/U1 壳体/U2 纹理/U3 动画/C1~C3 口径对齐修复 = Direct PR 面（经评估不触 spec 语义冻结面；实现波触及冻结面即升级 Full RFC 显式修订行）；DDS 纹理解码 = Direct PR 面（不触语义冻结面）。
  6. R1 材质修复升级条款：修复限 A/B harness host 参考管线消费面 → Direct PR；若波及 GPU 材质着色语义面（MaterialClosure 32B / display_pipeline 冻结面）→ 升级 Full RFC 显式修订行（判档争议向上取严）。
  7. G9.8b/G10.8b 同日放行先例 = 继承（7a full-run 先行完成后允许同日进 7b close-out；先例字面不扩展解释）。
  8. 复测臂口径 = 同 G10.5 host CPU 参考管线臂 + UE 5.8.1 MRQ 臂（契约参数 digest == G10.5 锁定值 0-byte；GPU 管线双端 A/B 面锚定 G14 不动，G10-N16 承接锚字面）。
  9. G10 defer-to-G11+ 18 行逐行处置：画质修复相关行承接（N7 DDS 解码面 → U2 修复面 G11.3 兑现；N10 HDR-FLIP 独立标定 → G11.2 M157 兑现；N17 M137 scalars.flip 演进位 → G11.5 触发评估；N6 BistroExterior → G11 语料扩容触发评估登记；N8 renderoffscreen / N11 M141 采样形态 / N16 GPU 管线面维持 defer 锚定 G14；N5 锚定 G13）；十锚 M99-clipmap 承接确认（裁决 4）、其余九锚维持 defer 承接锚字面 0-byte——逐行落 [G11_CANDIDATE_DECISIONS.md](G11_CANDIDATE_DECISIONS.md) §2。
  10. 压测资产二进制**不入 git**（外部缓存 K: 盘，仓库内只登记清单/许可/digest 元数据——沿 G10 裁决 9）；数字 CI 步骤 `post-interlock actual-next-free allocation` 重申确认。
- **G15 后无限续期授权登记**：用户指令「允许在 G15 后无限制新建里程碑继续优化」留痕（G10_CONTRACT §7 同字面援引）——G15 收口若未达真实可商用标准，按同治理范式续立 G16+（每期仍独立走立项/治理波/互锁/full-run，不因授权免除任何机器门）。
- **RFC 编号**：Full RFC 编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free=28` 领取（RFC-0028）；RXS/RD/U/RX/数字 CI 均延迟到实现互锁开放后按 actual `next_free` 领取。

---

## 8. Implementation activation / Close-out（只追加区）

<!-- 首条未来记录只能是 G-G11-3 互锁实测与 implementation_status 解锁凭据；其后追加逐波验收与 close-out。当前不得写 PASS、不得预填 run URL。 -->

### §8.1 G-G11-3 implementation_status 解锁记录（2026-08-16）

- **互锁实测**：`py -3 ci/check_g11_implementation_interlock.py --require-ready` **VERDICT=READY，exit=0**（2026-08-16 真跑留痕）——事实门六条全绿：① G11_CONTRACT status=active；② G11.1 治理交付四件齐备（G11_PLAN / G11_CANDIDATE_DECISIONS / G11_ACCEPTANCE_MAP / CI_GATES）；③ g11_budget.json 非空可加载零 estimated（13 条 measured_local：2 bench 回归守护锚沿 G10.1 同协议复测〔1.2034 ms / 26.2772 GB/s，evidence/g11_baseline_*_20260816T081700Z.json〕+ 11 行锁定差距 measured delta 闭环基线锚〔evidence/g11_closure_baseline_*_20260816T084102Z.json，上游 M140 门真跑复核清单完整在位〕）；④ RFC-0028 Agent Approved（D-409 第 1 轮独立评审会话 12 findings〔3 high + 5 med + 4 low〕全部 disposition，v0.2 修法批；同环境单一模型 provenance 偏差按 v1.73/v1.90/v1.102 先例如实登记 §9.1 并留 G11.7b 终审复核锚）；⑤ ledger reserved_in_flight[G11] 登记在树（number_ledger v1.113）；⑥ `ci/check_g11_acceptance_map.py` 三向比对 PASS（13 P0 + 1 go P1，14 key）。一致性门 C1~C4 全绿（C3 数字步骤零预占 0 处 / workflow g11 token 0 处 / ci/g11_*_smoke.py 预放零 / C4 三面 g11 实现面命中 0 处——裸字面「G11」在 spec RXS-0391 合法存续，C4 扫描面校准为 gate-key/脚本命名 token，validator docstring 登记）。`--selftest` 12 RED + 1 GREEN + 1 TREE 全过。
- **用户开工指令留痕**：2026-08-15 主会话「/goal 帮我完成 G10-15 的内容，自主派发调研 agent 和进行决策，里程碑推进时组织 agent-team 完成，要求彻底完成对标 UE5 渲染器的目标……修复画面中出现的细节问题……最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在 G15 后无限制新建里程碑继续优化」指令（G10_CONTRACT §7 同字面援引）——G10~G15 六期分期与全期自主推进授权，**G11.1 立项与 G11.2+ 开工授权同源**（契约 §7 逐字登记）；2026-08-16 G11.1 治理波任务下达（起草 G11 治理四件套 + 候选决策表 + 验收映射并完成立项留痕）为该指令在 G11 期的执行留痕。
- **编号重校准**：G11.1 全程零数字 claim——RFC-0028 按立项实测 namespaces.RFC `next_free=28` 领取（number_ledger v1.113：on_tree_max 27→28、next_free 28→29）；RXS（next_free=392 快照）/ CI_step（next_free=196 快照）/ RD（next_free=45）/ U / RX_error / MR / SG / D 均维持不动，一律 `post-interlock actual-next-free allocation`（互锁开放后重读 actual next_free 再按需 claim/materialize，禁推测号与草案建议值）；`py -3 ci/check_number_ledger.py` PASS（spec RXS 头 373 个零同号碰撞；ledger 14 命名空间保留号被尊重）。
- **front matter flip**：`implementation_status: blocked` → `unblocked`、`active_scope: g11_1_governance_only` → `g11_1_governance_only + g11_2_plus_implementation_waves`（本条同批）。G-G11-3 三条件（validator READY + 用户开工指令留痕 + 编号按 actual next_free 重新校准）齐备留痕；G11.2 起每个实现 PR 必须把 `check_g11_implementation_interlock --require-ready` 作为前置 required check，spec-first + RED 先行。
- **异己并发工作树面**：本 flip 只含 front matter 双字段 + 本条追加；工作树异己会话 src/ 未提交面（rurix-asset/rurix-render geometry/gi/shadow/ssr/ktx2/hzb/restir/sdf_trace/smrt 声明面）维持未提交、不混入本批（立项裁决 1 / MAP §3.1，G10.8b §8.10 先例同模）。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10 §8.1 同模）。`Assisted-by: Kimi-K3（G11.1 治理波）`（影响范围：G11_CONTRACT front matter 双字段 + §8.1 本条；验证方式：`py -3 ci/check_g11_implementation_interlock.py --require-ready` READY exit=0 + `--selftest` 12 RED+1 GREEN+1 TREE 实测输出留痕）。
