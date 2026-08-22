<!-- Assisted-by: Claude Fable 5（G14plus 波0 D-409 对抗性评审轮次——与起草轮次隔离） -->
# RFC-0030 D-409 对抗性评审记录（第 1 轮，2026-08-22）

> **性质**：D-409 对抗性评审交付物（G14plus 波0 治理批）。评审对象 = `rfcs/0030-g14plus-pipeline-structural-optimization.md` v0.1（起草轮次产物）。
> **Provenance 偏差如实登记**：评审者与起草者同模型同会话族；独立性兑现形态 = **评审轮次隔离**（评审轮次零复用起草轮次结论，逐条独立重读事实源后自证或反驳）+ 事实源独立核对清单（下表）。偏差与 RFC-0029 先例同族（同会话族评审轮次隔离），效力自限：跨工具评审者可得时建议补一轮；留 G14.12 M-h 门与 closeout 终审复核锚。
> **评审方法**：对 v0.1 七个语义面逐面执行三问——① 语义面是否与在树事实源冲突（逐文件重读核对）；② 判据是否机器可核且 RED 臂闭合；③ 实施者按字面执行是否存在静默错误窗口。

## 1. 事实源独立核对清单

| 事实源 | 核对结论 |
|---|---|
| `G14_CONTRACT.md` front matter guardrails（temporal 底座 0-byte 条款） | 字面 =「确需演进必须独立 Full RFC 显式修订行」——v0.1 §4.1 档位论证成立 |
| `ci/g14_rurix_pipeline_perf_smoke.py` temporal_base_0byte 检查 | diff 范围 = `src/rurix-render/src/temporal/` vs f4c8da0b——v0.1 初稿未写明该范围（F1） |
| `registry/deferred.json` RD-045 backfill_condition | 字面 =「定位根因+生产化缺陷修复+Full RFC 评估（触 RXS-0357 L2 确定性协议面）」——v0.1 §4.2 评估结论形态成立；但 N=20 统计置信未标注（F3） |
| `src/rurixc/src/vulkan_codegen.rs` RayFlags 面 | 恒 `OpaqueKHR`；`ray_query_check.rs` 三态协议初始化集——新内建须入集；`committed_t()` 在 first-hit 上返回任意命中的语义错误面 v0.1 未界定（F2） |
| `kernels/g13_tsr_resample.rx` / `g13_tsr_resolve.rx` | `ThreadCtx<1>` 无 numthreads 确认；逐像素独立无跨像素交互（文件头确定性协议注释）——调度重排位级不变论证成立 |
| `milestones/g13/g13_ue_upscale_gap_registry.json` | t50/t67 deficit 参照 = 本端 tier100 收敛帧——cornell 样本削减的参照系连锁风险确认（F4） |
| `milestones/g14/g14_3_stage_a_digest_anchor.json` | 锚描述字面 =「程序收割禁手写」；v0.1 未写收割前置门（F5） |
| `src/rurix-rt/src/render_exec.rs`（HEAD 基线，异己面已清场） | `execute_persistent_frame` 为既有消费面；G13 门等间接消费——0-byte 保留义务未写明（F6） |
| `rfcs/0029-*.md` 伞形先例 | 七面单章先例成立；单面回退语义按处置树登记不撤销 RFC（F7 裁决同模） |

## 2. Findings（分级 + disposition）

| # | 级别 | finding 字面 | disposition（v0.2 修法批落实） |
|---|---|---|---|
| F1 | high | §4.1 未显式声明 temporal/ 目录 diff 范围与 M-c 机核对应——实施者可能在 temporal 目录内加文件自以为「加性安全」实则打红 M-c | §4.1 L2 增补「`src/rurix-render/src/temporal/` 目录 git diff（vs G14.0 ref f4c8da0b）恒空」机核字面 |
| F2 | high | §4.6 未界定 first-hit query 上 `committed_t()`/`committed_primitive_index()` 消费的语义错误——返回任意命中，误用即静默错图且 digest 双跑自洽（双跑同错不可检出） | §4.6 L2 消费边界字面 + conformance reject 语料义务 + 主射线禁用字面 |
| F3 | med | §4.2 L4 的 N=20 快筛零检出可能被下游误读为修复闭环——历史检出率 p≈1.9% 下 N=20 零检出置信仅 ~68% | L4 统计诚实字面 + 全战役累计 ≥150 轮口径 + RD-045 维持 open 裁决字面 |
| F4 | med | §4.4 阴影阶梯无「按序启用」硬约束——执行压力下可能跳级直上样本削减（deficit 参照系连锁重标定的最重路径） | L2 阶梯禁跳级 + 每级前置「上级实测差距 >3%」+ t100 参照系连锁警示字面 |
| F5 | med | §4.7 锚重收割无前置门——若在 RD-045 修复验证前收割，锚可能锚定到漂移态（后续全部 digest 守护失义） | §4.7 前置门字面：M-c 复跑绿一次 + RD-045 修复后复现测试零检出 |
| F6 | low | §4.3 L2 未声明既有 `execute_persistent_frame` 0-byte 保留——G13 门等既有消费方回归面不明 | L2 「submit+collect 顺序调用等价形态 0-byte 保留」字面 |
| F7 | low | 伞形七面偏大，单面回退（如 §4.4 超带回退）时 RFC 状态语义不清 | 裁决登记：各面条款独立分波兑现，单面回退按 §4.4 L3 处置树登记不撤销 RFC（RFC-0029 先例同模）——不修文，登记于 §9.1 |

## 3. 评审结论

v0.1 七个语义面与在树事实源零冲突（核对清单 9 项全过）；F1~F6 六项修法义务 + F7 一项裁决登记，全部于 v0.2 修法批落实。**建议翻 Agent Approved 的前置**：主会话核对本 RFC ↔ 契约 §8.8 立项记录 ↔ MAP 附录 A M-h 行三面一致。

签署：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署；D-409 评审轮次隔离形态，provenance 偏差如实登记见头注）。
