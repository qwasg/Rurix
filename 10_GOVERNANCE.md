# 10 — 治理与项目组织

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 主要输入：r7（治理骨架与防 AI 幻觉）、H03/H06（上一项目纪律遗产）
> 关联决策：D-003（开源时点）、D-401 ~ D-407（见 [13](13_DECISION_LOG.md)）
> 地位：**全员与全部 AI agent 必读**。本文与 [14](14_ENGINEERING_DISCIPLINE.md) 共同构成项目的"宪法层"。

---

## 1. 治理哲学：薄治理、厚约束

r7 的核心结论：4–6 人（本项目：单人 + AI 集群）不应复制 Rust 的组织规模，应复制它**把语义承诺锁进流程的硬约束闭环**——提案先行、feature gate 隔离、规范可测试、稳定化有报告、破坏性变更进 edition。组织可以薄到一个人，约束必须硬到机器可执行。

## 2. 组织结构（D-401）

### 2.1 闭门期（现在 → MVP）

单人所有者 + AI 集群的现实下，r7 的"3 人治理小组"以**角色帽**模拟：

| 角色帽 | 职责 | 行使方式 |
|---|---|---|
| 语言负责人（Language Lead） | 语义/语法/类型系统的最终裁决 | 所有者本人；Full RFC 的批准签字 |
| 实现负责人（Implementation Lead） | 编译器/运行时架构裁决 | 所有者本人或授权 AI 提案 + 人工批准 |
| 质量与发布负责人（Quality & Release Lead） | 验收门、基准、发布卡点 | **机器执行**：CI 门禁 + [14](14_ENGINEERING_DISCIPLINE.md) 的契约/预算体系充当此角色的不可贿赂版本 |

关键设计：第三顶帽子刻意做成"流程即人格"——单人项目最大的治理风险是自我放水，对策是把质量角色完全外包给不可绕过的机器门禁（上一项目已验证此模式可行，H03）。

### 2.2 开源后（MVP+）

三角色实体化为真实的人；引入 FCP-lite：语义/unsafe/FFI/edition/破坏性变更需 3 人中至少 2 人同意且含语言负责人，5–7 天公开等待窗（r7）。贡献者→评审者→维护者的晋升路径文档化。

## 3. 变更三档门（D-402）

| 档 | 适用 | 流程 |
|---|---|---|
| **Direct PR** | 文档措辞、纯重构、测试补充、不改语义的 bug fix | 评审 + CI 绿 |
| **Mini-RFC** | 规范内 bug fix、诊断措辞策略、内部开关、工具行为变更 | 必须先有失败测试；单页提案 + 语言或实现负责人批准 |
| **Full RFC** | 新语法/类型系统变更/运行时语义/unsafe 边界/FFI ABI/稳定化/edition/设计原则（[04](04_DESIGN_PRINCIPLES.md)）修改/死亡路线触碰 | RFC 合入后才可实现；实现置于 feature gate 后 + tracking issue + spec diff + conformance 测试 + stabilization report + FCP-lite |

判档争议向上取严。AI agent 无权自行判档为 Direct PR（§7）。

## 4. 仓库一等公民目录（D-403）

进入实现期的主仓库结构承诺（r7）：

```
spec/           语言规范：唯一语义事实源（FLS 风格条款：Syntax / Legality /
                Dynamic Semantics / UB / Implementation Requirements，
                条款编号 RXS-####，traceability matrix 工具生成）
rfcs/           已接受的 RFC 存档（编号不复用）
conformance/    语义验收测试（唯一验收边界；每条款 ≥1 测试锚定）
tests/ui/       诊断 golden 测试（.stderr snapshot）
unsafe-audit/   unsafe 原语注册表 + 验证义务（RustBelt 式）+ 审计记录
agents/         AI agent 规则文件（AGENTS.md 等，强制加载）
```

**规范领导实现**（r7 对 FLS 的反向教训）：先写条款再写实现；缺条款的语义 PR 必须先补 spec。设计期的本文档集在实现期降格为历史档案（[00](00_MASTER_INDEX.md) §6.4），spec/ 接管事实源地位。

## 5. 特性生命周期（D-404）

Rust 机制裁剪版：

```
想法 → Full RFC → 合入 rfcs/
  → 实现于 feature gate 后（#[feature(views_transpose)]，nightly 工具链可用）
  → tracking issue（实现状态/未决问题/测试清单）
  → 稳定化条件：spec 条款齐 + conformance 齐 + UI 测试齐 + 两个里程碑无重大修订
  → stabilization report → FCP-lite → 进入 stable
```

- "实现可运行 ≠ 语义已承诺"（r7）写入贡献指南首条。
- **Edition 机制**：v1.0 前不需要（没有稳定用户）；开源 + 1.0 后引入年度 edition 窗口承载破坏性迁移（含 `rx fix --edition` 自动迁移），机制设计预留在 span 层（span 携带 edition，r1——编译器从第一天按此实现，成本极低，收益是未来不重构）。

## 6. 稳定性与版本政策（D-405）

- **版本**：SemVer。0.x（MVP 期）：minor 可破坏，错误码与诊断 JSON schema 尽早稳定（工具生态依赖）。1.0（开源后第一个 LTS 质量版本）：stable 面破坏只能走 edition。
- **稳定面清单**（P-10）：语言语法（stable 特性）、std/core/gpu 的 stable API、C ABI 导出约定、`rurix.toml`/`rurix.lock` schema、诊断 JSON schema、错误码含义（可加不可改义）。
- **不承诺稳定**：内部 IR、PTX 产物形状、telemetry 字段集、nightly 特性。
- **发布节奏**：MVP 期里程碑驱动；开源后 6 周 train 模式（Rust 节奏的放宽版），nightly 每日。
- **工具链发布门**：conformance 全绿 + UI golden 全绿 + L1 基准无 Critical 回归 + SBOM/签名齐备（[08](08_RUNTIME_AND_TOOLING.md) §9）——发布即质量负责人角色帽的机器化行使（§2.1）。

## 7. AI 贡献政策（D-406，从第一天生效）

上一项目实证经验（H06 §4）+ Linux kernel/LLVM 2025 先例（r7）的合成，**对所有者本人的 AI 使用同样生效**：

1. **Human-in-the-loop**：AI 产出必经人类批准合入；AI 不得代签任何形式的署名承诺。
2. **Provenance**：实质性 AI 内容标注 `Assisted-by: <tool>:<model>`；提交说明含影响范围与验证方式。
3. **规范先行**：改 `src/` 前必读相关 spec 条款；语义 PR 必须引用条款号（RXS-####），缺条款先补 spec（走对应档位）。
4. **验证强制**：完成声明必须附带 conformance/UI/单测命令的真实输出；**数字必须来自命令输出**，禁止凭记忆或推断填写（上一项目反漂移核心手段）。
5. **禁区**：AI 不得定义/修改 UB 条款、内存模型映射（[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §4.2）、FFI ABI、安全包络边界——这些只能由人类经 Full RFC 落笔。
6. **unsafe 纪律**：每个 unsafe 块附 `// SAFETY:` 注释引用 unsafe-audit 注册表条目；单块单操作；无注册条目的 unsafe 是 CI 错误。
7. **反 extractive contribution**：AI 不得以"提交了再说"的方式把验证成本转嫁给评审（LLVM 政策原则）。
8. 规则文件（`agents/AGENTS.md`）是所有 AI 会话的强制上下文；其修改是 Mini-RFC 级。

开源后升级：缺 provenance/验证输出/条款号的 PR 由 CI 自动阻断（r7 的第一年路线）。

## 8. 贡献指南要点（开源时发布）

- 三档门自助判定表 + RFC 模板（动机/设计/备选/对 spec 的 diff/未决问题）。
- 评审 SLA 与晋升路径；行为准则采用 Contributor Covenant。
- 上游政策：对 LLVM 的修补优先 upstream，pin 的 fork 补丁必须带 upstream issue 链接（防 fork 漂移）。

## 9. 抗混乱长期规则（anti-chaos charter）

写给五年后的自己与社区，防"语言腐化"的永久条款：

1. 任何年份的新增 stable 表面积（语法产生式 + std API 项）设预算上限，超出须 edition 级讨论——语言复杂度是负债（C++ 教训）。
2. 死亡路线清单（[03](03_POSITIONING_AND_LANDSCAPE.md) §4）的解除只能由项目所有者批准且一次解除一条。
3. 设计原则（[04](04_DESIGN_PRINCIPLES.md)）修改是最高档变更；P-01（strict-only）与 P-13（AI 治理）标记为**准永久条款**（修改需额外的 30 天公示）。
4. 弃用政策：stable API 弃用 → 至少一个 edition 周期的 warning → 下一 edition 移除；永不静默移除。
5. 错误码、spec 条款号、deferred 编号、RFC 编号永不复用。

## 10. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
