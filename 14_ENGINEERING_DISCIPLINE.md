# 14 — 工程纪律与验证体系

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 来源：上一项目 P7–P15 共 9 个阶段、约 60 个里程碑迭代成型的方法论（H03），按 H06 的迁移建议升级载体后移植。
> 地位：**全员与全部 AI agent 必读**。与 [10](10_GOVERNANCE.md) 共同构成宪法层；本文管"怎么做事"，10 号管"谁能决定什么"。

---

## 1. 里程碑契约制

每个里程碑（[11](11_ROADMAP.md) 的 M0–M8 及其内部 1–2 周小里程碑）开工前先写**契约文档**，固化四要素：

1. 范围（in-scope / out-of-scope / 显式 deferred）；
2. 交付物清单（代码、测试、bench、文档各多少）；
3. 验收门（哪些测试套件、什么数字算过）；
4. guardrails（哪些东西字节不许动）。

**close-out 只追加**：验收记录与修订日志追加在契约尾部，既有条款 0-byte 修改。这条纪律在上一项目约 60 个里程碑中实现了零验收漂移（H04 §1），是 AI 重度参与下最关键的反漂移机制（AI 极易在执行中悄悄重定义成功）。

**载体升级（H06）**：契约的结构化字段（范围/交付物/验收门/guardrails/deferred 引用）写成 YAML 头 + markdown 正文，核对项可被脚本提取执行。契约模板含"语义变更需 RFC 编号"字段（与 [10](10_GOVERNANCE.md) §3 联动）。

## 2. Guardrails：机器执行的不变量清单

每个里程碑 close-out 必须通过 guardrail 脚本（非人工 checklist——上一项目证明 `git diff --stat` 字节核对远比"评审说没改"可靠，H03 §2）：

**Rurix 的常驻 guardrail 集（随阶段增长）**：

- 历史预算 JSON `git diff --stat` 为空；
- stable 公开面（语法 stable 集 / std-core-gpu stable API / C ABI / 诊断 JSON schema / 错误码语义）无未走 RFC 的变更（API 快照 diff）；
- spec 条款不被实现 PR 顺手改写（spec/ 目录变更必须携带对应档位标记）；
- 既有计数器/派生指标不消失不回归；
- 既有 conformance / UI golden / IR golden 全绿且 snapshot 未被未审批 bless；
- NVIDIA 再分发白名单审计通过；
- unsafe 块均有注册条目（unsafe-audit 完整性扫描）。

## 3. 预算 JSON + 命名空间门禁

上一项目 `regression.py` 框架直接移植（H05 §2 标注★级资产），断言内容替换：

- **编译性能预算**：冷启动编译 / 增量 check 延迟 / 单 kernel PTX 重生成 / 内存峰值（[07](07_COMPILER_ARCHITECTURE.md) §6）；
- **运行时 kernel 预算**：L1/L2 基准吞吐与延迟（[08](08_RUNTIME_AND_TOOLING.md) §4）；
- **结构**：每阶段一个 `mX_budget.json`（entries + ratio_assertions + counter_assertions），多预算合并加载 + 命名空间强制前缀 + 冲突检测；每新增预算配标准 namespace check 单测（上一项目模板照搬）。
- **占位阈值规则（P-09 硬化）**：无真实证据的阈值标 `estimated`，evaluator 自动 skip 且输出 skip_reason 留痕——但**占位存活不得超过 2 个里程碑**，逾期该项自动判 FAIL 并阻塞所在里程碑关闭。这是对上一项目"占位阈值带到封板"教训的机制性反转（H04 §2.2）。

## 4. Deferred 模型（债务显式化）

- 任何做不完/做不了的事必须编号注册：`RD-###`（Rurix Deferred），写明内容、原因、回填条件、承接里程碑。
- **载体升级**：单一结构化注册表 `deferred.json`（编号/状态/承接者/回填条件），文档只引用编号——不再四处手工同步（上一项目 CLAUDE.md 膨胀的直接解药，H03 §4）。
- 生命周期：deferred 只能被**继承**（换承接里程碑，留痕）或**关闭**（附证据），不能消失。
- stub 双侧标注：占位实现必须在代码（`// STUB(RD-###)`）与注册表双侧可见（H06 §4 第 4 条）。

## 5. 证据分级与基准纪律

- **三级证据**：`measured_local`（真实硬件、锁频、协议化采样）> `simulated/estimated`（占位）> 无证据。所有性能叙述必须标注级别。
- **采样协议**（r11，已工具化为 `rx bench`，[08](08_RUNTIME_AND_TOOLING.md) §4）：L0 环境验证前置 → warmup/稳态 → 50×3 trimmed mean → IQR → Mann-Whitney U 回归判定（1% Warning / 5% Critical）。
- **环境画像随证据存档**：驱动版本/锁频状态/WDDM-HAGS/TDR 配置进每份证据 JSON（schema 固定，沿用上一项目"受限环境降级但 schema 不变"约定）。
- **性能型里程碑铁律**：证据通道不存在 → 里程碑不得关闭（P-09）。每阶段至少一项真实硬件证据交付物，杜绝连续纯骨架阶段（H06 §6）。
- **计数器规则**：任何优化先布计数器再实现最后定阈值（P-07）；计数器合入后 2 个里程碑内必须有非零真实证据，否则降级或删除（H02 §5 教训的硬化）。

## 6. 测试纪律

| 机制 | 内容 | 来源 |
|---|---|---|
| 子进程隔离 | GPU 测试与编译器崩溃类测试全部子进程化，崩溃不连坐 harness | H03 §6（验证有效）+ H06 §1 |
| 已知失败基线 | 环境性失败维护为逐字节不变的基线文件，新增失败立即可见 | H03 §6 |
| UI golden | 诊断 `.stderr` snapshot + 受控 bless 流程（bless 是审批动作不是日常操作） | r1/r9 + [07](07_COMPILER_ARCHITECTURE.md) §5 |
| IR golden | MIR/LLVM IR/PTX 三层 snapshot 锁 codegen 形状（NVPTX 雷区回归集挂此机制） | r2 + [07](07_COMPILER_ARCHITECTURE.md) §11 |
| conformance | spec 条款 ↔ 测试 traceability（工具生成矩阵）；唯一语义验收边界 | r7 + [10](10_GOVERNANCE.md) §4 |
| 测试配额 | 每个新特性固定最小测试配额（语义 ≥N 条 conformance + 诊断 ≥1 条 UI + 必要时 IR golden），mirror 同类既有结构 | H03 §6 配额纪律 |
| Sanitizer | Compute Sanitizer（memcheck/racecheck）nightly 全跑；unsafe 代码变更强制本地跑 | r5 + [08](08_RUNTIME_AND_TOOLING.md) §5 |
| 差分测试 | grammar-based fuzz 里程碑级跑（非日常门禁） | r7 |

## 7. Spike Gating（防范围蔓延）

机制照搬上一项目（4 次 not_triggered 全部留痕的验证有效机制，H04 §1）：对每个诱惑方向立一份 gating 记录——候选方向、触发条件决策树、当前判定。条件不满足则正式记录 `not_triggered` 关闭；条款 0-byte 修改、只追加决策记录。

**Rurix 首批永久 gating 清单**（合并 [03](03_POSITIONING_AND_LANDSCAPE.md) §4 死亡路线 + [11](11_ROADMAP.md) §2 红线 + 各文档登记项）：

| 方向 | 触发条件（满足才允许 Full RFC 立项） |
|---|---|
| MLIR kernel island | [07](07_COMPILER_ARCHITECTURE.md) §7.1 三条件之一 |
| Tensor Core / WGMMA / TMA intrinsics | L2 基准证明 GEMM 类负载是真实用户瓶颈 + r3 所述中层抽象成熟度复评 |
| 多后端（AMD/Metal/Vulkan/SPIR-V） | G2 完成 + agent 解除红线 3（D-008） |
| autodiff / 可微渲染 | 永久 gating；生态包层面探索不动语言核心 |
| kernel fusion / 稀疏结构 | 同上 |
| 声明宏 | G1 后真实样板痛点清单 ≥ 3 类且 derive 不可覆盖 |
| registry | D-312 触发条件 |
| Python 嵌入 | 永久 not_triggered（红线 1） |
| 自举 | 5 年期评估（[11](11_ROADMAP.md) §6） |

## 8. CI 三层门禁

结构照搬（H03 §5），第一周即真跑（上一项目 YAML-only 验证的 D11.8-2 教训）：

- **PR Smoke**：核心单测 + conformance 子集 + UI golden + guardrail 脚本 + 小型 bench 冒烟，失败即红；
- **Nightly**：全量测试 + 全量 bench + Sanitizer + 预算门禁合并加载；
- **Release**：bench 严格模式（无容错跳过）+ hard block + 签名/SBOM/许可审计 + artifact 上传。

GPU runner 就是开发机（RTX 4070 Ti 自托管 runner）——单人项目没有"等 CI 机器"的借口，这是 P-09 在基础设施上的兑现。

## 9. 文档纪律

- 单一结构化事实源 + 生成视图（P-11）：deferred/决策/错误码/预算走 JSON 注册表，spec 是语义事实源，人类可读文档由工具生成或只做引用。
- 修订只追加；数字必须来自命令输出并附命令；文档引用必须带路径/编号。
- 状态快照文件（继任 CLAUDE.md 角色的项目状态文件）设尺寸预算（< 32KB），超限即拆分归档——不重蹈 72KB 不可读覆辙（H04 §2.5）。

## 10. AI 协作操作规程（与 [10](10_GOVERNANCE.md) §7 配套的执行细则）

1. 每个 AI 会话强制加载 `agents/AGENTS.md`（含本文与 10 号文档的摘要 + 验证命令清单）。
2. AI 声明"完成"必须附验证命令真实输出；声明"性能提升"必须附 `rx bench` 证据 JSON 路径。
3. AI 修改语义相关代码必须在 PR 描述引用 spec 条款号或 deferred/RFC 编号。
4. 契约/预算/guardrail/注册表文件对 AI 是**只追加**目标——任何对既有条目的修改自动触发审查（agent 自主审查并留痕，无无卡点）。
5. 周期性 drift 审计：对照契约验收门与实际合入内容做差异扫描（上一项目"漂移可 diff 出来"原则的周期化）。

## 11. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
| v1.1 | 2026-06-29 | §10.4 解除"自动触发审查"为 agent 自主审查留痕（同步 10 §7 v2.0、AGENTS v3.0 agent 完全自主化） |
