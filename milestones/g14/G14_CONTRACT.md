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

### §8.3 G14.2 修订与测量波验收记录（2026-08-19）——G-G14-4 字面兑现：M-a(M172) 登记表 UE 方差带结构化对账修订（步骤 250，10 checks 全绿）+ M-b(M173) UE benchmark 臂正式帧率测量（步骤 251，13 checks 全绿）+ 波聚合门 g14.wave.2.exit（步骤 252）VERDICT=PASS 六 facts

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g14.p0.m_a.registry_variance_band_reconciliation`（步骤 250） | G13 §8.7 承接锚逐字兑现：gaplib 正典单源结构化对账面在树（`reconcile_registry_structured`：身份面逐字节 + Rurix 侧/结构常量位级 + UE 侧程序产方差带）+ selftest 绿 + M-c/M-d 双门脚本接线面机核（旧「在树非逐字节相等」字节冻结面移除字面）+ **修订后 M-c/M-d 全门复跑双绿**（子进程真跑，UE MRQ + Rurix device 双臂，evidence 151636Z/161339Z 新鲜 PASS）+ G13 锁定双登记表 8+2 行终态 0-byte（复跑前后逐字节一致机核）+ UE 方差带程序产入 g14_budget 两条目（upscale band_rel=0.00767988 / lumen band_rel=0.00217574，门内三样本 max 两两相对差 ×2.0，禁手写 P-09）+ RED 四臂独立有效（UE 大方差检出/带内吸收/Rurix 位级检出/身份面检出）+ UE 确定性控制面调研结论登记 + budget_eval 全 PASS | host+device（复跑子进程真跑面） | evidence/g14_m_a_registry_variance_band_reconciliation_20260819T151636Z.json（10/10） | PASS |
  | `g14.p0.m_b.ue_benchmark_arm_measurement`（步骤 251） | G10-N11 承接锚逐字兑现：臂 B `-game -benchmark` 命令面闭集（RXS-0380 L2 + G14.2 探针链实证形态——CsvProfile 双臂逗号分隔〔HandleCSVProfileCommand 每调用只处理 Args[0] 源码实证〕+ Windows 原始命令行字符串传参〔list2cmdline 重引号陷阱实证〕+ benchmark 虚拟步进时序 + frames 完成落盘窗）+ 双场景 × 三超分档 × 三轮进程级独立运行（逐轮独立 UE 子进程 + 命令 digest 归一面 + CSV 逐轮互异）+ **MRQ 开销剥离 measured 量化**（同场景同档 MRQ 臂 frameRenderDuration〔M-c evidence 只消费面〕− benchmark 稳态帧时逐格差值：cornell 771~853ms / bistro 897~948ms 捕获合并开销实测）+ 环境画像七元组 + DLSS engagement NGX SrcRect→DestRect 档位读回机核 + 契约相机 auto-activation 对齐读回 + 50×3 trimmed mean 跨轮中位数入 g14_budget 六条目（cornell 2.039~2.179ms / bistro 3.124~4.243ms measured_local 零 estimated）+ RED 三臂独立有效 | host+device（UE 子进程真跑面） | evidence/g14_m_b_ue_benchmark_arm_measurement_20260819T165451Z.json（13/13） | PASS |
  | `g14.wave.2.exit`（步骤 252） | 波聚合门只读汇总六 facts：G13 锁定双登记表 0-byte（在树 == HEAD 逐字节）+ 双门 RED 臂独立有效（6 臂）+ g14_budget 8 条目 measured_local 零 estimated + budget_eval 全 PASS（220/0-skip strict）+ M-a 承接锚兑现面 + M-b 三轮独立性+开销剥离 6 格 + G5~G13 closed 面 0-byte（vs G14.0 ref f4c8da0b committed diff 闭集 = {g10_gap_registry_lib.py / g13_ue_upscale_parity_smoke.py / g13_ue_lumen_gi_parity_smoke} 授权面，工作树闭集 = {g12_pt_sampler_selection.json 异己登记}） | host 只读（不重跑子门） | evidence/g14_wave2_exit_20260819T170842Z.json（六 facts 全绿） | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g14_wave2_exit_check.py --gate g14.wave.2.exit` → **VERDICT = PASS，exit=0**（六 facts 逐行打印不遮蔽）；`py -3 ci/g14_wave2_exit_check.py --selftest` → ALL PASS（负样本缺 evidence 红 + 真树聚合不遮蔽机核）。
- **③ 验收命令逐字输出（2026-08-19 真跑留痕，仓库根目录）**：
  - `py -3 ci/g14_registry_variance_band_reconciliation_smoke.py --selftest` → selftest PASS（schema 闭集 + 2 函数面臂）；`--gate` → VERDICT=PASS checks=10/10（M-c/M-d 复跑子进程 exit=0 status=pass fresh=True 双绿）。
  - `py -3 ci/g14_ue_benchmark_arm_measurement_smoke.py --selftest` → selftest PASS（schema 闭集 + 3 函数面臂）；`--gate` → VERDICT=PASS checks=13/13（18 轮 UE benchmark 子进程真跑全绿）。
  - `py -3 ci/g10_gap_registry_lib.py --selftest` → SELFTEST PASS（10 RED + 1 GREEN 既有面 + 结构化对账 5 RED + 2 GREEN 新面）。
  - 守卫套件全 PASS：`py -3 ci/check_structure.py` → PASS；`py -3 ci/check_schemas.py` → PASS（本批新增 g14_m_a/m_b/wave2 五前缀路由）；`py -3 ci/check_number_ledger.py` → PASS（CI_step on_tree_max 252/next_free 253 校准后实测）；`py -3 ci/budget_eval.py --strict` → PASS（220 pass/0 skip）；`py -3 ci/g14_interlock_check.py --require-ready` → VERDICT=READY。
  - 起草期 FAIL 轨迹（诚实留档不删）：M-a 首跑 135504Z（budget 腿 evidence_file 形态 KeyError——budget_eval 默认路读 results.trimmed_mean，修订 = g14.ue_variance_band 分派支 + measured-entry 件面）+ M-b 首跑 163334Z（CSV 元数据尾解析 `",".join` 逐字符陷阱 + env capture_arm 缺键）/ 164425Z（同面二跑）+ wave2 首跑 170522Z（提交前 legacy_0byte 工作树态 FAIL——批次 A 落盘后转绿 170842Z；并发/序贯面沿 G13.6 序贯教训同模登记）。
- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G14-3 互锁 READY（§8.2）→ 本波 G-G14-4 兑现；数字步骤 250~252 按落盘前实测 actual next_free 顺位领取（ledger v1.143/v1.144 校准同批）；G14.3 Rurix 管线性能波（M-c）开工面开放。
  - **探针链实证留痕（G14.2 测量面奠基）**：.tmp 探针 1~8 + 相机对齐探针——①`stat unit` 不落逐帧日志（死路）；②CsvProfile 引号丢失致裸命令空参（传参陷阱实证）；③`-seconds` 计虚拟秒（benchmark 固定步进——采集完成即退的时序面）；④processing thread 落盘窗（frames 完成后须留 ticking 窗）；⑤CsvProfile 命令每调用只处理 Args[0]（startfile 与 frames 须拆两次调用，逗号分隔多命令）；⑥`AutoActivateForPlayer` 挂 CameraActor 非 CameraComponent（UPROPERTY 面实证）；⑦benchmark 臂默认 TSR（GPU/TemporalSuperResolution 列在）、DLSS cvar 链接管后 TSR 列消隐 + NGX SrcRect 读回 = engagement 机核双面。一次性探针件不入 commit（留痕本条）。
  - **MRQ 开销量级登记（G10-N11 核心实测产出）**：同场景同档 MRQ 臂 frameRenderDuration（773~951ms/帧）vs benchmark 臂稳态帧时（2.0~4.2ms/帧）——捕获合并开销 ≈ 200~450× 管线帧时量级（逐格 measured 值入 g14_m_b evidence parity.mrq_overhead 面）；G13 M-c 帧率基线（MRQ 含开销口径）自此仅作零通过线登记面，不作任何对标输入。
  - **not-triggered / 维持 open 面**：M61/M52/M100-high/SAFE-GPU/M127/M98-l4/M114-strand/M118-hdr-cal/M125-adopt3/G10-N6/N8/N17/G11-N5/N8/N9/G12-N10/N12/N13/G13-N7/N8/N9 与 SG-010 维持 defer/留档（G14_CANDIDATE_DECISIONS §1/§5 字面 0-byte）；G14.3~G14.5b 未跑（后续波次面）。
  - **异己并发工作树面**：本批只含 G14 车道文件（按文件名显式择取）；异己会话 src/ 未提交面维持未提交、零消费、零混入（立项裁决 1）；共享面（check_schemas/workflow/ledger）经 `.tmp/g14_2_replay.py`/`.tmp/g14_2b_replay.py` 幂等重放落盘 + 提交前单命令链压死竞态窗口（一次性工具不入 commit）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G13 §8.x 同模）。`Assisted-by: Kimi-K3（G14.2 修订与测量波）`（影响范围：ci/g10_gap_registry_lib.py〔reconcile_registry_structured 加性面 + selftest 扩臂〕+ ci/g13_ue_upscale_parity_smoke.py + ci/g13_ue_lumen_gi_parity_smoke.py〔结构化对账接线 + UE 探针格标定段，§8.7 授权面〕+ ci/budget_eval.py〔g14 分派支加性〕+ ci/g14_registry_variance_band_reconciliation_smoke.py + ci/g14_ue_benchmark_arm_measurement_smoke.py + ci/g14_wave2_exit_check.py 三新建 + milestones/g14/harness/〔g14_2_ue_bench.py + ue_python/g14_2_bench_camera_align.py〕新建 + milestones/g14 五 evidence schema 新建 + g14_budget.json 首建 8 条目 + ci/check_schemas.py〔五前缀三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 250~252〕+ registry/number_ledger.json〔CI_step 249→252/next_free 253 + revision_log v1.143/v1.144〕+ 本契约 §8.3 本条 + evidence 本批真跑件〔M-a 151636Z + M-b 165451Z + wave2 170842Z + M-c/M-d 复跑件 + 标定/bench 条目件 + FAIL 轨迹四件〕；验证方式：块③逐字命令输出——双 P0 门 10/13 checks 全绿 + 波聚合门六 facts PASS + 三 selftest 红绿留痕 + 守卫套件全 PASS + 互锁 READY 维持）。

### §8.4 G14.3 Rurix 管线性能波验收记录（2026-08-20）——G-G14-5 字面兑现：M-c(M174) Rurix 生产管线性能面（步骤 253，10 checks 全绿）+ 波聚合门 g14.wave.3.exit（步骤 255）VERDICT=PASS 六 facts

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g14.p0.m_c.rurix_pipeline_perf`（步骤 253） | 契约 §4.2 M-c 逐字：release 生产管线（`g14_3_pipeline_perf`，DeviceFrameSession 持久车道——AS/场景 SSBO 常驻 + GPU timestamp telemetry + tsr session 常驻 SSBO 变体〔G13.3 kernel 0-byte 消费〕+ DLSS readback HOST_CACHED 修复〔325→13.7ms〕）+ 三后端 × 双场景 × 三档 **54 轮进程级独立运行** 50×3 measured 入 g14_budget 18 条目（零 estimated）+ **G13.4 倒挂登记面消除**（tier67>tier100 实测倒挂不再成立：全后端全场景 t50<t67<t100 正常序）+ **优化前后 measured 对照**（G13.3 tsr 基线锚：t50 1132.8→25.9ms / t67 2178.0→27.0ms / t100 1349.5→38.4ms，−97%~−98%）+ 固定 seed 双跑位级一致（cornell t67 tsr_device converged_digest sha256:e9bc79a7… 双跑同值）+ temporal 底座 0-byte（vs G14.0 ref f4c8da0b）+ G13.4 车道画质对照锚（SSIM=0.99461008 deficit 0.00539 ≤ 锚定带 0.01078 守护带复核）+ RED 三臂独立有效（kernel-tamper 重编检出〔代码面字面篡改 SPV digest 必异〕/seed-change 复跑检出/one-shot-masquerade 车道形态检出） | host+device（三后端真跑面） | evidence/g14_m_c_rurix_pipeline_perf_20260820T000025Z.json（10/10） | PASS |
  | `g14.wave.3.exit`（步骤 255） | 波聚合门只读汇总六 facts：M-c 门 RED 臂独立有效 + g14_budget M-c 18 条目 + 画质锚守护位齐备 measured_local + budget_eval 全 PASS + 倒挂消除与优化前后对照 measured 面 + 双跑位级 + temporal 底座 0-byte + G13.4 车道画质锚带守护复核 + G5~G13 closed 面 0-byte（committed diff 闭集 ⊆ 授权面） | host 只读 | evidence/g14_wave3_exit_20260820T004828Z.json（六 facts 全绿） | PASS |

- **② 波聚合门实测输出**：`py -3 ci/g14_wave3_exit_check.py --gate g14.wave.3.exit` → **VERDICT = PASS，exit=0**；`--selftest` → ALL PASS。
- **③ 验收命令逐字输出（2026-08-20 真跑留痕，仓库根目录）**：
  - `cargo build --release -p rurix-render --bin g14_3_pipeline_perf --features vendor-upscale` → Finished 绿。
  - `py -3 ci/g14_rurix_pipeline_perf_smoke.py --selftest` → selftest PASS（schema 闭集）；`--gate` → VERDICT=PASS checks=10/10（54 轮 bench 真跑 + 双跑位级 + RED 三臂真跑检出）。
  - 守卫套件全 PASS：check_structure PASS；check_schemas PASS（本批新增 g14_m_c 三前缀 + g14_wave3_exit_ 路由）；check_number_ledger PASS（CI_step on_tree_max 255/next_free 256）；budget_eval --strict PASS（239 pass/0 skip——M-c 18 条目 + 画质锚位 + 既有 220 全绿）；g14_interlock --require-ready → READY。
  - 起草期 FAIL 轨迹（诚实留档不删）：M-c 首跑 203951Z（bench 汇总行 regex 组序号错 IndexError）+ 二跑 213000Z（倒挂/temporal 两 check 调用反向——check() 语义为 cond False 即记 FAIL，修正为 not 形式）+ 三跑 221930Z（kernel-tamper 臂篡改目标落于注释字面 SPV 无异——改代码面字面 `+ 0.5 + jx` 后 SPV digest 必异实证）+ 四跑 231109Z（RED 聚合 check 调用反向——check(red_ok) 正形修正）。
- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G14-4（§8.3）→ 本波 G-G14-5 兑现；数字步骤 253~255 按落盘前实测 actual next_free 顺位领取（ledger v1.145 校准同批）；G14.4 双端对标波（M-d）开工面开放。
  - **生产车道架构裁决登记**：DeviceFrameSession 持久车道（一次性 dispatch 车道 G13.3 实测 ~160ms/帧 设备重建面不可作生产面）；内容模型 = 逐三角 albedo/emission + point/quad 灯 + 契约相机（与 G13.4 逐字同模——M-d 画质守护可比性锚，逐像素纹理面归 G15 画质窗）；GI 臂 = 直接光唯一（G13.4 内容模型同模；多反弹 GI 臂复用 g9 面不同构，不引入 host 锚不存在能量项——bin 头注 + receipt gi_arm 字段登记）。
  - **优化残留未消费位如实登记**：fence/readback N 帧流水（最小 API 扩展建议 = submit/collect 分离 + 逐 slot 独立 cmd——render_exec.rs 并发会话大改面，本波不触）/ mv host 计算 kernel 化（temporal 底座 0-byte 面不动）/ vendor evaluate 固有面（FSR 18~27ms/DLSS ~18ms@bistro 实测构成登记）/ kernel 8×8 workgroup 实测 −8~−18%（ray query 访存延迟主导非占用率瓶颈，量级预期不成立如实登记）。
  - **帧率对标前瞻登记（M-d 面）**：本波车道实测（cornell 16.6~39.8ms / bistro 149~346ms 档）vs UE benchmark 臂（cornell 2.04~2.18ms / bistro 3.12~4.24ms）——通过线 ×1.00 当前面不达，M-d 正式对标逐格判定 + 差距登记表显式登记（不冒充），结构性继续优化面（raster-primary 车道 / 零拷贝 vendor interop / present 路径口径）为 G14.x 延续波/G16+ 承接（用户授权字面）。
  - **异己并发工作树面**：本批只含 G14 车道文件（按文件名显式择取；src/rurixc 六文件 = compute_numthreads 加性面、vendor_upscale.rs = telemetry/HOST_CACHED 面、Cargo.toml = g13_4 登记块补登〔G13.4 期并发回退遗失面恢复〕+ g14_3 bin 纯追加）；异己会话 src/ 未提交面（render_exec.rs 大改/gi/restir.rs/sdf_trace.rs/shadow/smrt.rs/ssr//ktx2_read.rs 及各 mod.rs 改写面）维持未提交、零消费、零混入（立项裁决 1）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G13 §8.x 同模）。`Assisted-by: Kimi-K3（G14.3 Rurix 管线性能波）`（影响范围：src/rurix-render/src/bin/g14_3_pipeline_perf.rs 新建〔forbid(unsafe_code)〕+ src/rurix-render/kernels/g14_3_direct_gi.rx 新建 + src/rurixc/{mir.rs,mir_build.rs,iface_extract.rs,vulkan_codegen.rs,dxil_codegen.rs,dxil_spirv.rs} compute_numthreads 加性面 + src/rurix-rt/src/vendor_upscale.rs〔env 分解遥测 + pack 常驻环 + DLSS HOST_CACHED readback〕+ src/rurix-render/Cargo.toml〔g13_4 登记块补登 + g14_3 bin 纯追加〕+ ci/g14_rurix_pipeline_perf_smoke.py + ci/g14_wave3_exit_check.py 新建 + milestones/g14 三 evidence schema 新建 + g14_budget 18+1 条目 + ci/check_schemas.py〔四前缀三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 253~255〕+ registry/number_ledger.json〔CI_step 252→255/next_free 256 + revision_log v1.145〕+ 本契约 §8.4 本条 + evidence 本批真跑件〔M-c 000025Z PASS + FAIL 轨迹四件 + 18 格 measured-entry 件 + 画质锚件 + wave3 004828Z〕；验证方式：块③逐字命令输出——M-c 门 10/10 checks 全绿 + 波聚合门六 facts PASS + 守卫套件全 PASS + 互锁 READY 维持）。

### §8.5 G14.4 双端对标波验收记录（2026-08-20）——G-G14-6 诚实红面留档：M-d(M175) 双端帧率正式对标（步骤 254，8/8 checks 全绿、**通过线 0/18 格达标 → VERDICT=FAIL 诚实红不冒充**）+ 波聚合门 g14.wave.4.exit（步骤 256）VERDICT=FAIL（六 facts 全绿，红源唯一 = M-d 通过线未达，聚合不遮蔽）

- **① 独立断言清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g14.p0.m_d.dual_end_fps_parity`（步骤 254） | 契约 §4.2 M-d 逐字：同场景同输出分辨率同超分档双端 A/B 三轮进程级独立运行（UE 臂 = M-b benchmark harness 逐格复跑〔harness 子进程逐轮自持 GPU 锁 + CSV 新鲜度机核 mtime≥started_epoch−5s〕；Rurix 臂 = M-c 生产管线 bench〔门侧逐格持锁〕；50×3 trimmed mean 跨轮中位数 M141/M165 冻结口径）+ **通过线 ×1.00 逐格判定：达标 0/18、未达标 18/18**——cornell ratio 0.0606~0.1504（UE 2.106~2.198ms vs Rurix 14.006~36.247ms）、bistro ratio 0.0116~0.0221（UE 3.047~3.966ms vs Rurix 137.729~341.457ms）+ 逐轮守护带 18 格 × 3 轮比值入 evidence + 画质零降级守护绿（G13 锁定双门最新 evidence PASS 消费 + G14.3 车道锚带在树 threshold=0.0107798）+ 差距登记表 g14_fps_gap_registry.json **18 行** gaplib 正典形落盘（cornell 9 + bistro 9，只登记不拟合 RXS-0392）+ budget_eval 全 PASS + RED 三臂独立有效（single-round/mixed-caliber/unmet-masquerade 检出） | host+device（双端真跑面） | evidence/g14_m_d_dual_end_fps_parity_20260820T012652Z.json（8/8 checks 绿，pass_line 未达） | **FAIL（诚实红——通过线未达如实登记不冒充）** |
  | `g14.wave.4.exit`（步骤 256） | 波聚合门只读汇总六 facts 全绿：M-d 门 RED 臂独立有效 + 双端三轮 measured 18 格齐备 + 通过线判定与登记表行数一致（18==unmet_count，gaplib 校验绿）+ 画质零降级守护绿 + 逐轮守护带齐备 + G5~G13 closed 面 0-byte（committed 闭集 ⊆ 授权面）；红源唯一 = M-d host_section_pass=False | host 只读 | evidence/g14_wave4_exit_20260820T022321Z.json（六 facts 全绿） | **FAIL（诚实红聚合面）** |

- **② 波聚合门实测输出**：`py -3 ci/g14_wave4_exit_check.py --gate g14.wave.4.exit` → **VERDICT = FAIL，exit=1**（六 facts PASS + required_gates 行 FAIL——M-d 通过线未达诚实面）；`--selftest` → ALL PASS（负样本空目录红 + 真树聚合 VERDICT==子门实测态不遮蔽机核）。
- **③ 验收命令逐字输出（2026-08-20 真跑留痕，仓库根目录）**：
  - `py -3 ci/g14_dual_end_fps_parity_smoke.py --selftest` → selftest PASS（schema 闭集）；`--gate` → VERDICT=FAIL checks=8/8 pass_line=未达标（双端 6+54 轮真跑：UE 臂 18 轮 harness 子进程 + Rurix 臂 54 轮 bench + RED 三臂函数面真跑检出）。
  - 守卫套件全 PASS：check_structure PASS；check_schemas PASS（本批新增 g14_m_d_dual_end_fps_parity_ + g14_wave4_exit_ 双前缀三处纯追加）；check_number_ledger PASS（CI_step on_tree_max 256/next_free 257）；budget_eval PASS；g14_interlock --require-ready → READY 维持。
  - 起草期 FAIL 轨迹（诚实留档不删）：M-d 首跑嵌套锁死锁（门外层持锁 × UE harness 子进程跨进程自持锁互等——前会话 085327Z 实例与并发实例 30min 零进展发现，kill 三进程〔66448/66476/68580〕后修订为分臂锁纪律：UE 臂 harness 逐轮自持锁〔M-b 同律〕/ Rurix 臂门侧逐格持锁〔M-c 同律〕）+ CSV 新鲜度机核加性（M-b 同名 tag 陈旧数据防混面）——修复后全门一次通过。
- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G-G14-5（§8.4）→ 本波 G-G14-6 诚实红留档（通过线未达不充绿叙述面）；数字步骤 254（G14.3 批 workflow 预登记注面本批脚本落地兑现）/256（落盘前实测 next_free 顺位领取，ledger v1.146 校准同批）；**G14.5a 穷举不阻塞**（§4.2 M-d 行字面）；通过线达标面 = G14.x 延续波/G16+ 承接继续优化（§7 裁决 7 + 用户 2026-08-19 授权字面），M-d 逐波复跑追踪 ratio 改善如实登记。
  - **结构性优化取证登记（G14.x 延续波法定输入——三调研报告实测归因，禁先验承诺 P-09）**：a) bench 测量面 ~64ms@1080p（frame_content_digest 24.9MB payload 重建+标量 sha256 ~60ms + is_finite 全帧扫描 ~4ms）= 非生产路径固有面，剥离/门控 + 生产口径双列（frame_ms_measured/frame_ms_production）为 G14.x 第一波候选；b) scene_render 内 ~77ms host 面 = post-fence readback 14.9MB（疑 WC 内存首匹配无 HOST_CACHED，实测折算 ~200MB/s——G14.3 DLSS readback 同型缺陷已验证修法 ~1.8GB/s）+ read_f32 逐元素转换；c) mv 12.4ms = 纯 CPU 930Kpx 双循环（GPU 化接入点 = 同 session 第二 compute pass + mv SSBO，须先修 readback 内存型否则净亏 ~25ms）；d) upscale 109ms 分解 = pack f16+双程拷贝 ~28-32ms / slEvaluateFeature CPU 阻塞 ~45-55ms（NGX CUDA interop 黑盒面，RURIX_VENDOR_TIMING=1 内建遥测实测裁决为第一动作）/ submit_wait ~10-15ms / readback 13.7ms——零拷贝 Stage A（pack 直写 staging + f16 SIMD + upscale_into 驻留缓冲，行为零漂移）/Stage B（VK_KHR_external_memory_win32 导入 + GPU mv，additive API 不触 trait/temporal 0-byte guardrail）方案在档；e) fence 全串行（当帧等待）——N 帧流水 = submit/collect 分离 + per-slot cmd/params/descriptor/query + 输出双缓冲（render_exec.rs 结构性大改，位级安全论证在档）；f) raster-primary MVP 可行性 = Vulkan VS+FS 生产可用（G7.5b diff=0 全绿；RXS-0171 冻结仅 DXIL 路不构成依赖）+ 诚实边界（主射线仅占 RT 工作 1/17，单独不足达 ×1.00；16 阴影射线/px 结构替代 = 收益最大治理成本最高项，触 G13.4 画质锚同模面需独立立项）。
  - **G14.x 延续波波序登记**：① 测量面剥离/门控 → ② readback HOST_CACHED → ③ vendor Stage A → ④ mv GPU 化 → ⑤ fence 流水 → ⑥ vendor Stage B 零拷贝 → ⑦ raster-primary MVP（主射线占比 measured 取证）→ ⑧ 阴影结构面（独立立项评估）；逐波 M-d 复跑 + 差距登记表行集收敛追踪。
  - **异己并发工作树面**：本批只含 G14 车道文件（按文件名显式择取）；共享面（ci/check_schemas.py / .github/workflows/pr-smoke.yml / registry/number_ledger.json）经 .tmp/g14_4_shared_replay.py 幂等重放 + 提交前单命令链压死竞态（并发回退偏差两起留痕：check_schemas 追加面与 pr-smoke 步骤 256 块被并发回退，重放恢复；ledger 数字面未受影响）；异己会话 src/ 未提交面维持未提交、零消费、零混入（立项裁决 1）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G13 §8.x 同模）。`Assisted-by: Kimi-K3（G14.4 双端对标波）`（影响范围：ci/g14_dual_end_fps_parity_smoke.py + ci/g14_wave4_exit_check.py 新建 + milestones/g14 双 evidence schema 新建 + milestones/g14/g14_fps_gap_registry.json 首建 18 行 + ci/check_schemas.py〔双前缀三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 256〕+ registry/number_ledger.json〔CI_step 255→256/next_free 257 + revision_log v1.146〕+ 本契约 §8.5 本条 + evidence 本批真跑件〔M-d 012652Z 诚实红 + wave4 022320Z/022321Z〕；验证方式：块③逐字命令输出——M-d 门 8/8 checks 绿 + 通过线 0/18 诚实红 + 波聚合门六 facts 全绿 + 双 selftest 红绿留痕 + 守卫套件全 PASS + 互锁 READY 维持）。

### §8.6 G14.6 口径与 host 面优化波验收记录（2026-08-20）——§7 裁决 7 延续波程序面：M-f(M176) 生产口径双列 + vendor Stage A 位级零漂移（步骤 257，7/7 checks 全绿 VERDICT=PASS）+ 波聚合门 g14.wave.6.exit（步骤 258）VERDICT=PASS 六 facts 全绿

- **① 独立断言清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g14.p0.m_f.production_caliber_stage_a`（步骤 257） | G14_ACCEPTANCE_MAP 附录 A M-f 行逐字：bench receipt 双列口径落盘（frame_ms 全量 G14.3 兼容 + frame_ms_production = 全量 − bench 测量面 tail〔is_finite 全帧校验 + frame_content_digest payload 重建+sha256〕）逐格不变量机核 0 < production ≤ full（三探针格 cornell-box t67 × 三后端真跑：tsr full=26.908/prod=18.089/tail=8.818ms、dlss full=16.557/prod=7.719/tail=8.838ms、fsr full=15.748/prod=6.810/tail=8.938ms）+ **vendor Stage A 位级零漂移 3/3**（DLSS pack 直写 mapped staging 消 ~px·21B 二次 memcpy + DLSS/FSR 输出驻留写消逐帧 ~out_px·12B 分配——三探针格末帧 digest == g14_3_stage_a_digest_anchor.json 冻结锚逐字一致）+ RURIX_VENDOR_TIMING=1 分解遥测六段 measured（pack=1.255/sl_book=0.010/upload=0.018/**evaluate=0.083**/submit_wait=0.554/readback=0.842ms——evaluate 黑盒段实测 <0.1ms，§8.5 d) 条先验量级 45~55ms 证伪如实登记，G14.4 调研 R1 裁决动作兑现）+ Stage A 前后全量口径对照行（pre 锚 = M-d 012652Z evidence：dlss 16.659→16.557ms −0.6%、fsr 16.583→15.748ms −5.1%、tsr 25.536→26.908ms 未触面方差内）+ 三探针格 production 口径 measured 入 g14_budget（阈 = 实测 ×1.5 守护带，measured_local 零 estimated）+ budget_eval 全 PASS + RED 双臂独立有效（caliber-masquerade/digest-drift 函数面真跑检出） | host+device（探针格真跑面） | evidence/g14_m_f_production_caliber_stage_a_20260820T034312Z.json（7/7 checks 绿） | **PASS** |
  | `g14.wave.6.exit`（步骤 258） | 波聚合门只读汇总六 facts 全绿：① M-f RED 臂独立有效 + ② M-c 回归面最新绿（Stage A 后复跑 PASS 10/10，evidence 034925Z——既有判据零降级）+ ③ M-d v2 守护面绿（production_caliber_v2 + stage_a_digest_drift_guard 双真——**18 格 × 3 轮末帧 digest == 冻结锚全矩阵位级零漂移** + 锚 18/18 格在树）+ ④ g14_budget production 口径 3 条目 measured_local + budget_eval 全 PASS + ⑤ M-d 通过线诚实红面登记（unmet=18 == 登记表 18 行，不充绿）+ ⑥ G5~G13 closed 面 0-byte（committed 闭集 ⊆ 授权三面 + 工作树闭集 = g12_pt_sampler_selection 异己登记面） | host 只读 | evidence/g14_wave6_exit_20260820T070405Z.json（六 facts 全绿） | **PASS** |

- **② 波聚合门实测输出**：`py -3 ci/g14_wave6_exit_check.py --gate g14.wave.6.exit` → **VERDICT = PASS，exit=0**（required_gates M-f PASS + 六 facts 全 PASS，聚合不遮蔽机核维持）；`--selftest` → ALL PASS（负样本空目录红 + 真树聚合 VERDICT==子门实测态一致性双臂）。
- **③ 验收命令逐字输出（2026-08-20 真跑留痕，仓库根目录）**：
  - `py -3 ci/g14_production_caliber_stage_a_smoke.py --gate g14.p0.m_f.production_caliber_stage_a` → VERDICT=PASS checks=7/7（探针格三后端真跑 + vendor 遥测探针 30 帧 + budget 写入/评估 + RED 双臂）。
  - `py -3 ci/g14_rurix_pipeline_perf_smoke.py --gate g14.p0.m_c.rurix_pipeline_perf` → VERDICT=PASS checks=10/10（Stage A 后回归复跑 54 轮：双跑位级 + 画质锚带 SSIM=0.99461 deficit≤0.0107798 守护带复核绿 + t50/t67/t100 倒挂消除维持 + RED 三臂检出）。
  - `py -3 ci/g14_dual_end_fps_parity_smoke.py --gate g14.p0.m_d.dual_end_fps_parity`（v2 口径）→ checks=10/10 全绿、VERDICT=FAIL 诚实红维持（通过线 0/18：cornell ratio 0.0791~0.4363〔UE 2.102~2.304ms vs Rurix 生产口径 5.280~27.251ms〕、bistro ratio 0.0116~0.0456〔UE 3.009~4.892ms vs Rurix 生产口径 66.037~423.094ms〕——低端格较首跑全量口径显著收敛：fsr t50 ratio 0.0221→0.0456 约 ×2.06、dlss t50 0.0193→0.0395；bistro t67-fsr/t100 四格全量口径较首跑 +73~+104% 环境漂移如实登记〔UE 臂 t100 同跑 +23% 旁证热节流态，3.966→4.892ms〕，ratio 收敛追踪归后续延续波 M-d 逐波复跑面）。
  - 守卫套件全 PASS：check_schemas PASS（本批新增 g14_m_f_production_caliber_stage_a_ + g14_wave6_exit_ 双前缀纯追加 + M-d schema checks 段 anyOf 双相〔8 键冻结首跑面/10 键 v2〕沿 M130 先例）；budget_eval PASS；check_number_ledger PASS（CI_step on_tree_max 258/next_free 259）；g14_interlock --require-ready → READY 维持。
  - 起草期 FAIL/偏差轨迹（诚实留档）：a) M-f 首跑 034037Z 7/7 checks 绿但 schema source_ref const 一字差（全角；U+FF1B vs 半角 U+003B，位置 73）→ FAIL；修复后重跑绿，**FAIL 件与三件孤儿探针件起草期删除未在档**（与 G14.3/G14.4「FAIL 轨迹在档」纪律偏差如实登记，不复述为在档）；b) M-d v2 首跑全量测量完成后 ①b 块 NameError（DIGEST_ANCHOR_PATH 定义行被并发会话回退——同批编辑存活面 = CHECK_KEYS 10 键/run_rurix_cell v2/①b 块/RED 加性臂，独定义行失）→ evidence 未落盘；定义恢复 + 存活面核查（selftest PASS + 模块面核验）后重跑一次通过，回退留痕。
- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G14.4 诚实红留档（§8.5）→ 本波 §7 裁决 7 延续波程序面（附录 A M-f 行不进 §1 冻结 5 行闭集，acceptance_map_check EXPECTED_P0 机核面 0-byte）；数字步骤 257/258 落盘前实测 next_free 顺位领取（ledger v1.147 校准同批）；M-d 通过线达标面仍归 G14.x 后续延续波/G16+（§7 裁决 7 + 用户 2026-08-19 授权字面），本波生产口径双列为通过线裁决提供口径面而非达标承诺（P-09）。
  - **Stage A 兑现面登记**（§8.5 波序 ③）：DLSS 臂 pack 改直写 mapped staging（pack_buf 中转面删除）+ DLSS/FSR 双臂输出改调用方驻留切片直写（upscale_into 加性公共面，既有 upscale/probe_validation_frame 签名与行为 0-byte）；位级零漂移 = 18 格全矩阵实证（M-d v2 digest guard 双真）非探针格单面；unsafe-audit U58 行 G14.6 扩注在档（0 新 U 号，from_raw_parts_mut 镜像 U8 既有纪律）。
  - **vendor 分解遥测裁决登记**（§8.5 d) 条 R1 动作兑现）：六段稳态 mean = pack 1.255 / sl_book 0.010 / upload 0.018 / evaluate 0.083 / submit_wait 0.554 / readback 0.842ms（cornell t67 dlss 30 帧探针）；**evaluate 黑盒段实测 <0.1ms——§8.5 调研先验「slEvaluateFeature CPU 阻塞 ~45-55ms」证伪**，vendor 路径整段 ~2.7ms 量级，upscale 段非 bistro 重格主瓶颈（生产口径下 cornell vendor 格 5.3~7.7ms vs tsr 14.6~27.3ms 旁证 vendor 双臂 host 面已非首优）；后续 Stage B（VK_KHR_external_memory_win32 零拷贝）收益上限按本遥测重估登记，波序 ⑥ 优先级让位 ② scene readback HOST_CACHED（~60-70ms@1080p 疑 WC 面）/④ mv GPU 化（12.4ms CPU 双循环）/⑤ fence 流水——P2 穷举决策表归 G14.5a 窗裁决。
  - **口径面边界声明**：frame_ms_production = 全量 − tail（is_finite+digest 双bench 测量固有面），M-d v2 起通过线判定消费生产口径、全量列随行登记（median_full_caliber_ms 逐格入 evidence）；以全量冒充生产即 RED（M-f 臂① + M-d v2 加性臂生产口径不变量函数面真跑检出）。
  - **异己并发工作树面**：本批只含 G14 车道文件（按文件名显式择取）；并发回退两起留痕（M-d smoke DIGEST_ANCHOR_PATH 定义行 + M-f smoke SOURCE_REF 全角回流面——均恢复重验）；异己会话 src/ 未提交面（rurix-asset/uc06/uc08/ssr/hzb/restir/smrt 等）维持未提交、零消费、零混入（立项裁决 1）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模）。`Assisted-by: Kimi-K3（G14.6 口径与 host 面优化波）`（影响范围：ci/g14_production_caliber_stage_a_smoke.py + ci/g14_wave6_exit_check.py 新建 + milestones/g14 双 evidence schema 新建 + milestones/g14/g14_3_stage_a_digest_anchor.json 首建 18 格 + src/rurix-rt/src/vendor_upscale.rs〔Stage A 双臂〕+ src/rurix-render/src/bin/g14_3_pipeline_perf.rs〔双列口径 + 驻留输出面〕+ ci/g14_rurix_pipeline_perf_smoke.py〔run_bench receipt production 面 fail-closed〕+ ci/g14_dual_end_fps_parity_smoke.py〔v2 10 键 + ①b 机核 + RED 加性臂〕+ milestones/g14/g14_m_d_dual_end_fps_parity_evidence_schema.json〔anyOf 双相〕+ milestones/g14/g14_budget.json〔production 3 条目〕+ milestones/g14/G14_ACCEPTANCE_MAP.md〔附录 A M-f 行〕+ unsafe-audit/rurix-rt.md〔U58 G14.6 扩注〕+ ci/check_schemas.py〔双前缀纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 257/258〕+ registry/number_ledger.json〔CI_step 257→258/next_free 259 + revision_log v1.147〕+ 本契约 §8.6 本条 + evidence 本批真跑件〔M-f 034312Z PASS + M-c 034925Z 回归绿 + M-d 053525Z v2 诚实红 + wave6 exit 070405Z/070603Z/070605Z〕；验证方式：块③逐字命令输出——M-f 门 7/7 checks 绿 + Stage A 位级零漂移 18 格全矩阵 + vendor 六段遥测 measured + M-c 回归 10/10 绿 + M-d v2 10/10 checks 绿诚实红维持 + 波聚合门六 facts 全绿 + 双 selftest 红绿留痕 + 守卫套件全 PASS + 互锁 READY 维持）。

### §8.7 G14.7 vendor 转换并行化波验收记录（2026-08-20）——§7 裁决 7 延续波程序面：M-g(M177) vendor 转换并行化位级零漂移 + 同码 A/B measured（步骤 259，7/7 checks 全绿 VERDICT=PASS）+ 波聚合门 g14.wave.7.exit（步骤 260）VERDICT=PASS 六 facts 全绿

- **① 独立断言全绿清单**：

  | gate（symbolic key） | 独立布尔断言 | host/device | evidence 路径 | 结果 |
  |---|---|---|---|---|
  | `g14.p0.m_g.vendor_parallel_conversion`（步骤 259） | G14_ACCEPTANCE_MAP 附录 A M-g 行逐字：vendor_upscale.rs 四区打包（color f16/depth/mv/reactive）与双臂输出回读转换（DLSS 连续 RGBA / FSR 行距对齐）改像素带并行（std::thread::scope 带切分，元素零依赖，带内逐值同式同序）+ fsr-dbg 逐帧诊断打印门控 + **位级零漂移三机核**（三探针格〔bistro t67 dlss/fsr 并行面 + cornell t67 dlss 阈下单带面〕末帧 digest == g14_3_stage_a_digest_anchor 冻结锚逐字一致 + RURIX_VENDOR_PAR=0 串行对照臂 bistro t67 dlss digest 同锚〔并行 ≡ 串行 ≡ 锚 三角机核〕+ Rust 函数面 g14_7_parallel_conversion_bitexact 单测真跑绿〔并行/串行合成字节面逐位一致 + 带数决策纯函数锚〕）+ **同码 A/B measured**（bistro t67 dlss 交错四跑稳态 mean：pack 串行 19.275ms → 并行 3.685ms〔Δ=15.590ms ×5.23〕、readback 串行 31.730ms → 并行 5.762ms〔Δ=25.968ms ×5.51〕——双段方向机核绿，改善量 measured 登记不设先验阈值 P-09）+ 双探针格（bistro t67 dlss/fsr）production 口径 measured 入 g14_budget（阈 = 实测 ×1.5 守护带程序产）+ budget_eval 全 PASS + RED 双臂独立有效（digest-drift/direction-masquerade 函数面真跑检出） | host+device（探针格真跑面） | evidence/g14_m_g_vendor_parallel_conversion_20260820T100256Z.json（7/7） | **PASS** |
  | `g14.wave.7.exit`（步骤 260） | 波聚合门只读汇总六 facts 全绿：① M-g RED 臂独立有效 + ② M-c 回归面最新绿（并行化后复跑 PASS 10/10，evidence 102952Z——既有判据零降级 + 画质锚带 SSIM=0.99461008 deficit≤0.0107798 守护带复核绿）+ ③ M-d v3 守护面绿（production_caliber_v2 + stage_a_digest_drift_guard 双真——**18 格 × 3 轮末帧 digest == 冻结锚全矩阵位级零漂移（并行化后）** + 锚 18/18 格在树）+ ④ g14_budget production 口径 bistro 双探针格条目 measured_local + budget_eval 全 PASS + ⑤ M-d 通过线诚实红面登记（unmet=18 == 登记表 18 行，不充绿）+ ⑥ G5~G13 closed 面 0-byte（committed 闭集 ⊆ 授权三面 + 工作树闭集 = g12_pt_sampler_selection 异己登记面） | host 只读 | evidence/g14_wave7_exit_20260820T142429Z.json（六 facts 全绿） | **PASS** |

- **② 波聚合门实测输出**：`py -3 ci/g14_wave7_exit_check.py --gate g14.wave.7.exit` → **VERDICT = PASS，exit=0**（required_gates M-g PASS + 六 facts 全 PASS，聚合不遮蔽机核维持）；`py -3 ci/g14_wave7_exit_check.py --selftest` → ALL PASS（负样本空目录红 + 真树聚合 VERDICT==子门实测态一致性双臂）。
- **③ 验收命令逐字输出（2026-08-20 真跑留痕，仓库根目录）**：
  - `cargo build --release -p rurix-render --bin g14_3_pipeline_perf --features vendor-upscale` → Finished 绿；`cargo test --release -p rurix-rt --features vendor-upscale g14_7_parallel_conversion_bitexact` → test result: ok（1 passed——并行/串行字节面逐位一致 + 带数决策锚 + 阈下小格强制单带面）。
  - `py -3 ci/g14_vendor_parallel_conversion_smoke.py --selftest` → selftest PASS（schema 闭集 + 双臂红绿）；`--gate` → VERDICT=PASS checks=7/7（探针格三后端真跑 + 串行对照臂 + 交错四跑 A/B + RED 双臂）。
  - `py -3 ci/g14_rurix_pipeline_perf_smoke.py --gate g14.p0.m_c.rurix_pipeline_perf` → VERDICT=PASS checks=10/10（并行化后回归复跑 54 轮：双跑位级 + 画质锚带守护复核绿 + t50/t67/t100 倒挂消除维持 + RED 三臂检出）。
  - `py -3 ci/g14_dual_end_fps_parity_smoke.py --gate g14.p0.m_d.dual_end_fps_parity`（v3 口径）→ checks=10/10 全绿、VERDICT=FAIL 诚实红维持（通过线 0/18——**vendor 重格 ratio 显著收敛**：bistro t67 fsr 0.0174→0.0342〔+96.6%〕/ t67 dlss 0.0235→0.0307〔+30.6%〕/ t100 dlss 0.0119→0.0153〔+28.4%〕/ t100 fsr 0.0140→0.0186〔+33.1%〕/ t50 dlss 0.0395→0.0503〔+27.4%〕/ t50 fsr 0.0456→0.0545〔+19.5%〕；tsr/轻格环境漂移如实登记〔UE 臂同跑 cornell 2.304→3.314ms/bistro t50 3.009→4.481ms +44~+59% 旁证热节流窗口——双端同漂不冒充单边改善〕；通过线 ×1.00 维持未达，不冒充）。
  - 守卫套件全 PASS：check_structure PASS；check_schemas PASS（本批新增 g14_m_g/g14_wave7 双前缀 + G14.5a/5b 四前缀共七前缀纯追加——**G14.7 双前缀追加面遭并发回退一起，经 .tmp/g14_7_schemas_replay.py 幂等重放恢复留痕〔当日起第三起共享面并发回退，G13.4~G14.6 同模处置〕**）；check_number_ledger PASS（CI_step on_tree_max 260/next_free 261 校准时点实测）；budget_eval PASS（244 pass/0 skip——M-g bistro 双探针格条目入账）；g14_interlock --require-ready → READY 维持。
  - 起草期 FAIL/偏差轨迹（诚实留档）：a) Rust 测试初版直接调 std::env::set_var（edition 2024 unsafe 面）→ 重构为显式带数变体（_bands 族 + par_band_count_with 纯函数）后绿；b) **同批并行 Edit 同文件互相覆盖两起**（_bands 分裂与测试改写面丢失——序贯编辑修复，G13.6 序贯教训复验留痕）；c) wave7 selftest 真树一致性臂产 FAIL evidence 一件（100227Z——M-g evidence 在树前诚实面，在档 0-byte）。
- **④ 门序 / 偏差 / not-triggered 登记面摘要**：
  - **门序**：G14.6（§8.6）→ 本波 §7 裁决 7 延续波程序面（附录 A M-g 行只追加，§1 冻结 5 行闭集 0-byte，acceptance_map_check EXPECTED_P0 机核面不动）；数字步骤 259/260 落盘前实测 next_free 顺位领取（ledger v1.148 校准同批）；M-d 通过线达标面仍归 G14.x 后续延续波/G16+（§7 裁决 7 + 用户 2026-08-19 授权字面）。
  - **并行化兑现面登记**（§8.5 d 条后半 + §8.6 波序vendor host 转换面）：pack 四区（color f16/depth/mv/reactive）与双臂输出回读转换像素带并行（带数决策 par_band_count 纯函数锚：阈下小格恒单带 + RURIX_VENDOR_PAR=0 串行对照臂）；**A/B 实测 ×5.23/×5.51（pack/readback 双段，bistro t67 dlss 稳态 mean 交错四跑）**；bistro t67 dlss vendor 臂遥测 total 由 ~36-41ms 收敛至 ~7.5-8.3ms（探针窗实测）；位级零漂移 = 18 格全矩阵 + 串行对照臂 + Rust 函数面三机核实证。
  - **优化残留阻塞面维持登记**（G14.5a 穷举承接锚）：scene readback HOST_CACHED（render_exec.rs 异己整文件改写面——v1.147 登记字面维持）/ fence N 帧流水（同面）/ mv GPU 化（temporal 底座 0-byte，Full RFC 程序面）/ vendor Stage B（遥测重估让位维持）/ raster-primary MVP / 阴影结构面——六面 G16+ 承接（P2 穷举 G14-N9~N14 行）。
  - **异己并发工作树面**：本批只含 G14 车道文件（按文件名显式择取）；异己会话 src/ 未提交面维持未提交、零消费、零混入（立项裁决 1）；共享面（check_schemas/workflow/ledger）经 .tmp 幂等重放落盘 + 提交前单命令链压死竞态窗口（一次性工具不入 commit）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模）。`Assisted-by: Kimi-K3（G14.7 vendor 转换并行化波）`（影响范围：src/rurix-rt/src/vendor_upscale.rs〔par_band_count(_with) 带数决策 + pack_vendor_inputs(_serial/_bands) 四区打包并行主体 + convert_out_(serial/par/par_bands/pitched_par/pitched_par_bands) 回读转换并行主体 + DLSS/FSR 双臂 pack/readback 调用点接线 + fsr-dbg 门控 + g14_7_parallel_conversion_bitexact 单测〕+ ci/g14_vendor_parallel_conversion_smoke.py + ci/g14_wave7_exit_check.py 新建 + milestones/g14 双 evidence schema 新建 + milestones/g14/G14_ACCEPTANCE_MAP.md〔附录 A M-g 行只追加〕+ unsafe-audit/rurix-rt.md〔U58 G14.7 扩注，0 新 U 号〕+ ci/check_schemas.py〔七前缀纯追加，重放面〕+ .github/workflows/pr-smoke.yml〔步骤 259/260〕+ registry/number_ledger.json〔CI_step 258→260/next_free 261 + revision_log v1.148〕+ 本契约 §8.7 本条 + evidence 本批真跑件〔M-g 100256Z PASS + M-c 102952Z 回归绿 + M-d 122608Z v3 诚实红 + wave7 exit 142429Z PASS + wave7 selftest 真树臂 100227Z FAIL 件〕；验证方式：块③逐字命令输出——M-g 门 7/7 checks 绿 + 位级零漂移三机核（探针格 + 串行对照臂 + Rust 函数面）+ A/B 双段方向机核 measured + M-c 回归 10/10 绿 + M-d v3 10/10 checks 绿诚实红维持 + 波聚合门六 facts 全绿 + 双 selftest 红绿留痕 + 守卫套件全 PASS + 互锁 READY 维持）。

### §8.8 G14plus 波0 治理立项批验收记录（2026-08-22）——§7 裁决 7 延续波程序面（G14.8~G14.12 五波立项）：用户 2026-08-22 授权字面立项 + G14-N8~N14 承接窗口提前兑现（P2 表后事件登记）+ G14-N6 异己面 patch 存档清场 + RFC-0030 伞形 Agent Approved + M-h 延续波收口门 materialize（步骤 265 实测领取）

- **① 立项授权与载体裁决**：用户 2026-08-22 指令「帮我一次性完成G14硬收尾，要求门禁严格全绿。先优化再测试以减少工期……本次任务可附加为G14plus作为文档记录……本次进程允许视为超越G类里程碑的超大项目纠正优化案，不需要考虑工作量，务必完成任务使项目达到预期」（G14PLUS_RECORD §1 逐字登记）；载体 = §7 裁决 7 字面二选一取 **G14.x 延续波**（G14.8 测量与确定性基线 / G14.9 host 面与 RT 白给 / G14.10 帧循环重构 / G14.11 结构条件波〔仅当 G14.10 后 M-d 探针复跑仍有未达格〕/ G14.12 复测收口——波序与依赖 G14PLUS_RECORD §2）；目标判据 = M-d 18 格通过线 ×1.00 全达标 + 画质零降级守护带内 + RD-045 确定性缺陷修复 + soak/closeout 全绿收口（既有判据字面 0-byte，达标 = G-G14-6/G-G14-9「达标如实登记」分支兑现）。
- **② 治理产物清单**：
  | 产物 | 路径 | 核验 |
  |---|---|---|
  | P2 表后事件登记（G14plus 立项条） | G14_P2_DECISIONS.md 表后只追加区 | G14-N8~N14 七行承接锚窗口提前逐行命中论证 + G14-N6 处置形态变更 + RD-045 联动；穷举闭集 42 行 0-byte |
  | G14PLUS_RECORD v1.0 | milestones/g14/G14PLUS_RECORD.md | 叙事总档首建（授权/波序/优化清单十项/处置档案/复测轨迹基线）；判据零承载（指针制） |
  | RFC-0030 v1.0 **Agent Approved** | rfcs/0030-g14plus-pipeline-structural-optimization.md | 伞形七面（mv GPU 化 temporal 演进显式修订行 / RD-045 确定性协议缺陷修复评估 / first-hit 语言内建 / TSR kernel 变体 / readback+FIF 结构面 / 阴影与主可见性条件条款 / 锚重收割程序）；编号按落盘前实测 RFC next_free=30 领取；D-409 第 1 轮对抗评审 F1~F7 全 disposition（评审全文 milestones/g14/design/rfc0030_adversarial_review.md，provenance 偏差如实登记 + 效力自限 + M-h/closeout 终审复核锚）；主会话三面一致核对（本契约 §8.8 ↔ MAP 附录 A ↔ RFC）后翻 Approved。**Approved 不构成实现许可**——各波实施按 §8.9+ 只追加验收承载 |
  | M-h 延续波收口门 materialize | ci/g14_continuation_closeout_smoke.py + milestones/g14/g14_m_h_continuation_closeout_evidence_schema.json + MAP 附录 A M-h 行 + pr-smoke 步骤 265 | `g14.p0.m_h.continuation_closeout`（M179）六 checks（锚重收割三证/M-d 18-18 达标/登记表空表终态/RD-045 登记完备/波记录在树/RED 双臂）；`--selftest` → **SELFTEST PASS（schema 闭集 3 + 判定函数 2 GREEN + 6 RED）**；真脚本真步骤沿 G14.1 治理三门先例（当前 --gate 必然诚实 FAIL——前置未满足，G14.12 真跑）；附录 A 行不进 §1 冻结 5 行闭集与 G14.5b 既有八 facts 阻断闭集 |
  | ledger v1.151 校准 | registry/number_ledger.json | RFC 29→30 / CI_step 264→265（next_free 266）+ reserved_in_flight[G14plus]；`.tmp/g14plus_w0_ledger_replay.py` 幂等重放落盘（二跑 IDEMPOTENT 实证）；`py -3 ci/check_number_ledger.py` → PASS |
- **③ G14-N6 异己面处置（保管形态变更，零消费维持）**：14 个已跟踪异己文件（src/rurix-rt/src/render_exec.rs〔F1 D3D12 import 脚手架 +378/−6 实质 + CRLF 整文件重写〕+ 5 配套 + 5 研究面 mod.rs + evidence/d3d12_interop_smoke.json + milestones/g12/g12_pt_sampler_selection.json〔异己 timestamp〕+ ci/check_schemas.py〔纯 CRLF 漂移 -w diff=0〕）经 `git diff HEAD --output` 导出 patch **双份存档**（K:\rurix-ext\archive\alien_worktree_20260822\alien_tracked.patch，1,514,122 bytes，**sha256:5bd8ebfa6f580b90fab0370c0eb2f8522a612f284613b25f0f4d30282e7f509e** + .tmp/g14plus_archive 副本）后 `git checkout` 回退 HEAD；完整性核验 `git apply --check --reverse` **exit=0**（patch 可逆向应用 = 归属会话可随时恢复，恢复路径自理）；6 个 untracked 研究面文件（ktx2_read/hzb/restir/sdf_trace/smrt/ssr）同步双存档、原文件保留工作树（mod.rs 回退后不在编译树，零消费无害）；三张 G14 登记表（g14_budget/g14_fps_gap_registry/g14_ue_variance_samples）保留工作树态（门产刷新 + 只追加样本，回退将删除合法样本；G14.12 复测全量重写）。G14-N9/N10 承接锚「异己面清结后」字面自此兑现（porcelain 实测 render_exec.rs 零漂移）。
- **④ 验收命令逐字输出（2026-08-22 真跑留痕，仓库根目录）**：
  - `py -3 ci/g14_continuation_closeout_smoke.py --selftest` → **SELFTEST PASS (schema 闭集 3 + 判定函数 2 GREEN + 6 RED)，exit=0**。
  - `py -3 .tmp/g14plus_w0_ledger_replay.py` → APPLIED（RFC 29->30, CI_step 264->265, reserved_in_flight[G14plus], revision_log v1.151）；二跑 → **IDEMPOTENT**。
  - 守卫套件：`py -3 ci/check_schemas.py` → **PASS**（本批新增 g14_m_h 前缀 load/validator/路由三处纯追加）；`py -3 ci/check_number_ledger.py` → **PASS**；`py -3 ci/g14_acceptance_map_check.py --gate g14.wave.1.acceptance_map` → **VERDICT=PASS**（附录 A M-h 行只追加不触 §1 冻结 5 行 EXPECTED_P0 机核——M-f/M-g 先例同模，evidence 20260822T035603Z）。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G10~G14 §8.x 同模）。`Assisted-by: Claude Fable 5（G14plus 波0 治理立项批）`（影响范围：rfcs/0030-g14plus-pipeline-structural-optimization.md 新建 + milestones/g14/design/rfc0030_adversarial_review.md 新建 + milestones/g14/G14PLUS_RECORD.md 新建 + milestones/g14/G14_P2_DECISIONS.md〔表后事件登记只追加一条〕+ milestones/g14/G14_ACCEPTANCE_MAP.md〔附录 A M-h 行只追加〕+ ci/g14_continuation_closeout_smoke.py + milestones/g14/g14_m_h_continuation_closeout_evidence_schema.json 新建 + ci/check_schemas.py〔g14_m_h 三处纯追加〕+ .github/workflows/pr-smoke.yml〔步骤 265〕+ registry/number_ledger.json〔v1.151〕+ 本契约 §8.8 本条 + 异己面 patch 存档〔K 盘 + .tmp 不入 commit〕；验证方式：块④逐字命令输出——M-h selftest 红绿留痕 + ledger 幂等重放实证 + 守卫套件全 PASS + acceptance_map 门 PASS）。

### §8.9 G14.8 测量与确定性基线波验收记录（2026-08-22）——锁频不可用降级如实登记（环境画像+冷却门控替代）+ flip-trace 诊断臂扩展（RD-045 backfill_condition 字面动作）+ RD-045 基线漂移率 N=20 drift=0/20（快筛口径）

- **① 锁频面**：`nvidia-smi -lgc` 普通权限 exit=4 + UAC 提权被用户取消——锁频不可用如实登记；降级 = 环境画像登记（时钟/温度查询面可用）+ 重型 bench 前冷却门控；UE 臂基线重测取消裁决（锁频不可用后无新环境态，M-d 复测自然重测）。
- **② flip-trace**：`g14_3_pipeline_perf.rs` bench 腿 `RURIX_G14_FLIP_TRACE=<dir>` 逐帧 digest 轨迹（`frame_digests_*.jsonl`；digest 本就逐帧计算，数据面位级零漂移；G12_5_BENCH_FLIP_TRACE 前例同模）。
- **③ RD-045 基线 N=20**：HEAD 码面（47cd0750 副本 binary）bistro-interior/t50/tsr_device 20 轮进程级独立 bench 逐轮末帧 digest 对 stage_a 锚——**drift=0/20**（~101s/轮；统计诚实 = p≈1.9% 基础率下 N=20 零检出置信 ~68%，快筛非闭环；修复证据依赖全战役累计轮次，RFC-0030 §4.2 L4 字面）；两次脚本启动失败留痕（RURIX_VK_VALIDATION env 缺失 / --expect-digest 语义误用，0 有效数据轮）。
- **④ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署）。`Assisted-by: Claude Fable 5（G14plus G14.8 波）`（影响范围：src/rurix-render/src/bin/g14_3_pipeline_perf.rs〔flip-trace 加性段〕+ G14PLUS_RECORD §4 G14.8 记录 + 本契约 §8.9 本条 + .tmp/g14plus_rd045/ 测量产物〔不入 commit〕；验证方式：N=20 全 MATCH 输出 + flip-trace 编译过）。

### §8.10 G14.9 host 面与 RT 白给波验收记录（2026-08-22）——RFC-0030 §4.3/§4.5/§4.6 三域并行兑现：first-hit 内建全链（RayFlags=0x5 字面）+ FIF submit/collect 分离（顺序路位级等价）+ TSR 调度变体 8×8 + 背光跳射线；L0 位级探针双格 PASS；AS flags 漂移即弃 + SSBO cached 惩罚两起实测归因修订诚实登记

- **① 三域改动**：编译器域（rurixc 13 文件 + conformance 4 文件——`ray_query_initialize_first_hit` 全链，MIR first_hit 字段默认路径 W1/W2 golden manifest 重编 BYTE-IDENTICAL，524+ 测试全绿，trace_matrix 388/388）；执行器域（render_exec.rs/vk.rs——FIF=2 submit/collect 拆分〔GPU 真跑两帧 readback 逐位相等〕+ `submit_with_frame_update`/`collect` 加性公共 API + per-slot 资源 + `create_device_buffer` prefer_cached 参数；rurix-rt 209 过/2 败——两败 HEAD 基线既有非本批引入如实登记）；kernel 域（g14_8_tsr_resample/resolve.rx 变体〔原 g13 kernel 0-byte 保留——RD-045 归因对照臂〕+ bin dispatch 2D 化 + M-c 门 SPV 路径切换〔门脚本内部修订面〕+ g14_3_direct_gi.rx 背光 keep 跳射线与阴影臂 first-hit 切换〔spirv-dis：主射线 %uint_1 / 阴影 %uint_5〕）。
- **② 两起实测归因修订（诚实红轨迹）**：a) AS PREFER_FAST_TRACE——bistro t50 末帧 digest 漂移（c099fc86…≠锚），单变量 bisect 确认漂移源（共面 tie-break），按 RFC-0030 §4.8「漂移即弃」放弃并 revert（vk.rs 两处基线 flags 留痕注释）；b) SSBO 本体切 HOST_CACHED——scene GPU 8.58→30.5ms（散写 snooped 内存一致性惩罚；HEAD kernel + 新 render_exec 隔离测试排除 kernel 嫌疑），修订 = SSBO 恒 WC（HEAD 行为 0-byte）+ cached 留 staging 用途 + 输出 DEVICE_LOCAL 终态归 G14.10。
- **③ L0 位级探针终态**：cornell-box/t67/tsr_device 双跑 converged digest 三方一致（== pre 锚 sha256:e9bc79a7…）**PASS**；bistro-interior/t50/tsr_device 末帧 digest == stage_a 锚（cd35a878…）**PASS**——first-hit 存在性等价 + 跳射线恒零门 + TSR 变体调度重排 + FIF 顺序路等价四项「位级不变」承诺全部实证。
- **④ 性能过渡态登记**（不作对标输入）：bistro t50 tsr prod 156.06ms（TSR OUT_COLOR 24.9MB WC 回读过渡税；TSR kernel 8×8 生效旁证 = cached 对照轮 upscale 120.29→38.24ms）；scene GPU 12.49ms（+3.9 vs 基线微扰待 G14.10 后归因）；G14.10 并 session + 关生产回读为消税面。
- **⑤ 签署块**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署）。`Assisted-by: Claude Fable 5（G14plus G14.9 波，三实施域并行）`（影响范围：src/rurixc 13 文件 + conformance/rayquery accept/reject 双语料 + conformance/traceability_matrix.{json,md} 再生成 + src/rurix-rt/{render_exec.rs,vk.rs} + src/rurix-render/kernels/{g14_3_direct_gi.rx,g14_8_tsr_resample.rx,g14_8_tsr_resolve.rx} + src/rurix-render/src/bin/g14_3_pipeline_perf.rs + ci/g14_rurix_pipeline_perf_smoke.py〔SPV 路径〕+ G14PLUS_RECORD §4 G14.9 记录 + 本契约 §8.10 本条；验证方式：L0 双格 PASS 逐字输出 + spirv-dis RayFlags 字面 + golden manifest BYTE-IDENTICAL + rurixc 524+/rurix-rt 209 测试结果 + bisect 归因轨迹）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-19 | 首版（G14.1 治理波立项）：双门状态 + 五波结构 + 5 P0 独立断言表（M-a 登记表方差带修订 / M-b UE benchmark 臂测量 / M-c Rurix 管线性能 / M-d 双端帧率对标+画质零降级守护 / M-e 回归门+漂移监控）+ guardrails 十三条 + Deferred 处置 + 立项裁决七条。 |
