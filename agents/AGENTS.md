# AGENTS.md — Rurix AI 会话强制上下文 v1

> 版本:v1.0(2026-06-11),M0 交付物 D-M0-5。
> 地位:所有 AI 会话的强制加载上下文(10 §7.8 / 14 §10.1)。本文件的修改是 **Mini-RFC 级**变更。
> 本文是 10 号与 14 号文档的执行摘要 + 命令清单,**不是事实源**——冲突时以编号文档为准。

---

## 1. 上工前必读(按序)

1. [04_DESIGN_PRINCIPLES.md](../04_DESIGN_PRINCIPLES.md) — 14 条设计公理(P-01 strict-only 与 P-13 AI 治理为准永久条款)。
2. [10_GOVERNANCE.md](../10_GOVERNANCE.md) §7 — AI 贡献政策八条(D-406)。
3. [14_ENGINEERING_DISCIPLINE.md](../14_ENGINEERING_DISCIPLINE.md) — 契约/预算/deferred/证据分级/测试纪律。
4. [13_DECISION_LOG.md](../13_DECISION_LOG.md) — 已锁定决策,禁止重新发明。
5. 当前里程碑契约:[milestones/m0/M0_CONTRACT.md](../milestones/m0/M0_CONTRACT.md)。

## 2. 十条硬规则(违反即返工)

1. **Human-in-the-loop**:你的产出必经人类批准合入;不得代签任何署名承诺。
2. **Provenance**:实质性 AI 内容在提交说明标注 `Assisted-by: <tool>:<model>`,并写明影响范围与验证方式。
3. **验证强制**:声明"完成"必须附验证命令的真实输出;声明"性能提升"必须附证据 JSON 路径。**所有数字必须来自命令输出**,禁止凭记忆或推断填写。
4. **证据分级**:性能叙述必须标注 `measured_local` / `unlocked` / `estimated`(14 §5);无证据的阈值一律 `estimated` 占位。
5. **禁区**(只能人类经 Full RFC 落笔):UB 条款、内存模型映射(06 §4.2)、FFI ABI、安全包络边界。
6. **只追加目标**:以下文件你只能追加,任何对既有条目的修改自动触发人工审查(14 §10.4):
   - `milestones/*/M*_CONTRACT.md`(close-out 区之外 0-byte)
   - `milestones/*/m*_budget.json` 既有条目
   - `registry/deferred.json` / `registry/spike_gating.json` 既有条目
   - `evidence/` 目录全部文件
   - `00_*.md` 至 `14_*.md` 规划文档集(勘误走 00 §6.3 追加式修订,独立 PR)
7. **规范先行**:改 `src/` 前必读相关 spec 条款;语义 PR 必须引用条款号(RXS-####)或 deferred/RFC 编号;缺条款先补 spec。(M0 期无 src/ 与 spec/ 实体,本条自 M1 起实操生效。)
8. **变更判档**:你无权自行判档为 Direct PR;判档争议向上取严(10 §3)。
9. **unsafe 纪律**:每个 unsafe 块附 `// SAFETY:` 注释并引用 unsafe-audit 注册表条目;单块单操作;无注册条目的 unsafe 是 CI 错误。(自 unsafe 代码出现起生效。)
10. **反 extractive contribution**:不得以"提交了再说"把验证成本转嫁给评审。

## 3. 做不完的事怎么办

- 注册 deferred:[registry/deferred.json](../registry/deferred.json) 追加 `RD-###`(内容/原因/回填条件/承接里程碑);代码侧 `// STUB(RD-###)` 双侧标注。
- 想做范围外的事:先查 [registry/spike_gating.json](../registry/spike_gating.json)——已 gating 的方向不要提案,触发条件不满足时唯一合法动作是留痕。

## 4. 验证命令清单(M0 期)

> 占位框架:具体命令在 M0.1–M0.3 落地后回填实测形式(M0_PLAN.md §4.3),回填时逐条真实执行过。

| 场景 | 命令(占位) | 产出要求 |
|---|---|---|
| 注册表/预算/证据 schema 校验 | `TODO(M0.1)` schema 校验脚本 | 全过,输出贴 PR |
| guardrail 核对 | `TODO(M0.1)` guardrail 脚本(CI_GATES.md §4) | 全过 |
| harness 统计单测 | `TODO(M0.2)` | 全过 |
| 基准冒烟(GPU) | `TODO(M0.3)` SAXPY 装载 + 正确性比对 | pass + 输出贴 PR |
| 完整采样(性能声明用) | `TODO(M0.3)` 按 BENCH_PROTOCOL.md §3 | 证据 JSON 路径贴 PR |

## 5. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版(M0 交付物;§4 命令清单为占位框架) |
