---
contract: G14
title: G14 正式帧率对标与渲染管线性能期
status: active
implementation_status: unlocked
active_scope: g14_1_governance_only
version: v1.0
date: 2026-08-19
timebox: "G14.1 治理波即刻执行（G13 已 closed）；G14.2~G14.5b 严格波次，工期在实现互锁开放后由 measured baseline 校准"
rfc_required: "G14.1 治理波零 RFC 消费——本波只落治理资产，RFC 命名空间 0-byte（实测 next_free=30 维持）。G14.2+ 实现波若触冻结面（UpscaleBackend trait 签名面 / temporal 底座历史接口面 / RXS-0357 参照器面 / G13 锁定双差距登记表终态 / G13 既有门判据语义）必须独立 Full RFC 经 D-409 对抗性评审后 Agent Approved，编号按起草时实测 registry/number_ledger.json namespaces.RFC next_free 领取，禁推测号；判档争议向上取严（10 §3）。M-c/M-d 门登记表对账结构化修订 = 门脚本内部健壮性修订面（§8 只追加验收记录口径，沿 G13.4/G13.5a 门内修复留痕先例），登记表终态本体 0-byte 不回写"
upstream_docs:
  - "milestones/g13/G13_CONTRACT.md §8.9（G13 closed 终态，2026-08-19，flip commit f4c8da0b + tag g13-closed；超分/Lumen 双差距登记表 8+2 行终态终审锁定 = G14 法定输入）"
  - "milestones/g13/G13_P2_DECISIONS.md v1.0（31 行闭集；defer-to-G14+ 24 行承接锚 = G14 法定输入，本契约 §7 候选决策逐行承接）"
  - "milestones/g13/G13_CANDIDATE_DECISIONS.md v1.0（36 行裁决全集写法范式 + G10-N11/N16 帧率承接锚原文）"
  - "milestones/g13/g13_ue_upscale_gap_registry.json + g13_ue_lumen_gap_registry.json（8+2 行终态只消费不回写）+ milestones/g12/g12_ue_pt_gap_registry.json（10 行终态只消费不回写）"
  - "milestones/g13/G13_CONTRACT.md §8.7（UE 厂商随机运行间方差事件四跑取证——M-c/M-d 门登记表对账结构化修订承接锚 = G14 治理波必须消费行）"
  - "milestones/g10/G10_P2_DECISIONS.md §5 G10-N11/N16 行（正式帧率对标三轮进程级独立运行 + MRQ 开销剥离口径 / GPU 管线双端 A/B 帧率面——锚定 G14 字面）"
  - "milestones/g11/G11_P2_DECISIONS.md G11-N3 行（GPU 管线画质差距 measured 面锚定 G14）"
  - "milestones/g10/harness/g10_5_ue_bench.py + spec/external_reference.md RXS-0380 L2（UE 臂 B `-game -benchmark` 命令面闭集——MRQ 开销剥离测量臂法定形态）"
  - "milestones/g13/g13_budget.json（G13 measured 帧时/质量基线 13 条目 measured_local——G14 优化对照基线输入）"
  - "deep-research/r11.md（GPU 频率锁定与 benchmark 统计口径调研面）"
  - "04 P-01/P-07/P-09/P-12/P-13；10 §3/§7/§9.5；14 §1/§3/§4/§5（同 G13 口径）"
implementation_unlock:
  required_all:
    - "G14.1 治理门全部完成且有真实验证记录"
    - "ci/g14_interlock_check.py --require-ready 输出 READY（互锁 validator 机器事实，不以叙述替代）"
    - "用户 G14.2 开工指令留痕（2026-08-19 指令全期授权面——「帧率对标UE5略高（不降级画质）」字面）"
    - "共享编号按互锁开放时 actual next_free 重新校准；数字 CI 步骤不得沿用推测号与草案建议值"
in_scope:
  - g14_1_governance_only
  - candidate_decisions_and_rd_mapping
  - p0_acceptance_mapping
  - g14_governance_three_gates_materialize
  - g14_2_registry_variance_band_and_ue_bench_wave
  - g14_3_rurix_pipeline_perf_wave
  - g14_4_dual_end_fps_parity_wave
  - g14_5_p2_exhaustive_decisions_and_closeout
out_of_scope:
  - g14_2_plus_while_implementation_interlock_is_red
  - absolute_image_quality_pass_line（绝对画质通过线 = G15 商用收口期面，G14 只承载「不降级画质」回归守护面——G13 锁定对拍 deficit 基线带内不劣化，不设绝对线）
  - frame_generation_fg_mfg_independent_layer（FG/MFG 独立层另判——G14 帧率通过线 = 真实渲染帧率，不含生成帧；G13-N7 承接锚字面维持）
  - path_tracer_scope_extension（路径追踪生产化 = G12 closed 面 0-byte；G14 性能面对标消费既有 PT 吞吐基线不扩面）
  - material_chain_extension（透射/焦散/镜面 IBL 材质链 = G15 画质量级收口面锚定，G14 不承接）
  - rewriting_g5_to_g13_closed_contracts_and_00_14
  - vendor_sdk_redistribution_or_vendoring
deferred_refs:
  - "registry/deferred.json RD-034/039/040/041/042/043/044（存续 open RD；只追加禁静默改判）"
deliverables:
  - id: D-G14-1
    check: "G14.1 完成门：D-G14-1~4 齐备并通过结构/schema/ledger/guardrail 核验；验收映射无缺行；无 src/spec/conformance 语义实现、零 RFC 消费；本门通过不自动开放实现"
  - id: D-G14-2
    check: "G14_CANDIDATE_DECISIONS：G13 defer-to-G14+ 24 行承接锚逐行转引处置 + open RD 七条逐条映射 + G14 新增候选逐行裁决，零空行"
  - id: D-G14-3
    check: "G14_ACCEPTANCE_MAP：5 个 P0 独立 symbolic gate key / 稳定脚本名 / evidence schema 目标路径 / 逐字判据，与契约 §4.2 双向逐字一致"
  - id: D-G14-4
    check: "治理三门（acceptance_map / candidate_decisions / implementation_interlock）真脚本真步骤 materialize，互锁诚实输出 BLOCKED（用户开工指令留痕核验面在树前）"
acceptance_gates:
  - id: G-G14-1
    check: "G14.1 完成门：D-G14-1~4 齐备并通过结构/schema/ledger/guardrail 核验；验收映射无缺行；无 src/spec/conformance 语义实现、零 RFC 消费；本门通过不自动开放实现"
  - id: G-G14-3
    check: "实现互锁门：ci/g14_interlock_check.py --require-ready 输出 READY + 用户 G14.2 开工指令留痕（2026-08-19 指令全期授权面「帧率对标UE5略高（不降级画质）」字面）+ 共享编号按 actual next_free 重新校准。任一条件不满足均保持 implementation_status=blocked"
  - id: G-G14-4
    check: "G14.2 退出门：M-a/M-b 两个 P0 独立断言全绿——M-c/M-d 门登记表 UE 方差带结构化对账修订（G13 §8.7 承接锚兑现）+ UE benchmark 臂正式帧率测量面（臂 B 三轮进程级独立运行 + MRQ 开销剥离 measured 量化）"
  - id: G-G14-5
    check: "G14.3 退出门：M-c P0 独立断言全绿——Rurix 生产管线性能面（release 生产管线 + 逐帧回读同步/拷贝主导面消除 + 三轮进程级独立运行 measured，优化前后对照入 budget 零 estimated）"
  - id: G-G14-6
    check: "G14.4 退出门：M-d P0 独立断言全绿——双端帧率正式对标通过线（Rurix 三轮进程级独立运行 trimmed mean 帧率 ≥ UE 同口径 benchmark 臂 ×1.00 = 略高下限，逐轮守护带登记）+ 画质零降级守护（G13 锁定对拍 deficit 基线带内不劣化）"
  - id: G-G14-7
    check: "G14.5a 决策门：G14 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G15+，零空行；defer 必有承接锚；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述"
  - id: G-G14-8
    check: "G14.5a 稳定门：全部 P0 与所有 go 的 P1 全量回归；G5~G13 既有判据 0-byte；帧率对标链路连续复跑 soak（量级沿 G13.5a 继承〔≥1800s〕或 measured 证明更短足够）；strict budget 非空、零 estimated/skip；既有 76 门（G9 34 + G10 14 + G11 14 + G12 9 + G13 5）零降级"
  - id: G-G14-9
    check: "G14.5b 收口门：验收映射、候选决策、RD 最终状态逐字一致；全部 P0 独立断言均 PASS；evidence/schema/预算终审；帧率对标结果终审定盘（达标/未达标如实登记不冒充；未达标按用户授权新建 G15+ 里程碑继续优化，画质零降级守护面终态锁定）；§8 只追加后 status active→closed"
guardrails:
  - "双状态不可混同：status=active 仅表示 G14.1 governance-only 已立项；在 G-G14-3 真实通过前 implementation_status=blocked，任何治理完成叙述不得冒充 G14.2 开工"
  - "G14.1 允许 milestones/g14、G14 专属治理三门（ci/g14_*_check.py + evidence schema + workflow 步骤按 actual next_free）、G14 专属 claim、deferred history 只追加；src/spec/conformance 0-byte、零 RFC 消费；G13/G12 双差距登记表终态 0-byte 不回写"
  - "G14 P0 实现门 CI 只冻结 symbolic gate key 与脚本名；numeric_step 一律写 post-interlock actual-next-free allocation。不得沿用推测号与草案建议值，不得预放空 workflow、空脚本或空 schema 壳（G14.1 治理三门为例外：本波即落盘真脚本真步骤）"
  - "每个 P0 必须独立布尔断言与独立 evidence subject；可共享一次进程执行，但聚合 PASS 不能遮蔽任一子断言 FAIL/SKIP"
  - "缺硬件/工具链仅可 dev_env_degrade 或 SKIP=not-triggered；两者均不充 P0 绿。host oracle、mock、isolated nonzero、既有最小见证、人工截图均不能替代目标门"
  - "M-d 帧率通过线纪律：通过线 = Rurix 三轮进程级独立运行 trimmed mean 帧率 ≥ UE 同口径 benchmark 臂 ×1.00（「略高」下限——用户 2026-08-19 指令「帧率对标UE5略高（不降级画质）」字面兑现）；双端同场景同输出分辨率同超分档位同统计口径（50×3 trimmed mean 沿 M141/M165 冻结协议）；以单轮数据/混合口径/MRQ 含开销数据冒充正式对标即 RED"
  - "画质零降级守护纪律：M-d 画质面 = G13 锁定对拍 deficit 基线带内不劣化（经 M-a 修订后方差带结构化对账面复跑核验）；G14 不设绝对画质通过线（归 G15 商用收口期）；优化致画质劣化静默即 RED"
  - "对标范围唯一法定来源：G13 立项前调研报告帧率面 + G13_P2_DECISIONS 24 行承接锚 + G12/G10 法定输入（G10-N11/N16、G11-N3、M114-strand、M100-high 帧率/性能窗）+ G13 §8.7 结构性修复承接锚；G14 不得无锚新立项；新发现差距进差距登记显式登记 + G14.5a 穷举，不得静默混入"
  - "G13/G12 gap registry 只消费不回写：g13_ue_upscale_gap_registry.json 8 行 + g13_ue_lumen_gap_registry.json 2 行 + g12_ue_pt_gap_registry.json 10 行终态 0-byte；G14 新产差距另立新表（milestones/g14/ 新文件），不回写 G12/G13 表"
  - "M165 漂移监控登记条款（G12-N13 承接）：G14 复跑面检出同型 digest 漂移即如实登记并升级评估（升级 = 生产化缺陷修复项 + Full RFC 评估）；零检出维持 open-defer 不写进全绿叙述"
  - "既有 76 门零降级：G9 34 key + G10 14 key + G11 14 key + G12 9 key + G13 5 key 绿面 0-byte；G5~G13 closed 契约与判据 0-byte；回归门独立 P0 断言（M-e）；M96 golden 门序机器阻断（D2-Q7）维持"
  - "UpscaleBackend/temporal 底座 0-byte 不接线（RD-041/RD040 承接锚口径）：G14 性能面优化经既有接口面进行，trait 签名与 temporal 底座历史接口面 0-byte；确需演进必须独立 Full RFC 显式修订行"
  - "UE 源码仅外部参照只读（F:\\UE_5.8 与 E:\\Kimi_Agent_Taichi Engine 优化计划\\references\\UnrealEngine 双树），零 vendoring、零片段复制进 src/spec；违反即 revert + 留痕（RFC-0027 字面）"
  - "主腿 = Vulkan RayQuery（M96 device 面）；DXIL RT blocked 维持（RD-034）；benchmark 臂 = spec RXS-0380 L2 臂 B `-game -benchmark` 命令面闭集内形态，schema 外开关注入即 fail-closed"
---

# G14 正式帧率对标与渲染管线性能期 契约

> 本契约是 G14 里程碑唯一事实源。front matter 双状态机：`status`（治理激活）与 `implementation_status`（实现解锁）严格分离。

## 1. 目标与双门状态

**目标（用户 2026-08-19 指令字面兑现面）**：「优化渲染管线效率，使帧率对标UE5略高（不降级画质）」——正式帧率对标（三轮进程级独立运行 + MRQ 开销剥离 + GPU 管线双端 A/B，G10-N11/N16/G11-N3 承接锚兑现）+ Rurix 生产管线性能优化（G13.4 登记的 debug+逐帧回读同步口径倒挂面消除）+ 帧率通过线（Rurix ≥ UE 同口径 ×1.00 = 略高下限）+ 画质零降级守护（G13 锁定对拍 deficit 基线带内不劣化）。「UE5 级」可核对基线沿用 G8 口径 = UE 5.8（G9_CONTRACT §1 字面；本机 UE 5.8.1-56057345 == M128 登记机核继承）。

**双门状态**：`status: active`（G14.1 governance-only）+ `implementation_status: blocked`（G-G14-3 事实互锁未过前 G14.2+ 禁止开工）。

## 2. 范围与波次

- **G14.1 治理波**（本波）：契约三件套 + 候选决策表（G13 defer 24 行逐行承接 + open RD 7 条映射 + G14 新增候选）+ 验收映射 5 P0 + 治理三门 materialize + 互锁诚实 BLOCKED。
- **G14.2 修订与测量波**（M-a + M-b）：M-c/M-d 门登记表 UE 方差带结构化对账修订（G13 §8.7 承接锚兑现——身份面逐字节 + Rurix 侧位级 + UE 侧程序产方差带）+ UE benchmark 臂正式帧率测量面（臂 B `-game -benchmark` 三轮进程级独立运行 + MRQ 开销剥离 measured 量化）。
- **G14.3 Rurix 管线性能波**（M-c）：生产管线（release + 逐帧回读同步/拷贝主导面消除）+ 三轮进程级独立运行 measured + 优化前后对照入 budget。
- **G14.4 双端对标波**（M-d）：同场景同档帧率 A/B 正式对标（通过线 = 略高下限）+ 画质零降级守护（G13 基线带内不劣化）。
- **G14.5a 决策+稳定波**：P2 穷举决策 + M-e 回归门 + stabilization soak。
- **G14.5b close-out**：终审八 facts + status flip 独立 commit + g14-closed tag。

## 3. 治理波交付物（D-G14-1~4）

见 front matter deliverables / acceptance_gates 逐字判据；本波零 RFC 消费、零 src/spec/conformance 语义实现。

## 4. P0 独立断言表

### 4.1 统一纪律

接入/落盘 + 冻结面 0-byte（UpscaleBackend trait 签名面与 temporal 底座历史接口面 / G13 锁定双差距登记表终态 / G11 GI 既有判据 / M96 golden 门序 D2-Q7）+ measured 面标定程序产阈禁手写（P-09）+ 不降级既有 76 门绿面。**G14 帧率通过线 = Rurix ≥ UE 同口径 ×1.00（略高下限）**；**G14 不设绝对画质通过线**（归 G15）。

### 4.2 五行 P0

| M 行 | symbolic gate key / 稳定脚本 | 独立硬判据（逐字） | 最晚波次 |
|---|---|---|---|
| M-a | `g14.p0.m_a.registry_variance_band_reconciliation`<br>`ci/g14_registry_variance_band_reconciliation_smoke.py` | M-c/M-d 门登记表 UE 方差带结构化对账修订（G13 §8.7 承接锚兑现）：身份面（gap_id 集/场景集/metric/kind/模块归属/行数）逐字节 + Rurix 侧测量值位级一致 + UE 侧测量值程序产方差带（门内 UE 探针格双跑方差底 ×headroom 程序产禁手写 P-09，真实内容变更 ≫方差带检出面维持）+ 修订后 M-c/M-d 全门复跑双绿（登记表在树态复跑不再误报厂商随机方差）+ RED 双臂（UE 侧大方差注入检出 / 小方差带内吸收）+ G13 锁定双登记表 8+2 行终态 0-byte 不回写 + UE 确定性控制面调研结论登记（cvar/收敛面，压缩方差底）；方差带手写冒充程序产即 RED；身份面漂移静默即 RED；修订后 M-c/M-d 复跑仍误报即 RED | G14.2 |
| M-b | `g14.p0.m_b.ue_benchmark_arm_measurement`<br>`ci/g14_ue_benchmark_arm_measurement_smoke.py` | UE 侧 benchmark 臂正式帧率测量（G10-N11 承接锚兑现）：臂 B `-game -benchmark` 命令面闭集（RXS-0380 L2）双场景（cornell-box + bistro-interior）× 超分档三轮进程级独立运行 measured（进程冷启动逐轮独立，缓存冷热面登记）+ MRQ 开销剥离 measured 量化（同场景 MRQ 臂 frameRenderDuration vs benchmark 臂帧时差值 = 捕获合并开销 measured，G10-N11 口径字面兑现）+ 环境画像七元组 + 锁频/时钟面登记（provenance 闭集沿 RXS-0380 L3）+ 50×3 trimmed mean 统计协议（M141/M165 冻结口径）入 g14_budget（measured_local 零 estimated）；以 MRQ 含开销数据冒充 benchmark 臂即 RED；单轮冒充三轮即 RED；estimated 冒充 measured 即 RED | G14.2 |
| M-c | `g14.p0.m_c.rurix_pipeline_perf`<br>`ci/g14_rurix_pipeline_perf_smoke.py` | Rurix 生产管线性能面：release 生产管线全链路帧时（G13.4 登记的 debug 构建 + 逐帧回读同步口径倒挂面〔tier67 > tier100 host 拷贝/同步主导〕消除——异步回读/提交面重叠 + 逐帧同步消除 + TSR device kernel 效率面）+ 双场景三后端三轮进程级独立运行 50×3 trimmed mean measured 入 g14_budget（零 estimated）+ 优化前后 measured 对照（G13 g13_budget 帧时基线条目为优化前锚）+ 固定 seed 位级确定性协议维持 + temporal 底座 0-byte；host 侧逐帧拷贝/同步主导倒挂未消除静默即 RED；estimated 冒充 measured 即 RED；确定性协议漂移即 RED | G14.3 |
| M-d | `g14.p0.m_d.dual_end_fps_parity`<br>`ci/g14_dual_end_fps_parity_smoke.py` | 双端帧率正式对标 + 画质零降级守护（G10-N16/G11-N3 帧率面兑现 + 用户「帧率对标UE5略高（不降级画质）」字面兑现）：同场景同输出分辨率同超分档位 GPU 管线双端 A/B（UE 臂 = M-b benchmark 臂测量面；Rurix 臂 = M-c 生产管线路径）三轮进程级独立运行 + 通过线 = Rurix 三轮 trimmed mean 帧率 ≥ UE 同口径 ×1.00（略高下限，逐轮守护带登记）+ 画质零降级守护（G13 锁定对拍 deficit 基线——经 M-a 修订后方差带结构化对账面复跑核验不劣化；G14 不设绝对画质通过线归 G15）+ 对标差距/未达标项显式登记 g14 差距登记表（不静默混入）；以单轮/混合口径/MRQ 含开销数据冒充正式对标即 RED；画质劣化静默即 RED；未达标冒充达标即 RED（未达标如实登记不阻塞 G14.5a 穷举，商用收口判定归 G15+） | G14.4 |
| M-e | `g14.p0.m_e.regression_drift_guard`<br>`ci/g14_regression_drift_guard_smoke.py` | 回归门 + 漂移监控：既有 76 门（G9 34 key + G10 14 key + G11 14 key + G12 9 key + G13 5 key）最新 evidence 全绿只读汇总（聚合不遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE）+ G14 触改面既有门重跑回归零降级（M96 golden 门序面真跑抽检 + M-c/M-d 修订门复跑面）+ M165 漂移监控登记（G14 复跑面同型 digest 漂移检出计数/零检出字面入 evidence，FAIL 件 0-byte 保留纪律继承）；既有门降级即 RED；聚合遮蔽即 RED；漂移检出未登记即 RED | G14.5a |

任一行缺失、合并后不可区分、非 PASS 或无对应 evidence schema，均阻断 G14.5b。**M-d 帧率通过线为 G14 唯一新设通过线**（略高下限字面）；G14 不设绝对画质通过线（归 G15 商用收口期）。

## 5. Guardrails

见 front matter guardrails 逐字（双状态不可混同 / 数字步骤延迟分配 / P0 独立断言 / M-d 帧率通过线纪律 / 画质零降级守护纪律 / 对标范围唯一法定来源 / G13+G12 gap registry 只消费不回写 / M165 漂移监控 / 既有 76 门零降级 / UpscaleBackend+temporal 底座 0-byte / UE 源码只读 / Vulkan 主腿 + 臂 B 命令面闭集）。

## 6. Deferred 处置

RD-034/039/040/041/042/043/044 七条总体 status 全维持 open（条目级四字段 0-byte；分项 go/defer 由 G14_CANDIDATE_DECISIONS 与 deferred history 只追加留痕）；G14.1 治理门登记 history 只追加（RD-039 +1〔M61〕/ RD-040 +1〔M100-high G14 窗登记〕/ RD-041 +1〔G13-N7 FG/MFG G14 重评窗结论登记〕，以落盘为准）。

## 7. 修订与开工裁决

1. **立项裁决**：现在立项；G14.0 不可变 ref = `f4c8da0b`（G13 close-out flip commit，tag `g13-closed`，立项时实测 HEAD；沿 G13.0 取 G12 flip commit 先例同模）。工作树带异己会话 src/ 未提交面——处置见裁决 1（沿 G10/G11/G12/G13 先例：异己面保持不混入 G14 车道、严禁消费）。
2. **用户开工指令留痕**：2026-08-19 用户指令全期授权面——「彻底完成对标UE5渲染器的目标……同时优化渲染管线效率，使帧率对标UE5略高（不降级画质）……最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」（本会话留痕）；G14.2 开工 = G-G14-3 互锁 READY 后由本授权面承载（「帧率对标UE5略高（不降级画质）」字面对齐）。
3. **同日放行先例继承**：5a full-run 先行完成后允许同日进 5b close-out（沿 G9.8b/G10.8b/G11.7b/G12.7b/G13.5b 先例链）。
4. **编号纪律**：治理三门步骤按落盘前实测 actual next_free 顺位领取（当前实测 CI_step next_free=247）；P0 实现门一律 post-interlock actual-next-free allocation。
5. **异己并发工作树面**：G14 带未提交项立项（2026-08-19 git status 实测异己面同 G13 期登记 + 异己 evidence 新件），保持不混入 G14 车道、严禁消费；G14 车道 commit 只含本车道文件（按文件名显式择取）。
6. **并发会话共享面纪律**：check_schemas.py / pr-smoke.yml / number_ledger.json 共享面经幂等重放脚本落盘 + 提交前单命令链压死竞态窗口（G13.4~G13.5b 四波先例同模，一次性工具 .tmp 不入 commit）。
7. **M-d 未达标诚实面**：帧率通过线未达标 = 如实登记不冒充（G-G14-6 不充绿叙述面）；按用户授权（「最终交付产物需要真实可商用，否则不要停止优化……允许在G15后无限制新建里程碑继续优化」）继续优化——G15 画质收口期前可按只追加程序新建 G14.x 延续波或 G16+ 里程碑承载继续优化面。

## 8. Implementation activation / Close-out（只追加区）

<!-- 首条未来记录只能是 G14.1 治理波验收与 G-G14-3 互锁实测面；其后追加逐波验收与 close-out。当前不得写 PASS、不得预填 run URL。 -->

### §8.1 G14.1 治理波验收记录（2026-08-19）——G-G14-1/G-G14-2 字面兑现：契约三件套 + 候选决策 31 行闭集 + 治理三门 materialize（步骤 247~249 实测领取）；互锁诚实 VERDICT=BLOCKED（workflow/ledger 接线面落盘前留痕）→ 接线后 READY（互锁解锁记录见后续 §8 子段）

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g14.wave.1.acceptance_map`（步骤 247） | G14_ACCEPTANCE_MAP §1 五行 P0 闭集全等（M-a 登记表 UE 方差带结构化对账修订 / M-b UE benchmark 臂正式帧率测量 / M-c Rurix 生产管线性能面 / M-d 双端帧率正式对标+画质零降级守护 / M-e 回归门+漂移监控）+ §2 零 go P1 空集 + 单一命名空间同 slug + numeric_step 全列 post-interlock 字面零预占 + MAP §1 ↔ CONTRACT §4.2 双向逐字一致 + 零空行 | host | evidence/g14_acceptance_map_check_20260819T131824Z.json（12/12 facts） | PASS |
  | `g14.wave.1.candidate_decisions`（步骤 248） | G14_CANDIDATE_DECISIONS 31 行闭集全等（§1 G13 defer 24 行——G10-N11/G10-N16 go 兑现窗 + 22 行 defer-to-G15+〔六行 G14 窗结论 + G11-N3 部分兑现 + G13-N7 G14 重评窗不立项如实登记〕；§2 open RD 7 行映射；§3 G14 新增 7 行）+ 裁决枚举合法 + 零空行 + 承接锚纪律 + defer 行 G15+ 重评窗字面 + go 行验收映射锚义务 + RD 条目级 status 全 open + MAP 5 key 互斥 | host | evidence/g14_candidate_decisions_check_20260819T131825Z.json（43/43 facts） | PASS |
  | `g14.gov.implementation_interlock`（步骤 249） | 事实门四项（① G13 closed + §8.9 签署块 + G14.0 不可变 ref f4c8da0b 登记；② 候选决策 31 行零空行 + deferred history 只追加 + MAP §1/§2 无缺行；③ 用户开工指令留痕「帧率对标UE5略高（不降级画质）」字面 + workflow 实测末号 == ledger on_tree_max 且 next_free == on_tree_max+1；④ 治理两门独立 PASS）+ 一致性门 C1~C4 + closed 三态 | host | evidence/g14_interlock_check_20260819T131031Z.json（**VERDICT=BLOCKED 诚实留痕**——③中 workflow/ledger 接线面在树前状态；接线后复跑见 §8.2） | BLOCKED（诚实） |

- **② 治理三门实测输出**：`py -3 ci/g14_acceptance_map_check.py --gate g14.wave.1.acceptance_map` → **VERDICT=PASS，exit=0**（12 facts）；`py -3 ci/g14_candidate_decisions_check.py --gate g14.wave.1.candidate_decisions` → **VERDICT=PASS，exit=0**（43 facts）；`py -3 ci/g14_interlock_check.py --gate g14.gov.implementation_interlock` → **VERDICT=BLOCKED**（诚实留痕件 131031Z：fact ③ 红 = 治理三门 247~249 workflow/ledger 接线面在树前——validator 能识别阻断不充绿，沿 G13.1 BLOCKED 先例）。
- **③ 验收命令逐字输出（2026-08-19 真跑留痕，仓库根目录）**：
  - `py -3 ci/g14_acceptance_map_check.py --selftest` → **SELFTEST PASS（9 RED + 1 GREEN + 真表臂），exit=0**。
  - `py -3 ci/g14_candidate_decisions_check.py --selftest` → **SELFTEST PASS（10 RED + 真表/合成双臂 GREEN），exit=0**。
  - `py -3 ci/g14_interlock_check.py --selftest` → **SELFTEST PASS（17 RED + 1 GREEN + 1 TREE），exit=0**（开工指令字面缺失→③红 / workflow 末号不一致→③红 / 数字预占注入→C3 FAIL 退 1 等全检出）。
  - 起草期 FAIL 轨迹（诚实留档不删）：g14_acceptance_map_check_20260819T130605Z.json（首轮 slug 机核面 FAIL）+ g14_acceptance_map_check_20260819T131711Z.json（并发会话回退 MAP M-a 脚本名致 two_way 漂移 FAIL——重放恢复后 131824Z 转绿；并发回退偏差沿 G13.4~G13.5b 同模登记）。
  - 守卫套件全 PASS（逐字输出行）：`py -3 ci/check_structure.py` → `[check_structure] PASS`；`py -3 ci/check_schemas.py` → `[check_schemas] PASS`（本批新增 g14 治理三前缀路由）；`py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS`（CI_step on_tree_max 249 / next_free 250 校准后实测 + reserved_in_flight[G14] 登记）；`py -3 ci/budget_eval.py --strict` → PASS（212 pass / 0 skip）。
- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G13 closed（f4c8da0b + tag g13-closed）→ 本波 G-G14-1/G-G14-2 治理门兑现；数字步骤 247~249 按落盘前实测 actual next_free=247 顺位领取（ledger v1.142 校准同批）；P0 实现门 symbolic key 零数字 claim（post-interlock actual-next-free allocation 字面维持）。
  - **slug 一致性修订留痕**：MAP §1 起草时 M-a/M-b 脚本名取 key 末段缩写形态（g14_registry_variance_band_smoke.py / g14_ue_benchmark_arm_smoke.py），与 §1「脚本一律 ci/g14_<slug>_smoke.py（slug 与 key 末段同字面）」纪律字面冲突——按纪律面向上取严修订为全 slug 形态（ci/g14_registry_variance_band_reconciliation_smoke.py / ci/g14_ue_benchmark_arm_measurement_smoke.py），契约 §4.2 + MAP §1 + 门脚本内嵌三面同步；首轮 FAIL 件 130605Z 与并发回退 FAIL 件 131711Z 在档 0-byte。
  - **候选决策窗结论如实登记（不写进全绿叙述）**：M100-high（G14 窗 = 未齐备）/ M114-strand（G14 窗 = 数据面部分落地）/ G10-N17（G14 窗 = 未消费）/ G11-N5（G14 窗 = 未齐备）/ G13-N7（G14 重评窗 = 不立项——真实渲染帧率口径字面）/ G11-N3（部分兑现 = A/B 出图面 M-d 承载，画质差距清单锚定 G15）。
  - **异己并发工作树面**：本批只含 G14 车道文件（按文件名显式择取）；异己会话 src/ 未提交面维持未提交、零消费、零混入（立项裁决 1）；共享面（check_schemas/workflow/ledger/deferred）经 `.tmp/g14_1_replay.py` 幂等重放落盘 + 提交前单命令链压死竞态窗口（一次性工具不入 commit）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G13 §8.x 同模）。`Assisted-by: Kimi-K3（G14.1 治理波）`（影响范围：milestones/g14/ 三件套新建〔G14_CONTRACT.md v1.0 + G14_CANDIDATE_DECISIONS.md v1.0 + G14_ACCEPTANCE_MAP.md v1.0〕+ ci/g14_acceptance_map_check.py + ci/g14_candidate_decisions_check.py + ci/g14_interlock_check.py 三新建 + milestones/g14 三 evidence schema 新建 + ci/check_schemas.py〔g14 治理三前缀 load/validator/路由三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 247~249，步骤 246 块后追加〕+ registry/number_ledger.json〔CI_step 246→249/next_free 250 + reserved_in_flight[G14] + revision_log v1.142〕+ registry/deferred.json〔RD-039 +1/RD-040 +1/RD-041 +1 history 只追加〕+ 本契约 §8.1 本条 + evidence/g14_acceptance_map_check_{130605Z,131711Z,131824Z} + g14_candidate_decisions_check_20260819T131825Z + g14_interlock_check_20260819T131031Z 真跑件；验证方式：块③逐字命令输出——治理双门 PASS + 互锁诚实 BLOCKED 留痕 + 三 selftest 红绿留痕 + 守卫套件全 PASS）。

### §8.2 G-G14-3 实现互锁解锁记录（2026-08-19）——implementation_status blocked→unlocked：事实门四项全绿 + 一致性门 C1~C4 全绿 + 用户开工指令留痕核验 + 共享编号 actual next_free 校准在案；G14.2 修订与测量波开工面开放

- **解锁前状态**：G14.1 治理批（dab50472）落盘后 `py -3 ci/g14_interlock_check.py --require-ready` → **VERDICT=READY，exit=0**（事实门四项全绿：① G13 closed + §8.9 + G14.0 ref f4c8da0b 登记；② 候选决策 31 行零空行 + deferred history 只追加 + MAP 无缺行；③ 用户开工指令留痕「帧率对标UE5略高（不降级画质）」字面在树 + workflow 实测末号 249 == ledger on_tree_max 249 == next_free−1 一致 + 治理三门 247~249 接线面在树；④ 治理两门独立 PASS——一致性门 C1~C4 全绿）。用户 G14.2 开工指令 = 2026-08-19 全期授权面（本契约 §7 裁决 2 逐字登记），共享编号按 actual next_free 校准（ledger v1.142）。
- **解锁动作**：front matter `implementation_status: blocked → unlocked`（洁净单行翻转）；`status: active` 维持（治理激活面不动）；G14.2 修订与测量波（M-a 登记表 UE 方差带结构化对账修订 + M-b UE benchmark 臂正式帧率测量）开工面开放。
- **纪律面**：解锁后 C3/C4 治理期口径自动不适用（两态口径沿 G10.4b~G13 先例，判据语义 0-byte）；P0 实现门数字步骤仍按各实现波落盘前实测 actual next_free 顺位领取（禁推测号维持）；异己面纪律 0-byte。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G13 §8.2 同模）。`Assisted-by: Kimi-K3（G14.1 治理波）`（影响范围：本契约 front matter `implementation_status` 字段 + §8.2 本条 + evidence/g14_interlock_check_<UTC> READY 真跑件；验证方式：`py -3 ci/g14_interlock_check.py --require-ready` VERDICT=READY exit=0 + 守卫套件全 PASS）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-19 | 首版（G14.1 治理波立项）：双门状态 + 五波结构 + 5 P0 独立断言表（M-a 登记表方差带修订 / M-b UE benchmark 臂测量 / M-c Rurix 管线性能 / M-d 双端帧率对标+画质零降级守护 / M-e 回归门+漂移监控）+ guardrails 十三条 + Deferred 处置 + 立项裁决七条。 |
