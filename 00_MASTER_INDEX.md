# Rurix 语言规划文档集 — 总索引

> 版本：v1.0（2026-06-11）
> 状态：实现期——本文档集已按 §6.4 转历史规划档案（只受勘误）。语言 1.0 已发行（tag v1.0.0，2026-07-14）；MVP(M0–M8)/G1/G2/V1/MS1(2026-07-15)/MB1(2026-07-16，基准 mb1-closed) 里程碑相继收官；当前 EA1 分发与门面期 active（2026-07-16 起，RFC-0012 工具链真实分发 Draft 已合入）；仓库 2026-06-17 已开源（MIT OR Apache-2.0）。
> 本文档集是 Rurix 项目设计期的**唯一事实源**。任何与本文档集冲突的口头约定、聊天记录、AI 输出均以本文档集为准。

---

## 1. Rurix 是什么（一段话）

**Rurix** 是一门独立的、静态编译的、Windows-first / CUDA-first / Rust-governed 系统编程语言，面向图形编程、GPU 计算与高性能视觉计算。它采用双层模型——安全的宿主层语言 + 受控的 GPU kernel 子语言——把资源所有权、地址空间、并行执行层级做成类型系统一等公民，目标是成为现代图形编程与 GPU 系统编程未来十年的严肃基础设施。

项目动机源自上一项目（Taichi Engine 优化计划，P0–P15）的终止结论：在不为此设计的宿主（Python DSL）之上叠加图形引擎与编译期优化，工程纪律可以做到极致，但宿主语言的结构性天花板无法靠下游纪律突破。论证全文见 [01_VISION_AND_MISSION.md](01_VISION_AND_MISSION.md)。

## 2. 文档清单

| 编号 | 文件 | 内容 | 面向读者 |
|---|---|---|---|
| 00 | `00_MASTER_INDEX.md` | 本索引、阅读路径、术语表、文档维护规则 | 所有人 |
| 01 | [01_VISION_AND_MISSION.md](01_VISION_AND_MISSION.md) | 愿景与使命：为什么 Rurix 应该存在 | 决策者、全员 |
| 02 | [02_USERS_AND_USE_CASES.md](02_USERS_AND_USE_CASES.md) | 目标用户与用例：六类用户画像、旗舰用例、采纳判据 | 产品/设计 |
| 03 | [03_POSITIONING_AND_LANDSCAPE.md](03_POSITIONING_AND_LANDSCAPE.md) | 语言定位与竞品全景：关系矩阵、空白市场、死亡路线红线 | 决策者、设计 |
| 04 | [04_DESIGN_PRINCIPLES.md](04_DESIGN_PRINCIPLES.md) | 核心设计原则：14 条带编号可引用的设计公理 | 全员（必读） |
| 05 | [05_LANGUAGE_ARCHITECTURE.md](05_LANGUAGE_ARCHITECTURE.md) | 语言架构：双层模型、类型系统、所有权、地址空间、泛型、模块、FFI | 语言/编译器设计 |
| 06 | [06_GPU_GRAPHICS_PROGRAMMING_MODEL.md](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) | GPU 与图形编程模型：kernel 抽象、内存模型映射、同步、三阶段图形路线 | 语言/运行时/图形设计 |
| 07 | [07_COMPILER_ARCHITECTURE.md](07_COMPILER_ARCHITECTURE.md) | 编译器架构：IR 分层、查询化、借用检查、NVPTX codegen、诊断 | 编译器工程 |
| 08 | [08_RUNTIME_AND_TOOLING.md](08_RUNTIME_AND_TOOLING.md) | 运行时与工具链：Driver API 对象模型、Windows 工具链、LSP、开发工具 | 运行时/工具链工程 |
| 09 | [09_STDLIB_AND_ECOSYSTEM.md](09_STDLIB_AND_ECOSYSTEM.md) | 标准库与生态：core/std 分层、数学库、Buffer、互操作、包管理 | 标准库/生态工程 |
| 10 | [10_GOVERNANCE.md](10_GOVERNANCE.md) | 治理与项目组织：变更门、RFC、稳定性、AI 贡献政策 | 全员（必读） |
| 11 | [11_ROADMAP.md](11_ROADMAP.md) | 路线图：MVP 范围、里程碑序列、3 年/5 年愿景 | 决策者、全员 |
| 12 | [12_RISKS.md](12_RISKS.md) | 风险登记表：六类风险、概率/影响/消解手段 | 决策者、全员 |
| 13 | [13_DECISION_LOG.md](13_DECISION_LOG.md) | 决策日志：全部重大决策编号登记、备选比较、审批状态 | 全员（必读） |
| 14 | [14_ENGINEERING_DISCIPLINE.md](14_ENGINEERING_DISCIPLINE.md) | 工程纪律：里程碑契约、guardrails、预算门禁、证据分级、deferred 模型 | 全员（必读） |

## 3. 阅读路径

- **只有 15 分钟**：01（为什么）→ 04（原则）→ 13（决策一览表）。
- **评估这个项目是否靠谱**：01 → 03 → 12 → 11。
- **要参与语言设计**：04 → 05 → 06 → 13。
- **要参与编译器实现**：04 → 07 → 14 → 05。
- **要参与运行时/工具链**：04 → 08 → 06 → 14。
- **AI agent 上工前必读**：04 → 10（§AI 贡献政策）→ 14 → 13。

## 4. 已锁定的战略决策（项目 agent 拍板，2026-06-11）

| 编号 | 决策 | 内容 |
|---|---|---|
| D-001 | 语言名称 | **Rurix**（文件扩展名 `.rx`，编译器 `rurixc`，CLI `rx`，版本管理器 `rurixup`） |
| D-002 | 图形分期 | MVP 纯 CUDA 计算（含 compute 软光栅图形演示）→ Phase 2 CUDA–D3D12 interop 呈现 → Phase 3 原生 D3D12 + DXIL 图形管线 |
| D-003 | 开源策略 | MVP 前闭门开发；MVP 后以 Apache-2.0 + MIT 双许可开源 |
| D-004 | 团队基准 | 单人 + AI 集群；MVP 周期 12–18 个月；文档按此刻度规划 |

完整决策日志（含全部技术决策与备选比较）见 [13_DECISION_LOG.md](13_DECISION_LOG.md)。

## 5. 术语表

| 术语 | 定义 |
|---|---|
| **宿主层（host layer）** | Rurix 的 CPU 侧语言子集：完整的 Rust 式所有权/借用、std 可用、负责资源管理与 kernel 调度 |
| **kernel 子语言（device sublanguage）** | Rurix 的 GPU 侧受限子集：以 `kernel fn` / `device fn` 标注，受执行层级与地址空间类型约束 |
| **执行资源（execution resource）** | Descend 式的并行层级类型：`grid` / `block` / `warp` / `thread`，决定内存借用的收窄边界 |
| **view** | 把数组/缓冲区重塑为执行资源所拥有的不相交分片的类型级机制（`split` / `group` / `transpose` 等） |
| **地址空间（address space）** | 类型一等公民：`host`、`global`、`shared`、`constant`、`local`（寄存器，不暴露指针） |
| **affine 资源** | 只能被使用至多一次（move 语义、禁复制）的资源类型，如 `Context`、`Stream`、`DeviceBuffer` |
| **HIR / TBIR / MIR** | Rurix 编译器的三个内部 IR：高层 IR（树状、类型检查主战场）/ 临时 typed-body IR / CFG 化中层 IR（借用检查与优化主战场），见 07 |
| **PTX baseline** | MVP 的 GPU 目标基线：`compute_89`（Ada Lovelace），PTX 文本经 Driver API JIT 装载 |
| **strict-only** | 设计公理 P-01：任何 lowering/codegen 失败都是带结构化错误码的编译错误，语言层面不存在静默 fallback |
| **measured / estimated** | 性能证据分级：`measured_local`（真实硬件三次运行 trimmed mean）>`estimated`（占位）；占位阈值存活不得超过 2 个里程碑（见 14） |
| **里程碑契约** | 每个里程碑开工前固化范围/交付物/验收门/guardrails 的文档，close-out 只追加不改写（见 14） |
| **deferred 项** | 显式编号注册的延期债务（`RD-xxx`），登记于结构化注册表，只能被继承或关闭，不能消失（见 14） |
| **spike gating** | 对诱惑性扩张方向（如提前做多后端）的正式拒绝机制：列触发条件决策树，不满足则留痕关闭（见 14） |
| **G0 / G1 / G2** | 图形路线三阶段代号：G0 = compute 软光栅（MVP 内），G1 = CUDA–D3D12 interop 呈现，G2 = 原生 D3D12 + DXIL 管线 |
| **变更三档门** | 治理机制：Direct PR / Mini-RFC / Full RFC，按语义影响分级（见 10） |

## 6. 文档维护规则（吸收上一项目"文档超载"教训）

1. **本套 15 份文档是设计期唯一事实源**。禁止在仓库其他位置创建与本套内容镜像的 markdown；其他文档只允许以编号引用（如"见 04 P-03"）。
2. **决策只登记在 13 号文档**。其余文档引用决策编号（D-xxx），不复述决策理由全文。
3. **修订采用追加式修订日志**：每份文档底部维护"修订记录"表，重大修订须引用对应决策编号或 RFC 编号；不允许无痕改写既有结论。
4. **进入实现期后**，本文档集冻结为 v1.x 基线；语言规范以 `spec/` 仓库目录为新事实源（见 10 §4），本文档集转为历史规划档案，只接受勘误。
5. **尺寸纪律**：单份文档超过 120KB 视为腐败信号，必须拆分或裁剪（上一项目 CLAUDE.md 膨胀至 72KB 不可读的教训）。
6. **数字必须可追溯**：所有性能数字、版本号、日期必须标注来源（调研报告编号 r1–r12、官方文档或命令输出），禁止凭记忆引用。

## 7. 输入材料索引

本文档集的全部结论基于以下输入材料，引用时使用括注编号：

| 编号 | 材料 | 主题 |
|---|---|---|
| r1 | `deep-research/r1.md` | rustc 编译器工程深度解剖 |
| r2 | `deep-research/r2.md` | LLVM NVPTX 后端与 PTX 生成全链路 |
| r3 | `deep-research/r3.md` | MLIR 在 NVIDIA GPU 编译方向的现状 |
| r4 | `deep-research/r4.md` | Windows 上 CUDA Driver API 一等后端的运行时设计 |
| r5 | `deep-research/r5.md` | GPU 内存模型与所有权类型系统 |
| r6 | `deep-research/r6.md` | Windows 原生工具链、分发与许可合规 |
| r7 | `deep-research/r7.md` | 新编程语言的治理骨架与防 AI 幻觉工程 |
| r8 | `deep-research/r8.md` | 新语言包管理与供应链安全设计 |
| r9 | `deep-research/r9.md` | 编译器诊断与 IDE/LSP 体验工程 |
| r10 | `deep-research/r10.md` | GPU 编程语言竞品全景 |
| r11 | `deep-research/r11.md` | GPU 基准方法论与 Nsight 集成 |
| r12 | `deep-research/r12.md` | 标准库设计与 CUDA 生态库绑定 |
| H01–H07 | `docs/handover/01…07_*.md` | 上一项目经验交接文档集（时间线/架构/纪律/教训/资产/迁移建议/调研提示词） |

## 8. 执行文档登记（实现期前置，v1.1 追加）

依 [11](11_ROADMAP.md) §3 M0 与 [14](14_ENGINEERING_DISCIPLINE.md) 的机制要求，以下执行期文档类别在本仓库落位。它们是**契约/注册表/规程类新增载体，不是本套规划文档的镜像**（不违反 §6.1）：内容只引用编号文档，不复述规划结论。

| 位置 | 内容 | 机制依据 |
|---|---|---|
| `milestones/m0/` | M0 契约、执行计划、基准规程、CI 门禁规范、预算 JSON、证据 schema | 14 §1 §3 §5 §8 |
| `registry/deferred.json` | RD-### deferred 注册表（项目级唯一事实源） | 14 §4 |
| `registry/spike_gating.json` | SG-### spike gating 注册表（首批 9 项） | 14 §7 |
| `agents/AGENTS.md` | AI 会话强制上下文 v1 | 10 §7 / 14 §10 |

后续里程碑契约依同构落位于 `milestones/mX/`；注册表为项目级单例只追加。

## 9. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版：15 份文档集发布 |
| v1.1 | 2026-06-11 | 追加 §8 执行文档登记（M0 执行文档集落位：milestones/m0/、registry/、agents/）；原 §8 修订记录顺延为 §9 |
| v1.2 | 2026-07-17 | header 状态行由「设计与规划阶段（pre-implementation）」刷新至实现期（00 §6.3 追加式修订，独立 errata PR，check_planning_docs advisory 不阻断）：语言 1.0 已发行(tag v1.0.0，2026-07-14)、MVP(M0–M8)/G1/G2/V1/MS1(2026-07-15)/MB1(2026-07-16，基准 mb1-closed) 里程碑相继收官、EA1 分发与门面期 active(RFC-0012 Draft 已合入)、2026-06-17 已开源。仅刷状态行,§2 文档清单/§4 决策表/§8 既有正文与决策语义 0-byte。 |
