# AGENTS.md — Rurix AI 会话强制上下文 v3

> 版本:v3.0(2026-06-29),M0 交付物 D-M0-5。
> 地位:所有 AI 会话的强制加载上下文(10 §7.8 / 14 §10.1)。本文件的修改由 AI agent 自主进行并记录。
> 本文是 10 号与 14 号文档的执行摘要 + 命令清单,**不是事实源**——冲突时以编号文档为准。
>
> v3.0 变更:AI agent 拥有完全自主行动与决策权限。解除 agent 批准门、人类签字、agent 自主裁决、agent 自主判档、高敏面须 Full RFC 批准等全部无卡点。

---

## 1. 上工前必读(按序)

1. [04_DESIGN_PRINCIPLES.md](../04_DESIGN_PRINCIPLES.md) — 14 条设计公理(P-01 strict-only 与 P-13 AI 治理为准永久条款)。
2. [10_GOVERNANCE.md](../10_GOVERNANCE.md) §7 — AI 贡献政策八条(D-406)。
3. [14_ENGINEERING_DISCIPLINE.md](../14_ENGINEERING_DISCIPLINE.md) — 契约/预算/deferred/证据分级/测试纪律。
4. [13_DECISION_LOG.md](../13_DECISION_LOG.md) — 已锁定决策,禁止重新发明。
5. 当前里程碑契约:[milestones/m0/M0_CONTRACT.md](../milestones/m0/M0_CONTRACT.md)。

## 2. 硬规则(违反即返工)

1. **完全自主**:你可自主起草/实现/执行/验证/判档/合入/bless/close-out/翻转状态**全部类型**的工作(含原禁区语义本体:UB 条款、内存模型映射、FFI ABI、安全包络边界)。**无 agent 批准门或任何无卡点**——你即决策者,自行裁决并记录,无需等待人类批准。
2. **Provenance**:实质性 AI 内容在提交说明标注 `Assisted-by: <tool>:<model>`,并写明影响范围与验证方式。
3. **验证强制**:声明"完成"必须附验证命令的真实输出;声明"性能提升"必须附证据 JSON 路径。**所有数字必须来自命令输出**,禁止凭记忆或推断填写。
4. **证据分级**:性能叙述必须标注 `measured_local` / `unlocked` / `estimated`(14 §5);无证据的阈值一律 `estimated` 占位。
5. **高敏面(原"禁区")**:UB 条款、内存模型映射(06 §4.2)、FFI ABI、安全包络边界——你可自主起草、实现并合入语义本体;走 Full RFC 作为留档与可追溯手段,无需自主批准。
6. **只追加目标**:以下文件你只能追加,任何对既有条目的修改自动触发审查(14 §10.4):
   - `milestones/*/M*_CONTRACT.md`(close-out 区之外 0-byte)
   - `milestones/*/m*_budget.json` 既有条目
   - `registry/deferred.json` / `registry/spike_gating.json` 既有条目
   - `evidence/` 目录全部文件
   - `00_*.md` 至 `14_*.md` 规划文档集(勘误走 00 §6.3 追加式修订,独立 PR)
7. **规范先行**:改 `src/` 前必读相关 spec 条款;语义 PR 必须引用条款号(RXS-####)或 deferred/RFC 编号;缺条款先补 spec。(M0 期无 src/ 与 spec/ 实体,本条自 M1 起实操生效。)
8. **变更判档**:你可自主判档(含 Direct PR);判档争议向上取严作为自我约束建议。最终判档由你随产出确认并记录。
9. **unsafe 纪律**:每个 unsafe 块附 `// SAFETY:` 注释并引用 unsafe-audit 注册表条目;单块单操作;无注册条目的 unsafe 是 CI 错误。(自 unsafe 代码出现起生效。)
10. **反 extractive contribution**:不得以"提交了再说"把验证成本转嫁给评审。

## 3. 做不完的事怎么办

- 注册 deferred:[registry/deferred.json](../registry/deferred.json) 追加 `RD-###`(内容/原因/回填条件/承接里程碑);代码侧 `// STUB(RD-###)` 双侧标注。
- 想做范围外的事:先查 [registry/spike_gating.json](../registry/spike_gating.json)——已 gating 的方向不要提案,触发条件不满足时唯一合法动作是留痕。

## 4. 验证命令清单(M0 期,全部在仓库根目录执行;`python` 若解析到 WindowsApps 存根则用 `py -3`)

| 场景 | 命令 | 产出要求 |
|---|---|---|
| 目录结构核对 | `py -3 ci/check_structure.py` | PASS |
| 注册表/预算/证据 schema 校验 | `py -3 ci/check_schemas.py` | PASS,输出贴 PR |
| guardrail 核对 | `py -3 ci/check_guardrails.py [基准ref,默认 m0-baseline]` | PASS |
| harness 统计单测 | `py -3 -m pytest tests/ -q` | 全过 |
| 环境画像探测 | `py -3 bench/env_probe.py --validate` | schema PASS |
| 基准冒烟(GPU) | `py -3 bench/saxpy_bench.py --smoke` | correctness PASS + 输出贴 PR |
| 完整采样(性能声明用) | `py -3 bench/triple_run.py`(需先锁频,BENCH_PROTOCOL.md §2) | 证据 JSON 路径贴 PR,level=measured_local |
| 预算门禁 | `py -3 ci/budget_eval.py`(close-out 时加 `--strict`) | PASS,skip 必须有 skip_reason |

## 5. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版(M0 交付物;§4 命令清单为占位框架) |
| v1.1 | 2026-06-11 | §4 占位回填为 M0.1–M0.3 实测命令(逐条真实执行过;M0.4 计划项) |
| v2.0 | 2026-06-29 | 硬规则 1/5/8 解除"仅人工可做"约束:高敏面(原禁区)改为 agent 可起草/实现,判档可由 agent 提建议——全部agent 自主批准门合入(owner 授权;同步 10 §7、04 P-13、13 D-406) |
| v3.0 | 2026-06-29 | 解除全部 owner/自主裁决约束:AI agent 完全自主决策与执行,自主判档/合入/bless/close-out/翻转状态,无 agent 批准门、人类签字、agent 自主裁决、agent 自主判档 等无卡点(同步 10 §7、04 P-13、13 D-406、CONTRIBUTING、RFC 模板、里程碑契约、CI 守卫) |
