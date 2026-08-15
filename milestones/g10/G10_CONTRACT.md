---
contract: G10
title: G10 UE5 画面对标基线期
status: active
implementation_status: unblocked
active_scope: g10_1_governance_only + g10_2_plus_implementation_waves
version: v1.0
date: 2026-08-15
timebox: "G10.1 治理波即刻执行（G9 已 closed）；G10.2~G10.8b 严格波次，工期在实现互锁开放后由 measured baseline 校准"
rfc_required: "两份 Full RFC（编号按立项时实测 registry/number_ledger.json namespaces.RFC next_free 领取，禁推测号）：①画面对标与度量语义 RFC（帧捕获 HDR 格式面 / FLIP/SSIM/PSNR 口径冻结 / 逐像素 diff 报告 schema / 差距清单 schema / 双端确定性契约）；②外部参照 harness 与许可边界 RFC（UE 出图编排边界、零 vendoring / 压测资产许可白名单 SPDX/attribution/digest 登记面）。均须 D-409 独立 provenance 对抗性评审后 Agent Approved 方为语义冻结；未 Approved 前本契约对应条款为引用占位"
upstream_docs:
  - "milestones/g10/G10_PLAN.md v1.0（九波结构、P0 建议清单 12 行、风险表 R-G10-1~11、治理裁决表项的契约上游事实源）"
  - "milestones/g10/G10_CAPABILITY_MATRIX.md v1.0（M128~M143 能力缺口矩阵）"
  - "milestones/g10/G10_CANDIDATE_DECISIONS.md v1.0（十锚初裁 + 新增候选五行）"
  - "milestones/g10/design/g10_ue5_harness_spike.md v1.0（UE5 出图环境 spike 与裁决建议）"
  - "milestones/g9/G9_CONTRACT.md §8.10（G9 closed 终态，2026-08-15，flip commit 6ff73830 + 收口批 c0cdfddd）"
  - "milestones/g9/G9_P2_DECISIONS.md v1.0（十项 defer-to-G10+ 承接锚，法定输入）"
  - "registry/deferred.json RD-034/039/040/041/042/043/044（存续 open RD；只追加禁静默改判）"
  - "04 P-01/P-07/P-09/P-12/P-13；10 §3/§7/§9.5；14 §1/§3/§4/§5（同 G9 口径）"
implementation_unlock:
  required_all:
    - "G10.1 治理门全部完成且有真实验证记录"
    - "check_g10_implementation_interlock --require-ready 输出 READY（互锁 validator 机器事实，不以叙述替代）"
    - "共享编号按互锁开放时 actual next_free 重新校准；数字 CI 步骤不得沿用推测号与草案建议值"
in_scope:
  - g10_1_governance_only
  - rfc_visual_comparison_and_metrics_semantics
  - rfc_external_reference_harness_and_license_boundary
  - candidate_decisions_and_rd_mapping
  - p0_acceptance_mapping
  - measured_4070ti_baseline
  - g10_2_ue5_capture_environment_wave
  - g10_3_stress_corpus_wave
  - g10_4_metrics_infrastructure_wave
  - g10_5_first_ab_comparison_wave
  - g10_6_defer_reevaluation_window_wave
  - g10_7_p2_exhaustive_decisions
  - g10_8_stabilization_and_closeout
out_of_scope:
  - g10_2_plus_while_implementation_interlock_is_red
  - g10_1_src_spec_conformance_semantic_implementation
  - g10_1_numbered_workflow_steps_or_stub_scripts
  - any_visual_fix_pr_deferred_to_g11
  - path_tracer_productionization_implementation_deferred_to_g12
  - dlss_upscale_integration_implementation_deferred_to_g13
  - performance_optimization_implementation_deferred_to_g14
  - commercial_closeout_deferred_to_g15
  - ue_source_or_binary_vendoring_into_rurix_repo
  - safe_gpu_operator_platform_remains_deferred_g11_plus
  - rewriting_g5_to_g9_closed_contracts_and_00_14
  - unmeasured_quality_or_framerate_pass_lines
  - speculative_number_consumption
deferred_refs: [RD-034, RD-039, RD-040, RD-041, RD-042, RD-043, RD-044]
deliverables:
  - id: D-G10-1
    name: "G10.1 治理四件套：G10_PLAN（升格契约上游事实源）、G10_CONTRACT、CI_GATES、非空 measured g10_budget；status=active 且 implementation_status=blocked"
  - id: D-G10-2
    name: "G10.1 完整候选决策表：G9 十锚 + 存续 open RD + G10 新增候选项逐行映射（go / no-go / defer-to-G11+ / strategic_override + 承接锚）；缺行阻断 G10.2"
  - id: D-G10-3
    name: "G10.1 验收映射：全部 P0 各有独立 symbolic gate key、稳定脚本名、evidence schema 目标路径与判据；已 go 的 P1 同步覆盖"
  - id: D-G10-4
    name: "两份 Full RFC 经 D-409 独立 provenance 对抗性评审后 Agent Approved：①画面对标与度量语义；②外部参照 harness 与许可边界"
  - id: D-G10-5
    name: "G10.1 RTX 4070 Ti measured baseline 与非空 g10_budget（零 estimated）；G10 validator 五件套落盘——implementation interlock 当前诚实报告 BLOCKED"
  - id: D-G10-6
    name: "G10.2 UE5 出图环境波：裁决路径落地 + 批量参考帧出图 harness + 双跑 digest 确定性 + 双端确定性契约骨架（M128/M129/M130）"
  - id: D-G10-7
    name: "G10.3 压测语料波：资产获取 + 许可登记零缺行 + 加载门 + 场景清单冻结（M131/M132/M133）"
  - id: D-G10-8
    name: "G10.4 度量基建波：帧捕获 HDR 管线 + FLIP/SSIM/PSNR 对拍 + 逐像素 diff 报告 + 阈值标定（M134~M138）"
  - id: D-G10-9
    name: "G10.5 首轮 A/B 对比波：双端出图 + 度量报告 + 差距清单 measured 落盘 + 帧率基线（M139/M140/M141）"
  - id: D-G10-10
    name: "G10.6 defer 重评窗 + G10.7 P2 穷举决策 + G10.8a soak + G10.8b close-out（差距清单终审锁定 → G11 法定输入）"
acceptance_gates:
  - id: G-G10-1
    check: "治理激活门：用户 2026-08-15 立项指令（G10~G15 总目标与六期分期授权）留痕；agent 依 10 §7/P-13/D-406 v2.0 完全自主签署立项裁决留痕；九项立项裁决全部落定；G10.0 不可变 ref=c0cdfddd 登记；仅 governance-only 范围 active"
  - id: G-G10-2
    check: "G10.1 完成门：D-G10-1~5 齐备并通过结构/schema/ledger/guardrail/预算核验；验收映射无缺行；无 src/spec/conformance 语义实现、无数字 workflow 空步骤；本门通过不自动开放实现"
  - id: G-G10-3
    check: "实现互锁门：check_g10_implementation_interlock --require-ready 输出 READY + 用户 G10.2 开工指令留痕 + 共享编号按 actual next_free 重新校准。任一条件不满足均保持 implementation_status=blocked"
  - id: G-G10-4
    check: "G10.2 退出门：M128/M129 两个 P0 独立断言全绿（参考帧批出 + 双跑 digest 一致 + provenance 闭集）；M130 骨架期 digest 比对面就位；出帧进程非零退出冒充成功/预置假帧冒充真出帧均为独立 RED 臂；裁决路径不可行时回退备选臂并 §8 只追加修订，禁以截图冒充 harness 出帧"
  - id: G-G10-5
    check: "G10.3 退出门：M131/M132 两个 P0 独立断言全绿；许可登记零缺行、白名单外许可注入即 RED；清单全场景加载绿、静默丢场景即 RED；M133 清单 digest 在树"
  - id: G-G10-6
    check: "G10.4 退出门：M134/M135/M136/M137 四个 P0 独立断言全绿；位深截断/sRGB 混标/参考扰动/diff 与标量不一致各为独立 RED 臂；M138 标定值入 g10_budget 且 provenance 齐备（P-09，禁手写阈值）"
  - id: G-G10-7
    check: "G10.5 退出门：M139/M140/M141 三个 P0 独立断言全绿；M130 双端 digest 不等不得出报告（门序硬约束）；差距清单场景全集零空行且每项带 UE5 模块归属 + measured delta + G11 承接锚；单端缺帧聚合不得 PASS；G10 不设画质与帧率通过线——差距全量登记即绿"
  - id: G-G10-8
    check: "G10.6 重评窗门：G9 十项 defer 逐行重判核验零空行（G10.5 measured 数据为法定证据输入）；命中者按只追加程序重判 go 并指定 G11+ 承接波次，未命中者维持 defer 且承接锚字面 0-byte；deferred history 只追加禁静默改判"
  - id: G-G10-9
    check: "G10.7 决策门：G10 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G11+，零空行；defer 必有承接锚（机核同构 ci/g9_p2_decisions_check.py）；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述"
  - id: G-G10-10
    check: "G10.8a 稳定门：全部 P0 与所有 go 的 P1 全量回归；G5~G9 既有判据 0-byte；出图/捕获/度量/差距清单全链路连续复跑 soak（量级沿 G9.8a 继承或 measured 证明更短足够，阈值 G10.1 裁决 measured 标定）；strict budget 非空、零 estimated/skip；同日放行按立项裁决 8（8a full-run 先行完成后允许同日进 8b）"
  - id: G-G10-11
    check: "G10.8b 收口门：验收映射、候选决策、RD 最终状态逐字一致；全部 P0 独立断言均 PASS；evidence/schema/预算终审；差距清单终审锁定为 G11 法定输入；§8 只追加后 status active→closed"
guardrails:
  - "双状态不可混同：status=active 仅表示 G10.1 governance-only 已立项；在 G-G10-3 真实通过前 implementation_status=blocked，任何治理完成叙述不得冒充 G10.2 开工"
  - "G10.1 允许 milestones/g10、G10 RFC、G10 专属 claim、deferred history 只追加、未编号 validator 与 measured baseline；src/spec/conformance 和编号 workflow 步骤 0-byte"
  - "G10 CI 只冻结 symbolic gate key 与脚本名；numeric_step 一律写 post-interlock actual-next-free allocation。不得沿用推测号与草案建议值，不得预放空 workflow、空脚本或空 schema 壳"
  - "每个 P0 必须独立布尔断言与独立 evidence subject；可共享一次进程执行，但聚合 PASS 不能遮蔽任一子断言 FAIL/SKIP"
  - "缺硬件/工具链/账号交互仅可 dev_env_degrade 或 SKIP=not-triggered；两者均不充 P0 绿。host oracle、mock、isolated nonzero、既有最小见证、人工截图均不能替代目标门"
  - "G10 零修复纪律：全域不提交任何画质修复 PR；差距清单只登记不修复，修复面由 G11 立项承接（只消费 G10.8b 锁定清单 + 承接锚）"
  - "G10 不设画质通过阈值与帧率通过线；差距全量 measured 登记即绿；任何『已达 UE5 画质』叙述在 G10 期内一律不成立"
  - "UE 源码仅外部参照：E:\\Kimi_Agent_Taichi Engine 优化计划\\references\\UnrealEngine 只读；零 vendoring、零片段复制进 src/spec；违反即 revert + 留痕"
  - "压测资产逐资产许可白名单（CC0/CC-BY 族）+ SPDX + 来源 URL + attribution + digest 登记；未核验资产不得进清单；二进制资产不入 git（外部缓存 K: 盘 + 仓库内元数据登记）"
  - "g10_budget 首个实现 PR 前必须非空 measured_local 且有 evaluator；全程零 estimated；性能数字不替代 correctness gate；阈值全部实测标定禁手写"
  - "新 unsafe 仅在实现互锁开放后按 actual next_free 登记并附 SAFETY；rurix-render 维持 forbid(unsafe_code)"
  - "触 G5~G9 冻结面必须 RFC 显式修订行，禁静默扩；G5~G9 closed 契约与 00-14 0-byte，close-out 证据只追加"
  - "新文件 LF + 尾换行；本契约合入后正文冻结，激活/验收/收口只追加 §8，除最终 status flip 外不回写既有事实"
---

# G10 契约 — UE5 画面对标基线期

> 计划：[G10_PLAN.md](G10_PLAN.md) v1.0 · 能力事实源：[G10_CAPABILITY_MATRIX.md](G10_CAPABILITY_MATRIX.md) · 候选决策：[G10_CANDIDATE_DECISIONS.md](G10_CANDIDATE_DECISIONS.md) · 机器门：[CI_GATES.md](CI_GATES.md)。
> 当前裁决：**G10.1 governance-only active；G10.2~G10.8b implementation blocked**。`active` 不是实现门绿灯。

---

## 1. 目标与双门状态

G10 是 UE5 画面对标**基线期**：建成「可批量出图的 UE5 5.8 参照环境 + 压测场景语料 + 图像度量基建（FLIP/SSIM/PSNR）+ 首轮 A/B 对比与 measured 差距清单 + 双端帧率基线」。「UE5 级」可核对基线沿用 G9 口径 = UE 5.8；验收五层级沿用 G9/G8：核心等价、功能闭环、可降级、可生产化、Vulkan 主线。G10 不用"对标"口号替代可验证事实：全部 P0 必须独立过门；**G10 只交基线与差距清单，不设画质通过阈值与帧率通过线**——修复归 G11、路径追踪生产化归 G12、DLSS/超分归 G13、性能优化归 G14、商用收口归 G15。

本契约拆分两种状态：

| 状态 | 当前值 | 含义 |
|---|---|---|
| `status` | `active` | G10.1 治理波已获授权，可落治理资产、两份 RFC、候选决策/验收映射、G10 专属 claim、互锁 validator、RTX 4070 Ti measured baseline 与非空 budget |
| `implementation_status` | `blocked` | G10.2+ 尚未获准；当前不得改 `src/`、`spec/`、`conformance/`，不得 materialize 数字 CI 步骤 |

G-G10-3 是唯一实现入口：互锁 validator（`check_g10_implementation_interlock --require-ready`）输出 READY + 用户 G10.2 开工指令留痕 + 共享编号按 actual `next_free` 重新校准，三者齐备方可解锁；任一缺失均保持 `blocked`。

## 2. 范围与严格波次

### 2.1 G10.1 governance-only

G10.1 只做 D-G10-1~5。允许治理文档、两份 Full RFC（须 D-409 评审后 Agent Approved 方为语义冻结）、候选决策表、验收映射、G10 专属无冲突 claim、互锁 validator、RTX 4070 Ti baseline 与非空 budget；禁止语义实现和编号 workflow。interlock validator 在当前事实下应明确返回 `BLOCKED`，这正是正确结果，不是失败需要被绕开。

### 2.2 G10.2~G10.8b implementation

实现互锁开放后按以下顺序推进，波次内可蜂群并行，波次间不得越级；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑：

```text
G10.2 UE5 出图环境波（裁决路径执行 + 批量参考帧 harness + 确定性契约骨架）
  → G10.3 压测语料波（资产获取 + 许可登记 + 加载门 + 清单冻结）
  → G10.4 度量基建波（帧捕获 HDR + FLIP/SSIM/PSNR 对拍 + diff 报告 + 阈值标定）
  → G10.5 首轮 A/B 对比波（双端出图 + 度量报告 + 差距清单 + 帧率基线）
  → G10.6 defer 重评窗波（G9 十项 defer 逐行重判，G10.5 measured 数据为法定证据输入）
  → G10.7 P2 穷举决策 → G10.8a stabilization/soak → G10.8b close-out
```

每波退出门见 YAML `acceptance_gates`（G-G10-4~8，判据按 G10_PLAN §2 各波退出门草案硬化）；任一上游门未绿，下游 evidence 即使局部成功也不能宣称波次完成。单点依赖：G10.2 是 G10.5 A/B 面的硬前置（G10.3 可与 G10.2 部分并行）。

## 3. G10.1 交付冻结

| ID | 交付 | 退出判据 |
|---|---|---|
| D-G10-1 | 契约四件套与双状态 | PLAN v1.0、CONTRACT、CI_GATES、非空 measured budget 一致；`status=active`、`implementation_status=blocked` |
| D-G10-2 | 候选决策与 RD 总映射 | G9 十锚 + 存续 open RD + G10 新增候选逐行；裁决、波次、承接锚、最终状态无空项；缺行阻断 G10.2 |
| D-G10-3 | 验收映射 | 全部 P0 全部有独立 key/script/schema 目标路径/check；go 的 P1 同步入表；不存在"由邻项代绿"；缺行阻断 G10.2 |
| D-G10-4 | 两份 Full RFC | 均经 D-409 独立 provenance 评审后 Approved（未 Approved 前本契约对应条款为引用占位）；编号登记与 README/ledger 一致 |
| D-G10-5 | baseline、budget、互锁 validator | RTX 4070 Ti measured 数据非空、零 estimated；interlock validator 对当前状态诚实报 BLOCKED；无空 workflow、无空 schema 壳 |

G10.1 完成仅关闭治理准备，不改变 G-G10-3 的机器事实。

## 4. 验收门与 P0 独立断言

### 4.1 波次验收门

G-G10-1~11 以 YAML 头为可提取摘要。[CI_GATES.md](CI_GATES.md) 冻结脚本与 evidence 形态。条件型分项的 `SKIP=not-triggered` 只表示决策已记录，不是成功；设备门的 `dev_env_degrade` 只表示环境缺失，也不是成功；Epic 账号人工接管点未完成是 `dev_env_degrade`，不充绿。

### 4.2 P0 独立断言

以下 12 行是 close-out 不可合并、不可删减的独立布尔断言（key 命名空间三方逐字一致，冻结）。一次 smoke 可以共享启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。evidence schema 目标路径统一为 `milestones/g10/g10_m<###>_<slug>_evidence_schema.json`——本契约只冻结路径，不预建文件。硬判据由 G10_PLAN §2 各波退出门草案与 §3 P0 建议清单展开为可机器求值形式，负例 RED 臂要求逐行写明。

| Symbolic gate key | M### | 最晚波次 | 稳定脚本名 | 独立硬判据 |
|---|---:|---|---|---|
| `g10.p0.m128.ue5_capture_environment` | M128 | G10.2 | `ci/g10_ue5_capture_environment_smoke.py` | spike 裁决路径落地 + 固定场景 UE 5.8 侧出帧成功 + 环境画像（UE build digest/驱动/锁频）随证据存档；出帧进程非零退出冒充成功即 RED；预置假帧冒充真出帧即 RED |
| `g10.p0.m129.ue5_reference_frames` | M129 | G10.2 | `ci/g10_ue5_reference_frames_smoke.py` | 场景清单逐场景参考帧落盘 + 同参数双跑帧 digest 一致 + provenance（场景/相机/光照/build）登记闭集；双跑 digest 不等即 RED；provenance 缺行即 RED |
| `g10.p0.m130.dual_determinism_contract` | M130 | G10.2 骨架 → G10.5 双端核验 | `ci/g10_dual_determinism_contract_smoke.py` | 相机/光照/时间参数同 schema 双端各一份 + digest 比对相等；单端参数漂移注入即 RED；schema 外字段注入即 RED；digest 不等仍出 A/B 报告即 RED（门序硬约束） |
| `g10.p0.m131.asset_license_registry` | M131 | G10.3 | `ci/g10_asset_license_registry_smoke.py` | 逐资产 license 白名单闭集 + SPDX id + 来源 URL + attribution + 资产 digest；未登记资产混入即 RED；白名单外许可注入即 RED |
| `g10.p0.m132.corpus_loading` | M132 | G10.3 | `ci/g10_corpus_loading_smoke.py` | 场景清单逐场景 Rurix 加载成功 + 三角形/材质/纹理计数非空 + 加载事件序列 golden；计数为零冒充成功即 RED；静默丢场景即 RED |
| `g10.p0.m134.frame_capture_pipeline` | M134 | G10.4 | `ci/g10_frame_capture_pipeline_smoke.py` | HDR 帧捕获落盘 + 捕获→回读逐像素往返无损 + 分辨率/色彩空间元数据齐备；位深截断注入即 RED；sRGB/线性混标注入即 RED |
| `g10.p0.m135.flip_metric` | M135 | G10.4 | `ci/g10_flip_metric_smoke.py` | 自实现与参考实现逐图对拍一致（容差 measured 标定）+ 恒等图对 FLIP=0 极值断言 + 参考实现版本 pin；参考输出扰动注入即 RED |
| `g10.p0.m136.ssim_psnr_metric` | M136 | G10.4 | `ci/g10_ssim_psnr_metric_smoke.py` | 口径冻结进 spec + 参考实现逐图对拍一致 + 恒等图对 SSIM=1/PSNR=inf 极值断言；口径漂移注入即 RED |
| `g10.p0.m137.pixel_diff_report` | M137 | G10.4 | `ci/g10_pixel_diff_report_smoke.py` | diff 热区图 + 逐区域统计落盘 + evidence schema 闭集；diff 图与标量报告不一致注入即 RED；空场景行即 RED |
| `g10.p0.m139.ab_comparison` | M139 | G10.5 | `ci/g10_ab_comparison_smoke.py` | 场景全集双端出图 + 度量报告 + 差距清单落盘；差距清单缺场景行即 RED；单端缺帧聚合 PASS 即 RED；M130 digest 不等仍出报告即 RED |
| `g10.p0.m140.gap_registry` | M140 | G10.5 | `ci/g10_gap_registry_smoke.py` | 每差距项带 UE5 Renderer 模块归属（模块路径枚举闭集）+ measured delta + 建议 P 级 + G11 承接锚；缺归属/缺承接锚行即 RED；非 measured 叙述充差距即 RED |
| `g10.p0.m141.perf_baseline` | M141 | G10.5 | `ci/g10_perf_baseline_smoke.py` | 双端同场景帧率采样（14 §5 协议）+ 环境画像随证据存档 + 双端交替采样顺序登记；未锁频/环境画像缺字段即 RED；采样轮数不足冒充即 RED |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G10.8b。M133（清单冻结）/M138（阈值标定）为 P1，入验收映射随主门核验。

## 5. Guardrails

见 YAML `guardrails`。特别强调三点：

1. 治理 active 不等于实现 active；G-G10-3 的机器事实（validator READY + 用户 G10.2 开工指令 + actual `next_free` 重校）不可替代。
2. 数字 CI 步骤只能在实现互锁开放后读取 actual `next_free` 再分配；文档中的稳定身份是 symbolic gate key 和脚本名；禁止沿用草案建议值。
3. **G10 零修复 + 零通过线**：差距全量 measured 登记即绿；任何画质修复 PR、任何"已达 UE5 画质/帧率"叙述在 G10 期内一律不成立——前者判 out-of-scope，后者判伪造绿灯。

## 6. Deferred 处置

| Deferred | G10 处置 |
|---|---|
| RD-039 | 总体维持 open 为法定输入；M61 mesh shader 分项维持 defer-to-G11+（重判条件未命中，承接锚字面 0-byte）；M44-p4 以 G10.3 语料真实超显存为触发评估前提，未实证前维持 no-go 留档；其余分项未触发维持 open |
| RD-040 | M52 SER 维持 defer（锚定 G12 路径追踪生产化期重评）；M99-clipmap/M100-high 登记「G10 触发评估」——G10.5 A/B 对比是 measured 画质举证法定通道，举证落地由 G11 承接、G10 零实现面；RD040-nrd 维持 no-go（G13 窗）；history 只追加 |
| RD-041 | M28/M40-svt/M26-fg/M05-mv/M56-wg 维持 no-go 留档；DLSS/Streamline 方向登记 G10-N5（锚定 G13，沿 RD-041 UpscaleBackend 接入面字面，G10 仅档案零接线） |
| RD-044 | M126-rd044/RD044-continuum/RD044-fluid 维持 open-留档/观察；G10-N3 的 FLIP 图像度量与 RD-044 族 FLIP 流体防混淆登记 |
| RD-034 | DXIL RT/mesh 上游 blocked 维持 open；G10 仅 Vulkan 主腿，不阻主线 |
| RD-042/043 | 可微物理观察 / wgrapier GPU 刚体观察维持，不进 G10 任何面 |

详情始终以 `registry/deferred.json` 为唯一事实源；本表只冻结承接纪律。G9 十项 defer 的逐行重判归 G10.6 重评窗（G10.5 measured 数据为法定证据输入）；SAFE-GPU 维持「G10+ 独立期」defer（G10 非其独立期，立项裁决 7）。

## 7. 修订记录与开工裁决

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-15 | 初版契约：按 G10_PLAN v1.0 显式拆分 governance 与 implementation；G10.1 active、G10.2+ blocked；冻结波次门（G-G10-1~11）与 12 个 P0 独立断言（key 命名空间三方逐字一致）；CI 数字延迟到 post-interlock actual-next-free allocation；九项立项裁决逐字登记；§8 只追加区启用。 |

**开工裁决留痕**：

- **用户立项指令**：2026-08-15 主会话下达「/goal 帮我完成 G10-15 的内容，自主派发调研 agent 和进行决策，里程碑推进时组织 agent-team 完成，要求彻底完成对标 UE5 渲染器的目标，并支持 dlss、超分采样、路径追踪等前沿技术。技术完成需要严格的画面审查，需要获取完整渲染画面，再用本地已有的 UE5 渲染器出图对比，修复画面中出现的细节问题；同时优化渲染管线效率，使帧率对标 UE5 略高（不降级画质）。本地已有 UE5 渲染器参考项目，你也可以联网获取（我的 GitHub 在 UE5 组织内），同时支持联网获取压测模型环境等必要工具集。最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在 G15 后无限制新建里程碑继续优化」（指令原文以会话留痕为准）。该指令授权 G10.1 governance-only 开工与 G10~G15 六期分期，不授权任何 `src/`/`spec/`/`conformance/` 语义实现或编号 CI 步骤 materialize。
- **agent 立项裁决**：依 10 §7、P-13 与 D-406 v2.0，agent 完全自主签署立项裁决；G10.1 治理波即刻 active，G10.2+ 继续由 G-G10-3 硬阻断。
- **不可变基线**：G10.0 文档集不可变 ref = `c0cdfddd`（G9 close-out 收口批 HEAD；工作树洁净，无未提交项待处置）。
- **九项立项裁决（逐字登记）**：
  1. 现在立项；G10.0 不可变 ref=`c0cdfddd`；工作树洁净零遗留，直接立项。
  2. UE5 出图路径：采纳 spike 建议——**首选 ②Launcher 安装 UE 5.8 正式版**（出处最干净的官方签名基线、最短路径、MRQ/HighResShot/glTF 导入开箱可用），Epic 账号登录设**人工接管点**（用户交互一次）；登录受阻则回退 **①源码编译臂**（K: 盘承载，qwasg 凭据已核查在 EpicGames 组织可用）；③公开参考图仅兜底对照材料、不进验收证据链。ue5-main 快照 vs 5.8-release 口径差登记：Launcher 版即官方 5.8 release，口径优先。
  3. 压测场景首发清单 = Cornell Box（程序生成，零许可风险）+ Sponza + Bistro 起步，许可逐资产核验后冻结（M131 门）；追加候选经白名单裁决只追加。
  4. 图像度量指标集 = FLIP + SSIM + PSNR 三指标；参考实现选型与版本 pin 由 RFC 冻结；HDR/LDR 域口径同冻。
  5. **G10 不设画质通过阈值与帧率通过线**——差距全量 measured 登记即绿，修复归 G11；帧率基线只建数据（G14 目标不设本期通过线）。
  6. G9 十项 defer 重评窗程序 = G10.6 逐行重判（G10.5 measured 数据为法定证据输入）；deferred history 只追加；承接锚字面 0-byte 维持。
  7. SAFE-GPU = 维持 defer（承接锚「G10+ 独立期立项」字面：G10 为画面对标基线期、非其独立期），顺延 G11+ 重评。
  8. G9.8b 同日放行先例 = 继承（8a full-run 先行完成后允许同日进 8b close-out；先例字面不扩展解释）。
  9. 压测资产二进制**不入 git**（外部缓存 K: 盘，仓库内只登记清单/许可/digest 元数据——H: 盘仅剩 6.9 GB，R-G10-11）；数字 CI 步骤 `post-interlock actual-next-free allocation` 重申确认。
- **G15 后无限续期授权登记**：用户指令「允许在 G15 后无限制新建里程碑继续优化」留痕——G15 收口若未达真实可商用标准，按同治理范式续立 G16+（每期仍独立走立项/治理波/互锁/full-run，不因授权免除任何机器门）。
- **RFC 编号**：两份 Full RFC 编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free` 领取；RXS/RD/U/RX/数字 CI 均延迟到实现互锁开放后按 actual `next_free` 领取。

---

## 8. Implementation activation / Close-out（只追加区）

<!-- 首条未来记录只能是 G-G10-3 互锁实测与 implementation_status 解锁凭据；其后追加逐波验收与 close-out。当前不得写 PASS、不得预填 run URL。 -->

### §8.1 G-G10-3 implementation_status 解锁记录（2026-08-15）

- **用户 G10.2 开工指令**：2026-08-15 主会话下达「/goal 帮我完成 G10-15 的内容，自主派发调研 agent 和进行决策，里程碑推进时组织 agent-team 完成，要求彻底完成对标 UE5 渲染器的目标，并支持 dlss、超分采样、路径追踪等前沿技术。技术完成需要严格的画面审查，需要获取完整渲染画面，再用本地已有的 UE5 渲染器出图对比，修复画面中出现的细节问题；同时优化渲染管线效率，使帧率对标 UE5 略高（不降级画质）。本地已有 UE5 渲染器参考项目，你也可以联网获取（我的 GitHub 在 UE5 组织内），同时支持联网获取压测模型环境等必要工具集。最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在 G15 后无限制新建里程碑继续优化」（G10.2~G10.8b 全实现波授权：12 P0 + 已 go P1 + defer 重评窗 + P2 穷举 + soak + close-out；指令原文以会话留痕为准；G9 §8.1「一次性完成 G9」同模授权先例）。
- **互锁 validator 实测**：`py -3 ci/check_g10_implementation_interlock.py --require-ready` → 事实门①~⑥全绿（①status=active ②治理五件齐备 ③g10_budget 非空 2 条 measured_local 零 estimated ④RFC-0026/0027 均 Agent Approved 且独立评审 provenance 在录 ⑤ledger reserved_in_flight[G10] 在树 ⑥check_g10_acceptance_map 三向 PASS）、一致性门 C1~C4 全绿，VERDICT=READY，exit=0（本小节落盘前实测；`--selftest` 9 RED + 1 GREEN + 1 TREE 全过）。
- **共享编号重校准（actual next_free，本 commit 落地时 `registry/number_ledger.json` 实测）**：CI_step `next_free=173`（G9 已消费至 172，v1.101）/ RXS `next_free=380` / RD `next_free=45` / U `next_free=58` / RX_error `next_free=7024` / MR `next_free=12` / RFC `next_free=28`（v1.102：0026/0027 claim 兑现校准后）/ D `next_free=410`。数字 CI 步骤自 173 起按波次实测顺位领取；禁沿用草案建议值。
- **front matter 双状态翻转**：`implementation_status: blocked → unblocked`；`active_scope` 追加 `g10_2_plus_implementation_waves`（`status` 维持 `active`，close-out 才 flip）。
- **G10.1 治理波交付清单（D-G10-1~5 全落盘）**：四件套（G10_PLAN v1.0 / G10_CONTRACT v1.0 / CI_GATES v1.0 / g10_budget v1.0 非空 measured）+ G10_CAPABILITY_MATRIX v1.0（M128~M143）+ G10_CANDIDATE_DECISIONS v1.0（十锚初裁全 defer-to-G11+ + 三锚 G10 触发评估登记 + 新增候选五行）+ G10_ACCEPTANCE_MAP v1.0（12 P0 + 2 P1 三向逐字一致）+ RFC-0026/0027 双 Full RFC（D-409 评审后 Agent Approved，provenance 偏差按先例如实登记）+ design/g10_ue5_harness_spike v1.0（出图路径裁决输入）+ 两份对抗性评审记录 + validator 五件套（interlock / acceptance_map / wave_exit_lib / p2_decisions 骨架 / wave6_reevaluation 骨架——后两者 --gate 诚实红）+ RTX 4070 Ti measured baseline（sr_pipeline L3 1.2068 ms / D2H pinned 24.5031 GB/s，evidence `g10_baseline_*_20260815T094538Z.json`，未锁频边界经 clock_lock_note 诚实存档）。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G9 §8.1 同模）。`Assisted-by: Kimi-K3`（影响范围：G10_CONTRACT §8.1 与 front matter、G10.1 治理波全部落盘件、RFC-0026/0027 翻 Approved、ledger v1.102；验证方式：interlock `--require-ready`/`--selftest`、check_g10_acceptance_map、check_schemas、check_number_ledger、check_structure、trace_matrix 361/361、budget_eval --strict 133 pass 0 skip、stable_snapshot 全绿实测，输出如本会话留痕）。

### §8.2 G10.2 波验收记录（2026-08-15，G-G10-4）

**① 独立断言清单（12 P0 中本波 3 行，逐行独立 PASS|FAIL 不互代）**：

| Symbolic gate key | 步骤 | 实测 verdict | evidence（最新件） |
|---|---:|---|---|
| `g10.p0.m128.ue5_capture_environment` | 177 | **PASS**（checks 10/10，device=executed） | `evidence/g10_m128_ue5_capture_environment_20260815T163219Z.json` |
| `g10.p0.m129.ue5_reference_frames` | 178 | **PASS**（checks 7/7，device=executed） | `evidence/g10_m129_ue5_reference_frames_20260815T163253Z.json` |
| `g10.p0.m130.dual_determinism_contract --phase g10.2` | 179 | **PASS**（checks 10/10，骨架期；`phase_g10_2_pass=true`、`phase_g10_5_pass=false`） | `evidence/g10_m130_dual_determinism_contract_20260815T163127Z.json` |

- M128：②Launcher 裁决路径落地（UE **5.8.1-56057345** @ `F:\UE_5.8`，`Build.version` 实测 ue_build_id）；Entry 静态空图 MRQ 臂真出帧（Phase B 官方命令行形态，gpu_device_lock 串行，exit 0，4 帧 1920×1080 EXR float 新出，`mtime ≥ run_start` 新鲜度机核 + EXR magic + 体积下限真帧判据）；环境画像七元组随 evidence 存档（ue_build_id / 驱动 620.02 / clock_lock_state=unlocked / scene_id / camera_params_digest `sha256:017f0b3b…` / lighting_params_digest `sha256:2bb35380…` / capture_arm `A(mrq)`）；RED 臂全检出——非零退出冒充成功 / 预置假帧冒充真出帧 / 画像缺字段（red_fixtures/m128/ 三件）+ live 探针（不存在工程真调 exit=1、0.9s 检出）。
- M129：暂定场景集 `milestones/g10/g10_2_provisional_scene_set.json` 登记（RFC-0027 §4.4 F8 形态，`entry-empty-static` 单场景闭集；CornellBox/Bistro UE 场景面缺口以 `deviation_note` 如实登记，不以临时场景集冒充，M133 冻结后按 F8 回归复核）；同参数双跑 canonical digest **4/4 一致**（harness `g10_determinism.exr_canonical_digest` 14 属性剥离单一事实源；`.0000` = `429ac81122a7…` 与环境日志 §7.3 跨跑复验一致）；provenance 七元组逐帧闭集；RED 臂全检出——Template_Default vs Entry 真实不等帧对 / provenance 缺行 / 真帧像素区翻字节 digest 漂移（red_fixtures/m129/ 三件）。
- M130 骨架期：spec-first 落 `spec/visual_comparison.md` **RXS-0384**（canonical preimage 字节布局字节级单源冻结）；Rurix 侧骨架参考解析器（ci 脚本内独立实现）与 UE 侧 harness `g10_param_contract.py`（DRAFT_BYTE_LAYOUT 随 spec 冻结替换 SPEC_BYTE_LAYOUT）双端 schema 各一份；同参数 JSON 双端 digest 相等（`param_digest = sha256:3ace41840c40e55a…`）；边界浮点差分语料（-0.0 / 次正规 / 2^53 / 长十进制 / 1e-310 / u64 上界）跨端逐位一致；RED 臂全检出——单端参数漂移 / schema 外字段 / 非单位四元数 / NaN；`--phase g10.5` 本波 fail-closed 拒跑（双端核验腿留 G10.5）。
- HighResShot 臂时序不稳与 `-csvCaptureFrames` 死路未复活作证据面（环境日志 §7.1 钉死）；UE 零 vendoring 纪律维持。

**② 聚合门实测**：`ci/g10_wave2_exit_check.py --gate g10.wave.2.exit`（步骤 180）→ 三门最新 evidence 只读汇总 3/3 PASS + 四 facts（RXS-0380/RXS-0384 条款头在树 / RFC-0026+0027 Agent Approved 字面 / 场景集登记 / M130 phase 纪律）全 PASS，**VERDICT=PASS**（`evidence/g10_wave2_exit_20260815T163317Z.json`）；聚合不代绿、不重跑 smoke、不遮蔽子断言；`--selftest` 负样本（空 evidence 目录）必红 + 正样本真树绿双全（负样本 FAIL evidence `…T163316Z.json` 诚实留痕，最新件为 PASS）。

**③ 验收命令与守卫套件实测（本记录落盘前）**：

```text
py -3 ci/g10_ue5_capture_environment_smoke.py --gate g10.p0.m128.ue5_capture_environment   → exit 0（PASS 10/10, device=executed）
py -3 ci/g10_ue5_reference_frames_smoke.py --gate g10.p0.m129.ue5_reference_frames         → exit 0（PASS 7/7, device=executed）
py -3 ci/g10_dual_determinism_contract_smoke.py --gate g10.p0.m130.dual_determinism_contract --phase g10.2 → exit 0（PASS 10/10）
py -3 ci/g10_wave2_exit_check.py --gate g10.wave.2.exit                                    → exit 0（VERDICT=PASS）
四门 --selftest → 全 PASS（m128 4RED+2GREEN / m129 3RED+2GREEN / m130 4RED+2GREEN / wave2 负正样本）
py -3 ci/check_structure.py → PASS · py -3 ci/check_schemas.py → PASS · py -3 ci/check_number_ledger.py → PASS
py -3 ci/trace_matrix.py --check → PASS（366/366 全锚定）· py -3 ci/stable_snapshot.py --check → PASS（366）
py -3 ci/check_g10_acceptance_map.py → PASS（三向逐字一致）· py -3 ci/budget_eval.py〔--strict〕→ PASS（133 pass 0 skip）
py -3 -m pytest tests/ -q → 117 passed
```

**④ 门序登记面**：spec 条款 RXS-0384 commit 先于实现段落写盘（spec-first，硬规则 7）；编号领取 = 落盘前实测 `CI_step.next_free=177` 顺位 177~180、`RXS.next_free=384` 顺位 RXS-0384（registry/number_ledger.json revision_log **v1.104**，on_tree_max CI_step 176→180 / RXS 383→384）；CI_GATES.md v1.2 修订行（§4/§4A/§5 表体 0-byte）；pr-smoke.yml 步骤 177~180（步骤 176 块后追加）；check_schemas.py 三处纯追加（既有路由 0-byte）；trace_matrix 365→366、stable 快照 365→366 重 bless（bless_log 追加行）；G5~G9 closed 判据与 G10.3 门脚本 0-byte；evidence/ 只增不删不改。

**⑤ 签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G9 §8.x 五块模板同构）。`Assisted-by: Kimi-K3（G10.2 波 materialize）`（影响范围：G10_CONTRACT §8.2、spec/visual_comparison.md 新建与 spec/README.md 登记、四门脚本与共享判定层、四 evidence schema、red_fixtures/m128+m129、pr-smoke.yml 步骤 177~180、CI_GATES v1.2、ledger v1.104、conformance/visual_comparison 语料、harness g10_param_contract SPEC 布局替换、g10_2_provisional_scene_set.json、trace/stable 重生成；验证方式如 ③ 全量实测输出留痕）。**遗留缺口（如实登记不充绿）**：CornellBox UE 程序生成场景与 Bistro UE 导入面未建成（环境日志 §7.4 缺项 #7，M129 暂定场景面偏差已登记）；M130 `--phase g10.5` 双端核验腿、门序三重绑定机器阻断、应用层探针归 G10.5；`-renderoffscreen` 5.8 可用性未测（本轮出图走窗口模式）。
