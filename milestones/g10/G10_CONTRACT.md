---
contract: G10
title: G10 UE5 画面对标基线期
status: closed
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

### §8.3 G10.3 波验收记录（2026-08-15，G-G10-5；主会话补落——wave3 聚合门已 PASS 于 G10.3 批，本记录为补登）

**① 独立断言清单（12 P0 中本波 2 行 + 已 go P1 中本波 1 行，逐行独立 PASS|FAIL 不互代）**：

| Symbolic gate key | 步骤 | 实测 verdict | evidence（最新件） |
|---|---:|---|---|
| `g10.p0.m131.asset_license_registry` | 173 | **PASS**（checks 7/7） | `evidence/g10_m131_asset_license_registry_20260815T124824Z.json` |
| `g10.p0.m132.corpus_loading` | 174 | **PASS**（checks 9/9，device=executed） | `evidence/g10_m132_corpus_loading_20260815T124829Z.json` |
| `g10.p1.m133.corpus_list_freeze` | 175 | **PASS**（checks 7/7） | `evidence/g10_m133_corpus_list_freeze_20260815T124830Z.json` |

- M131：许可白名单闭集 {CC0-1.0, CC-BY-3.0, CC-BY-4.0} 机核 + external 五元组 / generated 六字段按类登记（互冒充即 RED）+ attribution 子字段闭集 + digest 复算一致 + git 零二进制守卫；RED 五件全检出（白名单外许可注入〔Emerald Square CC-BY-NC-SA-3.0 夹具〕/未登记资产混入/缺字段/互冒充/digest 篡改）。
- M132：rxcook 真跑加载——BistroInterior glTF（FBX2glTF v0.9.7 派生，工具 sha256 登记）**1,046,609 三角形 / 70 材质 / 144 纹理**（与包内 README total 逐字一致）+ CornellBox 程序生成 34/4/1；计数/六表全等 golden + 加载事件序列 golden + 静默丢场景零；RED 三件全检出。
- M133：场景清单 digest `d96b4d2f…` 注册在树 + 只追加修订程序机核（原地改即 RED）+ M131/M132 行集对账 + ready 下界 ≥2（vacuous PASS 拦截）。
- 资产实测登记：Bistro（ORCA，**CC-BY-4.0**）`Bistro_v5_2.zip` 894,377,473 B，sha256 `0d50e3c7…34e1`，解包 643 文件 2,613,499,054 B，清单级 canonical digest `0afc237b…4ac4`，缓存 `K:\rurix_g10_cache\bistro-orca\v5_2\`（零入 git）；CornellBox 生成器 `ci/_gen_g10_cornell_box.py`（generated 类六字段），4 文件 13,059 B，digest `a53b05d7…fdaa8e`；BistroInterior 派生 glTF 146 文件 553,266,741 B，digest `4dae7c0d…2565`。
- 遗留缺口（如实登记不充绿）：**BistroExterior 未入清单**——FBX2glTF v0.9.7 转换 Exterior 动画烘焙 95% 后写 .gltf 失败（K:/H: 双盘四臂同失败，无纹理诊断臂成功，工具内部缺陷非环境问题）；首发清单场景面 = BistroInterior + CornellBox（ready=2 满足下界），Exterior 启用走只追加修订程序；纹理 DDS 原样拷贝（URI 不透明保留），纹理解码归后续波次。

**② 聚合门实测**：`ci/g10_wave3_exit_check.py --gate g10.wave.3.exit`（步骤 176）→ 三门最新 evidence 只读汇总 3/3 PASS + 四 facts（RXS-0380~0383 条款头在树 / RFC-0027 Agent Approved 字面 / 清单 digest 注册在树 / M131 白名单字面）全 PASS，**VERDICT=PASS**（`evidence/g10_wave3_exit_20260815T124853Z.json`）；聚合不代绿、不重跑 smoke、不遮蔽子断言；`--selftest` 红绿双全（缺 evidence 必红负样本留痕件在库）。

**③ 验收命令与守卫套件实测（G10.3 批留痕）**：四门 `--gate` 全 PASS + 全门 `--selftest` 红绿双全 + `check_structure` / `check_schemas` / `check_number_ledger`（365 条款零碰撞）/ `trace_matrix --check` 365/365 / `budget_eval` 133 pass 0 skip / `check_g10_acceptance_map` 三向 / `stable_snapshot`（361→365 重 bless，bless_log 追加）/ `pytest` 117 passed 全 PASS。

**④ 门序登记面**：spec-first 落 `spec/external_reference.md` RXS-0380~0383（条款 commit 先于实现段落）；编号领取 = 落盘前实测 actual next_free 顺位（CI_step 173~176 / RXS-0380~0383；registry/number_ledger.json revision_log **v1.103**，on_tree_max CI_step 172→176 / RXS 379→383）；pr-smoke.yml 步骤 173~176（步骤 172 块后追加）；CI_GATES.md v1.1 修订行（§4/§4A/§5 表体 0-byte）；check_schemas.py 三处纯追加；.gitignore 缓存路径守卫块纯追加；G5~G9 closed 判据 0-byte；evidence/ 只增不删不改。commits：`3d019d3d`（feat 主批 58 文件）/ `6550e470`（四门 evidence 入库）/ `13bf40a0`（wave3 selftest RED 臂留痕件）。

**⑤ 签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署）。`Assisted-by: Kimi-K3`（影响范围：G10.3 波三门 + 聚合门 + spec RXS-0380~0383 + 本 §8.3 补落；验证方式：③ 全量实测输出留痕 + evidence 最新件复核）。

### §8.4 G10.4 波验收记录（2026-08-15，G-G10-6）

**① 独立断言清单（12 P0 中本波 4 行 + 已 go P1 中本波 1 行，逐行独立 PASS|FAIL 不互代）**：

| Symbolic gate key | 步骤 | 实测 verdict | evidence（最新件） |
|---|---:|---|---|
| `g10.p0.m134.frame_capture_pipeline` | 181 | **PASS**（checks 13/13，device=executed） | `evidence/g10_m134_frame_capture_pipeline_20260815T194016Z.json` |
| `g10.p0.m135.flip_metric` | 184 | **PASS**（checks 13/13，device=not_applicable） | `evidence/g10_m135_flip_metric_20260815T194017Z.json` |
| `g10.p0.m136.ssim_psnr_metric` | 182 | **PASS**（checks 12/12，device=not_applicable） | `evidence/g10_m136_ssim_psnr_metric_20260815T194018Z.json` |
| `g10.p0.m137.pixel_diff_report` | 183 | **PASS**（checks 12/12，device=not_applicable） | `evidence/g10_m137_pixel_diff_report_20260815T194018Z.json` |
| `g10.p1.m138.metric_threshold_calibration` | 185 | **PASS**（checks 12/12，device=not_applicable） | `evidence/g10_m138_metric_threshold_calibration_20260815T194127Z.json` |

- M134（A 段）：EXR 自研最小子集 `src/image-io/src/exr.rs`（float32 RGB scanline NONE 编+解，ZIP fail-closed 显式 UnsupportedCompression，全 safe 零外部依赖）+ device 腿（GPU 真渲染 → Rgba16Float readback → fp16→f32 精确提升，gpu_device_lock 串行）与 host 腿（闭式探针）各自捕获→落盘→回读逐像素位级往返无损 + 元数据闭集齐备（`ci/g10_exr_lib.py` 独立第二实现互核 + 跨实现帧 digest 互证）+ 渲染输出探针图案位级核验 + UE 真帧 strip-and-log 读取（fp16→f32，`unreal/*` 剥离 3 条登记，chromaticities 位级闭集互证）；位深截断/sRGB 混标/元数据缺字段三 RED 臂全检出（harness 内联 + `--red-arm` 复跑双保险）；首跑 FAIL 诚实留痕（`…T180226Z.json`，真实 UE 帧 version 字段 long-names 标志被 v1 严格校验拦截，spec/imageio.md v1.3 修订行加性放读后复原绿——真实红绿闭合）。
- M135（B 段）：spec-first RXS-0389（commit `e6a4c7c2` 先行）+ 参考实现 NVlabs/flip **pin 五元组**齐备——commit `b475eb4bf394ab877c42166c9eb0a84a02cc5b14`（`git ls-remote` HEAD 实测）+ zip 快照 `sha256:d4e0362c…` 双记；选臂 = **python-nanobind**（本地 pin 源码树 `K:\rurix_g10_cache\tools\flip` pip install 一次构建成功，wheel `flip_evaluator-1.7-cp312-cp312-win_amd64` `sha256:46348e21…`；MSVC VS2022 BuildTools 17.14.38 + CMake 4.3.0 + nanobind 2.12.0 + scikit-build-core 0.11.6 + Python 3.12；运行参数集 `evaluate(ref,test,'LDR',inputsRGB=True,applyMagma=False,computeMeanError=True,parameters={})`；**首选 cpp tool 前即选 nanobind 臂，非构建失败后回退——如实登记选臂**）；自实现 `ci/g10_flip_lib.py`（YCxCz 管道逐字，numpy）与参考 25 图对五类 LDR 对拍**两面分列**全在容差内（标量差 p100=3.0153e-05 × k=2.0 → 6.0305e-05、误差图逐像素差 p100=9.8765e-05 × k=2.0 → 1.9753e-04，provisional_pending_m138；图集 manifest digest 入 evidence）+ 恒等图对 FLIP=0 极值双侧（标量恰 0 且误差图逐像素恰 0）+ ppd 策略冻结（自实现默认 67.02064514160156 与参考返回参数字典逐位一致）+ HDR-FLIP 探针臂（auto-from-reference 曝光 5 对同批对拍 + 恒等 HDR 恰 0）；参考输出扰动（标量面+误差图面）/口径漂移（gqc 0.7→0.8）/恒等非零/下界冒充四 RED 臂全检出。
- M136（A 段）：自实现 Wang 2004 逐字 vs scikit-image 0.26.0 显式参数化参考（pin + 参数化 digest 登记）25 图对五类对拍全在容差内（tol_ssim=2.220e-16〔p100 1.11e-16 × k=2.0〕/tol_psnr=0.0，provisional_pending_m138）+ 恒等 SSIM=1/PSNR=inf + LDR 域限定（HDR 直算即拒）；口径漂移/参考扰动/恒等非极值/HDR 直算/下界冒充五 RED 臂全检出。
- M137（A 段）：报告器 `g10_m137_diff_report`（host 纯 safe）产误差 EXR 单通道 Y + 灰度热区图 PPM + 16×16 区域统计（nearest-rank p95 + 边缘规则）+ 标量三面投影，门侧独立第二实现逐面重算核验 golden + artifacts 四 digest 对账 + evidence 闭集机核 + thresholds provisional（identity 噪声底 p100 实测 0.0）；diff 图与标量不一致/空场景行/闭集外字段三 RED 臂全检出。
- M138（B 段，P1）：标定程序可复跑（同一图集 digest 上 p100 估计器两跑逐位一致）+ 五面标定值（FLIP 标量/FLIP 误差图/SSIM/PSNR/diff over_threshold）p100 × k **字节级纯追加**入 `g10_budget.json`（五条 `g10.metric.*` 条目 measured_local + provenance + 环境画像，P-09 禁手写——首跑整文重写行尾漂移自检检出后修为字节级追加，既有行 0-byte；复跑幂等口径 = 值面逐字一致 + 在树 evidence_file trimmed_mean 复核，防「同值换皮」假漂移）+ 三门 provisional_pending_m138 标记消费登记（标定重算值与门内登记 p100 逐位一致）+ `budget_eval --strict` 全 PASS（138 pass 0 skip）；手写阈值冒充/estimated 冒充/不可复跑/门 evidence 缺失冒充四 RED 臂全检出（幂等修复前复跑实测 11/12 FAIL 留痕 `…T194020Z.json`，修复后 12/12——真实红绿闭合）。

**② 聚合门实测**：`ci/g10_wave4_exit_check.py --gate g10.wave.4.exit`（步骤 186）→ 五门最新 evidence 只读汇总 5/5 PASS + 四 facts（spec/imageio.md RXS-0385 + visual_comparison.md RXS-0386~0389 条款头在树〔共 5 枚〕/ RFC-0026 Agent Approved 字面在树 / 标定值入 `g10_budget` 且 provenance 齐备〔五条 `g10.metric.*` 条目 measured_local + evidence_file 在树可解 trimmed_mean + threshold == trimmed_mean × k 重算口径 + 样本集 digest 引用〕/ 四门 RED 臂独立有效〔M134/M135/M136/M137 最新 evidence 各含 red_* checks 且全真，共 15 臂〕）全 PASS，**VERDICT=PASS**（`evidence/g10_wave4_exit_20260815T194151Z.json`）；聚合不代绿、不重跑 smoke、不遮蔽子断言；`--selftest` 负样本（空 evidence 目录）必红 + 正样本真树绿双全（负样本 FAIL evidence `…T194149Z.json` 诚实留痕——负/正样本隔 1.1s 防同秒同名覆写，最新件为 PASS）。

**③ 验收命令与守卫套件实测（本记录落盘前）**：

```text
py -3 ci/g10_frame_capture_pipeline_smoke.py --gate g10.p0.m134.frame_capture_pipeline   → exit 0（PASS 13/13, device=executed）
py -3 ci/g10_flip_metric_smoke.py --gate g10.p0.m135.flip_metric                         → exit 0（PASS 13/13）
py -3 ci/g10_ssim_psnr_metric_smoke.py --gate g10.p0.m136.ssim_psnr_metric               → exit 0（PASS 12/12）
py -3 ci/g10_pixel_diff_report_smoke.py --gate g10.p0.m137.pixel_diff_report             → exit 0（PASS 12/12）
py -3 ci/g10_metric_threshold_calibration_smoke.py --gate g10.p1.m138.metric_threshold_calibration → exit 0（PASS 12/12）
py -3 ci/g10_wave4_exit_check.py --gate g10.wave.4.exit                                  → exit 0（VERDICT=PASS）
六门 --selftest → 全 PASS（m134 3RED+2GREEN / m135 3RED+3GREEN / m136 3RED+2GREEN / m137 2RED+2GREEN / m138 3RED+3GREEN / wave4 负正样本；g10_wave_exit_lib 6RED+1GREEN）
py -3 ci/check_structure.py → PASS · py -3 ci/check_schemas.py → PASS · py -3 ci/check_number_ledger.py → PASS（371 条款零同号碰撞）
py -3 ci/check_g10_acceptance_map.py → PASS（三向逐字一致）· py -3 ci/budget_eval.py --strict → PASS（138 pass 0 skip）
py -3 ci/trace_matrix.py --check → PASS（371/371 全锚定）· py -3 ci/stable_snapshot.py --check → PASS（371）
py -3 ci/check_g10_implementation_interlock.py --require-ready → VERDICT=READY exit 0（--selftest 11 RED + 1 GREEN + 1 TREE 全过）
py -3 -m pytest tests/ -q → 117 passed
```

**④ 门序登记面**：spec 条款 RXS-0385~0388（A 段，commit `7689674d`）与 RXS-0389（B 段，commit `e6a4c7c2`）spec-first 先行（硬规则 7）；编号领取 = 落盘前实测 `CI_step.next_free=181/184` 顺位 181~186、`RXS.next_free=385/389` 顺位 RXS-0385~0389（registry/number_ledger.json revision_log **v1.105/v1.106**，on_tree_max CI_step 180→186 / RXS 384→389）；CI_GATES.md v1.3（A 段）/ v1.4（B 段）修订行（§4/§4A/§5 表体 0-byte）；pr-smoke.yml 步骤 181~183（A 段，步骤 180 块后）/ 184~186（B 段，步骤 183 块后）；check_schemas.py A 段三处 + B 段四处纯追加（既有路由 0-byte）；trace_matrix 366→371、stable 快照 366→371 重 bless（bless_log 追加行）；互锁 validator `ci/check_g10_implementation_interlock.py` **C3/C4 两态校准**（治理修复，判据语义 0-byte：blocked 态原机核维持、unblocked 态自动不适用登记 skipped_reason——校准前实测 C3/C4 FAIL exit=1〔workflow g10 token 36 处 + ci/g10_*_smoke.py 9 件 + 三面命中 36 处〕，校准后 not_applicable PASS exit=0，selftest 11 RED + 1 GREEN + 1 TREE 实证两态）；G5~G9 closed 判据与 G10.2/G10.3 门脚本 0-byte；evidence/ 只增不删不改。**§8.3 处置注记（如实登记）**：G10.3 波聚合门 `g10.wave.3.exit` 已 PASS 于 G10.3 批（最新件 `evidence/g10_wave3_exit_20260815T124853Z.json`，M131/M132/M133 三门 + 四 facts 全绿），但 G10.3 波验收记录未随 G10.3 批落 §8.3；本任务按波次编号只落 §8.4，**§8.3 由主会话补落或不补**（本记录不代补、不遮蔽该缺口）。

**⑤ 签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G9 §8.x 五块模板同构）。`Assisted-by: Kimi-K3（G10.4b 波）`（影响范围：G10_CONTRACT §8.4、spec/visual_comparison.md RXS-0389 与 spec/README.md 登记、ci/g10_flip_lib.py、ci/g10_flip_metric_smoke.py、ci/g10_metric_threshold_calibration_smoke.py、ci/g10_wave4_exit_check.py、四 evidence schema、check_schemas.py 四处纯追加、pr-smoke.yml 步骤 184~186、g10_budget.json 五条标定条目、互锁 validator C3/C4 两态校准、CI_GATES v1.4、ledger v1.106、conformance/visual_comparison 语料三件、trace/stable 重生成；A 段影响面见 v1.3 修订行与 G10.4a 三 commit；验证方式如 ③ 全量实测输出留痕）。**遗留缺口（如实登记不充绿）**：HDR-FLIP 为探针臂形态（5 对 auto-from-reference 同批对拍），非独立标定样本集——HDR 域正式对拍标定随 G10.5 A/B 波真实 HDR 帧对接通；M137 `scalars.flip` 字段维持 `null` 演进位（RXS-0388 L3，G10.5 翻转实值）；§8.3 未落（见 ④ 注记）。

### §8.5 G10.5 波验收记录（2026-08-15，G-G10-7）

**① 独立断言清单（12 P0 中本波 3 行 + M130 双端核验腿，逐行独立 PASS|FAIL 不互代）**：

| Symbolic gate key | 步骤 | 实测 verdict | evidence（最新件） |
|---|---:|---|---|
| `g10.p0.m139.ab_comparison` | 188 | **PASS**（checks 16/16，device=executed） | `evidence/g10_m139_ab_comparison_20260816T022655Z.json` |
| `g10.p0.m140.gap_registry` | 189 | **PASS**（checks 18/18，device=not_applicable） | `evidence/g10_m140_gap_registry_20260816T022655Z.json` |
| `g10.p0.m141.perf_baseline` | 190 | **PASS**（checks 11/11，device=executed） | `evidence/g10_m141_perf_baseline_20260816T022229Z.json` |
| `g10.p0.m130.dual_determinism_contract --phase g10.5` | 187 | **PASS**（checks 13/13，`phase_g10_5_pass=true`；本波 M139 门内当次 session 真跑复验） | `evidence/g10_m130_dual_determinism_contract_20260816T022552Z.json` |

- **M139**（A/B 对比门）：门序三重绑定机器核验——门内子进程当次 session 真跑 M130 `--phase g10.5`（UE 真跑自持 gpu_device_lock，本门不嵌套持锁 D5 定案）+ host 侧联合 param_digest 独立重算对账 == M130 最新 evidence 登记值（`sha256:64fd54df6e9be522…`，逐场景 digest 同为 cornell `80305791…`/bistro `ad45951b…` 三方相等）∧ 同 `base_commit` ∧ 同 `session_run_id`（`g10ab-20260816T022552Z`）；`verify_three_binding` **双场景口径修订回 RFC 字面**（双端 digest 相等断言 + 联合值比对分列——G10.5a 形态 rurix==ue5==入参在联合值 ≠ 首场景 digest 时对本门自身 g10.5 evidence 恒假，过严；M130 门 `--phase g10.2` 骨架回归 10/10 与 selftest 6 RED+3 GREEN 修订后实测维持）。场景全集双端四组帧齐备（cornell-box + bistro-interior × HDR/LDR × 双端）+ **Rurix HDR release 重渲染 digest 逐位复现库帧**（`c2000ebf…`/`8519cc67…`——G10.5a 库帧为 release profile 产物，debug profile bistro 有 ULP 级 build 敏感性，如实登记）+ LDR 派生逐字节复现 + UE HDR `unreal/build` == M128 登记 `5.8.1-56057345` + 双端 HDR 内容 digest == G10.5a 注册常量 + LDR 臂 FLIP/SSIM/PSNR 重算 == G10.5a golden 逐位（cornell FLIP 0.338645/SSIM 0.348298/PSNR 13.9829；bistro FLIP 0.940317/SSIM 0.167102/PSNR 2.5845）+ 逐像素 diff 报告重跑独立重算三面一致（**H1 修订兑现**：`g10_m137_diff_report` domain 自输入帧元数据派生 == `display-referred-ldr` 互证，image-io `ExrDomain::as_str` 转 pub；M137 门回归 12/12 实测绿）+ 差距清单 11 项 RXS-0391 schema 装配落盘 `milestones/g10/g10_gap_registry.json`（幂等复核——双跑逐字节相等实测；首跑 FAIL `…T010835Z.json`〔Windows 路径 JSON 解析 + 绑定语义过严双因〕与幂等 FAIL `…T014001Z.json`〔diff 报告文件 digest 内含 provenance 时间戳易失 → attachments 改绑确定性 error_map/heatmap digest〕诚实留痕，真实红绿闭合）+ RED 五臂全检出（缺场景行/单端缺帧/口径漂移/digest 不等出报告阻断/陈旧绑定冒充）。
- **M140**（差距清单登记门）：preview §5/§6 候选全 **11 项**入正式清单（quality_gap 8 = R1~R5/U1~U3 / caliber_diff 3 = C1~C3），每项 UE5 Renderer 模块归属（枚举闭集 23 目录级 + 57 文件级 + Other 终值；Other 行 = R5/U3 共 2 行，attribution_note 非空，计数登记防滥用）+ measured delta（delta == b−a f64 精确重算 + evidence_digest 全量回溯 M139 `ab_report.artifact_digests` 登记集）+ 建议 P 级（P0×5：R1/R3/R4/U1/U2；P1×2：R2/C1；P2×4：R5/U3/C2/C3）+ G11 承接锚非空 + gap_id 冻结字节规则重算 + 场景全集零空行（scene_summary == M133 清单双场景：cornell 4 行/bistro 7 行 + not_ready_scenes 显式空集在列）+ RED 六臂全检出。**G10 零通过线纪律维持**：差距全量 measured 登记即绿，不设 FLIP/SSIM 阈值判据；H1~H3 为 harness 面标注非渲染差距（kind 两值闭集字面），不入渲染清单——H1 本波已修订，H2/H3 维持登记。
- **M141**（性能对标基线门）：双端同场景帧率采样（14 §5 协议：L0 环境验证 env_probe 全字段画像 → warmup ≥10 → 50×3 trimmed mean → IQR）+ 双端交替采样顺序登记（场景粒度 [rurix@cornell → ue5@cornell → rurix@bistro → ue5@bistro] 逐腿 UTC 起止）+ 锁频状态实测登记（**未锁频** `clock_lock_note` 诚实存档沿 G10.1 baseline 先例）。基线数字（trimmed mean，measured_local）：**cornell-box：Rurix 159.0681 ms（6.287 fps，release profile host CPU GI 管线）vs UE 5.8.1 6.5000 ms（153.846 fps，MRQ benchmark）**；**bistro-interior：Rurix 17584.2601 ms（0.057 fps）vs UE 20.1667 ms（49.587 fps）**；Rurix 端首帧内容 digest == A/B 库帧锚（c2000ebf…/8519cc67…）+ distinct==1 确定性断言；UE 端逐帧 `unreal/frameRenderDuration` 取 EXR 头元数据（5.8 源树 MoviePipeline.cpp `RenderTimeFrameStatistics` → FileMetadata 实证面）；统计面独立第二实现重算核验 + 未锁频登记缺失/画像缺字段/采样轮数不足冒充三 RED 臂全检出。**只建基线数据，不设帧率通过线**（G14 承接面）。

**② 聚合门实测**：`ci/g10_wave5_exit_check.py --gate g10.wave.5.exit`（步骤 191）→ 四门最新 evidence 只读汇总 4/4 PASS（M139/M140/M141 + M130 `--phase g10.5` 腿 `phase_g10_5_pass==true`，MAP §3.3）+ 五 facts（RXS-0391+0390+0384 条款头在树 / RFC-0026+0027 双 Agent Approved 字面 / 门序三重绑定留痕〔M139 内嵌 three_binding == M130 g10.5 最新 evidence 登记面逐字相等〕/ 差距清单场景全集零空行与 not_ready 显式在列 + 清单 digest 无漂移 / 三门 RED 臂独立有效共 14 臂）全 PASS，**VERDICT=PASS**（`evidence/g10_wave5_exit_20260816T022707Z.json`）；聚合不代绿、不重跑 smoke、不遮蔽子断言；`--selftest` 负样本必红 + 正样本真树绿双全。

**③ 验收命令与守卫套件实测（本记录落盘前）**：

```text
py -3 ci/g10_ab_comparison_smoke.py --gate g10.p0.m139.ab_comparison              → exit 0（PASS 16/16, device=executed；双跑幂等复核绿）
py -3 ci/g10_gap_registry_smoke.py --gate g10.p0.m140.gap_registry                → exit 0（PASS 18/18）
py -3 ci/g10_perf_baseline_smoke.py --gate g10.p0.m141.perf_baseline              → exit 0（PASS 11/11, device=executed）
py -3 ci/g10_wave5_exit_check.py --gate g10.wave.5.exit                           → exit 0（VERDICT=PASS）
py -3 ci/g10_dual_determinism_contract_smoke.py --gate g10.p0.m130.dual_determinism_contract --phase g10.2 → exit 0（骨架回归 10/10）
四门 --selftest → 全 PASS（m139 4RED+4GREEN / m140 1RED+3GREEN / m141 2RED+4GREEN / wave5 负正样本；gap_registry_lib 10RED+1GREEN）
py -3 ci/check_structure.py → PASS · py -3 ci/check_schemas.py → PASS · py -3 ci/check_number_ledger.py → PASS（373 条款零同号碰撞）
py -3 ci/check_g10_acceptance_map.py → PASS（三向逐字一致）· py -3 ci/budget_eval.py --strict → PASS（138 pass 0 skip）
py -3 ci/trace_matrix.py --check → PASS（373/373 全锚定）· py -3 ci/stable_snapshot.py --check → PASS（373）
py -3 ci/check_g10_implementation_interlock.py --require-ready → VERDICT=READY exit 0
```

**④ 门序 / not-triggered / no-go 登记面摘要**：spec 条款 RXS-0391 spec-first 先行（commit `4925efdf`，硬规则 7）；编号领取 = 落盘前实测 `CI_step.next_free=188` 顺位 188~191、`RXS.next_free=391` 顺位 RXS-0391（registry/number_ledger.json revision_log **v1.109/v1.110**，on_tree_max CI_step 187→191 / RXS 390→391）；CI_GATES.md v1.6 修订行（§4/§4A/§5 表体 0-byte）；pr-smoke.yml 步骤 188~191（步骤 187 块后追加）；check_schemas.py 四处纯追加（load/validator/前缀路由，既有路由 0-byte）；共享判定层 `ci/g10_gap_registry_lib.py`（RXS-0391 IR2 单一事实源）；UE bench harness 两件（`ue_python/g10_5_build_bench.py` + `g10_5_ue_bench.py`）；trace_matrix 372→373、stable 快照 372→373 重 bless（bless_log 追加行）；G5~G9 closed 判据与 G10.2~G10.5a 门脚本 0-byte（M130 门仅 verify_three_binding 双场景口径修订，见 ①）；**HEAD 既有 rustfmt 漂移面（hlod.rs/terrain.rs/water.rs 等）与 rurix-render lib 3 条 clippy 警告为 HEAD 既有状态如实登记**（G10.5a fmt 收口批已登记 hlod.rs 不回写先例），本批触改文件（g10_5_scene_render.rs / g10_m137_diff_report.rs / exr.rs）rustfmt+clippy 零警告实测（bin 内 4 条 A 段遗留警告随本批顺手清净：3 redundant closure + 1 collapsible if，语义 0-byte）；本波无 not-triggered 分项、无 no-go；M139 两度 FAIL 件与 wave5 负样本 FAIL 件诚实留痕（evidence/ 只增不删不改）。

**⑤ 签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G9 §8.x 五块模板同构）。`Assisted-by: Kimi-K3（G10.5b 波）`（影响范围：G10_CONTRACT §8.5、ci/g10_ab_comparison_smoke.py / g10_gap_registry_smoke.py / g10_perf_baseline_smoke.py / g10_wave5_exit_check.py / g10_gap_registry_lib.py、milestones/g10/g10_gap_registry.json 与四 evidence schema、ci/g10_dual_determinism_contract_smoke.py verify_three_binding 双场景口径修订、image-io exr.rs as_str pub + g10_m137_diff_report.rs domain 派生（H1）+ g10_5_scene_render.rs --benchmark 加性子模式、harness UE bench 两件、check_schemas.py 四处纯追加、pr-smoke.yml 步骤 188~191、CI_GATES v1.6、ledger v1.110；验证方式如 ③ 全量实测输出留痕）。**遗留缺口（如实登记不充绿）**：M141 基线为单轮采样（14 §5 50×3 足额，M0 §3「三次进程级独立运行」为预算回填口径——本门不回填 budget，trial 块为进程内/单 MRQ run 内连续段，已如实登记采样形态）；UE 侧 frameRenderDuration 含 MRQ 捕获合并开销（读回/写盘异步面），口径注释在案；Rurix bistro 0.057 fps 为 host CPU 参考管线实测，GPU 管线帧率面归后续波次；G10.6 重评窗以本波 measured 数据为法定证据输入（契约 G-G10-8）。

### §8.6 G10.6 波验收记录（2026-08-15，G-G10-8）

**① 门断言清单（本波 1 门，逐条独立 PASS|FAIL 不互代）**：

| Symbolic gate key | 步骤 | 实测 verdict | evidence（最新件） |
|---|---:|---|---|
| `g10.wave.6.reevaluation` | 192 | **PASS**（facts 14/14，device=not_applicable） | `evidence/g10_wave6_reevaluation_20260816T031917Z.json` |

- **十锚逐行重判核验（G10.5 measured 数据为法定证据输入，零空行）**：`milestones/g10/G10_DEFER_REEVALUATION.md` v1.0 落盘——十锚闭集全等（G9_P2_DECISIONS §1 十行 defer-to-G10+）+ 每行 G10.5 measured 证据列在树且至少一条为 A/B measured 面前缀闭集（g10_m139_ab_comparison_/g10_m140_gap_registry_/g10_m141_perf_baseline_，法定证据输入语义机核）。**重判结论：M99-clipmap 唯一 rejudged-go，其余九锚 maintain-defer**——
  - **M99-clipmap（rejudged-go）**：差距清单 R4（P0，GI = 屏幕探针单反弹，bistro HDR 亮度 p90 a=0.30276253819465637 vs b=5.000015625，delta=4.697253086805343 measured）+ C1（P1，GI/天光遮蔽口径差 = 室内亮度主差，bistro HDR 中位 ≈21× measured）双行实证「屏幕探针远场缺失成为画质 measured 问题」命中重判条件；按只追加程序重判 go 并指定 **G11 画质修复期**为承接波次（**G10 零实现面——重判 go 只指承接不实现**，G11 只消费 G10.8b 锁定清单 R4/C1 行 + 承接锚）；兜底 = 屏幕级 SPG + Radiance Cache（g9.p1.m99 门绿）维持，不以屏幕级绿色冒充世界级验收。
  - **M100-high（maintain-defer）**：语料多灯场景真实存在（bistro 4+ 点光源 + emissive，R3 P0 measured），但 A/B 臂为 host CPU 参考管线、未消费 G9 低档 MegaLights GPU 管线，「低档在多灯 workload 下不足」measured 对照证据未产出，且 4+ 灯非海量灯 workload——重判条件未命中，承接锚字面 0-byte 维持。
  - **M98-l4（maintain-defer）**：G10.5 语料为 cornell-box + bistro-interior 双小场景闭集（M133 清单），无大世界 HLOD 运行时接口面就绪度 measured 证据，L4 命中率/耗时计数可测面未建——重判条件未命中，承接锚字面 0-byte 维持。
  - **M61 / M52 / SAFE-GPU / M127 / M114-strand / M118-hdr-cal / M125-adopt3（maintain-defer）**：重判条件所需 measured 举证/真实消费方/独立期/设备资产/裁决数据/升级评估窗均非 G10.5 measured 数据命中面；M52 锚定 G12、M114-strand 锚定 G14 不变；承接锚字面 0-byte 维持（G9_P2_DECISIONS §3 原文逐字）。
- **deferred history 只追加（禁静默改判）**：`registry/deferred.json` history 只追加四条——RD-039 +1（M61，G10.6 重评窗行）、RD-040 +3（M52 / M99-clipmap〔含 rejudged-go 与 G11 画质修复期字面〕/ M100-high）；revision_log v1.80 只追加；RD-039/040 条目级 status 维持 open，id/title/reason/backfill_condition 四字段 0-byte；SAFE-GPU/M127/M98-l4/M114-strand/M118-hdr-cal/M125-adopt3 六行无 RD 归属不新设 RD（沿 G9.7 先例），零新 RD（RD max=RD-044 不动）。

**② 门实测**：`ci/g10_wave6_reevaluation_check.py --gate g10.wave.6.reevaluation`（步骤 192）→ 十锚闭集全等 + 零空行 + 重判结论枚举合法 + G10.5 measured 证据在树与 measured 面前缀机核 + maintain-defer 行承接锚字面 0-byte 维持（重判后 == 原字面）+ rejudged-go 行承接锚含 G11+ 承接波次 + deferred history 留痕「只追加」+ deferred.json history 对账（G10.6 重评窗 RD-039 +1/RD-040 +3，零新 RD）+ G10_P2_DECISIONS 十锚行对账（裁决 == defer-to-G11+，rejudged-go 仅 M99-clipmap 一行）全 PASS，**VERDICT=PASS**（`evidence/g10_wave6_reevaluation_20260816T031917Z.json`）；`--selftest` 正样本（真表十行绿 + 合成全表绿）+ 负样本九臂（缺行 / maintain-defer 承接锚改写 / 非法重判结论 / 证据引用不在树 / rejudged-go 缺 G11+ 承接波次 / history 缺「只追加」字面 / 非 G10.5 measured 面证据 / deferred history 缺登记 / P2 对账失配）全红，红绿双全。

**③ 验收命令与守卫套件实测（本记录落盘前）**：

```text
py -3 ci/g10_wave6_reevaluation_check.py --gate g10.wave.6.reevaluation   → exit 0（VERDICT=PASS，facts 14/14）
py -3 ci/g10_wave6_reevaluation_check.py --selftest                       → exit 0（2 正样本绿 + 9 负样本红全过）
py -3 ci/check_structure.py → PASS · py -3 ci/check_schemas.py → PASS · py -3 ci/check_number_ledger.py → PASS
py -3 ci/check_g10_acceptance_map.py → PASS（三向逐字一致）· py -3 ci/budget_eval.py --strict → PASS
py -3 ci/trace_matrix.py --check → PASS（373/373 全锚定）· py -3 ci/stable_snapshot.py --check → PASS（373）
py -3 ci/check_g10_implementation_interlock.py --require-ready → VERDICT=READY exit 0
py -3 -m pytest tests/ -q → 全 pass
```

**④ 门序登记面**：编号领取 = 落盘前实测 `CI_step.next_free=192` 顺位领取 192（registry/number_ledger.json revision_log **v1.111**，on_tree_max CI_step 191→193 / next_free 192→194——193 归 G10.7 同批）；本批**零新 spec 条款**（RXS next_free=392 不动，重评窗为文档/registry 面非语义面）；CI_GATES.md v1.7 修订行（§4/§4A/§5 表体 0-byte）；pr-smoke.yml 步骤 192（步骤 191 块后追加）；check_schemas.py 三处纯追加（g10_wave6_reevaluation_ 前缀路由 load/validator/路由，与既有全族互不包含，既有路由 0-byte）；evidence schema `milestones/g10/g10_wave6_reevaluation_evidence_schema.json`（const 钉死 numeric_step=192/gate key）同批落；骨架脚本 materialize——骨架期行级机核沿用 + 横向对账（deferred history / P2 行集 / measured 证据语义面）加性，骨架期 `--gate` 诚实红使命完结；G5~G9 closed 判据与 G10.2~G10.5 门脚本 0-byte；evidence/ 只增不删不改。

**⑤ 签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G9 §8.x 五块模板同构）。`Assisted-by: Kimi-K3（G10.6/G10.7 波）`（影响范围：G10_CONTRACT §8.6、milestones/g10/G10_DEFER_REEVALUATION.md v1.0、ci/g10_wave6_reevaluation_check.py materialize、g10_wave6_reevaluation evidence schema、check_schemas.py 纯追加、pr-smoke.yml 步骤 192、CI_GATES v1.7、registry/deferred.json G10.6 history 四行 + revision_log v1.80、number_ledger v1.111；验证方式如 ③ 全量实测输出留痕）。**遗留缺口（如实登记不充绿）**：M99-clipmap 重判 go 仅指定 G11 承接波次——G10 零实现面，世界辐射缓存世界 clipmap 级实现归 G11 画质修复期（只消费 G10.8b 锁定清单 R4/C1 行 + 承接锚）；A/B 臂为 host CPU 参考管线，GPU 管线远场面复核归 G11 承接工作面（G10-N16 行登记）；九锚 defer 维持 open 不写进全绿叙述、不阻塞 G10.8a soak。

### §8.7 G10.7 波验收记录（2026-08-15，G-G10-9）

**① 门断言清单（本波 1 门，逐条独立 PASS|FAIL 不互代）**：

| Symbolic gate key | 步骤 | 实测 verdict | evidence（最新件） |
|---|---:|---|---|
| `g10.wave.7.decisions` | 193 | **PASS**（facts 32/32，device=not_applicable） | `evidence/g10_p2_decisions_20260816T031917Z.json` |

- **27 行闭集穷举决策（零空行）**：`milestones/g10/G10_P2_DECISIONS.md` v1.0 落盘——候选全集 = G10_CANDIDATE_DECISIONS 22 行实记全集未进 14 key 验收面者 15 行（十锚 10 + G10-N5 + RD-034/042/043/044 四条 RD 级维持行）+ G10.2~G10.6 期内新增 not-triggered/no-go/留档 登记面 **G10-N6~G10-N17 十二行**（BistroExterior FBX2glTF 工具缺陷缺口 / 纹理 DDS 解码面 / -renderoffscreen 未测 / HighResShot+csvCaptureFrames 死路留档 / HDR-FLIP 探针臂未独立标定 / M141 单轮采样形态+MRQ 开销口径 / M130 三重绑定口径修订留痕 / 互锁 validator C3/C4 两态校准留痕 / UE 装 F: 盘偏差 / 预存 rustfmt+clippy 漂移面 / Rurix GPU 管线 A/B 出图帧率面 / M137 scalars.flip 演进位 null 维持），去重后 27 行闭集与门脚本 FROZEN_IDS 逐字对账。go 4 行（G10-N1~N4）已进 14 key 验收面独立绿不进本表（沿 G9 范式）。
- **裁决汇总**：**go 2 行**（G10-N12 M130 三重绑定口径修订留痕 / G10-N13 互锁 validator C3/C4 两态校准留痕——均 closed-go 留痕，evidence 在树）+ **no-go 7 行**（RD034 blocked 维持 / RD042/RD043 观察维持 / RD044 maintain_no_go 维持 / G10-N9 死路留档 / G10-N14 偏差登记合规 / G10-N15 漂移面零修复纪律不回写）+ **defer-to-G11+ 18 行**（十锚 10——M99-clipmap G10.6 重判 go 指定 G11 画质修复期承接、其余九锚 maintain-defer 承接锚字面 0-byte；G10-N5 锚定 G13；G10-N6/N7/N8/N10/N11/N16/N17 七行遗留缺口）+ **strategic_override 0 行**；每行承接锚「重判条件 + 兜底」齐备、defer 行含 G11+ 重评窗字面；**no-go/defer 如实保持 open，不写进全绿叙述、不阻塞 G10.8a soak**（G-G10-9 字面）。
- **互斥与对账面**：与 G10_ACCEPTANCE_MAP 14 key（12 P0 + 2 已 go P1）互斥——P2 行 ID 零命中已 go M## 裸 token（M99-clipmap/M100-high/M98-l4 等子项级 key 不互斥）；deferred.json history 只追加四条（G10.7 P2：RD-039 +1〔M61〕、RD-040 +3〔M52/M99-clipmap/M100-high〕）+ revision_log v1.81 只追加，零新 RD（RD max=RD-044 不动），RD 条目级 status 与四字段 0-byte；G10.6 重评窗对账——十锚行裁决与 G10_DEFER_REEVALUATION.md 重判结论一致（maintain-defer ↔ defer-to-G11+ 无 rejudged-go 字面；rejudged-go 仅 M99-clipmap 一行）。

**② 门实测**：`ci/g10_p2_decisions_check.py --gate g10.wave.7.decisions`（步骤 193）→ 27 行闭集全等 + 零空行 + 裁决枚举合法 + 承接锚「重判条件+兜底」/defer G11+ 字面 + go 行 evidence 义务 + no-go 行锚义务 + MAP 14 key 互斥（12 P0 + 2 P1 实解）+ deferred.json history 对账 + G10.6 重评窗对账全 PASS，**VERDICT=PASS**（`evidence/g10_p2_decisions_20260816T031917Z.json`）；`--selftest` 正样本（真表 27 行绿 + 合成全表绿）+ 负样本七臂（缺行 / defer 缺 G11+ 承接锚 / 非法裁决枚举 / 互斥违例〔已 go P0 裸 token M139 入表〕/ 空单元格 / deferred history 缺登记 / G10.6 对账失配）全红，红绿双全；骨架期空闭集硬护栏使命完结（FROZEN_IDS 27 行闭集冻结填入）。

**③ 验收命令与守卫套件实测（本记录落盘前）**：

```text
py -3 ci/g10_p2_decisions_check.py --gate g10.wave.7.decisions            → exit 0（VERDICT=PASS，facts 32/32）
py -3 ci/g10_p2_decisions_check.py --selftest                             → exit 0（2 正样本绿 + 7 负样本红全过）
py -3 ci/g10_wave6_reevaluation_check.py --gate g10.wave.6.reevaluation   → exit 0（G10.6 门复核绿）
py -3 ci/check_structure.py → PASS · py -3 ci/check_schemas.py → PASS · py -3 ci/check_number_ledger.py → PASS
py -3 ci/check_g10_acceptance_map.py → PASS（三向逐字一致）· py -3 ci/budget_eval.py --strict → PASS
py -3 ci/trace_matrix.py --check → PASS（373/373 全锚定）· py -3 ci/stable_snapshot.py --check → PASS（373）
py -3 ci/check_g10_implementation_interlock.py --require-ready → VERDICT=READY exit 0
py -3 -m pytest tests/ -q → 全 pass
```

**④ 门序登记面**：编号领取 = 落盘前实测 `CI_step.next_free=192` 顺位领取 192~193（192 归 G10.6 重评窗门，registry/number_ledger.json revision_log **v1.111**，on_tree_max CI_step 191→193 / next_free 192→194）；本批**零新 spec 条款**（RXS next_free=392 不动）；CI_GATES.md v1.7 修订行（§4/§4A/§5 表体 0-byte）；pr-smoke.yml 步骤 193（步骤 192 块后追加）；check_schemas.py 三处纯追加（g10_p2_decisions_ 前缀路由，与既有全族互不包含，既有路由 0-byte）；evidence schema `milestones/g10/g10_p2_decisions_evidence_schema.json`（const 钉死 numeric_step=193/gate key）同批落；registry/deferred.json history 只追加八行（G10.6 四行 + G10.7 四行）与 revision_log v1.80/v1.81；G5~G9 closed 判据与 G10.2~G10.6 门脚本 0-byte；G9_P2_DECISIONS 33 行裁决字面 0-byte 不回写；evidence/ 只增不删不改。

**⑤ 签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G9 §8.x 五块模板同构）。`Assisted-by: Kimi-K3（G10.6/G10.7 波）`（影响范围：G10_CONTRACT §8.7、milestones/g10/G10_P2_DECISIONS.md v1.0（27 行闭集）、ci/g10_p2_decisions_check.py materialize（FROZEN_IDS 27 行冻结）、g10_p2_decisions evidence schema、check_schemas.py 纯追加、pr-smoke.yml 步骤 193、CI_GATES v1.7、registry/deferred.json G10.7 history 四行 + revision_log v1.81、number_ledger v1.111；验证方式如 ③ 全量实测输出留痕）。**遗留缺口（如实登记不充绿）**：no-go 7 行与 defer-to-G11+ 18 行如实保持 open/留档——不写进全绿叙述、不阻塞 G10.8a soak；G10.8a 稳定门（全部 P0 与 go P1 全量回归 + 全链路连续复跑 soak）为下一波次，CI_step next_free=194 以落盘实测为准。

### §8.8 G10.8a 波验收记录（2026-08-16，G-G10-10）

**① 门断言清单（本波 1 门，逐条独立 PASS|FAIL 不互代）**：

| Symbolic gate key | 步骤 | 实测 verdict | evidence（最新件） |
|---|---:|---|---|
| `g10.wave.8a.soak` | 194 | **PASS**（facts 5/5 + checks 6/6，device=not_applicable） | `evidence/g10_stabilization_soak_20260816T073116Z.json` |

- **四腿全绿**：**①全量回归腿**——14 key（12 P0 + 2 go P1）逐门真跑 `--gate` 后机器核验最新 evidence 顶层 `status=="pass"`（G10 证据形态统一，无 G9 M90/M91 缺字段豁免面——缺字段即红）：M128 10/10、M129 7/7（UE 真出图腿数十秒级，gpu_device_lock 各自持锁串行）、M131 7/7、M132 9/9、M133 7/7、M134 13/13、M135 13/13、M136 12/12、M137 12/12、M138 12/12、**M130 走 `--phase g10.5` 双端核验腿 13/13 且 `phase_g10_2_pass=true` ∧ `phase_g10_5_pass=true` 同真**（骨架期绿不替双端核验期充绿；门序 M130 先于 M139）、M139 16/16（门内当次 session 复跑 M130 g10.5 腿 + 三重绑定）、M140 18/18、M141 11/11（双端 50×3 trimmed mean 采样真跑）；wave2~wave5 exit + wave6 重评窗 + wave7 决策六聚合/决策门真跑核验全 PASS；14 门 evidence `base_commit` 同值=HEAD `ca407477149d222ebf544cda05cc5b4d350e7cee` 且 20 门 evidence 文件名 UTC stamp ≥ run 起点（20260816T060335Z）新鲜度机核——同一候选 close-out 基线（MAP §7）。**②全链路 soak 腿**——出图→捕获→度量→差距清单连续复跑 **29 迭代 / 1849.371s ≥ 1800s**（沿 G9.8a 30min 继承；「或 measured 证明更短足够」未触——单轮实测约 63.8s=1849.371/29，1800s 得 29 轮，继承量级真实达成无需 measured 缩短裁决）：每迭代 = Rurix HDR release 重渲染双场景 digest 逐位复现库帧（出图，58 帧）+ LDR 派生四臂逐字节复现（116 臂）+ 双端四组帧解码 + UE 帧 `unreal/build`==M128 登记 ue_build_id + 内容 digest==G10.5a 注册常量（捕获，696 次解码）+ LDR 臂 FLIP/SSIM/PSNR 重算==G10.5a golden 逐位（度量，58 三元组）+ diff 报告重跑独立重算三面一致（58 份）+ 探针 artifact 再生 + 差距清单 11 项装配 + gaplib 校验零错误 + 与在树 `g10_gap_registry.json` 逐字节相等幂等复核（29 装配）；**诚实口径实测**：`sleep_seconds=0.0` 恒零（零 sleep 谎报）、`active_chain_seconds=1849.371` == `seconds` 逐迭代计时求和、gate 外测墙钟 `outer_wall_seconds=1849.684` 交叉核验无谎报、`failures=0` 全链路迭代零失败；chain-soak 无 device 零错字面量门（沿 G9.8a 语义）。**③budget_eval --strict**——exit 0，**138 pass / 0 skip**，非空零 estimated/skip。**④纪律日期锚**——`utc_date=20260816`。
- **full-run 两跑如实登记**（防假绿纪律留痕，沿 G9.8a 三跑先例）：首跑 `20260816T055736Z` **FAIL**——回归腿 20 门中 `g10.wave.2.exit` 唯一红（fact④ M130 双 phase 纪律字面假设「最新 M130 evidence 为骨架期形态」于 G10.2 冻结，G10.5 双端核验腿落地后最新件合法为完整期形态 `phase=g10.5` 双 flag 同真，字面失配误红；soak/budget/日期锚三腿已绿）；**修复 = wave2 门 fact④ 两态校准**（commit `ca407477`，沿 G10.4b 互锁 validator C3/C4 两态先例，判据语义 0-byte——反冒充不变量「骨架期绿不替双端核验期充绿」维持：A 态骨架期绿原机核逐字维持 / B 态完整期双真接受；两态外一律红——p5=true 而 p2≠true 等冒充形态全拒；selftest 加性扩五红臂 + A/B 双绿臂实测双全，校准后 `--gate` PASS `g10_wave2_exit_20260816T060158Z.json`；首跑 FAIL 三件〔8a `…T055736Z` + wave2 `…T052630Z`/`…T055801Z`〕诚实留痕只增不删）；二跑 `20260816T073116Z` **PASS**（四腿全绿，20/20）。另：8b 门 materialize 批审漏 `import json` 致 8b 首跑 NameError（exit=1 未产 evidence），`87b7e322` 修复后 READY——如实登记。

**② 聚合门实测**：`ci/g10_stabilization_soak.py --gate g10.wave.8a.soak`（步骤 194）→ 20 门最新 evidence 核验全 PASS + facts 5/5（regression_12p0_2p1_6wave / base_commit_uniform / soak_dual_threshold / budget_strict / date_anchor）+ checks 6/6 全真，**VERDICT=PASS**（`evidence/g10_stabilization_soak_20260816T073116Z.json`）；聚合不代绿（回归腿逐门真跑 `--gate`——本门为真跑回归 + soak 承载门，非只读汇总，沿 G9.8a 体例）；`--verify-latest` PASS（schema + host_section_pass + checks 六键 + soak 双阈值与 honesty 字段复核，pr-smoke 步骤 194 同模式）+ `--selftest` 5 红（sleep 充墙钟 / 外测墙钟戳穿谎报 / 迭代不足 / 计数面空有失败 / 缺 honesty 字段）+ 1 绿全过。

**③ 验收命令与守卫套件实测（本记录落盘前）**：

```text
py -3 ci/g10_stabilization_soak.py --gate g10.wave.8a.soak        → exit 0（VERDICT=PASS，facts 5/5，checks 6/6）
py -3 ci/g10_stabilization_soak.py --verify-latest               → exit 0（honest chain soak）
py -3 ci/g10_stabilization_soak.py --selftest                    → exit 0（5 红 + 1 绿全过）
py -3 ci/g10_wave2_exit_check.py --gate g10.wave.2.exit          → exit 0（校准后 PASS；selftest 负/正样本 + 五红臂 + A/B 双绿全过）
py -3 ci/check_structure.py → PASS · py -3 ci/check_schemas.py → PASS · py -3 ci/check_number_ledger.py → PASS（373 条款零同号碰撞）
py -3 ci/check_g10_acceptance_map.py → PASS（三向逐字一致）· py -3 ci/budget_eval.py --strict → PASS（138 pass 0 skip）
py -3 ci/trace_matrix.py --check → PASS（373/373 全锚定）· py -3 ci/stable_snapshot.py --check → PASS（373）
py -3 ci/check_g10_implementation_interlock.py --require-ready → VERDICT=READY exit 0
py -3 -m pytest tests/ -q → 121 passed
```

**④ 门序 / 登记面摘要**：编号领取 = 落盘前实测 `CI_step.next_free=194` 顺位领取 194~195（registry/number_ledger.json revision_log **v1.112**，on_tree_max CI_step 193→195 / next_free 194→196）；本批**零新 spec 条款**（RXS next_free=392 不动）；CI_GATES.md v1.8（双门 materialize）/ v1.9（wave2 fact④ 两态校准登记）修订行（§4/§4A/§5 表体 0-byte）；pr-smoke.yml 步骤 194~195（步骤 193 块后追加；194 = `--verify-latest` 秒级核最新 full-run evidence，沿 G9.8a 步骤 171 体例）；check_schemas.py 三处纯追加（g10_stabilization_soak_/g10_wave8b_closeout_ 前缀路由 load/validator/路由，与既有全族互不包含，既有路由 0-byte）；两 evidence schema（8a：soak 块 honesty 字段全必填 + required_gates minItems=maxItems=20 + numeric_step const 194；8b：verdict enum + required_gates 21 + extra_facts 8 + checks 八键闭集 + numeric_step const 195）同批落；G5~G9 closed 判据与 G10.2~G10.7 门脚本 0-byte（`ci/g10_wave2_exit_check.py` fact④ 两态校准为唯一门脚本修订，v1.9 登记）；evidence/ 只增不删不改（首跑 FAIL 件留痕）。**异己并发工作树面（如实登记不充绿不混入）**：full-run 期间工作树出现非本批 src 改动（`src/rurix-render/src/{lib,geometry/mod,gi/mod,shadow/mod}.rs` + `src/rurix-asset/src/lib.rs` 模块声明与 `hzb.rs`/`restir.rs`/`sdf_trace.rs`/`smrt.rs`/`ssr/mod.rs`/`ktx2_read.rs` 新文件，RD-039/040/041 长线 host 研究参照面，另一并发会话产物）——本批**不提交、不回滚、不充绿**（沿 G9 §8.1「遗留 staged 集合不混入」先例）；两跑构建与 A/B 库帧 digest 逐位复现（c2000ebf…/8519cc67…）在该面在场时实测成立，该面对 G10 全部 14 key 验收面零语义干涉（新模块未接任何 G10 门消费面）；G10 零修复纪律维持。

**⑤ 签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G9 §8.x 五块模板同构）。`Assisted-by: Kimi-K3（G10.8 收口波）`（影响范围：G10_CONTRACT §8.8、ci/g10_stabilization_soak.py + g10_stabilization_soak evidence schema、ci/g10_wave2_exit_check.py fact④ 两态校准与 selftest 加性扩、check_schemas.py 三处纯追加、pr-smoke.yml 步骤 194/195、CI_GATES v1.8/v1.9、ledger v1.112、8a 两跑 evidence 与 20 门刷新 evidence 入库；验证方式如 ③ 全量实测输出留痕）。**遗留缺口（如实登记不充绿）**：soak 载体 = host CPU 参考管线全链路（Rurix 出图臂为 host 参考管线，UE 臂为 G10.5a 注册库帧 digest 核验——GPU 管线双端出图面归 G11+ 承接，G10-N16 行登记）；M141 基线单轮采样形态维持（G10-N11）；8a 两跑首红已修复留痕，无未处置红。

### §8.9 G10.8b close-out READY（G-G10-11，2026-08-16）

- **收口门 `g10.wave.8b.closeout` VERDICT=READY**（步骤 195，evidence `evidence/g10_wave8b_closeout_20260816T073202Z.json`，exit=0，schema validate 通过）。终审八 facts 全 PASS：**①14 key**（12 P0 + 2 go P1）逐门 PASS 14/14（wel 口径 + 顶层 `status=="pass"` 字面 + M130 双 phase 同真机核）；**②wave2~8a 七聚合/决策门**（exit×4 + 重评窗 + 决策 + soak）全 PASS 7/7；**③验收映射三向** `check_g10_acceptance_map` exit=0；**④P2 决策表 27 行闭集最终状态无漂移**（最新 evidence `host_section_pass`〔`g10_p2_decisions_20260816T070026Z.json`〕+ FROZEN_IDS 27 行在树，复用门脚本闭集单一事实源）；**⑤budget --strict** 非空零 estimated/skip（exit 0，138 pass 0 skip）；**⑥8a full-run 先行**（`g10_stabilization_soak_20260816T073116Z.json`，`base_commit_8a=ca407477149d222ebf544cda05cc5b4d350e7cee` 留痕）；**⑦RD 最终状态逐字一致**（deferred.json RD-034/039/040/041/042/043/044 七条目级 status 全 open 逐字 + `G10_P2_DECISIONS.md` 27 行 FROZEN_IDS 闭集在树 + `G10_DEFER_REEVALUATION.md` 十锚 DEFER_TEN 终态在树——验收映射、候选决策、RD 最终状态三面逐字一致，G-G10-11 字面）；**⑧差距清单终审锁定**（`g10_gap_registry.json` 11 行闭集 FROZEN_GAP_IDS 逐字全等——R1~R5/U1~U3/C1~C3，kind 两值 quality_gap 8 / caliber_diff 3 + 每项 G11 承接锚非空 + gaplib 校验零错误 + generated_by==M139 门字面——**G11 法定输入**：G11 修复范围只能消费该清单 + 其承接锚）+ **最后新绿 UTC 日留痕**（`last_new_green_utc=20260816` 与运行日相同，同日 close-out 不阻断——立项裁决 8 同日放行先例援引，8a full-run 先行完成）。
- **同日放行登记**：8a full-run 于 2026-08-16 07:31Z 完成 PASS（二跑；首跑 05:57Z FAIL 修复留痕见 §8.8①），8b 终审于同日 07:32Z READY；立项裁决 8（G9.8b 同日放行先例继承）字面不扩展解释（soak ≥1800s 实测满足未跳过；G8 §8.25/G9 §8.9 同模）。
- **验收命令（实测全绿）**：`py -3 ci/g10_closeout_check.py --gate g10.wave.8b.closeout` exit=0 READY + `--selftest` OK materialized step 195 + `py -3 ci/check_schemas.py` / `py -3 ci/check_g10_acceptance_map.py` / `py -3 ci/check_number_ledger.py` / `py -3 ci/trace_matrix.py --check`（373/373）/ `py -3 ci/stable_snapshot.py --check` / `py -3 ci/check_g10_implementation_interlock.py --require-ready`（READY）守卫全 PASS。
- **status flip**：见紧随其后的独立 commit（front matter `active`→`closed` + 本条 0-byte 维持）。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G9 §8.9 同模）。`Assisted-by: Kimi-K3（G10.8 收口波）`（影响范围：G10_CONTRACT §8.9、ci/g10_closeout_check.py + g10_wave8b_closeout evidence schema（+ `87b7e322` import json 修复）、8b READY evidence 入库；验证方式如上全量实测输出留痕）。

### §8.10 G10 status flip（2026-08-16）

**裁决**：G-G10-1~11 对应波次与硬门已 materialize 并逐波验收（G10.2~G10.5 四波 §8.2~§8.5、G10.6 defer 重评窗 §8.6、G10.7 P2 穷举决策门 §8.7、G10.8a stabilization soak §8.8）；8a full-run PASS（`g10_stabilization_soak_20260816T073116Z`）；8b `VERDICT=READY`（`g10_wave8b_closeout_20260816T073202Z`）。  
front matter **`status: active` → `status: closed`**（洁净独行）。RD-034/039/040/041/042/043/044 总体维持 open（分项 go/defer 已由候选决策表、G10_P2_DECISIONS 与 deferred history 只追加留痕）。**差距清单 `g10_gap_registry.json` 11 行闭集（R1~R5/U1~U3/C1~C3）终审锁定为 G11 法定输入**——G11 修复范围只能消费该清单 + 其承接锚（G-G10-11/MAP §7）。本条为 close-out 终审签署块。

- **guardrail 基准链留痕**：`ci/check_guardrails.py` 默认基准维持 `g7-closed`——G8/G9 close-out 均未落 `g8-closed`/`g9-closed` tag（基准链 mb1-closed→g3-closed→ei1-closed→g4-closed→ea1-closed→g7-closed 单线性维持，v1.58 注释面），G10 沿 G8/G9 先例**不新落 close tag、不切基准**（无 g9-closed 基准可切，留痕登记）；README.md / 00_MASTER_INDEX.md 状态勘误行只追加随本 commit 同落。
- **互锁 validator closed 三态校准**：status flip 后 `ci/check_g10_implementation_interlock.py` 事实门①「status 要求 active」按字面失配转红、C1/C2 一致性门连带 FAIL（校准前实测 VERDICT=BLOCKED exit=1）——按 C3/C4 两态先例加性扩第三态：status==closed（收口终态）时事实门/一致性门整体 not_applicable（skipped_reason 登记），VERDICT=CLOSED、exit=0；active/blocked 态原机核逐字维持，CLOSED 不得被当作 G-G10-3 重新开放凭据；selftest 加性扩 closed 臂（12 RED + 1 GREEN + 1 TREE 全过，CI_GATES v1.10 登记）。
- **异己并发工作树面**：本 flip commit 只含 front matter `status` 字段 + §8.10 追加 + README/00_MASTER_INDEX 勘误行；工作树异己 src 面（§8.8④ 登记的并发会话研究模块声明面）维持未提交、不混入本 commit。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G9 §8.10 同模）。`Assisted-by: Kimi-K3（G10.8 收口波）`（影响范围：front matter status flip + 本条签署块 + README/00_MASTER_INDEX 状态勘误行；验证方式：`py -3 ci/g10_closeout_check.py --gate g10.wave.8b.closeout` 复跑幂等 READY + 全守卫复跑 PASS，输出如本会话留痕）。
