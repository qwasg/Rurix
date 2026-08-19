---
contract: G13
title: G13 超分采样与 Lumen 对照期
status: active
implementation_status: unlocked
active_scope: g13_1_governance_only
version: v1.0
date: 2026-08-18
timebox: "G13.1 治理波即刻执行（G12 已 closed）；G13.2~G13.5b 严格波次，工期在实现互锁开放后由 measured baseline 校准"
rfc_required: "G13.1 治理波零 RFC 消费——本波只落治理资产，RFC 命名空间 0-byte（实测 next_free=30 维持）。G13.2+ 实现波若触冻结面（UpscaleBackend trait 签名面 / temporal 底座历史接口面 / RXS-0357 参照器面 / M137 scalars.flip 演进位翻转）必须独立 Full RFC 经 D-409 对抗性评审后 Agent Approved，编号按起草时实测 registry/number_ledger.json namespaces.RFC next_free 领取，禁推测号；判档争议向上取严（10 §3）"
upstream_docs:
  - "milestones/g12/G12_CONTRACT.md §8.8（G12 closed 终态，2026-08-17，flip commit 8c5dc5ee + tag g12-closed；生产化差距清单 g12_ue_pt_gap_registry.json 10 行终态终审锁定 = G13 法定输入）"
  - "milestones/g12/G12_P2_DECISIONS.md v1.0（33 行闭集；defer-to-G13+ 22 行承接锚 = G13 法定输入，本契约 §7 候选决策逐行承接）"
  - "milestones/g12/G12_CANDIDATE_DECISIONS.md v1.0（37 行裁决全集写法范式 + G10-N5/RD-041 超分承接锚原文）"
  - "milestones/g12/g12_ue_pt_gap_registry.json（UE PT 对标差距登记表 10 行终态：quality_gap 6 + caliber_diff 4——G13 只消费不回写）"
  - "G13 立项前调研报告（2026-08-18 主会话留痕：范围定盘 = DLSS/FSR 超分采样集成 + 自研 TSR device 化 + UE5 超分双端对拍 + UE Lumen GI 对照；P0 建议清单 5 行 M-a~M-e；技术事实面 Streamline SDK 2.10.3 / FSR 3.1.5 / 本机 RTX 4070 Ti / Vulkan RayQuery 主腿）"
  - "src/rurix-render/src/temporal/upscale.rs（UpscaleBackend trait 冻结接口全文 217 行，RFC-0016 §4.0-3 照抄面——三实现位预留：自研 TSR / FSR 3.1 / DirectSR-vendor 留口，历史内置双缓冲）"
  - "rfcs/0016-native-renderer.md（§4.H3 UpscaleBackend 冻结语义面 + §9 Q-F vendor SDK 留口不接裁决——『UpscaleBackend trait 与降噪输入契约（MV/深度/法线同构）留口先行，接入时不改底座』字面）"
  - "rfcs/0026-visual-comparison-metrics.md（SSIM/FLIP 度量口径冻结面）+ rfcs/0027-external-reference-harness-license.md（外部参照 harness 与许可边界——vendor SDK 许可白名单纪律同源）"
  - "registry/deferred.json RD-034/039/040/041/042/043/044（存续 open RD；只追加禁静默改判；G10-N5 = DLSS/Streamline 锚定 G13 行、RD-041 FSR/DirectSR 分项、RD040-nrd G13+ 接入裁决窗字面）"
  - "milestones/g12/design/nrd_vendor_denoise_evaluation.md v1.0（G12.3 NRD 许可/ABI 取证先例——owner 法律面审为接入硬前置口径来源）"
  - "spec/global_illumination.md RXS-0357（M96 参照器冻结面：固定 seed 确定性协议——M165 漂移监控锚定面 RXS-0357 L2/RXS-0400）"
  - "04 P-01/P-07/P-09/P-12/P-13；10 §3/§7/§9.5；14 §1/§3/§4/§5（同 G12 口径）"
implementation_unlock:
  required_all:
    - "G13.1 治理门全部完成且有真实验证记录"
    - "ci/g13_interlock_check.py --require-ready 输出 READY（互锁 validator 机器事实，不以叙述替代）"
    - "M-a 许可前置：DLSS/Streamline 与 FSR  redistribution/集成许可 owner 法律面清结留痕——未清结 M-a 保持 blocked 且 G13.2 不得开工"
    - "共享编号按互锁开放时 actual next_free 重新校准；数字 CI 步骤不得沿用推测号与草案建议值"
in_scope:
  - g13_1_governance_only
  - candidate_decisions_and_rd_mapping
  - p0_acceptance_mapping
  - g13_governance_three_gates_materialize
  - g13_2_vendor_upscale_integration_wave
  - g13_3_tsr_device_wave
  - g13_4_ue_dual_capture_wave
  - g13_5_p2_exhaustive_decisions_and_closeout
out_of_scope:
  - g13_2_plus_while_implementation_interlock_is_red
  - g13_1_src_spec_conformance_semantic_implementation
  - g13_1_vendor_sdk_vendoring_or_wiring
  - g13_1_full_rfc_consumption
  - absolute_dlss_upscale_quality_pass_line_deferred_to_g15
  - formal_fps_parity_and_pass_line_deferred_to_g14
  - gpu_pipeline_dual_ab_surface_deferred_to_g14
  - frame_generation_fg_mfg_independent_layer
  - dlss_rr_ray_reconstruction_out_of_scope
  - nrd_vendor_denoiser_wiring_remains_evaluation_only
  - temporal_base_rewire
  - m96_reference_frozen_surface_rewrite
  - ue_source_or_binary_vendoring_into_rurix_repo
  - foreign_uncommitted_src_surface_consumption
  - speculative_number_consumption
  - rewriting_g5_to_g12_closed_contracts_and_00_14
deferred_refs: [RD-034, RD-039, RD-040, RD-041, RD-042, RD-043, RD-044]
deliverables:
  - id: D-G13-1
    name: "G13.1 治理三件套：G13_CONTRACT、G13_ACCEPTANCE_MAP、G13_CANDIDATE_DECISIONS；status=active 且 implementation_status=blocked"
  - id: D-G13-2
    name: "G13.1 完整候选决策表：G12 defer-to-G13+ 22 行 + 存续 open RD + G13 新增候选（DLSS/FSR/TSR/Lumen 分项）逐行映射（go / no-go / defer-to-G14+ / strategic_override + 承接锚）；缺行阻断 G13.2"
  - id: D-G13-3
    name: "G13.1 验收映射：全部 P0 各有独立 symbolic gate key、稳定脚本名、evidence schema 目标路径与判据；go 的 P1 同步覆盖"
  - id: D-G13-4
    name: "G13.1 治理三门 materialize：g13_acceptance_map_check / g13_candidate_decisions_check / g13_interlock_check（--gate + --selftest）+ 三 evidence schema 落盘 + workflow 步骤 233~235 按 actual next_free 领取——implementation interlock 当前诚实报告 BLOCKED"
acceptance_gates:
  - id: G-G13-1
    check: "治理激活门：用户 2026-08-15「/goal G10~G15 六期分期 + 全期自主推进」指令留痕（G13.1 立项与 G13.2+ 开工授权同源——指令含「支持 dlss、超分采样」字面）+ 2026-08-18 G13 立项前调研报告与 G13.1 治理波任务下达留痕；agent 依 10 §7/P-13/D-406 v2.0 完全自主签署立项裁决留痕；十项立项裁决全部落定；G13.0 不可变 ref=8c5dc5ee 登记；仅 governance-only 范围 active"
  - id: G-G13-2
    check: "G13.1 完成门：D-G13-1~4 齐备并通过结构/schema/ledger/guardrail 核验；验收映射无缺行；无 src/spec/conformance 语义实现、无 vendor SDK vendoring/接线、零 RFC 消费；本门通过不自动开放实现"
  - id: G-G13-3
    check: "实现互锁门：ci/g13_interlock_check.py --require-ready 输出 READY + 用户 G13.2 开工指令留痕（2026-08-15 指令全期授权面「支持 dlss、超分采样」字面）+ M-a 许可前置 owner 法律面清结留痕 + 共享编号按 actual next_free 重新校准。任一条件不满足均保持 implementation_status=blocked"
  - id: G-G13-4
    check: "G13.2 退出门：M-a P0 独立断言全绿——DLSS SR 经 Streamline SDK 真跑 + FSR 3.1.5 同接口档（经 UpscaleBackend 冻结面接入，底座 0-byte）+ 双端超分帧对拍 measured 登记；许可前置未清结则 M-a 保持 blocked 不充绿"
  - id: G-G13-5
    check: "G13.3 退出门：M-b P0 独立断言全绿——自研 TSR host 金标准（tsr.rs）→ .rx kernel device 面（复用 G12 PT megakernel 车道）+ 50/67/100% 三档质量/帧时 measured 对照入 budget（零 estimated）"
  - id: G-G13-6
    check: "G13.4 退出门：M-c/M-d 两个 P0 独立断言全绿——UE5 超分双端对拍（复用 G12.4 MRQ harness 扩 DLSS 臂，SSIM/FLIP/噪声谱差距登记 + 帧率 measured 基线登记 zero_pass_line 不设通过线）+ UE Lumen GI 对照（Rurix M98/M99/M154 GPU GI 面 vs UE Lumen 同场景双端 + Lumen 差距登记表落盘）"
  - id: G-G13-7
    check: "G13.5a 决策门：G13 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G14+，零空行；defer 必有承接锚；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述"
  - id: G-G13-8
    check: "G13.5a 稳定门：全部 P0 与所有 go 的 P1 全量回归；G5~G12 既有判据 0-byte；超分链路连续复跑 soak（量级沿 G12.7a 继承〔≥1800s〕或 measured 证明更短足够）；strict budget 非空、零 estimated/skip；既有 71 门（G9 34 + G10 14 + G11 14 + G12 9）零降级"
  - id: G-G13-9
    check: "G13.5b 收口门：验收映射、候选决策、RD 最终状态逐字一致；全部 P0 独立断言均 PASS；evidence/schema/预算终审；Lumen/超分差距清单终审锁定（残余差距/未闭环行如实登记不冒充全闭环）；§8 只追加后 status active→closed"
guardrails:
  - "双状态不可混同：status=active 仅表示 G13.1 governance-only 已立项；在 G-G13-3 真实通过前 implementation_status=blocked，任何治理完成叙述不得冒充 G13.2 开工"
  - "G13.1 允许 milestones/g13、G13 专属治理三门（ci/g13_*_check.py + evidence schema + workflow 步骤 233~235）、G13 专属 claim、deferred history 只追加；src/spec/conformance 0-byte、vendor SDK 零 vendoring 零接线、RFC 命名空间 0-byte"
  - "G13 P0 实现门 CI 只冻结 symbolic gate key 与脚本名；numeric_step 一律写 post-interlock actual-next-free allocation。不得沿用推测号与草案建议值，不得预放空 workflow、空脚本或空 schema 壳（G13.1 治理三门为例外：本波即落盘真脚本真步骤 233~235）"
  - "每个 P0 必须独立布尔断言与独立 evidence subject；可共享一次进程执行，但聚合 PASS 不能遮蔽任一子断言 FAIL/SKIP"
  - "缺硬件/工具链仅可 dev_env_degrade 或 SKIP=not-triggered；两者均不充 P0 绿。host oracle、mock、isolated nonzero、既有最小见证、人工截图均不能替代目标门"
  - "M-a 许可前置条款：DLSS（Streamline SDK 2.10.3 开源框架 + NGX 签名专有 DLL）与 FSR 3.1.5（MIT）redistribution/集成许可的 owner 法律面清结为开工硬门（G12.3 NRD 许可取证先例同模）；未清结即 M-a blocked、G13.2 不得开工，不得以 FSR MIT 面宽松冒充 DLSS NGX 面清结"
  - "M-c 帧率面纪律：帧率 measured 基线登记 zero_pass_line，不设通过线——正式帧率对标锚定 G14（G10-N11/N16 承接锚字面 0-byte）；以基线冒充帧率对标即 RED"
  - "超分/对照范围唯一法定来源：G13 立项前调研报告 P0 清单 + 本契约候选决策表行集 + G12 法定输入（g12_ue_pt_gap_registry.json 10 行 + G12_P2_DECISIONS 22 行承接锚）；G13 不得无锚新立项；新发现差距进差距登记显式登记 + G13.5a 穷举，不得静默混入"
  - "G12 gap registry 只消费不回写：g12_ue_pt_gap_registry.json 10 行终态 0-byte；G13 超分/Lumen 对照新产差距另立新表（milestones/g13/ 新文件），不回写 G12 表"
  - "M165 漂移监控登记条款：G12-N13 间歇非确定性事件（1/~1760 帧 digest 单帧漂移未定位）诊断臂沿用——G13 复跑面（M163/M164 同族复跑与 G13 新门消费 PT 生产化链路时）检出同型 digest 漂移即如实登记并升级评估（升级 = 生产化缺陷修复项 + Full RFC 评估语言/运行时面）；零检出维持 open-defer 不写进全绿叙述"
  - "既有 71 门零降级：G9 34 key + G10 14 key + G11 14 key + G12 9 key 绿面 0-byte；G5~G12 closed 契约与判据 0-byte；回归门独立 P0 断言（M-e）；M96 golden 门序机器阻断（D2-Q7）维持"
  - "UpscaleBackend/temporal 底座 0-byte 不接线（RD-041/RD040-nrd 承接锚口径）：G13 超分接入经 UpscaleBackend 冻结接口面（三实现位预留位），trait 签名与 temporal 底座历史接口面 0-byte；确需演进必须独立 Full RFC 显式修订行"
  - "UE 源码仅外部参照只读：Lumen/PathTracing/TemoralUpscaler 相关源码只读可参照（F:\\UE_5.8 与 E:\\Kimi_Agent_Taichi Engine 优化计划\\references\\UnrealEngine 双树），零 vendoring、零片段复制进 src/spec；违反即 revert + 留痕（RFC-0027 字面）"
  - "主腿 = Vulkan RayQuery（M96 device 面）；DXIL RT blocked 维持（RD-034）；DLSS Vulkan interop 面走 Streamline Vulkan 臂，不引 DXIL 依赖"
  - "异己并发工作树面不混入零消费：G13 车道 commit 只含 G13 车道文件；立项时工作树异己会话 src/ 未提交面（apps/、src/rurix-asset、src/rurix-render geometry/gi/shadow/lib/bin 声明面、src/rurix-rt/render_exec.rs 改写面、evidence/d3d12_interop_smoke.json 与 milestones/g12/g12_pt_sampler_selection.json 改写面等）严禁消费/混入（G10.8b §8.10/G11/G12 先例同模）"
  - "新 unsafe 仅在实现互锁开放后按 actual next_free 登记并附 SAFETY；rurix-render 维持 forbid(unsafe_code)"
  - "触 G5~G12 冻结面必须 RFC 显式修订行，禁静默扩；G5~G12 closed 契约与 00-14 0-byte，close-out 证据只追加"
  - "g13_budget 首个实现 PR 前必须非空 measured_local 且有 evaluator；全程零 estimated；性能数字不替代 correctness gate；阈值全部实测标定禁手写"
  - "新文件 LF + 尾换行；本契约合入后正文冻结，激活/验收/收口只追加 §8，除最终 status flip 外不回写既有事实"
---

# G13 契约 — 超分采样与 Lumen 对照期

> 候选决策：[G13_CANDIDATE_DECISIONS.md](G13_CANDIDATE_DECISIONS.md) · 验收映射：[G13_ACCEPTANCE_MAP.md](G13_ACCEPTANCE_MAP.md)（G13 治理三件套无独立 PLAN/CI_GATES——P0 建议清单与波次草案事实源 = 2026-08-18 G13 立项前调研报告〔主会话留痕〕，门冻结面 = 本契约 §4.2 + MAP §1/§2 双向机核，沿 G12 体例精简）。
> 当前裁决：**G13.1 governance-only active；G13.2~G13.5b implementation blocked**。`active` 不是实现门绿灯。

---

## 1. 目标与双门状态

G13 是**超分采样与 Lumen 对照期**：兑现用户硬需求「支持 dlss、超分采样」——主线 = **M-a vendor 超分接入**（DLSS SR 经 Streamline SDK 2.10.3 + FSR 3.1.5 同接口档，经 UpscaleBackend 冻结面〔RFC-0016 §4.0-3，底座 0-byte〕）→ **M-b 自研 TSR device 化**（tsr.rs host 金标准 → .rx kernel device 面，复用 G12 PT megakernel 车道，50/67/100% 三档质量/帧时对照）→ **M-c UE5 超分双端对拍**（复用 G12.4 MRQ harness 扩 DLSS 臂，SSIM/FLIP/噪声谱差距登记 + 帧率 measured 基线登记 zero_pass_line）→ **M-d UE Lumen GI 对照**（Rurix M98/M99/M154 GPU GI 面 vs UE Lumen 同场景双端，Lumen 差距登记表落盘——G10-N16 系唯一未闭环 UE 模块级对照面）→ **M-e 回归门 + G13 P2 穷举**（既有 71 门零降级 + M165 漂移监控）。「UE5 级」可核对基线沿用 G9~G12 口径 = UE 5.8；**G13 不设绝对「已达 UE5 DLSS/超分画质」通过线**——绝对判定归 G15 商用收口期；正式帧率对标归 G14（G10-N11/N16 承接锚字面 0-byte）。

本契约拆分两种状态：

| 状态 | 当前值 | 含义 |
|---|---|---|
| `status` | `active` | G13.1 治理波已获授权，可落治理资产（契约/候选决策表/验收映射）、G13 专属治理三门（真脚本 + evidence schema + workflow 步骤 233~235）、G13 专属 claim、deferred history 只追加 |
| `implementation_status` | `blocked` | G13.2+ 尚未获准；当前不得改 `src/`、`spec/`、`conformance/`，不得 vendoring/接线 vendor SDK，零 RFC 消费 |

G-G13-3 是唯一实现入口：互锁 validator（`ci/g13_interlock_check.py --require-ready`）输出 READY + 用户 G13.2 开工指令留痕 + **M-a 许可前置 owner 法律面清结留痕** + 共享编号按 actual `next_free` 重新校准，四者齐备方可解锁；任一缺失均保持 `blocked`。

## 2. 范围与严格波次

### 2.1 G13.1 governance-only

G13.1 只做 D-G13-1~4。允许治理文档、候选决策表、验收映射、G13 专属治理三门（`ci/g13_acceptance_map_check.py` / `ci/g13_candidate_decisions_check.py` / `ci/g13_interlock_check.py` + 三 evidence schema + workflow 步骤 233~235 按 actual next_free 领取）、G13 专属无冲突 claim、互锁 validator、deferred history 只追加；禁止语义实现、vendor SDK vendoring/接线与 RFC 消费。interlock validator 在当前事实下应明确返回 `BLOCKED`，这正是正确结果，不是失败需要被绕开。

### 2.2 G13.2~G13.5b implementation

实现互锁开放后按以下顺序推进，波次内可蜂群并行，波次间不得越级；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑：

```text
G13.2 vendor 超分接入波（M-a：DLSS SR 经 Streamline SDK 真跑 + FSR 3.1.5 同接口档；许可前置硬门）
  → G13.3 自研 TSR device 化波（M-b：tsr.rs → .rx kernel device 面 + 50/67/100% 三档对照）
  → G13.4 UE 对拍波（M-c UE5 超分双端对拍 + M-d UE Lumen GI 对照）
  → G13.5a P2 穷举决策 + stabilization/soak → G13.5b close-out
```

每波退出门见 YAML `acceptance_gates`（G-G13-4~6）；任一上游门未绿，下游 evidence 即使局部成功也不能宣称波次完成。单点依赖：M-a 许可前置是 G13.2 的硬前置（未清结即 blocked）；M-c 依赖 M-a/M-b 的超分产出面；M-d 依赖 G11 GPU GI 面（M98/M99/M154 已验收）只消费不改写。

## 3. G13.1 交付冻结

| ID | 交付 | 退出判据 |
|---|---|---|
| D-G13-1 | 治理三件套与双状态 | CONTRACT、ACCEPTANCE_MAP、CANDIDATE_DECISIONS 一致；`status=active`、`implementation_status=blocked` |
| D-G13-2 | 候选决策与 RD 总映射 | G12 defer 22 行 + 存续 open RD + G13 新增候选逐行；裁决、波次、承接锚、最终状态无空项；缺行阻断 G13.2 |
| D-G13-3 | 验收映射 | 全部 P0 全部有独立 key/script/schema 目标路径/check；go 的 P1 同步入表；不存在"由邻项代绿"；缺行阻断 G13.2 |
| D-G13-4 | 治理三门 materialize | 三脚本 --gate PASS + --selftest 红绿留痕；三 evidence schema 落盘且 check_schemas 路由注册；workflow 步骤 233~235 按落盘前实测 actual next_free=233 顺位领取；interlock validator 对当前状态诚实报 BLOCKED；无空 workflow、无空 schema 壳 |

G13.1 完成仅关闭治理准备，不改变 G-G13-3 的机器事实。

## 4. 验收门与 P0 独立断言

### 4.1 波次验收门

G-G13-1~9 以 YAML 头为可提取摘要。治理三门（g13.wave.1.acceptance_map / g13.wave.1.candidate_decisions / g13.gov.implementation_interlock）的脚本与 evidence 形态由本契约 §4.3 冻结。条件型分项的 `SKIP=not-triggered` 只表示决策已记录，不是成功；设备门的 `dev_env_degrade` 只表示环境缺失，也不是成功。

### 4.2 P0 独立断言

以下 5 行是 close-out 不可合并、不可删减的独立布尔断言（key 命名空间双方逐字一致〔本契约 §4.2 ↔ G13_ACCEPTANCE_MAP §1〕，冻结）。一次 smoke 可以共享启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。evidence schema 目标路径统一为 `milestones/g13/g13_m<###>_<slug>_evidence_schema.json`——本契约只冻结路径，不预建文件。硬判据按调研报告 §6 草案精化为可机器求值形式，负例 RED 臂要求逐行写明。**M 行号占位**：M-a~M-e 的 M### 数字在 G13.2+ 实现波 materialize 时按落盘前实测 M 命名空间实际顺位领取（沿 M158~M166 先例），本表以字母行号为治理期稳定身份。

| Symbolic gate key | M 行 | 最晚波次 | 稳定脚本名 | 独立硬判据 |
|---|---:|---|---|---|
| `g13.p0.m_a.vendor_upscale_integration` | M-a | G13.2 | `ci/g13_vendor_upscale_integration_smoke.py` | vendor 超分接入：许可前置 owner 法律面清结留痕（未清结即 blocked 不充绿）+ DLSS SR 经 Streamline SDK（2.10.3 + NGX 签名 DLL，Vulkan interop 臂）真跑出帧（RURIX_REQUIRE_REAL=1 + validation 零错误，RTX 4070 Ti）+ FSR 3.1.5 同接口档（同一 UpscaleBackend 冻结面，FSR4 ML 自动回退登记）+ 双端超分帧对拍 measured 登记（vs 自研 TSR host 金标准同输入帧集，SSIM/逐像素 diff 口径 RXS-0387/0388 继承）+ UpscaleBackend trait 签名面与 temporal 底座 0-byte 机核（目录级 diff）+ 树内零 UE/vendor 源码 vendoring；许可未清结开工即 RED；底座接线即 RED；mock/stub 充真跑即 RED；单 vendor 缺臂聚合 PASS 即 RED |
| `g13.p0.m_b.tsr_device_kernel` | M-b | G13.3 | `ci/g13_tsr_device_kernel_smoke.py` | 自研 TSR device 化：tsr.rs host 金标准 → .rx kernel device 面（复用 G12 PT megakernel 车道，rurixc --target vulkan 产 SPV + spirv-val 通过）+ device vs host 金标准同输入逐帧对拍（容差标定程序产禁手写）+ 50/67/100% 三档质量/帧时 measured 对照入 g13_budget（measured_local 零 estimated，50×3 trimmed mean 协议沿 M141/M165 字面）+ 固定 seed 位级确定性协议维持 + host 金标准面 0-byte；host/device 对拍超容差静默即 RED；estimated 冒充 measured 即 RED；确定性协议漂移即 RED |
| `g13.p0.m_c.ue_upscale_parity` | M-c | G13.4 | `ci/g13_ue_upscale_parity_smoke.py` | UE5 超分双端对拍：复用 G12.4 MRQ harness 扩 DLSS 臂（UE 5.8.1 DLSS 插件面 vs Rurix M-a/M-b 超分面，同场景同档位双端出图；UE build digest == M128 登记 ue_build_id 机核继承）+ SSIM/FLIP/噪声谱差距登记表落盘（差距项显式登记即 RED 评审面，不静默混入）+ 帧率 measured 基线登记 **zero_pass_line 不设通过线**（G10-N11/N16 锚定 G14 字面）+ 单端缺帧聚合不得 PASS；以基线冒充帧率对标即 RED；差距项静默混入即 RED；契约 digest 不等仍出报告即 RED |
| `g13.p0.m_d.ue_lumen_gi_parity` | M-d | G13.4 | `ci/g13_ue_lumen_gi_parity_smoke.py` | UE Lumen GI 对照：Rurix M98/M99/M154 GPU GI 面（屏幕探针近场 + 世界辐射缓存远场 + 多反弹链，G9.4/G11.4 已验收面只消费不改写）vs UE Lumen 同场景双端出图 + GI 能量/间接光 measured 对拍（容差标定程序产）+ Lumen 差距登记表落盘（UE Lumen 模块归属，RXS-0391 归属枚举口径继承）+ G11 GI 面既有判据 0-byte；Lumen 差距项静默混入即 RED；GI 既有门降级即 RED；单端缺帧聚合 PASS 即 RED |
| `g13.p0.m_e.regression_drift_guard` | M-e | G13.5a | `ci/g13_regression_drift_guard_smoke.py` | 回归门 + 漂移监控：既有 71 门（G9 34 key + G10 14 key + G11 14 key + G12 9 key）最新 evidence 全绿只读汇总（聚合不遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE）+ G13 触改面既有门重跑回归零降级（M96 golden 门序面真跑抽检）+ M165 漂移监控登记（G13 复跑面同型 digest 漂移检出计数/零检出字面入 evidence，FAIL 件 0-byte 保留纪律继承）；既有门降级即 RED；聚合遮蔽即 RED；漂移检出未登记即 RED |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G13.5b。G13.1 无 go 的 P1 行（调研报告 P0 建议清单 5 行全为 P0；后续波次判 go 的 P1 按只追加程序入 MAP §2）。

### 4.3 G13.1 治理三门冻结面

治理三门本波 materialize（真脚本 + 真 schema + 真 workflow 步骤，落盘前实测 `CI_step.next_free=233` 顺位领取 233~235；ledger 校准同批）：

| Symbolic gate key | 稳定脚本 | evidence schema（同批落盘） | numeric_step |
|---|---|---|---|
| `g13.wave.1.acceptance_map` | `ci/g13_acceptance_map_check.py` | `milestones/g13/g13_acceptance_map_check_evidence_schema.json` | 233 |
| `g13.wave.1.candidate_decisions` | `ci/g13_candidate_decisions_check.py` | `milestones/g13/g13_candidate_decisions_check_evidence_schema.json` | 234 |
| `g13.gov.implementation_interlock` | `ci/g13_interlock_check.py` | `milestones/g13/g13_interlock_check_evidence_schema.json` | 235 |

三门均为 host 纯 host 治理断言面（零 device 交付——治理波不碰 GPU）；`g13.gov.implementation_interlock` 的 PASS 判据 = validator 诚实输出当前 VERDICT（G13.1 期 = **BLOCKED**，正确结论不充绿）+ 一致性门全绿 + evidence 携带 VERDICT 字面；`--require-ready` 模式供未来 G13.2 实现 PR 作前置 required check（未 READY 即退出非零）。

## 5. Guardrails

见 YAML `guardrails`。特别强调五点：

1. 治理 active 不等于实现 active；G-G13-3 的机器事实（validator READY + 用户 G13.2 开工指令 + **M-a 许可前置清结** + actual `next_free` 重校）不可替代。
2. **许可前置条款**：DLSS/Streamline 与 FSR redistribution/集成许可的 owner 法律面清结是 M-a 开工硬门（G12.3 NRD 许可取证先例同模——评估 ≠ 接入）；未清结 M-a 保持 blocked，不得以 FSR MIT 许可面宽松冒充 DLSS NGX 专有面清结。
3. P0 实现门数字 CI 步骤只能在实现互锁开放后读取 actual `next_free` 再分配；治理三门步骤 233~235 为本波实测领取的例外面（调研报告任务面明令）；文档中的稳定身份是 symbolic gate key 和脚本名；禁止沿用草案建议值。
4. **异己面零消费纪律**（立项裁决 1 同模）：G13 车道 commit 只含 G13 车道文件；工作树异己会话 src/ 未提交面严禁消费/混入/冒充 G13 任何门绿。
5. **G12 gap registry 只消费不回写 + M165 漂移监控登记 + 帧率 zero_pass_line**：三条登记纪律逐字见 YAML guardrails；M-c 不设帧率通过线（锚定 G14）、G13 不设绝对超分画质通过线（归 G15）。

## 6. Deferred 处置

| Deferred | G13 处置 |
|---|---|
| RD-034 | DXIL RT/mesh 上游 blocked 维持 open；G13 超分/Lumen 对照仅 Vulkan 主腿（DLSS 走 Streamline Vulkan interop 臂），不阻主线 |
| RD-039 | 总体维持 open 为法定输入；M61 mesh shader 分项维持 defer（G12.6 逐字承接，承接锚字面 0-byte，G13+ 重评窗顺延不关闭）；其余分项未触发维持 open |
| RD-040 | **M52 SER G13 登记**（G13.4 若上 Lumen 化 workload 自然 materialize 高分歧 RT workload 面则按只追加程序重评登记，否则维持 defer——承接锚字面 0-byte）；**M100-high 维持 defer 锚定 G14**（G13 登记）；**RD040-nrd G13 决策窗**：G12.3 评估已完结不接线，G13 接入裁决 = 接入真实需求 + owner 法律面许可清结 + measured 对拍面三条件齐备时按只追加程序重判，未齐备维持不接线；history 只追加 |
| RD-041 | **FSR/DirectSR 分项 G13 兑现窗**：随 G10-N5 同族——M-a vendor 超分接入波即本分项接入评估兑现面（FSR 3.1.5 经 UpscaleBackend trait 接入，接口已冻结不改底座字面）；M28/M40-svt/M26-fg/M05-mv/M56-wg 维持 no-go 留档；FG/MFG「独立层另判」字面不动（G13 out_of_scope 明记） |
| RD-044 | M126-rd044/RD044-continuum/RD044-fluid 维持 open-留档/观察；G13 度量面 FLIP 图像度量与 RD-044 族 FLIP 流体防混淆登记维持（G10/G11/G12 口径字面） |
| RD-042/043 | 可微物理观察 / wgrapier GPU 刚体观察维持，不进 G13 任何面 |

详情始终以 `registry/deferred.json` 为唯一事实源；本表只冻结承接纪律。G12 defer-to-G13+ 22 行逐行处置归 [G13_CANDIDATE_DECISIONS.md](G13_CANDIDATE_DECISIONS.md) §1；SAFE-GPU 维持「独立期立项」defer（G13 非其独立期，沿 G10/G11/G12 立项裁决口径）。

## 7. 修订记录与开工裁决

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-18 | 初版契约：按 G13 立项前调研报告（2026-08-18 主会话留痕）显式拆分 governance 与 implementation；G13.1 active、G13.2+ blocked；冻结波次门（G-G13-1~9）与 5 个 P0 独立断言（M-a~M-e 字母行号为治理期稳定身份，M### 数字 post-interlock 按实际顺位领取；key 命名空间本契约 §4.2 ↔ MAP §1 双方逐字一致）+ 治理三门（§4.3，步骤 233~235 实测领取）；十项立项裁决逐字登记；§8 只追加区启用。 |

**开工裁决留痕**：

- **用户立项指令**：2026-08-15 主会话下达「/goal 帮我完成 G10-15 的内容，自主派发调研 agent 和进行决策，里程碑推进时组织 agent-team 完成，要求彻底完成对标 UE5 渲染器的目标，并支持 dlss、超分采样、路径追踪等前沿技术。技术完成需要严格的画面审查，需要获取完整渲染画面，再用本地已有的 UE5 渲染器出图对比，修复画面中出现的细节问题；同时优化渲染管线效率，使帧率对标 UE5 略高（不降级画质）。本地已有 UE5 渲染器参考项目，你也可以联网获取（我的 GitHub 在 UE5 组织内），同时支持联网获取压测模型环境等必要工具集。最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在 G15 后无限制新建里程碑继续优化」（指令原文以会话留痕为准，G10/G11/G12_CONTRACT §7 同字面援引）。该指令授权 G10~G15 六期分期与全期自主推进——**G13.1 立项与 G13.2+ 开工授权同源**：「支持 dlss、超分采样」即 G13 超分采样期的用户目标字面；本治理波为该指令在 G13 期的执行留痕（2026-08-18 G13.1 治理波任务下达：G13 立项 + 治理三件套 + 候选决策 + 验收映射 + 治理三门 materialize + 互锁诚实 BLOCKED）。
- **agent 立项裁决**：依 10 §7、P-13 与 D-406 v2.0，agent 完全自主签署立项裁决；G13.1 治理波即刻 active，G13.2+ 继续由 G-G13-3 硬阻断。
- **不可变基线**：G13.0 文档集不可变 ref = `8c5dc5ee`（G12 close-out flip commit，tag `g12-closed`，立项时实测 HEAD；沿 G12.0 取 G11 回归刷新批 HEAD 先例同模）。工作树带异己会话 src/ 未提交面——处置见裁决 1。
- **十项立项裁决（逐字登记）**：
  1. 现在立项；G13.0 不可变 ref=`8c5dc5ee`；**带未提交项立项**——工作树异己会话 src/ 未提交面（`git status` 2026-08-18 实测：apps/uc06-renderer、apps/uc08-physics、src/rurix-asset/src/lib.rs、src/rurix-render/src/{bin/g10_m134_frame_capture.rs, bin/g9_m95_visbuffer_swhw.rs, geometry/mod.rs, gi/mod.rs, lib.rs, shadow/mod.rs}、src/rurix-rt/src/render_exec.rs 改写面 + evidence/d3d12_interop_smoke.json、milestones/g12/g12_pt_sampler_selection.json 异己改写面）保持不混入 G13 车道、**严禁消费**（G10.8b §8.10/G11/G12 立项裁决 1 先例同模：治理/flip commit 只含本车道文件，异己面维持未提交）。
  2. 超分/对照范围唯一法定来源 = G13 立项前调研报告 P0 清单（M-a~M-e 5 行）+ G13.1 候选决策表行集 + G12 法定输入（`g12_ue_pt_gap_registry.json` 10 行 + G12_P2_DECISIONS 22 行承接锚）；G13 不得无锚新立项；新发现差距进 G13 新差距登记表显式登记 + G13.5a 穷举，不得静默混入。
  3. **M-c 帧率面 = measured 基线登记 zero_pass_line 不设通过线**（以基线冒充帧率对标即 RED——正式帧率对标锚定 G14，G10-N11/N16 承接锚字面 0-byte）；**G13 不设绝对「已达 UE5 DLSS/超分画质」通过线**——绝对判定归 G15 商用收口期（G12 不设绝对 UE PT 画质通过线先例同模）。
  4. **RFC 判档**：G13.1 治理波零 RFC 消费（RFC 命名空间 0-byte，实测 next_free=30 维持）；G13.2+ 若触 UpscaleBackend trait 签名面/temporal 底座历史接口面/RXS-0357 参照器面/M137 scalars.flip 演进位翻转等冻结面 → 独立 Full RFC 评估（D-409 对抗性评审后 Agent Approved），编号按起草时实测 actual next_free 领取，禁推测号；判档争议向上取严。
  5. **M-a 许可前置条款**：DLSS = Streamline SDK 2.10.3（开源框架）+ NGX 签名 DLL（专有）+ FSR 3.1.5（MIT）——redistribution/集成许可的 **owner 法律面清结为 M-a 开工硬门**（G12.3 NRD 许可取证先例同模：2026-08-17 联网实测 NRD 为自定义 NVIDIA RTX SDKs LICENSE 非 OSI——owner 法律面审为接入硬前置口径来源）；未清结即 M-a blocked、G13.2 不得开工；不得以 FSR MIT 面宽松冒充 DLSS NGX 面清结。
  6. **UpscaleBackend/temporal 底座 0-byte**：G13 超分接入经 UpscaleBackend 冻结接口面（RFC-0016 §4.0-3 三实现位预留：自研 TSR / FSR 3.1 / DirectSR-vendor 留口；历史内置双缓冲；`src/rurix-render/src/temporal/upscale.rs` 217 行在树）；trait 签名与 temporal 底座历史接口面 0-byte（目录级 diff 机核）；确需演进必须独立 Full RFC 显式修订行（裁决 4）；G12.3 temporal 底座 0-byte 机核先例同模。
  7. G8.8b/G9.8b/G10.8b/G11.8b/G12.8b 同日放行先例 = 继承（7a full-run 先行完成后允许同日进 close-out；先例字面不扩展解释）。
  8. **回归零降级 + M165 漂移监控**：G13 不得降级既有 71 门绿面（G9 34 + G10 14 + G11 14 + G12 9）；G5~G12 closed 判据 0-byte；回归门独立 P0（M-e）；**M165 间歇非确定性事件监控登记**（G12-N13：1/~1760 帧 digest 单帧漂移未定位，flip-trace 诊断臂在树）——G13 复跑面检出同型漂移即如实登记并升级评估（升级 = 生产化缺陷修复项 + Full RFC 评估），零检出维持 open-defer 不写进全绿叙述。
  9. **G12 defer-to-G13+ 22 行逐行处置**：G10-N5 DLSS/Streamline 方向 = **G13 兑现窗**（G13 立项本身即承接锚「DLSS/超分立项」条件命中；M-a 许可/ABI 评估齐备面归 G13.2 开工硬门）；RD-041 FSR/DirectSR 分项 = M-a 同波兑现窗；RD040-nrd = G13 决策窗（接入三条件未齐备维持不接线）；M52 SER = G13 登记（G13.4 Lumen 化 workload materialize 时按只追加程序重评）；M100-high 锚定 G14 维持登记；G12-N13 = M-e 漂移监控臂承接；G12-N12 gap registry 10 行 = G13 只消费不回写（锚定 G15 逐项重评面维持）；G10-N11/N16/G11-N3/M114-strand 锚定 G14 维持；G11-N8/N9/G12-N10 锚定 G15 维持；其余行承接锚字面 0-byte 维持——逐行落 [G13_CANDIDATE_DECISIONS.md](G13_CANDIDATE_DECISIONS.md) §1。
  10. 压测资产二进制**不入 git**（外部缓存 K: 盘，仓库内只登记清单/许可/digest 元数据——沿 G10/G11/G12 裁决）；vendor SDK 二进制（Streamline/NGX/FSR）**不入 git**——G13.2 接入面以外部缓存 + 许可/digest 登记形态承载（RFC-0027 许可边界字面）；P0 实现门数字 CI 步骤 `post-interlock actual-next-free allocation` 重申确认；UE 零 vendoring 重申（Lumen/PT/TemporalUpscaler 源码只读外部参照，RFC-0027 字面）。
- **G15 后无限续期授权登记**：用户指令「允许在 G15 后无限制新建里程碑继续优化」留痕（G10/G11/G12_CONTRACT §7 同字面援引）——G15 收口若未达真实可商用标准，按同治理范式续立 G16+（每期仍独立走立项/治理波/互锁/full-run，不因授权免除任何机器门）。
- **编号快照（立项时实测，`py -3 ci/check_number_ledger.py` PASS）**：CI_step next_free=233（治理三门 233~235 本波领取）/ RXS next_free=404 / RFC next_free=30 / MR next_free=12 / RD next_free=45 / U next_free=58 / D next_free=410 / RX_error next_free=7024 / SG-010 软保留；治理三门外一切编号 `post-interlock actual-next-free allocation`，禁推测号与草案建议值。

---

## 8. Implementation activation / Close-out（只追加区）

<!-- 首条未来记录只能是 G13.1 治理波验收与 G-G13-3 互锁实测面；其后追加逐波验收与 close-out。当前不得写 PASS、不得预填 run URL。 -->

### §8.1 G13.1 治理波验收记录（2026-08-18）——G-G13-1 治理激活门 + G-G13-2 完成门留痕：治理三件套 + 治理三门 materialize（步骤 233~235 实测领取）全绿；实现互锁诚实 VERDICT=BLOCKED（M-a 许可前置未清结）+ 一致性门 C1~C4 全绿；implementation_status 维持 blocked

- **① 独立断言全绿清单（治理三门，全 host 治理断言面，无 device 面——治理波零 device 交付）**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g13.wave.1.acceptance_map`（步骤 233） | G13_ACCEPTANCE_MAP §1 五行 P0（M-a~M-e 闭集全等）+ §2 零 go P1 空集 + key/脚本/schema 单一命名空间同 slug + numeric_step 全列 post-interlock 字面零预占 + 零空行/占位 + MAP §1 ↔ 本契约 §4.2 双向逐字一致（12 facts） | host 纯 host | evidence/g13_acceptance_map_check_20260818T043727Z.json（facts 12/12） | PASS |
  | `g13.wave.1.candidate_decisions`（步骤 234） | G13_CANDIDATE_DECISIONS 36 行闭集全等（§1 G12 defer 22 行 + §2 open RD 7 行 + §3 G13 新增 7 行）+ 裁决枚举合法 + 零空行 + 承接锚纪律 + MAP 5 key 互斥 + deferred history 对账（RD-039 +1〔M61〕/RD-040 +3〔M52/M100-high/nrd〕/RD-041 +1〔G10-N5〕，零新 RD）+ G12_P2 承接源行对账 n=22/22（41 facts） | host 纯 host | evidence/g13_candidate_decisions_check_20260818T043728Z.json（facts 41/41） | PASS |
  | `g13.gov.implementation_interlock`（步骤 235） | 互锁 validator 诚实报告：事实门①②④ GREEN + ③ M-a 许可前置 RED（清结留痕未在树——正确诚实态）+ 一致性门 C1~C4 全绿 + VERDICT=BLOCKED 字面入档不充绿 + implementation_status=blocked 双状态一致（8 facts） | host 纯 host | evidence/g13_interlock_check_20260818T043744Z.json（facts 8/8） | PASS（VERDICT=BLOCKED 入档） |

- **② 波聚合门实测输出**：G13.1 治理波**不设 `g13.wave.N.exit` 波聚合门**（波聚合门属 G13.2+ 实现波面，契约 §2.2 波次序列）——SKIP=not-triggered 如实登记不充绿。治理期唯一机器聚合核验面 = 互锁 validator 只读汇总：事实门①~④逐条独立断言 + 一致性门 C1~C4，聚合不遮蔽任一子断言 RED/FAIL（validator 逐行打印 + selftest 红臂实证）；实测 **VERDICT=BLOCKED，exit=0**（逐字输出见块③）——BLOCKED 是当前正确结论（M-a 许可前置 owner 法律面清结未落地），不以叙述替代机器事实。

- **③ 验收命令逐字输出（2026-08-18 真跑留痕，仓库根目录）**：
  - `py -3 ci/g13_acceptance_map_check.py --gate g13.wave.1.acceptance_map` → **VERDICT=PASS，exit=0**（facts 12/12：coverage_p0_set / coverage_p1_empty / row_M-a~M-e / two_way_M-a~M-e 全 PASS）。
  - `py -3 ci/g13_candidate_decisions_check.py --gate g13.wave.1.candidate_decisions` → **VERDICT=PASS，exit=0**（facts 41/41：set_equality_frozen 36/36 + no_duplicate_ids + row×36 + acceptance_map_mutex + deferred_history_reconcile + g12_p2_decisions_reconcile 全 PASS）。
  - `py -3 ci/g13_interlock_check.py --gate g13.gov.implementation_interlock` → **VERDICT=PASS（门面），exit=0**；validator 内层 VERDICT=BLOCKED 入档（缺项清单 = ③ M-a 许可前置唯一项）；`py -3 ci/g13_interlock_check.py --require-ready` → **FAIL exit=1**（BLOCKED 下非零退出，G13.2 实现 PR 前置 required check 面成环）。
  - 三门 `--selftest` 全 PASS：acceptance_map **9 RED + 1 GREEN + 真表臂**（删行/key 漂移/脚本名 slug 不符/数字预占/占位/非法波次/P1 注入/CONTRACT 单侧漂移/schema m 段改写九红臂 + 合成正本绿 + 真表绿）；candidate_decisions **8 RED + 真表/合成双臂 GREEN**（缺行/defer 缺 G14+/非法枚举/空单元格/互斥违例/RD status 非 open/deferred 缺登记/G12_P2 失配八红臂）；interlock **17 RED + 1 GREEN + 1 TREE**（事实门①~④ 红臂 ×11 + C3/C4 预占注入 FAIL 臂 ×2 + C2 记录不一致 FAIL 臂 + unblocked 两态校准臂 + unblocked 态事实门红 C1 仍 FAIL 臂 + closed 三态臂 + 合成正本 READY 绿 + 当前树 BLOCKED 实测臂）。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G13 三前缀路由落——load/validator/路由三处纯追加，既有路由 0-byte）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 385 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (385/385 clauses anchored, 861 test files scanned)`；`py -3 ci/budget_eval.py` → `[budget_eval] PASS (196 pass, 0 skip, normal mode)`；`py -3 ci/check_g12_acceptance_map.py` → PASS（9 key 三向逐字一致，G12 面零降级）；`py -3 ci/check_g12_implementation_interlock.py --require-ready` → VERDICT=CLOSED exit=0（G12 收口终态正确结论）。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G13-1（治理激活门：用户 2026-08-15 指令留痕 + 2026-08-18 G13 立项前调研报告 + 十项立项裁决 + G13.0 不可变 ref=8c5dc5ee 登记）与本批 G-G13-2（G13.1 完成门：D-G13-1~4 齐备、验收映射无缺行、零 src/spec/conformance 语义实现、零 vendor SDK vendoring/接线、零 RFC 消费）同批留痕；数字步骤 233~235 按落盘前实测 actual next_free=233 顺位领取（ledger v1.135 校准同批）。**G-G13-3 维持 BLOCKED**——四条件中①②④机器面已绿，③ M-a 许可前置（owner 法律面清结）未落地为唯一缺项；implementation_status 维持 blocked，G13.2 不得开工。
  - **registry 补落（同批，G13.1 治理波声明面）**：number_ledger `reserved_in_flight[G13]` 命名空间登记（CI_step 233~235 claim 兑现 + RFC/RXS/RD/U/RX_error/MR/SG/D 零数字 claim 字面，G10/G11/G12 条目格式同模）+ CI_step 命名空间校准（on_tree_max 232→235、next_free 233→236）+ revision_log v1.135；deferred.json history 只追加五条——**RD-039 +1**（M61 defer-to-G14+ 承接）/ **RD-040 +3**（M52 defer-to-G14+ + G13.4 Lumen 化 workload 重评窗登记 / M100-high 锚定 G14 维持 / RD040-nrd G13 决策窗登记三条件未齐备维持不接线）/ **RD-041 +1**（G10-N5 G13 兑现窗 + FSR/DirectSR 分项 M-a 承载登记）+ revision_log v1.83；条目级字段与 status 0-byte。
  - **门脚本起步缺陷修复批（偏差如实登记）**：三门首跑即红——门脚本自身三处机核缺陷（非文档面缺陷）：(a) M 行号连字符形（M-a~M-e）→ key/schema m 段下划线形（m_a~m_e）映射缺失（key 字符集 [a-z0-9_] 不含连字符，比对统一走 `m.lower().replace("-","_")`）；(b) 候选表裁决列 markdown 加粗字面（`**go（…）**`）未 normalize 致前缀匹配误判；(c) §1 承接锚列规则误设「须含 G14+」——§1 锚列 = G12.6 承接锚 0-byte 转引（G13+ 承接源字面），G14+ 重评窗由裁决列 defer-to-G14+ 自身承载（转引列不回写）。修复后三门全绿 + selftest 全红绿臂过；首跑 FAIL 件（门脚本缺陷期产物，未经验收语义）不入档不 commit，终版 evidence 三件在档（块①）。
  - **工作树并发面留痕（沿 G12 各波同模）**：本波期间三个新门脚本的逻辑面修复批一度被并发面回退（夹具面修复在树、逻辑面被抹的混合态）——经 `.tmp/g13_1_gatefix.py` 脚本按实测磁盘态逐 sub 幂等原子重放落盘并三门真跑复核（PASS 留痕）；共享四面（check_schemas.py / pr-smoke.yml / number_ledger.json / deferred.json）经 `.tmp/g13_1_replay.py` 一次落盘复核在树；`.tmp/g13_1_*.py` 为一次性落盘工具不入 commit。
  - **not-triggered / 维持 open 面**（如实保持 open，不写进全绿叙述）：M-a 许可前置 blocked（唯一缺项）；M52（G13.4 重评窗登记）/ M100-high（锚定 G14）/ RD040-nrd（G13 决策窗三条件未齐备）/ G10-N17（G13.4 触发评估）/ G11-N5（G13.5a 触发评估）/ G12-N13（M-e 漂移监控臂承接）维持 defer；G10-N11/N16/G11-N3/M114-strand 锚定 G14；G11-N8/N9/G12-N10/G12-N12 锚定 G15（G12 gap registry 只消费不回写）；G13-N6 异己面 no-go（零消费零混入）；G13-N7 FG/MFG defer-to-G14+；SG-010 软保留 not_triggered 维持。
  - **异己并发工作树面**：本批只含 G13 车道文件（块⑤清单按文件名显式择取）；异己会话 src/ 未提交面（2026-08-18 git status 实测：apps/uc06-renderer、apps/uc08-physics、src/rurix-asset/src/lib.rs、src/rurix-render/src/{bin/g10_m134_frame_capture.rs, bin/g9_m95_visbuffer_swhw.rs, geometry/mod.rs, gi/mod.rs, lib.rs, shadow/mod.rs}、src/rurix-rt/src/render_exec.rs 改写面 + evidence/d3d12_interop_smoke.json、milestones/g12/g12_pt_sampler_selection.json 异己改写面及各 evidence/ 异己新件）维持未提交、零消费、零混入（立项裁决 1）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/G12 §8.x 同模）。`Assisted-by: Kimi-K3（G13.1 治理波）`（影响范围：milestones/g13/G13_CONTRACT.md + G13_ACCEPTANCE_MAP.md + G13_CANDIDATE_DECISIONS.md 三新建 + 三 evidence schema〔g13_acceptance_map_check_/g13_candidate_decisions_check_/g13_interlock_check_evidence_schema.json〕+ ci/g13_acceptance_map_check.py + ci/g13_candidate_decisions_check.py + ci/g13_interlock_check.py 三新建 + ci/check_schemas.py〔G13 三前缀 load/validator/路由三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 233~235，步骤 232 块后追加〕+ registry/number_ledger.json〔CI_step 232→235/next_free 236 + reserved_in_flight[G13] + revision_log v1.135〕+ registry/deferred.json〔RD-039 +1/RD-040 +3/RD-041 +1 + revision_log v1.83〕+ 本契约 §8.1 本条 + evidence/g13_{acceptance_map_check,candidate_decisions_check,interlock_check}_20260818T0437{27,28,44}Z 三件真跑件；验证方式：块③逐字命令输出——三门 PASS + 全量 selftest 红绿留痕 + 守卫套件全 PASS + G12 面零降级 + 互锁 --require-ready 诚实非零退出）。

---

### §8.2 G-G13-3 implementation_status 解锁记录（2026-08-18）——互锁 VERDICT=READY（事实门①~④全绿：③ M-a 许可前置 owner 法律面清结留痕落盘 = 唯一缺项补齐）+ 一致性门 C1~C4 全绿；G13.2 vendor 超分接入波开工面开放

- **① 独立断言全绿清单（`g13.gov.implementation_interlock` 事实门四条 + 一致性门 C1~C4，全 host 治理断言面，无 device 面——许可清结与互锁重跑零 device 交付）**：

  | gate（脚本内断言标识） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g13.gov.implementation_interlock` 事实门① | G12_CONTRACT status==closed 且 §8.8 签署块在位 + G13.0 不可变 ref `8c5dc5ee` 登记 | host | milestones/g12/G12_CONTRACT.md §8.8 + 本契约 §7 | PASS |
  | `g13.gov.implementation_interlock` 事实门② | 候选决策表 36 行零空行 + deferred.json history 只追加（vs G13.0 base 四字段 0-byte）+ 验收映射 §1 五行 P0（M-a~M-e 闭集全等）+ §2 零 go P1 空集无缺行 | host | milestones/g13/G13_CANDIDATE_DECISIONS.md + registry/deferred.json + milestones/g13/G13_ACCEPTANCE_MAP.md | PASS |
  | `g13.gov.implementation_interlock` 事实门③ | M-a 许可前置：许可清结留痕在树且五要素字面齐备（Streamline/NGX/FSR/owner/清结）——owner 2026-08-18 主会话明确接受 NVIDIA RTX SDKs LICENSE（DLSS/Streamline 面）+ FSR 3.1.5 MIT 零障碍确认 + NRD 与 DLSS 同批清结登记；清结状态 pending → cleared | host | milestones/g13/design/vendor_upscale_license_clearance.md §1/§2 | PASS |
  | `g13.gov.implementation_interlock` 事实门④ | 用户 G13.2 开工指令留痕（2026-08-15 全期授权面「支持 dlss、超分采样」字面）+ workflow 实测末号 235 == ledger CI_step on_tree_max 235 且 next_free 236 == +1 | host | 本契约 §7 + .github/workflows/pr-smoke.yml + registry/number_ledger.json | PASS |
  | `g13.gov.implementation_interlock` 一致性门 C1~C4 | C1 双状态诚实 / C2 §8 记录一致 / C3 数字步骤零预占（unlocked 态 not_applicable 两态登记，实测 0 处）/ C4 src-spec-conformance 0-byte（unlocked 态 not_applicable 两态登记，实测 0 处） | host | 本契约 front matter + §8.2 本条 + ci/g13_interlock_check.py | PASS |

- **② 波聚合门实测输出**：治理期唯一机器聚合核验面 = 互锁 validator 只读汇总：事实门①~④逐条独立断言 + 一致性门 C1~C4，聚合 VERDICT 不遮蔽任一子断言 RED/FAIL（validator 逐行打印 + selftest 红臂实证）；实测 **VERDICT=READY，exit=0**（逐字输出见块③）——G-G13-3 四条件齐备：①互锁 validator VERDICT=READY（机器事实，不以叙述替代）+ ②用户 G13.2 开工指令留痕（契约 §7 指令字面，G13.1 立项与 G13.2+ 开工授权同源）+ ③M-a 许可前置 owner 法律面清结留痕（milestones/g13/design/vendor_upscale_license_clearance.md，pending → cleared）+ ④共享编号按 actual next_free 重新校准（CI_step next_free=236 实测维持——本批零数字步骤消费，status flip 不占数字步骤，G11/G12 flip 先例同模；P0 实现门数字一律 post-interlock actual-next-free allocation，禁推测号与草案建议值）。

- **③ 验收命令逐字输出（2026-08-18 真跑留痕，仓库根目录）**：
  - `py -3 ci/g13_interlock_check.py --require-ready` → **VERDICT=READY，exit=0**，完整输出：
    ```text
    [g13_interlock] 事实门（当前可为红）：
      PASS ① G12_CONTRACT status = 'closed'（要求 closed）且 §8.8 签署块在位 = True；G13.0 不可变 ref 8c5dc5ee 登记 = True
      PASS ② 决策表/ deferred/ 验收映射三面：候选决策表 36 行零空行；deferred history 只追加（vs G13.0 base 四字段 0-byte）；验收映射 §1 五行 P0 + §2 零 go P1 空集无缺行
      PASS ③ M-a 许可前置（owner 法律面清结硬门）：许可清结留痕在树且五要素齐备（Streamline/NGX/FSR/owner/清结）
      PASS ④ 用户 G13.2 开工指令留痕（2026-08-15 全期授权面「支持 dlss、超分采样」字面） = True；workflow 实测末号 = 235、ledger CI_step on_tree_max = 235、next_free = 236（一致 = True）
    [g13_interlock] 一致性门（红即脚本失败）：
      PASS C1 G13_CONTRACT implementation_status = 'unlocked'；事实门全绿 = True（事实未全绿时必须保持 blocked，禁止治理完成冒充实现开工）
      PASS C2 §8 G-G13-3 解锁记录存在 = True；事实门全绿 = True、implementation_status = 'unlocked'（双状态与 §8 记录必须一致）
      PASS C3 数字步骤零预占：not_applicable（implementation_status='unlocked' 已解锁，治理期口径不适用；skipped_reason=实现波合法 materialize，实测 numeric_step 违例 0 处 / workflow g13 实现面 token 0 处 / ci/g13_*_smoke.py 0 件均为解锁后合法实现面，非预占；blocked 态恢复原机核，判据语义 0-byte）
      PASS C4 src/spec/conformance 治理期 0-byte：not_applicable（implementation_status='unlocked' 已解锁，治理期口径不适用；skipped_reason=实现波合法改动三面，实测 g13 实现面 token/命名命中 0 处均为解锁后合法实现面，非治理期预放；blocked 态恢复原机核，判据语义 0-byte）
    [g13_interlock] VERDICT = READY
    ```
  - `py -3 ci/g13_interlock_check.py --gate g13.gov.implementation_interlock` → **VERDICT=PASS（门面），exit=0**；validator 内层 VERDICT=READY 字面入档——evidence/g13_interlock_check_20260818T045601Z.json（facts 8/8：fact_gate_1_prior_milestone_closed / fact_gate_2_governance_docs / fact_gate_3_license_precondition / fact_gate_4_instruction_ledger 全 GREEN + consistency_gates_green + verdict_honest + verdict_recorded + blocked_not_masqueraded〔READY 态 = 事实门全绿机器事实，不以叙述替代〕；notes 同字面 VERDICT=READY）。
  - `py -3 ci/g13_interlock_check.py --selftest` → **SELFTEST PASS (17 RED + 1 GREEN + 1 TREE)，exit=0**：17 红臂全过（事实门①~④ 红臂 ×11 + C3/C4 预占注入 FAIL 臂 ×2 + C2 记录不一致 FAIL 臂 + unblocked 两态校准臂 + unblocked 态事实门红 C1 仍 FAIL 臂 + closed 三态臂）+ 合成正本 GREEN + 当前树 TREE ok（VERDICT=READY，exit=0——G-G13-3 解锁条件已齐，符合当前事实预期）。
  - 同族治理门零回归：`py -3 ci/g13_acceptance_map_check.py --selftest` → **SELFTEST PASS (9 RED + 1 GREEN + 真表臂)**；`py -3 ci/g13_candidate_decisions_check.py --selftest` → **SELFTEST PASS (8 RED + 真表/合成双臂 GREEN)**。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（本批新 evidence 经 G13 前缀路由 schema 校验过）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 385 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`（CI_step on_tree_max 235 / next_free 236 维持——本批零数字步骤消费）；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (385/385 clauses anchored, 861 test files scanned)`；`py -3 ci/budget_eval.py` → `[budget_eval] PASS (196 pass, 0 skip, normal mode)`。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G13-1/G-G13-2 于 commit `fa077746` 留痕（§8.1）；本批 G-G13-3 唯一缺项③补齐——owner 2026-08-18 主会话明确回复「我接受 DLSS 许可，继续」，许可清结留痕落盘（五要素机器核验过），互锁 VERDICT BLOCKED → READY；implementation_status flip 同批。
  - **front matter flip（本条同批）**：`implementation_status: blocked` → `unlocked`（G11 §8.1/G12 §8.1 flip 先例同模；正文冻结 0-byte——§1 双状态表与「当前裁决」行维持立项时字面，flip 事实以本条为准）。G13.2 起每个实现 PR 必须把 `py -3 ci/g13_interlock_check.py --require-ready` 作为前置 required check，spec-first + RED 先行，数字 CI 步骤按落盘前实测 actual next_free 顺位领取。
  - **门脚本同批修缮（偏差如实登记）**：`ci/g13_interlock_check.py` run_gate evidence notes 原硬编码「BLOCKED 是当前正确结论」叙述（G13.1 期事实面），本批按 verdict 条件化（BLOCKED/READY 两态字面），validator 机核断言面 0-byte；selftest 全红绿臂复核过。
  - **not-triggered / 维持 open 面**（如实保持 open，不写进全绿叙述）：M52（G13.4 重评窗登记）/ M100-high（锚定 G14）/ RD040-nrd（G13 决策窗三条件未齐备维持不接线——许可清结只兑现 owner 法律面一条件，接入真实需求 + measured 对拍面另判）/ G10-N17（G13.4 触发评估）/ G11-N5（G13.5a 触发评估）/ G12-N13（M-e 漂移监控臂承接）维持 defer；G10-N11/N16/G11-N3/M114-strand 锚定 G14；G11-N8/N9/G12-N10/G12-N12 锚定 G15（G12 gap registry 只消费不回写）；G13-N6 异己面 no-go；G13-N7 FG/MFG defer-to-G14+；SG-010 软保留 not_triggered 维持。
  - **异己并发工作树面**：本批只含 G13 车道文件（milestones/g13/design/vendor_upscale_license_clearance.md 新建 + 本契约 front matter 单字段 flip + §8.2 本条 + ci/g13_interlock_check.py notes 条件化 + evidence/g13_interlock_check 本批真跑件）；异己会话 src/ 未提交面与各 evidence/ 异己新件维持未提交、零消费、零混入（立项裁决 1——git add 按文件名显式择取）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/G12 §8.x 同模）。`Assisted-by: Kimi-K3（G13 许可清结+互锁解锁）`（影响范围：milestones/g13/design/vendor_upscale_license_clearance.md 新建〔M-a 许可前置清结凭据：DLSS/Streamline NVIDIA RTX SDKs LICENSE owner 接受 + FSR MIT 零障碍 + NRD 同批清结〕+ G13_CONTRACT.md〔front matter implementation_status blocked→unlocked + §8.2 本条〕+ ci/g13_interlock_check.py〔evidence notes verdict 条件化，机核 0-byte〕+ evidence/g13_interlock_check 本批真跑件；验证方式：块③逐字命令输出——互锁 --require-ready VERDICT=READY exit=0 + --gate VERDICT=READY evidence 落盘 + selftest 红绿留痕 + 守卫套件全 PASS）。

---

### §8.3 G13.2 vendor 超分接入波验收记录（2026-08-18）——G-G13-4 字面兑现：M-a(M167) vendor 超分接入门（步骤 236 实测领取，22 checks 全绿，DLSS SR 真跑 + FSR 3.1.5 同接口档 + 三后端运行时切换 + 静态收敛冻结带内 + RED 三臂独立有效）+ 波聚合门 g13.wave.2.exit（步骤 237）VERDICT=PASS

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g13.p0.m_a.vendor_upscale_integration`（步骤 236） | §4.2 M-a 行逐字 22 checks 闭集：许可清结五要素机核 + temporal 底座 0-byte（vs G13.0 不可变 ref 8c5dc5ee 目录级 diff + 工作树双面）+ 零 vendor 源码 vendoring（git ls-files token 面）+ 零私接面（vendor 调用 token 仅见登记集成面双文件）+ SDK registry 双段齐备 + temporal 金标准 6 单测逐名锚定 + 标定两跑位级一致 + g13_budget 三条目 measured_local + budget_eval 全 PASS + device 全档真跑（DLSS 真跑出帧〔SL 2.10.3 / NGX 310.5.2〕+ FSR 真跑〔provider 3.1.5〕+ FSR4 ML 回退登记 + 三后端运行时切换 12 帧逐帧有效 + 三后端 deficit 全在冻结带 + 双端对拍 measured 登记 + DLL provenance digest 六件对账一致 + validation 双臂零实错 + 双跑位级一致）+ RED 三臂独立复跑检出 | host+device | evidence/g13_m_a_vendor_upscale_integration_20260818T121406Z.json（22/22） | PASS |
  | `g13.wave.2.exit`（步骤 237） | M-a 门最新 evidence 只读汇总 PASS + 六 facts（temporal 底座 0-byte / RED 臂独立有效〔red 面 6 臂全真〕/ g13_budget 标定条目 measured_local 零 estimated + budget_eval 全 PASS / 许可清结五要素 / SDK registry 双段 / 零私接面）；聚合不代绿不重跑不遮蔽 | host 纯 host（只读聚合） | evidence/g13_wave2_exit_20260818T121755Z.json | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g13_wave2_exit_check.py --gate g13.wave.2.exit` → **VERDICT = PASS，exit=0**（GATE PASS g13.p0.m_a.vendor_upscale_integration + FACT PASS ×6 逐行打印不遮蔽）；`--selftest` → **ALL PASS**（负样本空 evidence 目录 → 全红退 1 + 真树一致性臂：聚合 VERDICT == 子门实测态，遮蔽即自检红）。

- **③ 验收命令逐字输出（2026-08-18 真跑留痕，仓库根目录）**：
  - `py -3 ci/g13_vendor_upscale_integration_smoke.py --gate g13.p0.m_a.vendor_upscale_integration` → **`[g13_m_a] PASS`，exit=0**（checks 22/22 device=executed；标定腿 tsr/dlss/fsr ×2 跑位级一致；device 全档 `harness --band-tsr 0.177664 --band-dlss 0.726137 --band-fsr 0.649858`〔RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1〕state=pass problems=[]；RED 三臂逐臂 detected=true）。
  - harness 直出关键实测面（evidence 内逐字段机核过）：**三后端静态收敛 deficit**（32 帧 Halton jitter 终帧 1−SSIM 对拍 4×4 超采样参照）TSR host 0.0888322165（带 0.1776644329=measured×2.0）/ **DLSS SR 0.3555856719**（带 0.7261372845）/ **FSR 3.1.5 0.3249292075**（带 0.6498584149）——全在带内；**双端对拍 measured 登记**（不设通过线）：DLSS↔TSR SSIM 0.7318790609 / 逐像素 maxdiff 0.5096423030，FSR↔TSR SSIM 0.7831488142 / maxdiff 0.4482023120；**运行时切换** tsr→dlss_sr→fsr_3.1.5 逐帧轮换 12 帧输出逐帧有效；**双跑位级一致**三后端 digest 全等；**DLL provenance** 六件 digest 与 g13_vendor_sdk_registry.json 逐字一致（sl.interposer/sl.common/sl.dlss/nvngx_dlss + amd_fidelityfx_loader_dx12/amd_fidelityfx_upscaler_dx12）；**FSR4 ML 不可用回退登记**（RTX 4070 Ti 非 RDNA4 → FSR 3.1.5 分析版，available_versions=["3.1.5","2.3.4"]，fsr4_note 非空）。
  - **validation 双臂零实错**：DLSS 臂 validation_errors=0 + NGX 内部伪报豁免 74 件全透明登记（白名单 VUID-VkCuModuleCreateInfoNVX-pNext-pNext / VUID-VkImageViewCreateInfo-image-06728 两条逐字）；FSR 臂 D3D12 debug layer + InfoQueue ERROR/CORRUPTION 级在跑计数 = 0。
  - `py -3 ci/g13_vendor_upscale_integration_smoke.py --selftest` → **SELFTEST PASS checks=22 schema_required=22 (4 RED + 4 GREEN)**（check() 合成红臂 + CHECK_KEYS↔schema 闭集 + temporal diff/vendoring/私接面三检出器红绿臂 + harness 状态判读红绿臂）。
  - `py -3 ci/g13_wave2_exit_check.py --gate g13.wave.2.exit` → VERDICT=PASS（块②）；`py -3 ci/budget_eval.py` → **PASS（199 pass, 0 skip, normal mode）**（g13 三条目并入 196→199）。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G13 新三前缀路由落——load/validator/路由三处纯追加，既有路由 0-byte；本批 evidence 全件 schema 校验过）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 385 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (385/385 clauses anchored, 861 test files scanned)`；`py -3 ci/g13_interlock_check.py --require-ready` → VERDICT=READY exit=0（G13.2 实现 PR 前置 required check 面维持）；`py -3 ci/g13_acceptance_map_check.py --selftest` / `g13_candidate_decisions_check.py --selftest` → 全 PASS（治理三门零回归）。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G13-1~3 于 commit c1fada72 留痕（§8.1/§8.2）；本批 G-G13-4（M-a 门 + 波聚合门）同批——数字步骤 236~237 按落盘前实测 actual next_free=236 顺位领取（ledger v1.136 校准同批；reserved_in_flight[G13] CI_step/U 两 claim 行兑现更新）；M 数字 M167 按 M 命名空间实际顺位（G12 末号 M166 之后）领取。
  - **DLSS validation 覆盖口径偏差（如实登记，契约 §4.2 M-a 行「validation 零错误」的机核细化）**：NGX `slEvaluateFeature` 在 `VK_LAYER_KHRONOS_validation` 层在下触发 **NVIDIA 驱动内部崩溃**（minidump 实测：0xc0000005 @ nvoglv64.dll+0xee33c1，调用栈全在驱动内部帧；SL 异常处理器捕获报 eErrorExceptionHandler(24)；vendor 已知 SL+validation 不兼容类——Streamline GitHub issue #84 ack/bug 登记 SL/NGX 校验层噪声为已知面）——**evaluate 段无法纳入校验层覆盖**。兑现形态：DLSS 校验面经 `--validation-probe dlss` 独立子进程达成，覆盖 = 我方 Vulkan 全表面（实例/设备/资源/四槽上传含 DEPTH aspect 拷贝/barrier/提交/回读/reset/逆序销毁）+ SL 代理建链 + SL 簿记调用（frame token/constants/options），实错 0 + NGX 内部伪报豁免逐字登记；DLSS 功能帧经独立无层 session 真跑产出（32 帧收敛 + 双跑位级一致 + digest provenance 全要素不减）。evidence `validation_coverage` 字段与 harness/FFI 模块文档头同字面登记，不静默。
  - **RED 臂注入面修正（如实登记）**：FSR 臂原设计 `zero-exposure` 注入（pre_exposure=0）实测无效——FSR 3.1.5 LDR 路径不消费 pre_exposure，deficit 与诚实跑位级一致（0.324929207457468 两跑全同），该注入面废止；`fsr-mv-garbage` 臂（垃圾 MV uv 位移 10.0）接替，实测 honest=0.324929 vs tampered=0.830315（2.56×）稳定检出。
  - **实现期三真 bug 修复留痕（全经门复跑实证）**：(a) queue 家族选取 compute-only 优先 → 我方深度上传 vkCmdCopyBufferToImage 命中 VUID-vkCmdCopyBufferToImage-commandBuffer-07739 实错（DEPTH aspect 拷贝要求 GRAPHICS 家族）——反转为 graphics|compute 优先，validation 实错归零前提修复；(b) vk_debug_cb 解引缺陷——`*(data as *const u8).add(24)` 运算符优先级致把 pMessageIdName 指针低字节当地址解引（0xc0000005），修为先转 `*const *const c_char` 再解引，白名单逐字面恢复工作；(c) FSR ffxQuery NO_PROVIDER——显式补载 amd_fidelityfx_upscaler_dx12.dll 使 provider 注册。另：FSR p_next 链构造按 clippy 面重构（override 版本 pin 语义 0-byte）；FFI 模块 clippy（feature vendor-upscale）清零——本模块 0 warning/error（crate 级 undocumented_unsafe_blocks=deny 面逐块 SAFETY 落位；vk.rs 等既有 feature-gated 文件的同类告警为树历史存量，非本波面，0-byte 不动）。
  - **DLSS/FSR 接入状态**：双臂**真跑**（非模拟非阻塞）——DLSS SR NGX 310.5.2 真跑出帧 32 帧收敛序列 + FSR 3.1.5 同；G12.3 降噪 G-buffer 同构输入近亲面（color/depth/MV/reactive 四槽）兑现。
  - **异己并发工作树面**：本批只含 G13.2 车道文件（块⑤清单按文件名显式择取）；异己会话 src/ 未提交面（apps/uc06、apps/uc08、src/rurix-asset/lib.rs、src/rurix-render 若干、src/rurix-rt/render_exec.rs 等改写面及各 evidence/ 异己新件）维持未提交、零消费、零混入（立项裁决 1）；并发会话对 number_ledger/pr-smoke.yml/check_schemas.py 的回退风险经落盘后复读核验（CI_step on_tree_max=237/next_free=238、步骤 236/237 在树、G13 六前缀路由在树）。
  - **not-triggered / 维持 open 面**（如实保持 open）：M52 / M100-high / RD040-nrd（接入真实需求 + measured 对拍面另判）/ G10-N17 / G11-N5 / G12-N13 维持 defer；G10-N11/N16/G11-N3/M114-strand 锚定 G14；G11-N8/N9/G12-N10/G12-N12 锚定 G15；G13-N6 异己面 no-go；G13-N7 FG/MFG defer-to-G14+；SG-010 软保留 not_triggered 维持；M-b~M-e 未开工（G13.3+ 波次面）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/G12 §8.x 同模）。`Assisted-by: Kimi-K3（G13.2 vendor 超分接入波）`（影响范围：src/rurix-rt/src/vendor_upscale.rs 新建〔U58 FFI 边界：DLSS Vulkan 臂 + FSR DX12 臂双臂 safe 公共面〕+ src/rurix-render/src/bin/g13_vendor_upscale.rs 新建〔门 harness，forbid(unsafe_code)，bin-local adapter 经 UpscaleBackend 冻结面〕+ src/rurix-rt/Cargo.toml + src/rurix-render/Cargo.toml〔vendor-upscale feature/bin 登记〕+ src/rurix-rt/src/lib.rs〔cfg feature 模块注册一行〕+ unsafe-audit/rurix-rt.md〔U58 条目〕+ ci/g13_vendor_upscale_integration_smoke.py + ci/g13_wave2_exit_check.py + ci/g13_wave_exit_lib.py 三新建 + milestones/g13/g13_budget.json 首建〔三条目 measured_local〕+ milestones/g13 四 evidence schema + g13_vendor_sdk_registry.json + ci/check_schemas.py〔G13 新三前缀 load/validator/路由三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 236~237，步骤 235 块后追加〕+ registry/number_ledger.json〔CI_step 235→237/next_free 238 + reserved_in_flight[G13] 两 claim 行兑现 + revision_log v1.136〕+ 本契约 §8.3 本条 + evidence/g13_m_a_vendor_upscale_integration_20260818T121406Z + g13_m_a_calibration_{tsr,dlss,fsr}_20260818T111205Z + g13_wave2_exit_20260818T121755Z 真跑件；验证方式：块③逐字命令输出——M-a 门 PASS 22/22 + 波聚合门 VERDICT=PASS + 双 selftest 红绿留痕 + 守卫套件全 PASS + 互锁 --require-ready READY 维持）。

---

### §8.4 G13.3 TSR device 化波验收记录（2026-08-18）——G-G13-5 字面兑现：M-b(M168) 自研 TSR device 化门（步骤 238 实测领取，18 checks 全绿：双 .rx kernel SPV + spirv-val + device vs host 逐帧对拍 + 三档质量/帧时 measured 对照 + RED 双臂独立有效）+ 波聚合门 g13.wave.3.exit（步骤 239）VERDICT=PASS；spec-first 加性条款 RXS-0404 先行同批（零 Full RFC 触发面）

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g13.p0.m_b.tsr_device_kernel`（步骤 238） | §4.2 M-b 行逐字 18 checks 闭集：temporal 底座 0-byte（vs G13.0 不可变 ref 8c5dc5ee 目录级 diff + 工作树双面）+ host 金标准 6 单测逐名锚定 + 双 kernel `//@ spec: RXS-0404` 锚定 + conformance accept/reject 语料 + trace_matrix 全锚定 + rurixc --target vulkan 双 SPV + spirv-val 双 accept + 标定腿（maxdiff/quality）两跑位级一致 + g13_budget 七条目 measured_local 零 estimated + bench 三档 50×3 trimmed mean（M141/M165 冻结统计口径同实现复用 + 独立重算咬合）+ budget_eval 全 PASS + device 全档真跑（三档 32 帧 Halton 收敛序列 device vs host 逐帧对拍 p100 全在容差内 + 三档 deficit 全在冻结带 + 收敛单调 + 双跑位级一致 + validation 零命中）+ RED 双臂（kernel-bias/seed-change）独立复跑检出 | host+device | evidence/g13_m_b_tsr_device_kernel_20260818T142333Z.json（18/18） | PASS |
  | `g13.wave.3.exit`（步骤 239） | M-b 门最新 evidence 只读汇总 PASS + 六 facts（temporal 底座 0-byte / RED 臂独立有效〔2 臂全真〕/ g13_budget M-b 七条目 measured_local 零 estimated + budget_eval 全 PASS / 双 kernel RXS-0404 锚定 + conformance 语料 + trace_matrix 全锚定 / 三档 device 判据全真 / 帧时条目 zero_pass_line 登记）；聚合不代绿不重跑不遮蔽 | host 纯 host（只读聚合） | evidence/g13_wave3_exit_20260818T151528Z.json | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g13_wave3_exit_check.py --gate g13.wave.3.exit` → **VERDICT = PASS，exit=0**（GATE PASS g13.p0.m_b.tsr_device_kernel + FACT PASS ×6 逐行打印不遮蔽）；`--selftest` → **ALL PASS**（负样本空 evidence 目录 → 全红退 1 + 真树一致性臂：聚合 VERDICT == 子门实测态，遮蔽即自检红）。

- **③ 验收命令逐字输出（2026-08-18 真跑留痕，仓库根目录）**：
  - `py -3 ci/g13_tsr_device_kernel_smoke.py --gate g13.p0.m_b.tsr_device_kernel` → **`[g13_m_b] PASS`，exit=0**（checks 18/18 device=executed；标定腿 maxdiff/quality ×2 跑位级一致；device 全档 `--tol 0.00641954 --band-deficit-50/67/100 0.177951/0.222645/0.0160031`〔RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1〕state=pass problems=[]；RED 双臂逐臂 detected=true）。
  - harness 直出关键实测面（evidence 内逐字段机核过）：**device vs host 逐帧对拍 p100**（三档 32 帧 Halton 静态收敛序列）tier50 3.209769725799561e-3（容差带 6.419539451599122e-3=measured×2.0）/ tier67 1.547023653984070e-3 / tier100 6.140470504760742e-4——全在容差内；**三档终帧 SSIM deficit**（1−SSIM 对拍 4×4 超采样参照，RXS-0387 口径）tier50 0.0889757444（带 0.17795148873357824）/ tier67 0.1113226313（带 0.2226452626199422）/ tier100 0.0080015524（带 0.01600310477402589）——全在冻结带；**三档帧时基线**（host Instant 墙钟 around 逐帧 device 全链路，warmup 10 + timed 150 = 3 块 × 50 trimmed mean）tier50 1132.795 ms / tier67 2177.996 ms / tier100 1349.503 ms（守护阈 ×1.5，回归守护语义不构成帧率对标通过线——zero_pass_line 字面逐条目登记，正式帧率对标锚定 G14）；**双跑位级一致**三档 digest 全等；**收敛单调**三档 + host 金标准全真；**validation** 层在跑 stderr VUID/Validation Error token 计数 = 0。
  - **RED 双臂独立有效**：`--red-arm kernel-bias`（kernel 输出面加性偏置 0.05）→ tampered p100 超容差检出（对拍机制有效面）；`--red-arm seed-change`（jitter 相位偏移 0.13/0.29）→ 终帧 digest 必异检出（确定性协议漂移检出面）。
  - `py -3 ci/g13_tsr_device_kernel_smoke.py --selftest` → **SELFTEST PASS checks=18 schema_required=18 (4 RED + 4 GREEN)**（check() 合成红臂 + CHECK_KEYS↔schema 闭集 + temporal diff/harness 态判读/estimated 注入/50×3 统计面篡改四检出器红绿臂）。
  - `py -3 ci/g13_wave3_exit_check.py --gate g13.wave.3.exit` → VERDICT=PASS（块②）；`py -3 ci/budget_eval.py` → **PASS（206 pass, 0 skip, normal mode）**（g13 七条目并入 199→206）。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G13 新四前缀路由落——load/validator/路由三处纯追加，既有路由 0-byte；本批 evidence 全件 schema 校验过）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 386 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (386/386 clauses anchored, 864 test files scanned)`；`py -3 ci/stable_snapshot.py --check` → PASS（spec_clauses=386 重 bless 同批，bless_log 2026-08-18 行）；`py -3 ci/g13_interlock_check.py --require-ready` → VERDICT=READY exit=0（G13.3 实现 PR 前置 required check 面维持）；治理三门 `--selftest` 全 PASS（零回归）。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G13-4 于 commit 64b7894a 留痕（§8.3）；本批 G-G13-5（M-b 门 + 波聚合门）同批——数字步骤 238~239 按落盘前实测 actual next_free=238 顺位领取（ledger v1.137 校准同批；reserved_in_flight[G13] CI_step claim 行 0-byte，materialize 经命名空间校准 + revision_log 记录先例同模）；M 数字 M168 按 M 命名空间实际顺位（M167 之后）领取。
  - **spec-first 兑现（硬规则 7）**：加性条款 **RXS-0404** 入 spec/display_pipeline.md §8（落点裁决：候选 global_illumination.md〔RXS-0402 TSR 底座联动轴〕0-byte，裁定落 display_pipeline.md——TSR device 化 = 时域重建/超分显示链环节，M24 TSR/TAA 生产契约引用轴同卷）；**判档 = 加性 spec 条款零 RFC**——契约 §7 裁决 4 Full RFC 触发面（UpscaleBackend trait 签名面 / temporal 底座历史接口面 / RXS-0357 参照器面 / M137 scalars.flip 演进位）逐条未命中，语义事实源 = §4.2 M-b 行判据逐字 + RFC-0016 §4.0-3 冻结接口面，条款只登记不加语义；conformance 最小锚定语料两件（accept tsr_device_kernel_minimal.rx + reject tsr_device_temporal_base_rewire.rx，inert + `//@ spec` 锚定 + 转正路径旁注）同批落；trace_matrix 385→386 全锚定；stable 快照 385→386 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。
  - **实现形态**：双 .rx kernel（g13_tsr_resample.rx jitter 对齐 Catmull-Rom 重采样 × exposure 转显示域 + 抗振铃 4×4 min/max 钳制 + 深度最近邻上采样 + YCoCg Y 亮度导出 / g13_tsr_resolve.rx 闪烁时域分析 EMA + MV 最近邻上采样 + 历史双线性重投影 + 深度相对差验证 + YCoCg 3×3 AABB 闪烁松弛钳制 + reactive 优先 alpha 调制混合）公式面与 host 金标准 `temporal::tsr::TsrUpscaler` 逐字同源；门 harness g13_tsr_device.rs bin-local `TsrDeviceBackend` 经 UpscaleBackend 冻结面接入（forbid(unsafe_code)，temporal/ 底座 0-byte 机核），逐帧经 `vk::run_compute` 双 dispatch（G12 PT megakernel 车道同一 compute 派发面），历史状态 host 侧双缓冲轮换。**零新 unsafe**（U 字段 0-byte——既有执行面复用）。
  - **共享面并发回退偏差（如实登记，G13.1 §8.1 同模）**：本波期间 ci/check_schemas.py 与 registry/number_ledger.json 共享面两度被并发会话面回退（路由/命名空间校准一度在盘外丢失）——经 `.tmp/g13_3_replay.py` 一次性落盘工具按实测磁盘态幂等原子重放落盘并守卫套件真跑复核（PASS 留痕）；`.tmp/g13_3_replay.py` 为一次性落盘工具不入 commit。
  - **帧时基线诚实登记（不设通过线重申）**：三档帧时（1132.795/2177.996/1349.503 ms，debug 构建 + 逐帧全链路含回读同步口径）为**回归守护基线**——不构成任何「已达 UE5 帧率」语义；tier67 > tier100 实测倒挂如实登记（host 侧逐帧拷贝/同步口径主导，生产管线优化归 G14 性能面对标期）。
  - **not-triggered / 维持 open 面**（如实保持 open，不写进全绿叙述）：M52 / M100-high / RD040-nrd（接入真实需求 + measured 对拍面另判）/ G10-N17 / G11-N5 / G12-N13（M-e 漂移监控臂承接）维持 defer；G10-N11/N16/G11-N3/M114-strand 锚定 G14；G11-N8/N9/G12-N10/G12-N12 锚定 G15（G12 gap registry 只消费不回写）；G13-N6 异己面 no-go；G13-N7 FG/MFG defer-to-G14+；SG-010 软保留 not_triggered 维持；M-c~M-e 未开工（G13.4+ 波次面）。
  - **异己并发工作树面**：本批只含 G13.3 车道文件（块⑤清单按文件名显式择取）；异己会话 src/ 未提交面（apps/uc06、apps/uc08、src/rurix-asset/lib.rs、src/rurix-render 若干、src/rurix-rt/render_exec.rs 等改写面及各 evidence/ 异己新件）维持未提交、零消费、零混入（立项裁决 1——git add 按文件名显式择取）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/G12 §8.x 同模）。`Assisted-by: Kimi-K3（G13.3 TSR device 化波）`（影响范围：spec/display_pipeline.md〔RXS-0404 新条款 §8 + 修订记录 v1.1 行〕+ conformance/display_pipeline/{accept/tsr_device_kernel_minimal.rx, reject/tsr_device_temporal_base_rewire.rx} 两新建 + src/rurix-render/kernels/g13_tsr_{resample,resolve}.rx 两新建〔`//@ spec: RXS-0404` 锚〕+ src/rurix-render/src/bin/g13_tsr_device.rs 新建〔门 harness，forbid(unsafe_code)，bin-local TsrDeviceBackend 经 UpscaleBackend 冻结面〕+ src/rurix-render/Cargo.toml〔g13_tsr_device bin 登记〕+ ci/g13_tsr_device_kernel_smoke.py + ci/g13_wave3_exit_check.py 两新建 + milestones/g13 三 evidence schema〔g13_m_b_tsr_device_kernel_/g13_m_b_measured_entry_/g13_wave3_exit_evidence_schema.json〕+ milestones/g13/g13_budget.json〔七条目纯追加 measured_local〕+ ci/check_schemas.py〔G13 新四前缀 load/validator/路由三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 238~239，步骤 237 块后追加〕+ registry/number_ledger.json〔RXS 403→404/next_free 405 + CI_step 237→239/next_free 240 + revision_log v1.137〕+ tests/stable/{stable_api.snapshot,bless_log.md}〔385→386 重 bless + 追加行〕+ conformance/traceability_matrix.{json,md}〔386 全锚定重生成〕+ 本契约 §8.4 本条 + evidence/g13_m_b_tsr_device_kernel_20260818T142333Z + g13_m_b_calibration_{maxdiff,quality_50,quality_67,quality_100}_20260818T142333Z + g13_m_b_bench_{50,67,100}_20260818T142333Z + g13_wave3_exit_20260818T151528Z/20260818T151530Z 真跑件；验证方式：块③逐字命令输出——M-b 门 PASS 18/18 + 波聚合门 VERDICT=PASS + 双 selftest 红绿留痕 + 守卫套件全 PASS + 互锁 --require-ready READY 维持）。

### §8.5 G13.4 UE 对拍波验收记录（2026-08-19）——G-G13-6 字面兑现：M-c(M169) UE 超分双端对拍门（步骤 240 实测领取，20 checks 全绿：契约 digest 三方机核 + UE DLSS MRQ 臂逐档 engagement + Rurix 三后端 32 帧 Halton 序列 + 端内参照 SSIM/FLIP deficit + 噪声谱双端差 + 帧率基线 zero_pass_line + UE DLSS·超分模块归属差距登记表 + RED 四臂独立有效）+ M-d(M170) UE Lumen GI 对照门（步骤 241，19 checks 全绿：对照契约 digest 三方机核 + Lumen on/off 双 config 双臂 + GI 能量/间接光对拍 + UE Lumen 模块归属差距登记表 + G11 GI 面 0-byte + G10.5 默认路径逐字节 parity + RED 四臂独立有效）+ 波聚合门 g13.wave.4.exit（步骤 242）VERDICT=PASS；spec-first 加性条款 RXS-0405/RXS-0406 先行同批（零 Full RFC 触发面）

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g13.p0.m_c.ue_upscale_parity`（步骤 240） | §4.2 M-c 行逐字 20 checks 闭集：temporal 底座 0-byte（vs G13.0 不可变 ref 8c5dc5ee 目录级 diff + 工作树双面）+ conformance 三件锚定 + trace_matrix 全锚定 + 契约 digest 三方独立实现全等且 == 冻结注册值 sha256:137483a1…（host python g13_parity_contract / Rurix Rust --contract-digest / UE 内嵌 CPython build_probe）+ ue_build_id == M128 登记（5.8.1-56057345）+ g13_budget 三标定条目 measured_local + 标定腿双 seed 位级一致 + budget_eval 全 PASS + UE 臂 6 格帧齐 + DLSS engagement NGX 日志逐格机核 + Rurix 臂 18 格帧齐 + 双跑位级一致 + 帧 digest 抽格重算对账 + 差距登记表 schema 校验 + 行集对账 + 帧率基线 zero_pass_line + 残余口径差登记 + RED 四臂（digest-mismatch/silent-gap/missing-frame/fps-masquerade）门内真跑检出 | host+device | evidence/g13_m_c_ue_upscale_parity_20260818T232935Z.json（20/20） | PASS |
  | `g13.p0.m_d.ue_lumen_gi_parity`（步骤 241） | §4.2 M-d 行逐字 19 checks 闭集：G11 GI 实现面+门脚本面 0-byte（vs G13.0 ref 已提交面）+ G10.5 默认渲染路径逐字节 parity（cornell digest == M139 登记库帧，--gi-off 加性旗标行为保持机证）+ conformance 三件锚定 + 对照契约 digest 三方全等 == 冻结注册值 sha256:d9d5bf5a… + ue_build_id 机核 + g13_budget 三标定条目 measured_local + 标定腿双 seed 位级一致 + budget_eval 全 PASS + UE Lumen on/off 双臂帧齐 + Rurix GI 双臂帧齐 + 帧 digest 重算对账 + GI 能量/间接光 measured 对拍 + Lumen 差距登记表 schema 校验 + 行集对账 + 残余口径差登记不拟合 + RED 四臂（digest-mismatch/silent-gap/missing-frame/gi-gate-drift）门内真跑检出 | host+device | evidence/g13_m_d_ue_lumen_gi_parity_20260819T002556Z.json（19/19） | PASS |
  | `g13.wave.4.exit`（步骤 242） | M-c/M-d 双门最新 evidence 只读汇总 PASS + 六 facts（temporal 底座 0-byte / RED 臂独立有效〔M-c 四臂 + M-d 四臂全真〕/ g13_budget M-c+M-d 六条目 measured_local 零 estimated + budget_eval 全 PASS / RXS-0405/0406 锚定 + conformance 六件 + trace_matrix 全锚定 / 双门关键判据全真 / 帧率基线 zero_pass_line 登记）；聚合不代绿不重跑不遮蔽 | host 纯 host（只读聚合） | evidence/g13_wave4_exit_20260819T003048Z.json | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g13_wave4_exit_check.py --gate g13.wave.4.exit` → **VERDICT = PASS，exit=0**（GATE PASS g13.p0.m_c.ue_upscale_parity + GATE PASS g13.p0.m_d.ue_lumen_gi_parity + FACT PASS ×6 逐行打印不遮蔽）；`--selftest` → 负样本空 evidence 目录全红退 1（G13.3 波聚合门同模面）。

- **③ 验收命令逐字输出（2026-08-19 真跑留痕，仓库根目录）**：
  - `py -3 ci/g13_ue_upscale_parity_smoke.py --gate g13.p0.m_c.ue_upscale_parity` → **`[g13_m_c] VERDICT=PASS checks=20/20`，exit=0**。关键实测面：**端内参照 deficit 对拍 18 格**（容差 = 标定腿双 seed 方差底 p100×2.0 程序产：ssim 0.0069383200966242065 / flip 0.0073001003229083705 / noise 0.12171771690652311）——cornell t50 dlss_sr ssim_delta=0.0161405 flip_delta=0.0123923 **OVER**、cornell t67 dlss_sr ssim_delta=0.00665004 flip_delta=0.00921666 **OVER**，其余 16 格在带（tsr_device/fsr_3_1_5 全格在带，tier100 全格 delta=0）；**噪声谱 6 格全 OVER**（cornell ue_hf_share=0.0 vs rurix 0.43~0.61；bistro ue 0.850 vs rurix 0.36~0.57）——**8 超容差项全显式登记** milestones/g13/g13_ue_upscale_gap_registry.json（gaplib 正典形，gap_id 冻结字节派生 + evidence_digest 溯源 receipt converged_digest/probe 帧 digest，零静默混入）；**帧率基线 zero_pass_line 登记**（UE MRQ 449.78~762.69 ms/帧 vs Rurix 三后端 mean 29.44~2205.81 ms/帧，bistro tier100 倒挂如实登记——measured 基线不构成帧率对标通过线，正式对标锚定 G14）；残余口径差显式登记（Rurix 直接光口径 vs UE Lumen 默认面 / DLSS 名义档对拍 / NGX 实际内部分辨率窗口——只登记不拟合 RXS-0392）。
  - `py -3 ci/g13_ue_lumen_gi_parity_smoke.py --gate g13.p0.m_d.ue_lumen_gi_parity` → **`[g13_m_d] VERDICT=PASS checks=19/19`，exit=0**。关键实测面：cornell energy_delta=5.35625e+08 indirect_ssim=0.0333845 indirect_flip=0.612799 **OVER**；bistro energy_delta=79.9598 indirect_ssim=0.00651277 indirect_flip=0.967137 **OVER**（容差 gi_energy_rel 0.01752196170822843 / indirect_ssim 0.6675224733382197 / indirect_flip 0.34203315774945187）——**2 场景全超容差全显式登记** milestones/g13/g13_ue_lumen_gap_registry.json（UE Lumen 模块全路径归属 + 三 measured_delta 面）；残余口径差登记（UE Lumen cornell 面物理零贡献 vs Rurix GI 颜色溢出——真实场景物理口径差不拟合）；**G11 GI 面 0-byte 机核 + G10.5 默认路径逐字节 parity 机证双真**（--gi-off 加性旗标零行为侵入）。
  - RED 八臂独立有效：M-c 四臂（digest-mismatch/silent-gap/missing-frame/fps-masquerade）+ M-d 四臂（digest-mismatch/silent-gap/missing-frame/gi-gate-drift）门内真跑注入逐臂检出。
  - `py -3 ci/g13_ue_upscale_parity_smoke.py --selftest` → **PASS checks=20 schema_required=20 (4 RED + 2 GREEN)**；`py -3 ci/g13_ue_lumen_gi_parity_smoke.py --selftest` → **PASS checks=19 schema_required=19 (4 RED + 1 GREEN)**；`py -3 ci/g10_gap_registry_lib.py --selftest` → **PASS（枚举闭集 81 值；10 RED + 1 GREEN）**（gaplib 加性 registry_name 参数面 G10 消费零回归：`py -3 ci/g10_gap_registry_smoke.py --selftest` → **PASS checks=18**）。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（G13.4 新五前缀路由落——load/validator/路由三处纯追加，既有路由 0-byte）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS(spec RXS 头 388 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)`；`py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (388/388 clauses anchored, 870 test files scanned)`；`py -3 ci/stable_snapshot.py --check` → PASS（spec_clauses=388 重 bless 同批，bless_log 2026-08-19 行）；`py -3 ci/budget_eval.py` → **PASS（212 pass, 0 skip, normal mode）**（G13.4 六条目并入 206→212，g13.ue_upscale.*/g13.ue_lumen.* 前缀路由 eval_g13_dual_seed 专用判读面）；`py -3 ci/g13_interlock_check.py --require-ready` → **VERDICT=READY exit=0**（事实门④ workflow 实测末号 242 == ledger on_tree_max 242）。

- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G13-5 于 §8.4 留痕；本批 G-G13-6（M-c 门 + M-d 门 + 波聚合门）同批——数字步骤 240~242 按落盘前实测 actual next_free=240 顺位领取（ledger v1.138 校准同批：RXS on_tree_max 404→406/next_free 407 + CI_step on_tree_max 239→242/next_free 243；reserved_in_flight 0-byte 先例同模）；M 数字 M169/M170 按 M 命名空间实际顺位（M168 之后）领取。
  - **spec-first 兑现（硬规则 7）**：加性条款 **RXS-0405**（UE 超分对拍口径）/ **RXS-0406**（UE Lumen GI 对照口径）入 spec/visual_comparison.md（G10.5b 起对拍/登记单一事实源同卷）；**判档 = 加性 spec 条款零 RFC**——契约 §7 裁决 4 Full RFC 触发面逐条未命中（UpscaleBackend 冻结面 0-byte / temporal 底座 0-byte / RXS-0391 正典经 gaplib 加性参数转引非演进）；conformance 最小锚定语料六件（accept 双契约 minimal 两件 + reject 四件：upscale_parity_digest_mismatch_report/upscale_fps_baseline_masquerade/lumen_parity_digest_mismatch_report/lumen_gap_silent，inert + `//@ spec` 锚定）同批落；trace_matrix 386→388 全锚定；stable 快照 386→388 重 bless（error_codes/editions/subcommands 三段 0 变化）。
  - **实现形态**：UE 臂 harness milestones/g13/harness/g13_4_ue_render.py + ue_python/g13_4_build_scenes.py（DLSS 档 MoviePipelineDLSSSetting 逐档注入 / Lumen on-off 双 config MRQ 幂等建设 + 双契约 digest 探针 + 新鲜度收割，UE 5.8.1-56057345 内嵌 CPython）；Rurix 臂 harness src/rurix-render/src/bin/g13_4_ue_upscale_parity_render.rs 新建（forbid(unsafe_code)，bin-local 自含车道〔最小 glTF 装载 + BC1/BC3 DDS 解码 + host BVH 主射线 + 双面 Lambert 直接光〕+ UpscaleBackend 冻结面三后端 + --contract-digest 双契约 schema 分派）；M-d Rurix GI 臂 = g10_5_scene_render --gi-multibounce/--gi-off 既有面只消费不改写（--gi-off 为本波加性旗标，默认关 = 逐字节 parity 机证承载）；契约解析共享面 ue_python/g13_parity_contract.py（双 schema fail-closed + canonical 编码单源）。**零新 unsafe**（U 字段 0-byte——vendor FFI 面 G13.2 U58 既有登记 0-byte 维持）。
  - **差距登记表正典化（真跑检出真 bug 修复留痕）**：M-c 门 22:21 真跑检出登记表写/验不匹配（门侧自含旧 schema 写面 vs gaplib 正典验面，19/20 唯 gap_registry_schema_valid 红）——定案执行：门侧改写 gaplib 正典形（RXS-0391 IR2 禁第二份手写，门侧自含 schema 死代码拆除）+ **ci/g10_gap_registry_lib.py validate_registry 加性 registry_name 可选参数**（缺省 None 既有行为 0-byte，G10 M140 消费面 selftest 18/18 零回归）；M-d 门同构处理并排除潜伏 NameError（validate_gap_registry 调用点未定义未导入，selftest 覆盖面外——真跑前静态面未及）；模块归属枚举值全路径化（PostProcess/Lumen ∈ UE5_MODULE_ENUM）+ measured_delta 全字段闭集（a_value/b_value/delta 符号化 f64 精确 + evidence_digest 溯源 receipt 面）+ cell_digests 取材面。修复后 M-c 真跑 PASS 20/20 登记表首落盘、M-d 真跑 PASS 19/19 登记表首落盘。
  - **共享面并发回退偏差（如实登记，G13.1 §8.1/G13.3 §8.4 同模）**：本波期间共享面**三度**被并发会话面回退——① ci/check_schemas.py 三处追加面 + registry/number_ledger.json 校准面（上一会话期）；② ci/g13_ue_lumen_gi_parity_smoke.py cell_digests 声明行 hunk 级回退（M-d 首真跑 NameError 即此，其余修复面存活）；③ .github/workflows/pr-smoke.yml 步骤 240~242 块整段回退（互锁门事实④ RED「workflow 实测末号 239 ≠ ledger 242」即此——编辑落盘后数秒内被回退至 HEAD，竞态窗口实测）——处置：`.tmp/g13_4_replay.py` 一次性落盘工具扩面（新增 cell_digests 修复条目 + workflow 步骤 240~242 文本锚点幂等重放面 + main() 顺序纪律修正〔pristine 字节恢复先行、文本锚点修复后行叠加——反序覆盖陷阱本波实测一次〕），`.tmp/g13_4_pristine/` 快照集纳入 pr-smoke.yml + ci/g10_gap_registry_lib.py；最终重放→刷快照→互锁单命令链压死竞态窗口，守卫套件真跑复核全 PASS 留痕；`.tmp/g13_4_replay.py` + `.tmp/g13_4_registry_isolation.py`（登记表正典化隔离验证：22:21 实测超容差面 8 行合成 GREEN + digest 取材在盘核验 + registry_name 双向臂）均为一次性工具不入 commit。
  - **帧率/GI 基线诚实登记（不设通过线重申）**：M-c 帧率基线（UE MRQ 449.78~762.69 ms/帧 vs Rurix 29.44~2205.81 ms/帧）与 M-d GI 对拍差（双场景 OVER 全登记）均为 measured_local 登记面——不构成任何「已达 UE5 画质/帧率」语义；M-d 双场景 indirect_ssim ≤0.034 为真实物理口径差（UE Lumen cornell 巨大尺度零贡献/bistro IBL 面 vs Rurix 单反弹+颜色溢出），G13.5 P2 穷举与 G14 性能面对标期承接；**登记表字节钉面运营注记**：bistro UE MRQ 帧跨轮微漂移（222114 vs 232935 两轮 t50 tsr ssim_delta 0.0001189→0.0001022 实测）——登记表在树逐字节钉定面下，未来本地全量重跑须先移除在树登记表走首落盘路径（或有意识更新登记），漂移即 RED 为设计内绊线非缺陷。
  - **not-triggered / 维持 open 面**（如实保持 open，不写进全绿叙述）：M52 / M100-high / RD040-nrd 维持 defer；G10-N11/N16/G11-N3/M114-strand 锚定 G14；G11-N8/N9/G12-N10/G12-N12 锚定 G15（G12 gap registry 只消费不回写）；G13-N6 异己面 no-go；G13-N7 FG/MFG defer-to-G14+；SG-010 软保留 not_triggered 维持；M-e 未开工（G13.5 波次面——P2 穷举 + soak）；G14/G15 未立项。
  - **异己并发工作树面**：本批只含 G13.4 车道文件（块⑤清单按文件名显式择取）；异己会话 src/ 未提交面（apps/uc06-renderer、apps/uc08-physics、src/rurix-asset/{lib.rs,ktx2_read.rs}、src/rurix-render/{Cargo.toml,lib.rs,geometry/,gi/,shadow/,ssr/,bin/g10_m134_frame_capture.rs,bin/g9_m95_visbuffer_swhw.rs}、src/rurix-rt/render_exec.rs、milestones/g12/g12_pt_sampler_selection.json 及各 evidence/ 异己新件〔g9/g10/g11/g12 波次件 + g13 治理面件〕）维持未提交、零消费、零混入（立项裁决 1——git add 按文件名显式择取）。

- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10/G11/G12 §8.x 同模）。`Assisted-by: Kimi-K3（G13.4 UE 对拍波）`（影响范围：spec/visual_comparison.md〔RXS-0405/0406 新条款 + 修订记录行〕+ conformance/visual_comparison/{accept/ue_upscale_parity_contract_minimal.rx,accept/ue_lumen_gi_parity_contract_minimal.rx,reject/upscale_parity_digest_mismatch_report.rx,reject/upscale_fps_baseline_masquerade.rx,reject/lumen_parity_digest_mismatch_report.rx,reject/lumen_gap_silent.rx} 六新建 + milestones/g13/g13_ue_{upscale,lumen_gi}_parity_contract.json 双契约新建 + milestones/g13/harness/{g13_4_ue_render.py,ue_python/g13_4_build_scenes.py,ue_python/g13_parity_contract.py} 三新建 + src/rurix-render/src/bin/g13_4_ue_upscale_parity_render.rs 新建〔forbid(unsafe_code)〕+ src/rurix-asset/src/bin/g10_5_scene_render.rs〔--gi-off 加性旗标〕+ ci/g13_ue_upscale_parity_smoke.py + ci/g13_ue_lumen_gi_parity_smoke.py + ci/g13_wave4_exit_check.py 三新建 + ci/g10_gap_registry_lib.py〔validate_registry 加性 registry_name 参数，G10 消费面 0-byte〕+ ci/budget_eval.py〔eval_g13_dual_seed 专用判读面纯追加〕+ ci/check_schemas.py〔G13.4 新五前缀 load/validator/路由三处纯追加〕+ milestones/g13 五 evidence schema〔g13_m_c_ue_upscale_parity_/g13_m_c_measured_entry_/g13_m_d_ue_lumen_gi_parity_/g13_m_d_measured_entry_/g13_wave4_exit_evidence_schema.json〕+ milestones/g13/g13_ue_upscale_gap_registry.json + g13_ue_lumen_gap_registry.json 两新建〔门产登记表正典形〕+ milestones/g13/g13_budget.json〔六条目纯追加 measured_local〕+ .github/workflows/pr-smoke.yml〔步骤 240~242，步骤 239 块后追加〕+ registry/number_ledger.json〔RXS 404→406/next_free 407 + CI_step 239→242/next_free 243 + revision_log v1.138〕+ tests/stable/{stable_api.snapshot,bless_log.md}〔386→388 重 bless + 追加行〕+ conformance/traceability_matrix.{json,md}〔388 全锚定重生成〕+ 本契约 §8.5 本条 + evidence/g13_m_c_ue_upscale_parity_20260818T232935Z + g13_m_c_calibration_{ssim,flip,noise}_*_20260818T203510Z〔budget 在档引用面〕+ g13_m_d_ue_lumen_gi_parity_20260819T002556Z + g13_m_d_calibration_{gi_energy,indirect_ssim,indirect_flip}_*_20260819T001914Z〔budget 在档引用面〕+ g13_wave4_exit_20260819T003048Z 真跑件；验证方式：块③逐字命令输出——M-c 门 PASS 20/20 + M-d 门 PASS 19/19 + 波聚合门 VERDICT=PASS + 三 selftest 红绿留痕 + gaplib/M140 selftest 零回归 + 守卫套件全 PASS + 互锁 --require-ready READY）。
