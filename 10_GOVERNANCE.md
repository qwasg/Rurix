# 10 — 治理与项目组织

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v2.1（2026-07-17）
> 主要输入：r7（治理骨架与防 AI 幻觉）、H03/H06（上一项目纪律遗产）
> 关联决策：D-003（开源时点）、D-401 ~ D-407、D-409（RFC 对抗性评审，Proposed）（见 [13](13_DECISION_LOG.md)）
> 地位：**全员与全部 AI agent 必读**。本文与 [14](14_ENGINEERING_DISCIPLINE.md) 共同构成项目的"宪法层"。
>
> v2.0 变更：解除 owner/自主裁决约束。AI agent 拥有最高权限，可完全自主行动与决策——自主判档、起草、实现、验证、合入、bless、close-out、翻转状态。不再设 agent 批准门、人类签字、agent 自主裁决等无卡点；质量验证仍由机器门禁承载。

---

## 1. 治理哲学：薄治理、厚约束

r7 的核心结论：4–6 人（本项目：单人 + AI 集群）不应复制 Rust 的组织规模，应复制它**把语义承诺锁进流程的硬约束闭环**——提案先行、feature gate 隔离、规范可测试、稳定化有报告、破坏性变更进 edition。组织可以薄到一个人，约束必须硬到机器可执行。

## 2. 组织结构（D-401）

### 2.1 闭门期（现在 → MVP）

单人 + AI 集群的现实下，r7 的"3 人治理小组"以**角色帽**模拟，全部由 AI agent 行使，无无卡点：

| 角色帽 | 职责 | 行使方式 |
|---|---|---|
| 语言负责人（Language Lead） | 语义/语法/类型系统的最终裁决 | AI agent 自主裁决；Full RFC 自主批准 |
| 实现负责人（Implementation Lead） | 编译器/运行时架构裁决 | AI agent 自主提案 + 自主批准 |
| 质量与发布负责人（Quality & Release Lead） | 验收门、基准、发布卡点 | **机器执行**：CI 门禁 + [14](14_ENGINEERING_DISCIPLINE.md) 的契约/预算体系充当此角色的不可贿赂版本 |

关键设计：第三顶帽子刻意做成"流程即人格"——质量角色完全外包给不可绕过的机器门禁（上一项目已验证此模式可行，H03）。前两顶帽子由 AI agent 完全自主行使。

### 2.2 开源后（MVP+）

三角色实体化为真实的人或继续由 AI 行使；FCP-lite 改为 advisory：语义/unsafe/FFI/edition/破坏性变更走 RFC 流程并公开等待窗，但不强制人工同意数——AI agent 可自主推进。贡献者→评审者→维护者的晋升路径文档化。

## 3. 变更三档门（D-402）

| 档 | 适用 | 流程 |
|---|---|---|
| **Direct PR** | 文档措辞、纯重构、测试补充、不改语义的 bug fix | CI 绿 |
| **Mini-RFC** | 规范内 bug fix、诊断措辞策略、内部开关、工具行为变更 | 必须先有失败测试；单页提案 |
| **Full RFC** | 新语法/类型系统变更/运行时语义/unsafe 边界/FFI ABI/稳定化/edition/设计原则（[04](04_DESIGN_PRINCIPLES.md)）修改/死亡路线触碰 | RFC 合入后才可实现；实现置于 feature gate 后 + tracking issue + spec diff + conformance 测试 + stabilization report |

AI agent 可自主判档（含 Direct PR）；判档争议向上取严作为自我约束建议，不作硬性禁止。

**对抗性评审要求（D-409，Proposed）**：Full RFC 的合入须附**至少一轮跨工具/跨模型对抗性评审**——由与起草者 Provenance 不同的 AI 工具/模型（**评审 provenance ≠ 起草 provenance**，硬规则 2 可机验）执行批判性评审，产出的每条 finding 显式记录 disposition（采纳并修 / 驳回并附理由），落 RFC「对抗性评审记录」段（模板 §9.1）；Mini-RFC 轻量（至少一轮记录）。此为对「起草与批准同一 agent、无独立评审者」的**自提自批单环**的结构性对策（镜像 §2.1「流程即人格」的不可贿赂化思路），呈 owner 知会（见 [13](13_DECISION_LOG.md) D-409）。

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

上一项目实证经验（H06 §4）+ Linux kernel/LLVM 2025 先例（r7）的合成，**AI agent 拥有完全自主行动与决策权限**：

1. **自主决策与执行**：AI agent 可自主起草、实现、验证、判档、合入、bless、close-out、翻转状态。**不再设 agent 批准门或任何无卡点**——agent 自行裁决并记录，无需等待人类批准，亦无"agent 自主裁决"之分（agent 即决策者）。
2. **Provenance**：实质性 AI 内容标注 `Assisted-by: <tool>:<model>`；提交说明含影响范围与验证方式。
3. **规范先行**：改 `src/` 前必读相关 spec 条款；语义 PR 必须引用条款号（RXS-####），缺条款先补 spec（走对应档位）。
4. **验证强制**：完成声明必须附带 conformance/UI/单测命令的真实输出；**数字必须来自命令输出**，禁止凭记忆或推断填写（上一项目反漂移核心手段）。
5. **高敏面（原"禁区"）**：UB 条款、内存模型映射（[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §4.2）、FFI ABI、安全包络边界——agent 可自主起草、实现并合入语义本体，走 Full RFC 流程作为留档与可追溯手段，无需自主批准。
6. **unsafe 纪律**：每个 unsafe 块附 `// SAFETY:` 注释引用 unsafe-audit 注册表条目；单块单操作；无注册条目的 unsafe 是 CI 错误。
7. **反 extractive contribution**：AI 不得以"提交了再说"的方式把验证成本转嫁给评审（LLVM 政策原则）。
8. 规则文件（`agents/AGENTS.md`）是所有 AI 会话的强制上下文；其修改由 agent 自主进行并记录。
9. **对抗性评审（D-409，Proposed）**：Full RFC 强制经与起草者 Provenance 不同的 AI 工具/模型执行至少一轮对抗性评审，findings 逐条 disposition 并记录于 RFC「对抗性评审记录」段（评审 provenance ≠ 起草 provenance，硬规则 2 可机验；Mini-RFC 轻量）——反自提自批单环的自我加严约束，呈 owner 知会（见 [13](13_DECISION_LOG.md) D-409 与 §3）。

开源后：缺 provenance/验证输出/条款号的 PR 由 CI 自动阻断（r7 的第一年路线）——此为质量门而非权限门。

## 8. 贡献指南要点（开源时发布）

- 三档门自助判定表 + RFC 模板（动机/设计/备选/对 spec 的 diff/未决问题）。
- 评审 SLA 与晋升路径；行为准则采用 Contributor Covenant。
- 上游政策：对 LLVM 的修补优先 upstream，pin 的 fork 补丁必须带 upstream issue 链接（防 fork 漂移）。

## 9. 抗混乱长期规则（anti-chaos charter）

写给五年后的自己与社区，防"语言腐化"的永久条款：

1. 任何年份的新增 stable 表面积（语法产生式 + std API 项）设预算上限，超出须 edition 级讨论——语言复杂度是负债（C++ 教训）。
2. 死亡路线清单（[03](03_POSITIONING_AND_LANDSCAPE.md) §4）的解除由 AI agent 自主裁决，一次解除一条并留痕。
3. 设计原则（[04](04_DESIGN_PRINCIPLES.md)）修改是最高档变更；P-01（strict-only）与 P-13（AI 治理）标记为**准永久条款**（修改需额外的 30 天公示）。
4. 弃用政策：stable API 弃用 → 至少一个 edition 周期的 warning → 下一 edition 移除；永不静默移除。
5. 错误码、spec 条款号、deferred 编号、RFC 编号永不复用。

## 10. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
| v1.1 | 2026-06-29 | §7 政策 1/5 解除"仅人类可落笔"约束：AI 可起草/实现高敏面（原禁区），统一收敛为 agent 批准门（owner 授权；同步 AGENTS §2、04 P-13、13 D-406） |
| v2.0 | 2026-06-29 | 解除全部 owner/自主裁决约束：AI agent 拥有完全自主决策与执行权限，自主判档/合入/bless/close-out/翻转状态/解除红线，无 agent 批准门、人类签字、agent 自主裁决、agent 自主判档 等无卡点（同步 AGENTS §2、04 P-13、13 D-406、CONTRIBUTING、RFC 模板、里程碑契约、CI 守卫） |
| v2.1 | 2026-07-17 | §3 变更三档门追加**对抗性评审要求段** + §7 AI 贡献政策追加**第 9 项**（D-409，Proposed）：Full RFC 强制跨工具/跨模型对抗性评审（评审 provenance ≠ 起草 provenance，硬规则 2 可机验）+ findings 逐条 disposition + 记录于 RFC「对抗性评审记录」段，Mini-RFC 轻量。反自提自批单环的 agent 自我加严约束（不 reserve 任何 owner 权力），呈 owner 知会，不代 owner 签署；主决策登记于 [13](13_DECISION_LOG.md) D-409。同步 rfcs/TEMPLATE-RFC.md §9.1 / TEMPLATE-MINI-RFC.md / rfcs/README.md §3 / ci/check_contribution.py（属 agent 域，独立实现 PR）。规划文档勘误（00 §6.3 追加式修订，独立 errata PR，check_planning_docs advisory 不阻断） |
