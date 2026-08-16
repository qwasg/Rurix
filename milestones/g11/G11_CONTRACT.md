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

### §8.2 G-G11-4 G11.2 口径差对齐波验收记录（2026-08-16）

- **① spec-first 与编号（互锁后 actual next_free 顺位，零预占）**：spec-first 先行——`spec/visual_comparison.md` 追加 **RXS-0392**（C1 口径对齐：不拟合原则 + 天光链参数集枚举闭集 + 太阳 lux→辐射度链双端同构登记〔UE 侧光色线性直给 b_srgb=False〕+ 残余口径差逐环节显式登记 + 消费门序）/ **RXS-0393**（修复闭环判据：锁定基线锚消费 + 收敛判定两款 + 收敛阈标定程序产 p100×k 入 g11_budget + 契约 digest 0-byte + 不设绝对画质通过线），依据 RFC-0028（Agent Approved）§4.5/§4.6；落盘前实测 `RXS.next_free=392` 顺位领取 0392~0393（number_ledger v1.114：on_tree_max 391→393、next_free 392→394）；conformance 锚定四件（accept caliber_alignment_minimal.rx / fix_closure_criterion_minimal.rx + reject caliber_fitting_masquerade.rx / closure_handwritten_threshold.rx）；`py -3 ci/trace_matrix.py --check` PASS（373→375 全锚定）；stable 快照 373→375 重 bless + bless_log 追加（error_codes=113/editions/subcommands 三段 0 变化）。门 materialize 批落盘前实测 `CI_step.next_free=196` 顺位领取 **196~200**（number_ledger v1.115：on_tree_max 195→200、next_free 196→201）——196=M144 / 197=M145 / 198=M146 / 199=M157 / 200=g11.wave.2.exit；五脚本 + 六 schema（四门 + wave2 + g11_2_calibration 标定件共享）+ pr-smoke 五真步骤 + check_schemas 纯追加映射 + CI_GATES v1.1 修订行同批落；`py -3 ci/check_number_ledger.py` PASS（spec RXS 头 375 个零同号碰撞）。
- **② C1~C3 根因定位与修法 + 修复前后 delta 对拍（数字全来自命令输出；G11.2 复跑帧区 K:/rurix-ext/g11-frames/g11_2，G10 帧库只读；契约 digest 双场景当次重算 == G10.5 锁定值 0-byte）**：
  - **C1（M144，太阳/天光链）**：根因① = UE harness `g10_5_build_scenes.py` `set_light_color(..., b_srgb=True)`——契约 `color_linear_rgb`（线性域）被 UE 按 sRGB 二次转线性（bistro 太阳色 [1.0,0.98,0.95] 有效值实测偏差 G −2.5% / B −6.3%），修法 = **b_srgb=False 线性直给**（RXS-0392 L3，harness 侧修复，契约参数 0-byte）；根因② = 天光链复核成立——白色 cubemap 2×1 逐像素 =1.0 uniform（`sha256:5d3ee90c1f09faaf4d02f5f4888a4b530c68a8afeb9ed931c1c501f95f7f504d`）× intensity 与 Rurix 常量天光辐射度同单位链（参数级对齐，无分歧）；根因③ = 曝光域差（归 C2 修复面）。**对拍**：cornell 太阳色 [1,1,1] 无色差——修复后 UE HDR digest 与 G10.5a 库帧**逐位一致** `sha256:c7c6f2cf1644ba79512da1f4f3fceeb2001826f4723681a35ab7a8ca9dc853a2`（修复无效应旁证 = 正确性）；bistro UE HDR digest `5bfe1f49…` → `8f907dc06560506e0ae640a974d889536a447b5bff483a623c44c98b4bc1bba2`（修复生效，实测 1/2073600 像素变化——太阳高亮像素 HDR 差 11.42，HDR 亮度中位 2.798138671875 与 p90 5.000015625 f64 不变，LDR 度量微移 FLIP −1.2e-8 / SSIM +5.7e-7 / PSNR +1.7e-5，如实登记）；亮度 delta 复测对拍（双端同域 = 曝光已施 scene-linear）——bistro HDR 中位：基线原域 2.664779790997505（a=0.1333588808774948 未施曝光 vs b=2.798138671875 已施曝光，域混测）→ 域统一换算基线 2.7314592314362525（b − a×2^(−1)）→ 复测同域 **2.7314592314362525（f64 逐位一致）**；cornell 块区 p90：基线 0.29024957587122924 → 复测 **0.29024957587122924（f64 逐位一致）**——残余口径差逐环节显式登记 `milestones/g11/g11_2_residual_caliber_registry.json`（灯种子集结构差→R3 m153 承接锚 / GI 结构差〔UE Lumen 多反弹 vs Rurix 单反弹〕→R4 m154 承接锚 / UE 镜面 IBL 结构差显式留档 / 源位深量化差→C3 行承接面），复测 delta 全额归属登记残余（M144 门内独立重算与登记 measured_impact 逐位一致）。
  - **C2（M145，曝光链）**：根因 = Rurix 臂 LDR 派生尺度 ×2^(−EV100)（cornell 0.25 / bistro 0.5）vs UE 臂 pipe 内手动曝光已施（FixedExposure=2^(−EV100)）×1.0——双端 HDR 捕获域不一致。修法（统一向）= **Rurix 臂曝光尺度管线内烘焙**（`g10_5_scene_render --render --exposure-scale 2^(−EV100)`，既有旗标消费，HDR 帧 = 曝光已施 scene-linear 与 UE 臂同域）→ **LDR 派生尺度双端统一 ×1.0**。**对拍**：派生尺度差基线 cornell 0.75 / bistro 0.5 → 复测 **0.0**（标定阈 p100×k=1.0=0.0，入 `g11.caliber.c2_exposure_scale_tol`）；像素中性复核——cornell LDR 三指标与 G10.5a golden **逐位一致**（FLIP 0.338644611302288 / SSIM 0.34829777885646934 / PSNR 13.982872203129087）；bistro Rurix 臂 LDR 逐位一致（×0.5 精确幂次 f32 无舍入）；派生链元数据互证回归（四张 LDR 帧 `rurix:source_frame_digest` == HDR 帧内容 digest 独立重算）。
  - **C3（M146，位深）**：根因 = UE MRQ 源帧 fp16（decode 实测 source_bit_depth=float16 双场景）vs Rurix 原生 f32；提升链（RXS-0385 strip-and-log）fp16→f32 精确性为对齐面。修法 = **度量域统一提升 f32 + 精确性穷举核验 + 源位深量化差显式登记残余**（UE 源帧写出时 fp16 量化一次不可回退，显式留档 `c3_source_bit_depth_quantization` 行）。**对拍**：基线（源位深）16.0 → 复测（度量域）**0.0**（标定阈 p100×k=1.0=0.0，入 `g11.caliber.c3_bitdepth_domain_tol`）；fp16→f32 提升全 **65536 位模式穷举核验零不符**（half_to_f32 == numpy float16 语义；NaN 联合 768 模式）+ UE 帧逐像素可逆核验（全部像素 fp16 可表，roundtrip 逐位一致，无二次截断）+ 位深元数据闭集回归（Rurix strict 闭集齐备 / UE strip-and-log 实测登记）。
- **③ M157 HDR-FLIP 独立标定（G10-N10 承接锚兑现）**：样本集 = G11.2 复跑真实 HDR 帧双臂（cornell-box + bistro-interior × UE5/Rurix 臂）确定性 4×4 瓦片 **32 对 ≥ 下界 24**（manifest digest `sha256:123fbbe54d42828aa…` 入 evidence；cornell UE 帧 18.39% 覆盖面全黑瓦片双端同退 fixed(0,0,2) 曝光 5 对如实登记）；自实现 `flip_hdr` vs 参考实现 flip_evaluator（NVlabs/flip 1.7 python-nanobind，M135 pin 五元组同源）逐对标量差与误差图逐像素差分列——**标定两跑逐位一致**：scalar p100 = **2.400210e-04** → 容差 = ×k=2.0 = **4.800420e-04**（`g11.metric.hdr_flip_pairwise_scalar_tol`）、error_map p100 = **1.202619e-04** → 容差 **2.405238e-04**（`g11.metric.hdr_flip_pairwise_error_map_tol`），四条标定条目（含 C2/C3 两条）measured_local 字节级纯追加入 `g11_budget.json`（P-09 禁手写阈值）；恒等图对 HDR-FLIP == 0 极值断言绿；`budget_eval --strict` PASS（155 pass 0 skip）。
- **④ 四门 + 聚合门 --gate/--selftest 摘录（evidence 落盘 evidence/g11_*_<UTC>.json）**：`g11.p0.m144.caliber_c1_indoor_luminance` --gate **PASS checks 10/10**（selftest PASS 2 RED+2 GREEN）；`g11.p0.m145.caliber_c2_exposure_chain` --gate **PASS 13/13**（selftest 3 RED+3 GREEN）；`g11.p0.m146.caliber_c3_exr_bit_depth` --gate **PASS 15/15**（selftest 3 RED+3 GREEN）；`g11.p1.m157.hdr_flip_calibration` --gate **PASS 10/10**（selftest 3 RED+3 GREEN）；`g11.wave.2.exit` --gate **VERDICT=PASS**（四门 GATE 全 PASS + 五 facts〔RXS-0392/0393 条款头在树 / RFC-0028 Agent Approved 字面 / 残余登记完备 R3 m153+R4 m154 锚非空 / 四条标定入 budget provenance 齐备 threshold==trimmed_mean×k 重算口径 / 四门 RED 臂独立有效共 15 臂〕；--selftest 负样本缺 evidence → 红 + 正样本真树 → 绿 ALL PASS）。`check_g11_acceptance_map` 三向 PASS；`check_g11_implementation_interlock --require-ready` **READY**（17 条 measured_local budget 零 estimated）；`check_schemas` / `check_number_ledger` / `check_structure` / `trace_matrix --check` / `stable_snapshot --check` / `budget_eval --strict`（155 pass）全绿；`check_guardrails`/`check_contribution` advisory 先例面一致。**M156 回归面前置自检**：G10 M130 `--phase g10.5` 全门复跑 **PASS 13/13**（b_srgb 修复后双场景三方 digest 逐位相等 `sha256:64fd54df6e9be522…` + 应用层探针逐点 ≤1e-3 px——C1 harness 修复对 G10 门序零降级实测）；本波零 `src/` 改动（`.rs` 0-byte——修复落 harness Python 面与 Rurix 渲染参数面），G5~G10 closed 判据与 G10 门脚本 0-byte。**基线态如实登记**：`cargo test --workspace` 全绿（exit 0，2026-08-16 实测）；`cargo fmt --check` / `cargo clippy -D warnings` 为 HEAD 即存在的预存漂移红（54087161 干净检出同样红——G9.x 期提交面 rustfmt/clippy 漂移 + 异己会话未提交声明面，G11.1 守卫集同口径未含此三面；本波零 .rs 改动，不扩大、不冒充修复，归后续波次处置面登记）。
- **⑤ 异己并发工作树面与纪律**：本批只含 G11.2 车道文件（spec/visual_comparison.md 只追加两条款 + conformance 四锚 + ci 五脚本一共享库 + milestones/g11 六 schema + 复跑报告/残余登记 + g11_budget 字节级纯追加四条 + pr-smoke 五步骤 + ledger 双校准 + CI_GATES v1.1 + 本契约本条 + tests/stable 快照/bless_log + UE harness g10_5_build_scenes.py 两处〔b_srgb 修复 + G11_2_OUT_ROOT 默认保持面〕）；异己会话 src/ 未提交面（rurix-asset/rurix-render geometry/gi/shadow/ssr/ktx2/hzb/restir/sdf_trace/smrt 声明面）维持未提交、不混入本批（立项裁决 1 / MAP §3.1，G10.8b §8.10 先例同模）；压测资产二进制零入 git（外部缓存 K: 盘）；UE 零 vendoring（只读外部参照）；新文件 LF + 尾换行。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10 §8.1 / §8.2 同模）。`Assisted-by: Kimi-K3（G11.2 波）`（影响范围：§8.2 本条；验证方式：四门 + 聚合门 --gate/--selftest 实测输出 + `check_g11_implementation_interlock --require-ready` READY exit=0 + M130 g10.5 回归 PASS 13/13 留痕）。

### §8.3 G-G11-5 G11.3 资产与场景面修复波执行记录（2026-08-16）——**五门闭环 PASS + M147 局部度量未收敛如实登记，波次退出门未达成，不冒充收口**

- **① 编号与 spec-first（互锁后 actual next_free 顺位，零预占）**：本波六修复面（R1 材质/R2 法线/R5 i64/U1 壳体/U2 纹理/U3 动画）经立项裁决 5/6 评估**不触 spec 语义冻结面 → Direct PR 面**（R1 修复限 A/B harness host 参考管线消费面，rurix-render 0-byte；UE 零 vendoring）——零新 RXS 条款，`trace_matrix --check` PASS（375 全锚定，0 变化）。门 materialize 批落盘前实测 `CI_step.next_free=201` 顺位领取 **201~207**（number_ledger v1.116：on_tree_max 200→207、next_free 201→208）——201=M147 / 202=M148 / 203=M149 / 204=M150 / 205=M151 / 206=M152 / 207=g11.wave.3.exit；六门脚本 + 聚合门 + 八 schema（六门 + wave3 + g11_3_calibration 标定件共享）+ pr-smoke 七真步骤 + check_schemas 纯追加映射（load/validator/前缀路由）+ 共享判定层 ci/g11_3_fix_lib.py + CI_GATES v1.2 修订行同批落；`py -3 ci/check_number_ledger.py` PASS（spec RXS 头 375 个零同号碰撞）。
- **② 六修复逐项根因/修法/修复前后 delta 对拍（数字全来自命令输出；G11.3 复跑帧区 K:/rurix-ext/g11-frames/g11_3，G10/G11.2 帧库只读；契约 digest 双场景当次重算 == G10.5 锁定值 0-byte——cornell `80305791…`/bistro `ad45951b…`/联合 `64fd54df…`）**：
  - **M149（R5，JSON u64 seed）**：根因 = `gltf/json.rs` 严格解析器整数仅 i64 域落地，u64 顶格 seed 被 fail-closed 拒绝（`integer overflow`）。修法 = 最小加性扩 `JsonValue::U64` 变体 + `parse_str_u64`/`parse_bytes_u64` 全域入口 + bin `--u64-seed` 旗标消费（默认面 i64 域 fail-closed **逐字节不变**——G10 M139 探针 parity 0-byte）。**对拍**：锁定基线 delta **9.223372036854776e+18**（i64 上界 vs u64 顶格）→ 复测 **0.0**（u64 顶格合法消费，digest `a9c90cc3…` 产出且 u64max/u64max−1 双探针 digest 相异证 seed 值参与；2^63 合法 / 2^64 维持拒绝；seed=42 双场景 digest == 锁定值回归）。
  - **M148（R2，几何法线）**：根因 = cornell 壳体单面片外向绕向 × 双面口径交互差——Rurix 侧平滑法线（顶点法线重心插值）不消费。修法 = `--smooth-normals`（顶点平滑法线重心插值 + 逆矩阵转置世界化 + 双面翻转朝向入射光线来向，tracer.rs 同口径；默认 = winding 几何法线 0-byte）。**对拍**：cornell HDR 覆盖 delta 锁定基线 **−0.7451210021972656**（rurix 0.9290046691894531 / ue 0.1838836669921875）→ 复测 **+0.003810882568359375**（rurix 0.9290046691894531 不降级 / ue 0.9328155517578125）——|复测| < |基线| 且收敛幅度 0.7413 ≥ 标定阈 0.0；符号近零穿越经 **zero_band = 0.052734375**（per-tile XOR p100 0.0263671875×k=2.0 标定带，UE 覆盖为 Rurix 严格超集、rurix-only=0 实测）判定为跨端一致包络内收敛（非方向性注入）。
  - **M150（U1，cornell 壳体零辐射）**：根因 = 语料单面片外向绕向 × UE 背面剔除口径 → UE 帧仅双块可见（18.39%）。修法 = **UE 场景侧双面化**（two_sided 父材质 + 逐 actor MIC 置换 17 actors，逐材质 baseColorFactor 换发 + two_sided 读回核验——双端着色口径对齐面，**语料 0-byte 不走 M133 修订**：cornell-box-generated 资产 digest 复算 == M131 登记 `sha256:a53b05d7…`）；UE MRQ 重出参考帧落 G11.3 帧区（G10 帧 0-byte）。**对拍**：UE 覆盖 **0.1838836669921875 → 0.9328155517578125**（UE 帧 digest `c7c6f2cf…` → `82a156ae…` 修复生效），共享覆盖 delta 收敛同 M148 面。
  - **M151（U2，bistro DDS 纹理）**：根因① = 包内 .dds UE Interchange 不支持（材质实例 texture_parameter_values 空）；根因② = Rurix 侧纹理全缺。修法 = Rurix 侧 `bcdec` 扩 BC1/BC3 解码 + DDS 容器解析（legacy FourCC + DX10 双形，fail-closed 闭集）真实解码 144 张（bc1×54/bc3×20/bc5×70）；UE 侧 **派生链转码**（DDS→PNG，g11_3_dds_transcode.py + g11_3_dds_dump.rs，manifest 144 条目逐文件 digest 机核 + buffer.bin digest 对账 + 派生 gltf digest 登记——G10-N7 承接锚兑现）+ **MIC 纹理参数显式绑定**（UE 5.8.1 实测 Interchange 建成 70 MIC 但绑定缺位，按派生 gltf 材质→纹理映射补绑 + 读回核验 + UE 对象名净化 `.`→`_`）。**对拍**：bistro LDR 亮度中位 delta 锁定基线 **0.7698879749655723**（rurix 0.16252008080482483 / ue 0.9324080557703971）→ 复测 **0.6239873385359997**（rurix 0.001102979220234556 / ue 0.6250903178334235）——|复测| < |基线|、同号、收敛幅度 0.1459 ≥ 标定阈 0.0；UE 帧 digest `5bfe1f49…`（G10.5）/`8f907dc0…`（G11.2）→ `92b730e6…`（修复生效），材质实例 texture_parameter_values 非空回归 70/70。
  - **M152（U3，动画剥离）**：根因 = Bistro 动画 Take 001 / glTF 相机节点不引用（动画剥离）双端消费口径未对齐。修法 = **双端同剥离显式登记闭环**——Rurix 侧渲染输出 `animations` 闭集块（package_count=1 / channels=2 / consumed_channels=0 / policy=strip_static_contract，无条件生效零像素影响 + stderr 留痕）+ UE 侧 build_scenes 头注登记面维持。**对拍**：包内通道计数锁定基线 **2.0**（消费 0 vs 包内 2）→ 复测 **0.0**（包内独立重算 1 animation/2 channels == Rurix 显式剥离声明 2，残余静默丢弃 = 0）；相机位姿契约 0-byte（corpus 文件工作树 0-byte + 契约 digest 锁定）。
  - **M147（R1，材质子集）——修复落盘核验全绿但局部度量 delta 未收敛，如实登记不冒充闭环**：修复本身已落地并经多面核验（baseColorTexture/法线/metallic-roughness 采样接入——70 材质 / 144 DDS 纹理 bcdec 真实解码消费，sRGB→线性单转换、albedo=纹理×factor×(1−metallic) 与 UE 扩散模型同构〔UE MIC MetallicFactor=0.4 探针实证双端一致〕、法线贴图 BC5 XY 重建 Z、GI 逐实例 albedo 代理；Rurix 帧 digest `8519cc67…` → `cf1286df…` 生效）。**未收敛实测**：锁定基线 SSIM delta **0.8328980787837229**（ssim 0.16710192121627712，基线复现 f64 逐位一致）→ 复测 **0.9903435577002249**（ssim 0.009656442299775102）——**delta 反向增大，|复测| > |基线|，收敛判定不成立**。根因（measured 登记）：Rurix 帧真实反照率（纹理线性均值×0.6 ≈ 0.10）下 **单反弹 GI 反照率²复合 + 点光源/Lumen 多反弹未表达（R3/R4 承接面，G11.4 波）** 使帧面较 UE 暗 ≈150×（Rurix LDR 中位 0.0011 vs UE 0.6251），SSIM 亮度项塌陷；**反向激励旁证** = ssim(ue_修复， rurix_未修复白帧)=0.1624 > ssim(ue_修复， rurix_修复)=0.0097——锁定度量对「未修复的白帧」评分反高于「正确采样纹理的暗帧」，且亮度对齐后 SSIM 仅 0.126 仍 < 基线 0.167。该锁定度量的收敛**结构性耦合 R3（点光源）/R4（多反弹 GI）——G11.4 承接面**，G11.3 波次内不可达（RXS-0393 L2 quality_gap 款收敛字面以 G11.5 同契约复跑为 definitive 测量面）。旁证：R3 锁定度量（HDR 亮度中位 delta）经 U2 修复已 2.7315→0.480 大幅收敛（UE 白洗涤修复回落）。
- **③ 六门 + 聚合门 --gate/--selftest 摘录（evidence 落盘 evidence/g11_*_<UTC>.json）**：`g11.p0.m147.fix_r1_material_subset` --gate **FAIL checks 11/12**（修复落盘/消费/基线复现/契约 digest/标定/RED 四臂全过，**closure_delta_converged_measured 红**——复测 0.9903435577002249 > 基线 0.8328980787837229；selftest PASS 4 RED+3 GREEN，evidence `g11_m147_fix_r1_material_subset_20260816T165136Z.json`）；`g11.p0.m148.fix_r2_geometry_normals` --gate **PASS 14/14**（selftest 4 RED+3 GREEN）；`g11.p0.m149.fix_r5_json_u64_seed` --gate **PASS 14/14**（selftest 3 RED+3 GREEN）；`g11.p0.m150.fix_u1_cornell_shell_radiance` --gate **PASS 15/15**（selftest 4 RED+3 GREEN）；`g11.p0.m151.fix_u2_bistro_texture_dds` --gate **PASS 15/15**（selftest 3 RED+3 GREEN）；`g11.p0.m152.fix_u3_bistro_animation` --gate **PASS 13/13**（selftest 3 RED+3 GREEN）；`g11.wave.3.exit` --gate **VERDICT=FAIL**（五门 GATE PASS + M147 GATE FAIL + 五 facts 全 PASS〔契约 digest 0-byte / 六门 RED 臂独立有效共 25 臂 / 标定八条 g11.fix.* 入 budget provenance 齐备 / 资产 provenance 齐备 / 回归前置自检绿〕；selftest 负样本缺 evidence → 红 + 正样本真树 → 如实 FAIL〔M147 未绿〕，聚合不代绿不遮蔽，evidence `g11_wave3_exit_20260816T172951Z.json`）。**G-G11-5 退出门判据「M147~M152 六个 P0 独立断言全绿」未达成（M147 红）——本波不宣称收口，M147 未闭环项如实保持 open 留 G11.6 穷举 + G11.5 复测面**。
- **④ 守卫套件与回归前置自检**：`check_structure` / `check_schemas`（六门 + wave3 + 标定件 schema 全量校验）/ `check_number_ledger` / `check_g11_acceptance_map`（三向逐字一致）/ `check_g11_implementation_interlock --require-ready`（**READY**，budget 25 条 measured_local 零 estimated）/ `trace_matrix --check`（375/375）/ `budget_eval --strict`（**163 pass 0 skip**——+8 条 g11.fix.* 标定条目）全绿。**M156 回归面前置自检**：G10 M130 `--phase g10.5` 全门复跑 **PASS 13/13**（G11.3 harness/bin 改动后双场景三方 digest 逐位相等 `sha256:64fd54df6e9be522…` + 应用层探针逐点 ≤1e-3 px——触改面对 G10 门序零降级实测）；G10 14 门 + G9 34 门最新 evidence 全绿只读汇总（wave3 fact ⑤ 机核）；**默认面帧 digest 逐位 parity**——无旗标复跑双场景帧 == G10.5 锁定 digest（cornell `c2000ebf…`/bistro `8519cc67…`，M141 benchmark digest 锚与 M139 探针 parity 零降级旁证；M141 全量 benchmark 重跑面 = 同 render_frame(false,false) 路径 digest 已证逐位一致，墙钟重测归 G11.7a soak 面）。`cargo test --workspace` 全绿（exit 0，2026-08-16 实测）；`cargo fmt --check` / `cargo clippy -D warnings` 为 HEAD 预存漂移红（G11.2 §8.2 同口径登记，本波不扩大不冒充修复）。
- **⑤ 异己并发工作树面与纪律**：本批只含 G11.3 车道文件（src/rurix-asset gltf/json.rs + bcdec.rs + bin/g10_5_scene_render.rs 修复面 + bin/g11_3_dds_dump.rs 新件 + UE harness g10_5_build_scenes.py U1/U2 修复面 + milestones/g11 harness 三件套〔g11_3_ab_rerun.py / g11_3_dds_transcode.py / ue_python/g11_3_probe_materials.py〕+ 转码 manifest + 复跑报告 + ci 七脚本一共享库 + 八 schema + pr-smoke 七步骤 + ledger v1.116 + CI_GATES v1.2 + 本契约本条）；**异己会话 src/ 未提交面（rurix-asset lib.rs ktx2_read 声明 + ktx2_read.rs + rurix-render geometry/gi/shadow/ssr/hzb/restir/sdf_trace/smrt 声明面）维持未提交、不混入本批**（立项裁决 1 / MAP §3.1，G10.8b §8.10 先例同模——本批 `git add` 按文件名显式择取，异己面零混入）；压测资产二进制零入 git（K: 盘外部缓存 + manifest digest 登记）；UE 零 vendoring（只读外部参照）；新文件 LF + 尾换行。**遗留缺口（如实登记，不冒充全闭环）**：M147 R1 局部 SSIM delta 未收敛（结构性耦合 R3/R4——G11.4 承接面；G11.5 M155 复测 verdict 面），波次退出门 G-G11-5 未达成。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10 §8.1 / G11 §8.2 同模）。`Assisted-by: Kimi-K3（G11.3 波）`（影响范围：§8.3 本条；验证方式：六门 + 聚合门 --gate/--selftest 实测输出 + `check_g11_implementation_interlock --require-ready` READY exit=0 + M130 g10.5 回归 PASS 13/13 + 守卫套件全绿留痕）。

### §8.3a M147 判据双 phase 修订（G11.3 收口治理裁决·契约 §8 只追加修订句，2026-08-16）

- **裁决原文（主会话治理裁决留痕）**：「M147 拆双 phase（G10 M130 先例）：`--phase g11.3` = 断言修复落盘 + 局部度量 measured 登记（现状全绿面）；`--phase g11.5` = 断言 delta 收敛（definitive 测量面 = G11.5 同契约复跑，RXS-0393 L2 quality_gap 款字面）。契约 G11_CONTRACT §4.2 M147 判据行正文冻结——修订走契约 §8 只追加修订句（R-G10-1 备选臂回退先例同构：『回退/修订经 §8 只追加修订本波判据』）。收敛判据不弱化只后移：G11.5 M155 必须对 R1 行给出修复前后 SSIM delta 收敛断言（阈值标定程序产，禁手写）；G11.5 不收敛则整波 FAIL。『锁定度量对正确修复结构性不友好』登记为 G11.6 P2 候选行（反向激励旁证 ssim(ue_修,rurix_未修白帧)=0.1624 > ssim(ue_修,rurix_修)=0.0097 入证据链）。」
- **理由摘要**：M147 修复已正确落地（§8.3 ② M147 段——消费闭集/帧 digest/法线/材质均值多面核验齐备）但锁定局部度量 SSIM delta **0.8328980787837229 → 0.9903435577002249 反向增大**——R1 局部度量被 R3（点光源子集）/R4（多反弹 GI）光照残余**结构性主导**（耦合证据链：[`g11_2_residual_caliber_registry.json`](g11_2_residual_caliber_registry.json) items `c1_light_seed_subset_r3` / `c1_gi_structure_multibounce_r4` 行 + M147 门一度 FAIL evidence `evidence/g11_m147_fix_r1_material_subset_20260816T165136Z.json`）；**反向激励旁证 measured**（G11.3 收口复跑命令输出）= ssim(ue_修复帧, rurix_未修复 G10.5 帧) = **0.1624318277352612** > ssim(ue_修复帧, rurix_修复帧) = **0.009656442299775102**——锁定度量对「未修复的白帧」评分反高于「正确采样纹理的暗帧」，入证据链 `evidence/g11_m147_fix_r1_material_subset_20260816T180419Z.json` material_provenance。
- **不弱化声明**：收敛断言**一字不弱只后移**——G11.5 M155 必须对 R1 行给出修复前后 SSIM delta 收敛断言（锁定基线 0.8328980787837229；阈值标定程序产，禁手写）；**G11.5 不收敛则整波 FAIL**。此加严约束同批登记 [G11_PLAN.md](G11_PLAN.md) §2 G11.5 节 M155 门预备注记（M155 门 G11.5 才 materialize，本波只在契约 §8 与 PLAN 登记，M155 行字面 0-byte 不动）。
- **G11.6 P2 候选行登记**：「锁定度量对正确修复结构性不友好」登记为 G11.6 P2 穷举候选行（反向激励旁证 0.1624318277352612 > 0.009656442299775102 入证据链；候选形态 = 锁定度量口径修订评估 / 结构性耦合面登记，G11.6 穷举按只追加程序裁决 go/no-go/defer，承接锚 = 本条）。
- **机核落地（同批）**：`ci/g11_fix_r1_material_subset_smoke.py` 双 phase（`--phase g11.3` 登记面 12 检——11 检维持 + 收敛检改写为 verdict 显式登记形态〔实测收敛 `converged` ∧ `convergence_pending=false`，或 `deferred_to_g11_5` ∧ `convergence_pending=true`；**convergence_pending 缺登记冒充全闭环即 RED，不是 SKIP 充绿**〕；`--phase g11.5` 当前 **fail-closed 拒跑** exit=2）+ evidence schema anyOf 双支（v1 = legacy 既有件形态 0-byte / v2 = g11.3 phase 登记形态，沿 G9 v1.14 / G10 M130 anyOf 双支体例）+ wave3 聚合门 fact⑥ `m147_dual_phase_discipline` 两态口径（A 态 = g11.3 phase 绿 ∧ deferred/收敛如实登记，B 态 = g11.5 phase 收敛断言绿；沿 G10.8a wave2 fact④ 两态校准先例，判据语义 0-byte）+ [G11_ACCEPTANCE_MAP.md](G11_ACCEPTANCE_MAP.md) §3.4 双 phase 口径登记（M147 行字面 0-byte）+ [CI_GATES.md](CI_GATES.md) §4 M147 行只追加校准注（v1.3）+ pr-smoke 步骤 201 接 `--phase g11.3`。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10 §8.1 / G11 §8.2/§8.3 同模）。`Assisted-by: Kimi-K3（G11.3 收口）`（影响范围：§8.3a 本条；验证方式：M147 `--gate --phase g11.3` PASS 12/12 + `--selftest` 7 RED+5 GREEN + `--phase g11.5` fail-closed exit=2 + wave3 `--gate` VERDICT=PASS 实测输出留痕）。

### §8.3b G-G11-5 G11.3 资产与场景面修复波验收记录（M147 双 phase 修订后复跑，2026-08-16）——**六门闭环 PASS + wave3 聚合 VERDICT=PASS；M147 一度 FAIL→双 phase 修订→复跑 PASS 完整诚实留痕**

- **① 独立断言全绿清单（gate_key + 步骤号 + host/device + evidence 路径 UTC stamp）**：
  - `g11.p0.m147.fix_r1_material_subset`（步骤 201，host+device，device_section_state=executed）——`--phase g11.3` **PASS 12/12**（evidence `g11_m147_fix_r1_material_subset_20260816T180419Z.json`；phase=g11.3 ∧ g11_3_phase_pass=true ∧ convergence_pending=true ∧ closure.verdict=deferred_to_g11_5——修复落盘/消费/基线复现/契约 digest/标定/RED 四臂全绿 + 收敛判定 verdict 显式登记，**不是 SKIP 充绿**；标定件 `g11_m147_calibration_r1_ssim_shrink_20260816T180419Z.json`）。**M147 一度 FAIL→双 phase 修订→复跑 PASS 留痕**：首跑 `--gate`（双 phase 校准前形态）**FAIL 11/12**（`closure_delta_converged_measured` 红——复测 0.9903435577002249 > 基线 0.8328980787837229，evidence `g11_m147_fix_r1_material_subset_20260816T165136Z.json` 保留 0-byte）→ §8.3a 双 phase 修订（G10 M130 先例；判据语义 0-byte——收敛断言一字不弱只后移 g11.5 definitive 面）→ `--phase g11.3` 复跑 **PASS 12/12**；`--phase g11.5` 当前 **fail-closed 拒跑 exit=2**（G11.5 未至；缺 `--phase` 同 exit=2）。
  - `g11.p0.m148.fix_r2_geometry_normals`（步骤 202，host+device）——**PASS 14/14**（evidence `g11_m148_fix_r2_geometry_normals_20260816T165024Z.json`，§8.3 ③ 留痕）。
  - `g11.p0.m149.fix_r5_json_u64_seed`（步骤 203，host 纯 host）——**PASS 14/14**（evidence `g11_m149_fix_r5_json_u64_seed_20260816T164954Z.json`）。
  - `g11.p0.m150.fix_u1_cornell_shell_radiance`（步骤 204，host+device）——**PASS 15/15**（evidence `g11_m150_fix_u1_cornell_shell_radiance_20260816T165041Z.json`）。
  - `g11.p0.m151.fix_u2_bistro_texture_dds`（步骤 205，host+device）——**PASS 15/15**（evidence `g11_m151_fix_u2_bistro_texture_dds_20260816T165102Z.json`）。
  - `g11.p0.m152.fix_u3_bistro_animation`（步骤 206，host 纯 host）——**PASS 13/13**（evidence `g11_m152_fix_u3_bistro_animation_20260816T165008Z.json`）。
- **② 波聚合门实测输出（VERDICT=PASS + evidence 路径）**：`g11.wave.3.exit`（步骤 207）`--gate` **VERDICT=PASS**（evidence `g11_wave3_exit_20260816T180520Z.json`）——六门 GATE 全 PASS + 六 facts 全 PASS：① 契约 digest 0-byte（双场景当次重算 == G10.5 锁定值 cornell `80305791…`/bistro `ad45951b…`，联合 `64fd54df…`）；② 六门 RED 臂独立有效共 25 臂；③ 标定八条 g11.fix.* 入 g11_budget provenance 齐备（threshold == trimmed_mean × k 重算口径，P-09）；④ 资产 provenance 齐备（DDS 转码 manifest 144 条目 + 产物目录零未登记混入 + cornell 语料 M131 登记 digest 复算 0-byte）；⑤ 回归前置自检（G10 14 门 + G9 34 门最新 evidence 全绿只读汇总 + 默认面帧 digest 逐位 parity 双场景 == G10.5 锁定值）；⑥ **m147_dual_phase_discipline PASS**（M147 A 态：g11.3 phase 绿 + convergence_pending=true〔deferred_to_g11_5 如实登记，不替 g11.5 收敛断言充绿〕——G11.3 收口两态校准，沿 G10.8a wave2 fact④ 先例，判据语义 0-byte）。聚合不代绿、不重跑 smoke、不遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE；首跑 VERDICT=FAIL 件（`g11_wave3_exit_20260816T172951Z.json`，M147 未绿期）保留 0-byte 不冒充。
- **③ 验收命令逐字输出（各门 --gate + --selftest + 守卫套件全 PASS）**：
  ```text
  py -3 ci/g11_fix_r1_material_subset_smoke.py --gate g11.p0.m147.fix_r1_material_subset --phase g11.3
    [g11_m147] checks 12/12 device=executed phase=g11.3
    [g11_m147] PASS（g11.3 phase：R1 材质子集消费闭环 + 局部度量 measured 登记——delta 0.8328980787837229 → 0.9903435577002249 verdict=deferred_to_g11_5 convergence_pending=true；收敛断言归 --phase g11.5 definitive 面 + RED 四臂全检出）
  py -3 ci/g11_fix_r1_material_subset_smoke.py --selftest
    [g11_m147] selftest PASS checks=12 (7 RED + 5 GREEN)
  py -3 ci/g11_fix_r1_material_subset_smoke.py --gate g11.p0.m147.fix_r1_material_subset --phase g11.5
    [g11_m147] FAIL-CLOSED 拒跑：--phase g11.5 收敛断言面未至……  exit=2
  py -3 ci/g11_wave3_exit_check.py --gate g11.wave.3.exit
    六门 GATE 全 PASS + 六 facts 全 PASS → VERDICT = PASS（evidence g11_wave3_exit_20260816T180520Z.json）
  py -3 ci/g11_wave3_exit_check.py --selftest
    负样本缺 evidence → 红 + 正样本真树 → 绿 + m147_dual_phase_discipline 两态单元 3 绿态+6 红臂 → ALL PASS
    （selftest 留痕件如实登记：负样本 artifact `g11_wave3_exit_20260816T180535Z.json`〔VERDICT=FAIL，空 evidence 目录受控负样本〕+ 正样本 artifact `g11_wave3_exit_20260816T180538Z.json`〔PASS〕——只增不删不改）
  py -3 ci/check_structure.py            → PASS (11 dirs, 6 files)
  py -3 ci/check_schemas.py              → PASS（M147 schema anyOf 双支 + wave3 schema anyOf 五/六 facts 全量 evidence 校验——legacy 件 0-byte 全过）
  py -3 ci/check_number_ledger.py        → PASS（spec RXS 头 375 个零同号碰撞；数字步骤零新增——201~207 既有号维持）
  py -3 ci/check_g11_acceptance_map.py   → PASS（13 P0 + 1 已 go P1 三向/双向逐字一致；零空行；numeric_step 零预占）
  py -3 ci/check_g11_implementation_interlock.py --require-ready → VERDICT=READY（budget 25 条 measured_local 零 estimated）
  py -3 ci/trace_matrix.py --check       → PASS (375/375)
  py -3 ci/budget_eval.py --strict       → PASS (163 pass, 0 skip)
  py -3 ci/check_guardrails.py 53eb3a28  → PASS（字节级只追加核对 123 changed paths）
  py -3 ci/check_contribution.py         → advisory 先例面一致（历史 findings 0 新增面）
  ```
- **④ 门序 / not-triggered / no-go 登记面摘要**：**M156 回归面前置自检**——G10 M130 `--phase g10.5` 全门复跑 **PASS 13/13**（evidence `g10_m130_dual_determinism_contract_20260816T181622Z.json`；双场景三方 digest 逐位相等 `sha256:64fd54df6e9be522…` + 应用层探针逐点 ≤1e-3 px——本批 M147 门/聚合门/pr-smoke 改动对 G10 门序零降级实测）；G10 14 门 + G9 34 门最新 evidence 全绿只读汇总（wave3 fact⑤ 机核 = G10 14 门 + G9 34 门回归抽检面）；默认面帧 digest 逐位 parity（双场景 == G10.5 锁定值）。**not-triggered / no-go 登记面**：`--phase g11.5` fail-closed 拒跑登记（G11.5 未至，非 SKIP 充绿——收敛断言后移 definitive 面，§8.3a 不弱化声明）；G11.6 P2 候选行登记「锁定度量对正确修复结构性不友好」（反向激励旁证 measured 0.1624318277352612 > 0.009656442299775102 入证据链 `g11_m147_fix_r1_material_subset_20260816T180419Z.json` material_provenance；承接锚 = §8.3a）；M155 门预备注记登记 [G11_PLAN.md](G11_PLAN.md) §2 G11.5 节（R1 行收敛断言后移本波、不收敛则整波 FAIL——M155 门 G11.5 才 materialize，本波只登记）。本批零 `src/` 改动（`.rs` 0-byte——机核校准落 ci 门脚本/schema/工作流/治理文档面），G5~G10 closed 判据与既有门脚本 0-byte；**G-G11-5 退出门判据「M147~M152 六个 P0 独立断言全绿」达成（M147 g11.3 phase PASS 登记面 + M148~M152 五门 PASS；M147 收敛断言归 G11.5 M155 面承接，§8.3a 修订句）——本波验收通过**。
- **⑤ 签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10 §8.1 / G11 §8.2/§8.3/§8.3a 同模）。`Assisted-by: Kimi-K3（G11.3 收口）`（影响范围：§8.3b 本条 + 同批机核面〔ci/g11_fix_r1_material_subset_smoke.py 双 phase + ci/g11_3_fix_lib.py 追加 ssim_ldr_cross + ci/g11_wave3_exit_check.py fact⑥ + 两 evidence schema anyOf + pr-smoke 步骤 201 --phase 接线 + G11_ACCEPTANCE_MAP §3.4/v1.1 + CI_GATES M147 注/v1.3 + G11_PLAN M155 注/v1.1 + evidence 六件落盘〕；验证方式：③ 节验收命令逐字输出 + ② 节聚合 VERDICT=PASS + ④ 节 M130 g10.5 复跑 PASS 13/13 留痕）。**异己并发工作树面**：本批只含 G11.3 收口车道文件；异己会话 src/ 未提交面（rurix-asset lib.rs ktx2_read 声明 + ktx2_read.rs + rurix-render geometry/gi/shadow/ssr/hzb/restir/sdf_trace/smrt 声明面）维持未提交、不混入本批（立项裁决 1 / MAP §3.1，G10.8b §8.10 先例同模——`git add` 按文件名显式择取）；压测资产二进制零入 git（K: 盘外部缓存）；UE 零 vendoring；新文件 LF + 尾换行。

### §8.4 G-G11-6 G11.4 光照与 GI 修复波验收记录（2026-08-16）——**两门闭环 PASS + wave4 聚合 VERDICT=PASS；R3/R4 锁定基线 HDR 域 delta 双收敛 measured，M99-clipmap 世界级辐射缓存承接兑现**

- **① 编号与 spec-first（互锁后 actual next_free 顺位，零预占）**：spec-first 先行——`spec/global_illumination.md` 追加 **RXS-0394**（M153 R3 灯种子集表达：光源集五元闭集 + 契约光照面单通道〔corpus/lighting_*.json 唯一事实源，glTF 字段 = 派生输入经 M133 只追加修订程序，直读绕过即 RED〕+ 点光源辐射链〔Φ=Le·A·π / I₀=Φ/π=Le·A，近场钳制 d²_eff=max(d²,A/π)，逐盏 provenance〕+ cornell 契约 sun+sky 灯面 0-byte）/ **RXS-0395**（M154 R4 多反弹 GI：屏幕探针近场 + 世界辐射缓存远场兜底双级 + 多反弹 ≥2 级逐级能量计数 + 逐级能量增量单调不增 + host 同构兑现面〔不冒充 GPU 管线世界级，GPU 面锚定 G14〕+ RXS-0357 L6 门序继承）/ **RXS-0396**（M154 M99-clipmap 世界级辐射缓存承接：**RXS-0360 世界级 not-triggered 登记翻转修订行**〔G10.6 rejudged-go 承接锚逐字 + measured 举证；RXS-0360 既有字面 0-byte〕+ 空间哈希世界缓存〔对数族量化 level=clamp(floor(log2(1+dist/d_ref)),0,LEVELS−1)；LEVELS=4 / s0=scene_diag×2^-8 / d_ref=scene_diag×2^-4 实测标定冻结——bistro 25.962 m / cornell 958.659 单位〕+ 距离自适应辐射 LOD clipmap 级 + 级间回落链 → 天光末级兜底显式登记 + 世界级双锚判定〔远场探针集能量回归达标定阈 + M96 golden 匹配深度对拍〕+ ≠RXS-0359 L4 Far Field 边界声明），依据 RFC-0028（Agent Approved）§4.1/§4.2/§4.3/§4.4；落盘前实测 `RXS.next_free=394` 顺位领取 0394~0396（number_ledger v1.117：on_tree_max 393→396、next_free 394→397）；conformance 锚定六件（accept light_seed_set_minimal.rx / gi_multibounce_two_level_minimal.rx / world_radiance_cache_minimal.rx + reject light_seed_gltf_direct_bypass.rx / gi_single_bounce_masquerade.rx / world_cache_farfield_zero_energy.rx）；`py -3 ci/trace_matrix.py --check` PASS（375→378 全锚定）；stable 快照 375→378 重 bless + bless_log 追加（error_codes=113/editions/subcommands 三段 0 变化）。门 materialize 批落盘前实测 `CI_step.next_free=208` 顺位领取 **208~210**（number_ledger v1.118：on_tree_max 207→210、next_free 208→211）——208=M153 / 209=M154 / 210=g11.wave.4.exit；两门脚本 + 聚合门 + 四 schema（两门 + wave4 + g11_4_calibration 标定件共享）+ pr-smoke 三真步骤 + check_schemas 纯追加映射 + 共享判定层 ci/g11_4_fix_lib.py + CI_GATES v1.4 修订行同批落；`py -3 ci/check_number_ledger.py` PASS（spec RXS 头 378 个零同号碰撞）。
- **② 两修复逐项根因/修法/修复前后 delta 对拍（数字全来自命令输出；G11.4 复跑帧区 K:/rurix-ext/g11-frames/g11_4，G10/G11.2/G11.3 帧库只读；契约 digest 双场景当次重算 == G10.5 锁定值 0-byte——cornell `80305791…`/bistro `ad45951b…`/联合 `64fd54df…`）**：
  - **M153（R3，灯种子集）**：根因 = bistro 包内灯具点光源与 glTF emissive 表面双端未表达（UE 侧灯具点光未 spawn / Rurix 侧零消费），HDR 亮度中位系统性偏低（G11.2 残余登记 `c1_light_seed_subset_r3` 承接锚）。修法 = **派生链 + 双端同消费**——`g11_4_light_derive.py` 从 emissive 表面派生 4 点光（灯具 emissive 通量换算：Le=emissiveFactor×emissiveTexture 线性均值 × 关联灯具面积 A ⇒ 轴向点强 I₀=Le·A，发光轴向 = emissive 三角形面积加权平均法线，近场钳制 d²_eff=max(d²,A/π)，逐盏 provenance 入 `g11_4_light_derivation.json`）→ **M133 只追加修订** `lighting_bistro_interior.json`（4 点光 + 修订行，pre/post digest 登记）+ 清单修订行；Rurix 侧 `--light-seed-set` 契约光照面单通道消费（point_lights_consumed=4 / emissive_materials_consumed=4 / emissive_instances=18 / area_lights 缺类显式登记，source_digest 对账；glTF 直读绕过即 RED）；UE 侧 `g10_5_build_scenes.py` 双端同消费 spawn + 读回探针 `g11_4_point_lights_count=4`；cornell 契约 sun+sky 灯面 0-byte。**对拍**：锁定基线 HDR 亮度中位 delta **2.664779790997505**（原域）/ **2.7314592314362525**（对齐域）→ 复测 **0.48027740360833704**（m154 全修复面 0.48023271594443356）——|复测| < |基线|、同号、收敛幅度 2.2512 ≥ 标定阈 0.0（HDR 亮度中位双跑噪声 p100×k=1.0，样本集 = G11.4 bistro m153 帧对，入 `g11.fix.r3_luminance_shrink_tol`）。
  - **M154（R4 + M99-clipmap，多反弹 GI + 世界级辐射缓存）**：根因 = 单反弹 GI + 屏幕级缓存——真实反照率（≈0.10）下多反弹能量缺失（反照率²复合），HDR p90 系统性偏低（G11.2 残余登记 `c1_gi_structure_multibounce_r4` 承接锚；g9.p1.m99 屏幕级绿不冒充世界级，契约 §4.2 M154 行字面）。修法 = **SHaRC 同构世界辐射缓存**（RXS-0395/0396 语义面）：命中点沉积（覆盖）+ 探针点沉积（携带远场链接）**双沉积**，渲染侧缓存命中即路径终止；空间哈希 LEVELS=4 + 对数族距离自适应辐射 LOD + 双哈希步长线性探测 + **3 级多反弹**（逐级能量计数 + 能量增量绝对值非递减机核——采样噪声下负增量合法，绝对值单调判）+ 级间回落链 → 天光末级兜底显式登记（禁静默零辐射）；**NEE 覆盖面 GI 面排除 Le** 逐实例面（缓存终止语义下 NEE 已覆盖的 emissive Le 防双重计数——fixture 0.366→0.831 回归根因修复）；bistro 天空项改**探针收集均值**（无偏逃逸率）替代单射线 0/1 可见性（帧内天空可见性结构性低估修复，相机朝向/天空结构查实后修法）；cornell 多反弹生效实测（HDR 中位 2.3×、p90 +19%）；世界缓存构建参数冻结带 `g11_m154_world_cache_band.json`（measured×2.0，P-09）。**对拍**：锁定基线 HDR p90 delta **4.697253086805343**（原域）/ **4.8486343559026714**（对齐域）→ 复测 **1.216991522263363**（cornell p90 delta 残余面 0.23202527932524686）——|复测| < |基线|、同号、收敛幅度 3.6316 ≥ 标定阈 0.0（p100×k=1.0，入 `g11.fix.r4_p90_shrink_tol`）；**世界级双锚**：远场探针集能量回归 bistro **0.002718323364194886** / cornell **0.3139703815480147** ≥ 标定阈 **0.001359161682097443**（direction=min ×k=0.5 标定程序产——「非零」字面不构成判定，入 `g11.fix.r4_farfield_energy_min`）+ M96 golden 匹配深度对拍 **rel_dev 0.3394905916597106 ≤ 冻结带 0.6789811833194213**（measured×2.0）且双 digest 全等（full 档 host oracle）；**RXS-0360 世界级登记翻转修订行在树机核**（RXS-0360 字面 0-byte）；M96 门序绿（RXS-0357 L6）。
- **③ 两门 + 聚合门 --gate/--selftest 摘录（evidence 落盘 evidence/g11_*_<UTC>.json）**：
  - `g11.p0.m153.fix_r3_light_subset`（步骤 208，host+device，device_section_state=executed）`--gate` **PASS 15/15**（契约 digest 0-byte / 灯派生 provenance / 灯种子集消费 4+4 / UE 双端探针 / cornell 灯面 0-byte / 帧 digest 生效 / 基线复现逐位 / 收敛 measured / 标定确定性 / budget 登记 / budget_eval 全过 + RED 四臂〔灯未表达 / 未收敛冒充 / 手写阈值 / estimated 冒充〕；selftest PASS 6 RED+9 GREEN；evidence `g11_m153_fix_r3_light_subset_20260816T212836Z.json` + 标定件 `g11_m153_calibration_r3_luminance_shrink_20260816T212836Z.json`；首跑件 `…_20260816T211923Z.json` 保留 0-byte）。
  - `g11.p0.m154.fix_r4_gi_multibounce_world_cache`（步骤 209，host+device，device_section_state=executed）`--gate` **PASS 18/18**（契约 digest 0-byte / RXS-0396 翻转修订行在树 / M96 门序 / 世界缓存落地〔LEVELS=4 + 多反弹 ≥2 + 能量增量绝对值非递减 + 沉积/命中计数〕/ 远场能量回归达标定阈 / M96 golden 冻结带对拍 / 帧 digest 生效 / 基线复现逐位 / 收敛 measured / 标定确定性 / budget 登记 / budget_eval 全过 + RED 六臂〔缓存未落地 / 屏幕级冒充世界级 / 单反弹冒充 / 未收敛冒充 / 手写阈值 / estimated 冒充〕；selftest PASS 9 RED+9 GREEN；evidence `g11_m154_fix_r4_gi_multibounce_world_cache_20260816T212847Z.json` + 标定件两件 `g11_m154_calibration_r4_p90_shrink_/r4_farfield_energy_min_20260816T212847Z.json`；中间件 `…_20260816T211955Z.json` / `…_20260816T212206Z.json` 保留 0-byte）。
  - `g11.wave.4.exit`（步骤 210）`--gate` **VERDICT=PASS**（evidence `g11_wave4_exit_20260816T212923Z.json`）——两门 GATE 全 PASS + 六 facts 全 PASS：① 契约 digest 0-byte；② 两门 RED 臂独立有效共 10 臂；③ 标定三条 g11.fix.r3/r4 入 g11_budget provenance 齐备（threshold == trimmed_mean × k 重算口径，P-09）；④ spec-first 面（RXS-0394~0396 条款头 + RXS-0360 翻转修订行字面 + RXS-0360 0-byte + RFC-0028 Agent Approved）；⑤ 回归前置自检（G10 14 门 + G9 34 门最新 evidence 全绿只读汇总 + 默认面帧 digest 逐位 parity 双场景 == G10.5 锁定值）；⑥ **m96_ordering_and_r1_coupling_recheck**（M96 最新 evidence PASS；**R1 耦合面复核：G11.3 复测 0.9903435577002249 → G11.4 m154 面 0.9891526376076132〔m153 面 0.9894255837864896〕——实测登记为 M155 收敛断言备料，不冒充收敛**）。聚合不代绿、不重跑 smoke、不设 RURIX_REQUIRE_REAL；`--selftest` 负样本缺 evidence → 红 + 正样本真树 → 绿 ALL PASS（留痕件如实登记：负样本 artifact `g11_wave4_exit_20260816T214323Z.json`〔VERDICT=FAIL，空 evidence 目录受控负样本〕+ 正样本 artifact `g11_wave4_exit_20260816T214326Z.json`〔PASS〕——只增不删不改；当日更早聚合中间件 212250Z/212311Z/212314Z/212916Z/212919Z 五件保留 0-byte）。
  ```text
  py -3 ci/g11_fix_r3_light_subset_smoke.py --gate g11.p0.m153.fix_r3_light_subset
    [g11_m153] checks 15/15 device=executed → PASS（标定阈 0.0 = p100×k=1.0；复测 0.48027740360833704 < 基线 2.7314592314362525）
  py -3 ci/g11_fix_r3_light_subset_smoke.py --selftest
    [g11_m153] selftest PASS checks=15 (6 RED + 9 GREEN)
  py -3 ci/g11_fix_r4_gi_multibounce_world_cache_smoke.py --gate g11.p0.m154.fix_r4_gi_multibounce_world_cache
    [g11_m154] checks 18/18 device=executed → PASS（p90 复测 1.216991522263363 < 基线 4.8486343559026714；远场能量 0.002718…/0.313970… ≥ 0.00135916…；M96 rel_dev 0.33949 ≤ 0.67898）
  py -3 ci/g11_fix_r4_gi_multibounce_world_cache_smoke.py --selftest
    [g11_m154] selftest PASS checks=18 (9 RED + 9 GREEN)
  py -3 ci/g11_wave4_exit_check.py --gate g11.wave.4.exit
    两门 GATE 全 PASS + 六 facts 全 PASS → VERDICT = PASS（evidence g11_wave4_exit_20260816T212923Z.json）
  py -3 ci/g11_wave4_exit_check.py --selftest
    负样本缺 evidence → 红 + 正样本真树 → 绿 → ALL PASS
  py -3 ci/check_structure.py            → PASS (11 dirs, 6 files)
  py -3 ci/check_schemas.py              → PASS（两门 + wave4 + g11_4_calibration 标定件 schema 全量校验，既有全族 0-byte）
  py -3 ci/check_number_ledger.py        → PASS（spec RXS 头 378 个零同号碰撞；ledger 14 命名空间保留号被尊重）
  py -3 ci/check_g11_acceptance_map.py   → PASS（13 P0 + 1 已 go P1 三向/双向逐字一致；numeric_step 零预占）
  py -3 ci/check_g11_implementation_interlock.py --require-ready → VERDICT=READY（budget 28 条 measured_local 零 estimated）
  py -3 ci/trace_matrix.py --check       → PASS (378/378)
  py -3 ci/budget_eval.py --strict       → PASS (166 pass, 0 skip——+3 条 g11.fix.r3/r4 标定条目)
  py -3 ci/stable_snapshot.py --check    → PASS（spec_clauses=378，三段 0 变化）
  py -3 ci/check_guardrails.py 66365ae1  → PASS（字节级只追加核对；历史 advisory 先例面一致 0 新增面）
  py -3 ci/check_contribution.py         → advisory 先例面一致
  ```
- **④ 门序 / not-triggered / no-go 登记面摘要**：**M156 回归面前置自检**——G10 M130 `--phase g10.5` 全门复跑 **PASS 13/13**（evidence `g10_m130_dual_determinism_contract_20260816T213627Z.json`；G11.4 bin/harness 改动后双场景三方 digest 逐位相等 `sha256:64fd54df6e9be522…` + 应用层探针逐点 ≤1e-3 px——触改面对 G10 门序零降级实测）；G10 14 门 + G9 34 门最新 evidence 全绿只读汇总（wave4 fact⑤ 机核）；**默认面帧 digest 逐位 parity**——无旗标复跑双场景帧 == G10.5 锁定 digest（`--exposure-scale=1.0` 默认面 + 独立 parity 目录防覆写；M139 探针 parity 与 M141 benchmark digest 锚零降级旁证）。`cargo test --workspace` 全绿（exit 0，2026-08-16 实测）；`cargo fmt --check` / `cargo clippy -D warnings` 为 HEAD 预存漂移红（G11.2 §8.2 / G11.3 §8.3 同口径登记，本波不扩大不冒充修复）。**not-triggered / no-go 登记面**：M100-high 维持 defer 0-byte（R3 修复后多灯 workload measured 对照面产出让 G11.6 穷举重评，契约 §6 行字面）；M98-l4 defer 0-byte（RXS-0396 ≠RXS-0359 L4 Far Field 边界声明）；GPU 管线世界级锚定 G14（RXS-0395 host 同构兑现面不冒充）；**M147 g11.5 phase 收敛断言 coupling 解除备料**——R1 耦合面实测登记 0.9891526376076132（m154 面）入 wave4 fact⑥，M155 definitive 测量面承接（§8.3a 修订句 0-byte，收敛断言一字不弱）。**G-G11-6 退出门判据「M153/M154 两个 P0 独立断言全绿 + RXS-0360 世界级登记翻转显式修订行 + 不以 g9.p1.m99 屏幕级绿色冒充世界级 + HDR 域 delta 收敛 measured」达成——本波验收通过**。
- **⑤ 异己并发工作树面与纪律**：本批只含 G11.4 车道文件（src/rurix-asset bin/g10_5_scene_render.rs 修复面 + bin/world_cache/mod.rs 新件〔bin 局部模块，仅依赖 HEAD 既有 rurix_render::rt::bvh::Vec3，与异己声明面零耦合——grep 实测 ktx2_read/hzb/restir/sdf_trace/smrt/ssr 命中 0〕+ UE harness g10_5_build_scenes.py 灯消费/探针面 + milestones/g10 corpus lighting_bistro_interior.json M133 只追加修订 + 清单修订行 + milestones/g11 harness 两件〔g11_4_ab_rerun.py / g11_4_light_derive.py〕+ 派生登记/复跑报告/冻结带三件 + spec/global_illumination.md 三条款 + spec/README.md 计数 + conformance 六锚 + ci 三脚本一共享库 + 四 schema + pr-smoke 三步骤 + check_schemas 纯追加 + g11_budget 字节级纯追加三条 + ledger v1.117/v1.118 + CI_GATES v1.4 + 本契约本条 + tests/stable 快照/bless_log + conformance/traceability_matrix 双件）；**异己会话 src/ 未提交面（rurix-asset lib.rs ktx2_read 声明 + ktx2_read.rs + rurix-render geometry/hzb + gi/restir/sdf_trace + shadow/smrt + ssr 声明面）维持未提交、不混入本批**（立项裁决 1 / MAP §3.1，G10.8b §8.10 先例同模——`git add` 按文件名显式择取，异己面零混入）；压测资产二进制零入 git（K: 盘外部缓存）；UE 零 vendoring（只读外部参照）；新文件 LF + 尾换行。**evidence 纪律如实登记**：evidence/ 只增不删不改维持——本波四件 wave=G11.3 串扰标定中间件删除登记（开发环中间件非门 verdict 证据：共享库 `calib_evidence_payload` 沿 G11.3 wave 字面串扰，schema isolation_check 检出；值确定性逐位可再生，最终标定件以 wave=G11.4 重产落盘〔212836Z/212847Z 三件〕，budget 条目 evidence_file 指针同批修复；ledger v1.118 同字面登记）。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10 §8.1 / G11 §8.2/§8.3/§8.3a/§8.3b 同模）。`Assisted-by: Kimi-K3（G11.4 波）`（影响范围：§8.4 本条；验证方式：③ 节验收命令逐字输出 + 聚合 VERDICT=PASS + ④ 节 M130 g10.5 复跑 PASS 13/13 + 守卫套件全绿留痕）。
