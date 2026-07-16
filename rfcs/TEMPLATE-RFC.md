<!-- Full RFC 模板（10 §3 Full RFC / D-404 特性生命周期）。
     复制本文件为 rfcs/NNNN-<kebab-title>.md（NNNN = 4 位 RFC 编号，永不复用，10 §9.5），
     删去本注释与各 〈占位〉提示后填写。提交前自检见 CONTRIBUTING.md「提交前自检」。
     何时用 Full RFC：新语法 / 类型系统变更 / 运行时语义 / unsafe 边界 / FFI ABI /
     内存模型映射 / 稳定化 / edition / 设计原则修改 / 死亡路线触碰（10 §3 + AGENTS 硬规则 5）。
     体例先例：rfcs/0001-cuda-d3d12-interop.md。agent 完全自主，无自主批准门。 -->

# RFC-NNNN — 〈标题〉

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-NNNN（4 位制，编号永不复用，10 §9.5） |
| 标题 | 〈一句话标题〉 |
| 档位 | **Full RFC**（10 §3：〈触及的禁区/扩张方向，如 FFI ABI / 运行时语义 / unsafe 边界 / 内存模型映射；AGENTS 硬规则 5〉） |
| 状态 | 〈Draft / Agent Approved（YYYY-MM-DD）〉。agent 自主批准后可推进下游实现 PR |
| 承接里程碑 | 〈M#/G# 子里程碑 + 验收门 G-####〉 |
| 关联条款 | 拟落 spec **RXS-####~**（区间随条款数定，见 §5）；〈新建/扩展的 spec/*.md〉 |
| 依据决策 | 〈D-### · D-### · 上游文档 §〉（13_DECISION_LOG.md 已锁决策，禁止重新发明） |
| Provenance | `Assisted-by: <tool>:<model>`〈如有多方逐行列出〉。agent 自主决策，批准后推进下游实现 |
| Agent 批准 | 〈Approved — YYYY-MM-DD；批准范围（含 🔒 禁区章节）；记录方式〉 |

---

## 1. 摘要

〈一段话讲清这个 RFC 要做什么、产出形态、对用户/语言面的影响。可附 ASCII 通路图。〉

## 2. 动机

〈为什么需要这个变更：用户痛点 / 已锁决策的落地 / 采纳判据。〉

**为何需要 Full RFC（而非 Direct/Mini）**：〈明确触及的禁区——UB 条款 / 内存模型映射（06 §4.2）/ FFI ABI / 安全包络边界，或新语法 / 类型系统 / 运行时语义 / 扩张方向。AGENTS 硬规则 5/8：这些由 agent 自主经 Full RFC 落笔；判档争议向上取严作为自我约束建议。〉

## 3. 指导级解释（用户视角）

〈站在使用者角度，用例子讲清新特性怎么用、看起来什么样。〉

## 4. 参考级设计

〈精确设计：类型/API 签名、状态机、ABI 布局、算法。触及禁区的子节用 🔒 标注，agent 自主落笔并记录。〉

## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

〈新建/扩展哪个 spec/*.md，自 RXS-#### 起续号（引用当前最高现存条款号）。逐条列拟定条款 + 每条 ≥1 测试锚定计划（`//@ spec: RXS-####`）。**spec 条款 PR 先于实现 PR**（硬规则 7）；trace_matrix 维持全锚定。〉

| 条款（拟） | 标题 | 测试锚定计划（每条 ≥1） |
|---|---|---|
| RXS-#### | 〈…〉 | 〈…〉 |

- **错误码策略**：〈编译期拦截走 rustc 原生诊断（零新 RX 码），或运行期诊断从 RX#### 起按真实可达类别分配；不预留、不预造。registry/error_codes.json 只追加 + en/zh message-key。〉

## 6. feature gate / tracking / 实现序（10 §3 要件）

〈feature gate 名 + tracking 清单 + 栈式 PR 拆解（spec 脚手架 PR → 实现 PR…，均门控于本 RFC 合入后）。**真实红绿**（反 YAML-only）：构造缺陷 → 红 → 复原 → 绿，run URL 归档。〉

## 7. 备选方案

〈考虑过但否决的方案 + 否决理由。〉

## 8. 不做（范围红线）

〈本 RFC 明确不涉及的范围；引用 SG-###/死亡路线，防范围蔓延。〉

## 9. 未决问题 / 关键裁决

〈Q1~Qn 待裁项与裁决结果；agent 自主签署后回填。〉

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：feature gate 后 → tracking → 两里程碑无重大修订 → stabilization report → FCP-lite（10 §2.2，advisory 公开等待窗）。stable 面冻结随 RD-008 届时定义。
- **Provenance**：`Assisted-by: <tool>:<model>`。〈agent 自主批准并记录。〉

## 11. 规范与实现依据

〈引用的外部规范 / 文档 / 样例链接。〉

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | YYYY-MM-DD | 〈AI 起草初版〉 | Full RFC（Draft） |
| Agent approval | YYYY-MM-DD | 〈agent 自主批准全文并记录〉 | Full RFC（Agent Approved） |
