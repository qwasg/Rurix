<!-- Mini-RFC 模板（10 §3 Mini-RFC = 单页提案 + 失败测试先行 + agent 自主批准）。
     复制本文件为 rfcs/mini-NNNN-<kebab-title>.md（MR-NNNN = Mini-RFC 序列，独立于
     Full-RFC 的 RFC-#### 命名空间，编号永不复用，10 §9.5），删去本注释与各 〈占位〉后填写。
     何时用 Mini-RFC：规范内 bug fix / 诊断措辞策略 / 内部开关 / 工具行为变更 /
     规则文件（agents/AGENTS.md）级修改。**必须先有失败测试**（10 §3）。
     体例先例：rfcs/mini-0001-async-buffer.md。判档不清 → 向上取严（自我约束建议）。agent 完全自主，无自主批准门。 -->

# Mini-RFC MR-NNNN — 〈标题〉

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-NNNN**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行，10 §3） |
| 标题 | 〈一句话标题〉 |
| 档位 | **Mini-RFC**（10 §3：〈量级——内部开关 / 工具行为 / 诊断措辞；**不触** UB / 内存模型映射 / FFI ABI / 安全包络禁区，见 §3〉）。〈agent 自主裁为 Mini-RFC（YYYY-MM-DD）〉 |
| 状态 | 〈Approved — YYYY-MM-DD（agent 自主批准并记录）〉 |
| 承接里程碑 | 〈M#/G# 子里程碑 + 验收门 G-####〉 |
| 关联条款 | 〈拟落 spec RXS-####~（区间随条款数定），或「零新 RXS」（复用既有条款 / 纯工具行为）〉 |
| 依据决策 | 〈D-### · 上游文档 § · 先例 MR-####〉 |
| Provenance | `Assisted-by: <tool>:<model>`。agent 自主决策，批准后推进下游 PR |
| 失败测试先行 | 〈path/to/failing_test —— 引用拟新增能力；当前 main 上 RED（能力尚不存在），实现 PR 落地后转为有意义的拦截/通过。10 §3 Mini「必须先有失败测试」〉 |

---

## 1. 摘要

〈一段话讲清要做什么、复用了什么既有面、产出形态。强调最大化复用、不重新发明。〉

## 2. 设计（用户视角 + 形态）

〈类型面 / API / 工具行为的具体形态；可附 code block 或表格。复用项逐一列出来源 + 形态（语义 0-byte 的标注清楚）。〉

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**：〈不触 AGENTS 硬规则 5 / 10 §7.5 禁区（UB / 内存模型映射 / FFI ABI / 安全包络）的论证。〉
- **非 Direct**：〈为何不是纯工程实现——通常因触及执行期新决策面 / 工具行为变更 / 规则文件级修改；硬规则 8「判档争议向上取严」+ 先例。〉
- **升档触发条件（实现期守卫）**：〈若实现期发现确需扩 ABI / 改借用检查器 / 触内存模型 / 安全包络，则**停手升 Full RFC**（向上取严），不在 spec/impl 自行落笔。〉

## 4. 错误码 / 影响 / 范围

〈错误码策略（零新 RX 码，或从 RX#### 起按需，不预造）；向后兼容（既有语义面 0-byte）；范围红线。〉

## 5. 失败测试先行（10 §3 Mini 硬性）

〈失败测试的路径 + 它编码的意图 + 当前 main 上为何 RED + 实现落地后如何转绿/转为有意义的拦截。〉

## 6. 影响 / 向后兼容 / 范围

- **向后兼容**：〈纯追加；既有语义面 0-byte；默认回归网是否依赖 device。〉
- **范围红线**：〈不做的事，引用 SG-###/红线。〉

## 7. Agent 批准

> 〈**Approved — YYYY-MM-DD**。agent 自主批准本 Mini-RFC（§2 形态 + §3 判档 + §4 错误码 + §6 范围）并记录。device 真跑 / 证据回填 / 计数器兑现 / 合入均由 agent 自主签署。〉

## 7.1 对抗性评审记录（轻量，D-409 Mini-RFC 档 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409）

> 〈**Mini-RFC 轻量**：至少一轮由与起草者 Provenance **不同**的 AI 工具/模型执行的对抗性评审记录（评审 provenance ≠ 起草 provenance）。findings 少可一行带过 disposition。Full RFC 的强制版见 [`TEMPLATE-RFC.md`](TEMPLATE-RFC.md) §9.1。〉
>
> - **评审者 provenance**：`Assisted-by: <评审 tool>:<评审 model>`（须 ≠ 起草 Provenance）。
> - **评审轮次**：〈第 N 轮，YYYY-MM-DD〉。
> - **Findings 与 disposition**：〈F1 …（采纳并修 §X／驳回 + 理由）；无实质 finding 则记「一轮评审无阻断项」并留评审者 provenance〉。
