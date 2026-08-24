# rfcs/ — RFC 通道与已接受 RFC 存档

> 所属治理：[`../10_GOVERNANCE.md`](../10_GOVERNANCE.md) §2（组织结构 / FCP-lite）· §3（变更三档门）· §5（特性生命周期）· §6（稳定性与发布）· §9.5（编号永不复用）。
> 贡献者落地说明见 [`../CONTRIBUTING.md`](../CONTRIBUTING.md)；所有 AI 会话强制上下文见 [`../agents/AGENTS.md`](../agents/AGENTS.md)。
> 仓库 2026-06-17 已 public（D-003/D-007，双许可 MIT OR Apache-2.0）。本文是**首批外部 RFC 通道**的 intake 与规程文档（G1.4 / MR-0003 实体化 10 §8 承诺）。

本目录是**已接受 RFC 的存档**，也是**提案 intake 通道**。编号永不复用（10 §9.5）。

---

## 1. 什么时候需要 RFC

先按[三档门自助判定表](../CONTRIBUTING.md#变更分档三档门)定档。**只有 Mini-RFC / Full RFC 需要在本目录留档**；Direct 变更直接走 PR，不进 rfcs/。

| 档位 | 是否进 rfcs/ | 形态 |
|---|---|---|
| **Direct** | 否 | CI 绿 |
| **Mini-RFC** | 是 → `rfcs/mini-NNNN-*.md` | 单页提案 + **失败测试先行**（10 §3） |
| **Full RFC** | 是 → `rfcs/NNNN-*.md` | RFC 合入后才可实现；feature gate + tracking issue + spec diff + conformance 测试 + stabilization report（10 §3 / §5） |

**判档争议向上取严**（10 §3，自我约束建议）。AI agent 可自主判档（含 Direct）并记录依据。

## 2. 怎么提一条 RFC（intake）

1. **定档**：见上表与 [`CONTRIBUTING.md`](../CONTRIBUTING.md#变更分档三档门)。
2. **开 issue**：用 [`.github/ISSUE_TEMPLATE/`](../.github/ISSUE_TEMPLATE/) 的 `RFC` / `Mini-RFC` 模板登记动机与拟议范围（可选但推荐，便于公开讨论与 FCP-lite 计时）。
3. **写提案**：复制模板
   - Full RFC：[`TEMPLATE-RFC.md`](TEMPLATE-RFC.md) → `rfcs/NNNN-<kebab-title>.md`
   - Mini-RFC：[`TEMPLATE-MINI-RFC.md`](TEMPLATE-MINI-RFC.md) → `rfcs/mini-NNNN-<kebab-title>.md`
   - 取**下一个未用编号**（见 §5 编号台账）；Full-RFC 的 `RFC-####` 与 Mini-RFC 的 `MR-####` 是**两个独立命名空间**，各自递增、均永不复用。
4. **失败测试先行（Mini/Full 均强制）**：提案须指向一个当前 `main` 上 RED 的失败测试（编码拟议意图），实现 PR 落地后转绿（10 §3）。
5. **开 PR**：PR 描述按 [`.github/PULL_REQUEST_TEMPLATE.md`](../.github/PULL_REQUEST_TEMPLATE.md) 勾选档位、附 provenance / 条款号 / 验证输出；`ci/check_contribution.py` 自动阻断缺项（10 §7 开源后 CI 阻断的兑现）。
6. **评审 + FCP-lite**：见 §3。**RFC 合入后**方可推进实现 PR（Full RFC 硬性，10 §3）；条款 PR 先于实现 PR（硬规则 7）。

## 3. FCP-lite 评审窗（开源后，D-401 / D-405 / 10 §2.2 §5）

开源后治理从「角色帽（闭门期）」实体化为三角色真实的人或继续由 AI 行使，并引入 **FCP-lite（Final Comment Period, lite）**，作为 advisory 公开讨论与等待机制，**不强制人工同意数**——AI agent 可自主推进：

- **触发面**：语义 / `unsafe` / FFI / edition / 破坏性变更（即 Full RFC 触发面，10 §3）走 FCP-lite 公开讨论。Mini-RFC 走单页提案（轻量，不强制公开等待窗）。
- **通过条件**：advisory——公开等待窗内收集意见，agent 自主裁决是否合入（不强制 ≥2/3 人工同意）。
- **对抗性评审（D-409，Proposed）**：闭门单人+AI 下 FCP-lite 无真实外部评审者，故 Full RFC 强制附**至少一轮跨工具/跨模型对抗性评审**——由与起草者 Provenance 不同的 AI 工具/模型（评审 provenance ≠ 起草 provenance）产出 findings 并逐条 disposition，记录于 RFC「对抗性评审记录」段（[`TEMPLATE-RFC.md`](TEMPLATE-RFC.md) §9.1；Mini-RFC 轻量，§7.1）；`ci/check_contribution.py` 对缺段/未区分 provenance 打印 advisory finding。反自提自批单环（10 §3 / §7 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409）。
- **公开等待窗**：进入 FCP 后 **5–7 天公开等待期**（r7），无未决阻断意见即合入；其间任何人可提出阻断点。
- **发布节奏**：开源后 **6 周 train 模式**（Rust 节奏的放宽版，nightly 每日）；稳定化经 stabilization report + FCP-lite 进入 stable（10 §5 §6）。
- **晋升路径**：贡献者 → 评审者 → 维护者，路径文档化（10 §2.2 / §8）；行为准则采用 Contributor Covenant（[`../CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md)）。

### 3.1 三人组成员与外部通道开放程度（agent 自主裁决）

> 三人组（Language Lead / Implementation Lead / Quality & Release Lead）的**具体成员**与**首批外部 RFC 通道的开放程度**（公开讨论范围、外部提案准入、FCP 投票人名单）由 AI agent 自主裁决并记录（10 §7 v2.0 / D-406）。agent 可命名、签署并开放通道，无无卡点。
>
> Quality & Release Lead 角色继续由**不可绕过的机器门禁**（CI 三层门 + [`../14_ENGINEERING_DISCIPLINE.md`](../14_ENGINEERING_DISCIPLINE.md) 契约/预算体系）行使（10 §2.1「流程即人格」）；Language / Implementation Lead 由 AI agent 自主行使，Full RFC 批准签字由 agent 自主裁决。

## 4. 上游政策

对 LLVM 的修补**优先 upstream**；pin 的 fork 补丁必须带 upstream issue 链接（防 fork 漂移，10 §8）。

## 5. 编号台账（永不复用，10 §9.5）

| 命名空间 | 已用 | 下一个未用 |
|---|---|---|
| Full RFC `RFC-####` | RFC-0001（[`0001-cuda-d3d12-interop.md`](0001-cuda-d3d12-interop.md)，G1.1）· RFC-0002（[`0002-shader-stages.md`](0002-shader-stages.md)，G2.1，**Approved 2026-06-23**）· RFC-0003（[`0003-dxil-backend.md`](0003-dxil-backend.md)，G2.2 MIR→DXIL 第二后端，**Approved 2026-06-23**）· RFC-0004（[`0004-spirv-dxil-graphics-backend.md`](0004-spirv-dxil-graphics-backend.md)，G2.2 图形=B，**Approved 2026-06-25**）· RFC-0005（[`0005-binding-layout-inference.md`](0005-binding-layout-inference.md)，G2.3 绑定布局推导，**Approved 2026-06-28**）· RFC-0006（[`0006-uc04-deferred-renderer.md`](0006-uc04-deferred-renderer.md)，G2.4 UC-04 deferred 渲染器 / 原生 D3D12 运行时出图路径，**Approved 2026-06-28**）· RFC-0007（[`0007-texture-sampling-memory-model.md`](0007-texture-sampling-memory-model.md)，G2.4 纹理采样内存模型，**Approved 2026-06-30**）· RFC-0008（[`0008-edition-stabilization.md`](0008-edition-stabilization.md)，G2.5 edition 机制与 stabilization，**Approved 2026-06-30**）· RFC-0009（[`0009-host-gpu-orchestration.md`](0009-host-gpu-orchestration.md)，MS1.2 single-source 宿主 GPU 编排 std::gpu + present typestate 面 + 宿主图像落盘桥，**Approved 2026-07-14**）· RFC-0010（[`0010-uc07-sim-renderer.md`](0010-uc07-sim-renderer.md)，MS1.3 UC-07 ruridrop 主语言渲染器/仿真二合一应用 + 主语言判据操作化，**Approved 2026-07-14**）· RFC-0011（[`0011-vulkan-spirv-backend.md`](0011-vulkan-spirv-backend.md)，mb1 Vulkan/SPIR-V 跨端第三后端 AMD 桌面 + Android compute+graphics,**Owner Approved 2026-07-15**——owner 明确指示解除红线 3 并继续）· RFC-0012（[`0012-toolchain-real-distribution.md`](0012-toolchain-real-distribution.md)，EA1 rurixup 工具链真实分发:FS 物化 + 活跃切换 + GitHub Releases 四级校验拉取 + 发布资产自动化,RD-025 兑现,**Draft 2026-07-16**——§9 Q-A~Q-D 经 milestones/ea1/OWNER_DECISION_PACKAGE.md 呈 owner,裁决 A/B 落地前不翻 Approved）· RFC-0013（[`0013-industrial-rendering.md`](0013-industrial-rendering.md)，G3 工业渲染期五特性面伞形 present/采样超集/bindless/render graph/mesh-task-RT，单 RFC 五面五章（G3_CONTRACT §7 v1.1，MB1 单期伞形先例），**Draft 2026-07-18**——Agent Approved 待 §9.1 对抗性评审〔评审 provenance ≠ 起草〕后翻，合入 gated on G-G3-1 开闸）· RFC-0014（[`0014-engine-integration.md`](0014-engine-integration.md)，EI1 引擎集成期双面单 RFC：Part A `#[export(c)]` C ABI 导出 codegen + `--emit=dll` cdylib 通道 + 编译器内建头文件生成（RD-009 兑现，D-113/P-11）/ Part B UC-05 最小 RHI + render graph 核心 + I1~I10 不变量「类型系统拦截 vs 计数器事后观测」对照报告，RXS-0250~0269 earmark，镜像 RFC-0010 单 RFC 双角色，**Agent Approved 2026-07-19**——§9.1 对抗性评审〔评审 provenance `claude-code:claude-opus-4-8` ≠ 起草 `claude-code:claude-fable-5`，三镜头 correctness/redline/implementability，15 findings 逐条 disposition，D-409〕完成，先于任何实现 PR，G-EI1-1）· RFC-0015（[`0015-engine-rendering.md`](0015-engine-rendering.md)，G4 引擎渲染期伞形四章：章 A 图形 RHI 化（raster/mesh pass + 采样/bindless/present 库化 + 自动 barrier + engine_host v3 三方对照）/ 章 B RD-035 执行面三项 / 章 C artifacts v2 + .rx 单源 Vulkan RHI（RD-031）/ 章 D C ABI v2 条件臂，RXS-0270~0299 claim，单伞形（G3 v1.1 先例），**Agent Approved 2026-07-23**——§9.1 对抗性评审〔评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 `Kimi Code CLI (Kimi)`，三镜头，18 findings 逐条 disposition，D-409；claude-code 403 环境留痕〕完成，先于任何实现 PR，G-G4-2）· RFC-0016（[`0016-native-renderer.md`](0016-native-renderer.md)，G5 原生渲染器期伞形八章：章 A 渲染调度 render graph 引擎库 / 章 B RHI 图形派发桥（Rust 级执行器主通道 + .rx submit 条件臂）/ 章 C 虚拟化几何（meshlet+DAG+VisBuffer 双路）/ 章 D VSM / 章 E 屏幕探针 GI / 章 F 光追与 AS 管理 / 章 G 材质场景流送 / 章 H 时域重建，渲染器调研七报告 P0–P2 主线承载，预期零新语言语义条款，**Agent Approved 2026-07-29**——§9.1 对抗性评审〔评审 provenance `cursor:claude-fable-5` ≠ 起草 `cursor:kimi-k3-max`，三镜头，7 findings 逐条 disposition〔2 blocker + 4 major 正文实改 + 1 minor 留痕〕，D-409〕完成，先于任何实现合入，G-G5-2）· RFC-0017（[`0017-engine-physics.md`](0017-engine-physics.md)，G6 渲染物理双轨期伞形五章：章 A rurix-physics 物理库边界（PhysicsWorld 固定步 / BodyId·ShapeId 不透明句柄 / ContactEvent 归一化有界队列 / QueryRay step 外并发 / SyncBudget）/ 章 B 渲染同步契约（五条纪律 + G5 冻结面 0-byte）/ 章 C FFI 与 unsafe 纪律（R-G6-1 裁决：自维护 JoltC FFI，rolt 停滞否决，U33 起）/ 章 D Rapier 快路径（feature 默认 off，容差对拍非逐位）/ 章 E Taichi Vulkan AOT 特效副轨（可选，失败登记 RD-042+），预期零新语言语义条款（06 §8.3），**Agent Approved 2026-07-31**——§9.1 对抗性评审〔评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 `Kimi Code CLI (Kimi)`，三镜头，17 findings 逐条 disposition〔2 blocker + 11 major + 4 minor 全部采纳并修〕，D-409；claude 403 环境留痕〕完成，先于任何实现 PR，G-G6-2） | RFC-0018（自由池，实际以 [`../registry/number_ledger.json`](../registry/number_ledger.json) 为准） |
| Mini-RFC `MR-####` | MR-0001（[`mini-0001-async-buffer.md`](mini-0001-async-buffer.md)，G1.2）· MR-0002（[`mini-0002-engine-integration.md`](mini-0002-engine-integration.md)，G1.3）· MR-0003（[`mini-0003-oss-community.md`](mini-0003-oss-community.md)，G1.4）· MR-0004（[`mini-0004-geometry.md`](mini-0004-geometry.md)，G1.4 生态二梯队）· MR-0005（[`mini-0005-fatbin-distribution.md`](mini-0005-fatbin-distribution.md)，G1.5 生产分发 fatbin;**台账滞后修正**:文件 2026-06-22 已落,本行随 MR-0008 PR 补登）· **MR-0006 / MR-0007 已被 GRX 影子分支(`codex/grx-godot-dxil-workspace`,closed,未合 main)claim**,main 侧跳号避撞(编号永不复用 10 §9.5,对齐 MR-0005 避撞 MR-0003/0004 教训;结构化登记见 [`../registry/number_ledger.json`](../registry/number_ledger.json) `namespaces.MR` + `off_tree_workflows[grx]`,守卫 [`../ci/check_number_ledger.py`](../ci/check_number_ledger.py))· MR-0008（[`mini-0008-stable-channel-manifest.md`](mini-0008-stable-channel-manifest.md)，V1.2 语言 1.0 stable channel 最小清单，**Approved 2026-07-14**）· MR-0009（[`mini-0009-toolchain-frontend.md`](mini-0009-toolchain-frontend.md)，post-V1 rurixup 工具链前端首切片:install/list/default 消费 stable channel，**Approved 2026-07-14**）· MR-0010（[`mini-0010-shadow-workflow-ledger.md`](mini-0010-shadow-workflow-ledger.md)，影子/off-tree 编号工作流登记机制:`registry/number_ledger.json` + `ci/check_number_ledger.py` 跨分支保留号守卫，**Approved 2026-07-17**）· MR-0011（[`mini-0011-ptxas-opt-pin.md`](mini-0011-ptxas-opt-pin.md)，G3.1 RD-027 处置护栏:`RURIXC_PTXAS_OPT` 环境开关注入 ptxas `-O<n>` 预编档(仅 `=0` 具 RD-027 护栏效力,缺省 0-byte)，**Approved 2026-07-18**） | MR-0012 |

> **RFC 台账校准（2026-08-02，number_ledger v1.38）**：上表 Full RFC 行尾的“RFC-0018（自由池）”是 G7.1 materialize 后遗留的历史快照；[`RFC-0018`](0018-compute-rayquery-device-frame.md) 已为 **Agent Approved**，当时下一个未用 Full RFC 为 **RFC-0019**。本追加式校准优先于该行旧尾注。
>
> **RFC 台账校准（2026-08-02，number_ledger v1.40）**：G8.1 治理波 materialize 三份 Full RFC，均于 2026-08-02 经 D-409 独立 provenance 对抗性评审（评审 `Assisted-by: Kiro:claude-opus-5 rfc-review-session` ≠ 起草 `Codex:gpt-5 rfc1x-drafter-session`）后 **Agent Approved**，findings 逐条 disposition 见各文 §9.1：
> - **RFC-0019**（[`0019-rendering-platform.md`](0019-rendering-platform.md)，G8 渲染平台语义：RT pipeline/SBT 增量、RD-037 单源 gfx submit、permutation/reflection/capability、TSR 与 WPO 时域语义、多层 closure 语义面、多队列 ownership/timeline、task 评估窗；17 findings〔1 blocker + 10 major 正文实改〕）
> - **RFC-0020**（[`0020-asset-pipeline.md`](0020-asset-pipeline.md)，G8 资产管线、确定性派生数据与 M01/M04 版本化页 ABI；17 findings〔3 blocker + 8 major 正文实改〕）
> - **RFC-0021**（[`0021-physics-platform.md`](0021-physics-platform.md)，G8 replay-first 物理平台：capture/replay、网络物理、破坏、角色/载具/布料资产链；20 findings〔4 blocker + 12 major 正文实改〕）
>
> 三份 RFC 的 **Agent Approved 只表示语义/治理评审完成，不解锁任何实现**：G8.2+ 仍由 `G8_CONTRACT` G-G8-3 与 `ci/check_g8_implementation_interlock.py` 硬门约束（当前诚实输出 `BLOCKED`）。当前下一个未用 Full RFC 为 **RFC-0022**。
>
> **RFC 台账校准（2026-08-09，number_ledger v1.73）**：G9.1 治理波 materialize 三份 Full RFC，均于 2026-08-09 经 D-409 对抗性评审（评审 `Assisted-by: Kimi Code CLI (Kimi) rfc00XX-adversarial-reviewer`（独立实例）≠ 起草 `Assisted-by: Kimi Code CLI (Kimi) rfc00XX-drafter`；首选跨工具评审者本环境不可得，同工具族偏差按 RFC-0015 §9.1 / number_ledger v1.29 先例如实登记于各 RFC §9.1）后 **Agent Approved**，findings 逐条 disposition 见各文 §9.1：
> - **RFC-0022**（[`0022-virtual-geometry-gi-semantics.md`](0022-virtual-geometry-gi-semantics.md)，G9 虚拟几何与 GI 语义：cluster DAG/CLAS 双腿/VisibleClusterSet 单源真相/页格式 v2/Surface Cache/四级追踪降级/probe 编码/M17 golden 门序；6 findings〔1 major + 5 minor〕全部 disposition，F-6 跨文档移交 G9_CANDIDATE_DECISIONS v1.1 落实）
> - **RFC-0023**（[`0023-gpu-driven-submission-shading.md`](0023-gpu-driven-submission-shading.md)，G9 GPU-driven 提交与着色系统：DGC 抽象/Execution Set/descriptor 全局表/command build node/IR 链接/变体预算/SER 原语/mesh shader 可选路径；含 G5 Barrier EB 冻结面显式修订行（AccessKind 新边 `StorageWrite→IndirectCommandRead`，RXS-0239 字面 0-byte）；4 findings〔2 major + 2 minor〕全部 disposition）
> - **RFC-0024**（[`0024-physics-platform-revision.md`](0024-physics-platform-revision.md)，G9 物理平台修订（RFC-0021 修订）：Field 系统/统一 particle view/双通道 tick/浮力走 Field 通道/Jolt 5.6 升级路径/神经变形研究轨边界；7 findings〔3 major + 4 minor〕全部 disposition）
>
> 三份 RFC 的 **Agent Approved 只表示语义/治理评审完成，不解锁任何实现**：G9.2+ 仍由 `G9_CONTRACT` G-G9-3 与 `ci/check_g9_implementation_interlock.py` 硬门约束（当前诚实输出 `BLOCKED`）。当前下一个未用 Full RFC 为 **RFC-0025**。
>
> **RFC 台账校准（2026-08-12，number_ledger v1.90）**：G9.5 D4（大世界×专项渲染器×显示管线）无伞形 RFC 缺口处置——Grep 实测 RFC-0016/0019/0022/0023 冻结面与 D4 链路面（M110~M120）无重叠，且 M115/M114 触 G5 冻结面 `MaterialClosure` 32B 扩展按 G9_CONTRACT guardrail 须 RFC 显式修订行（M104 先例 = RFC-0023 §4.4.3），MR（Mini-RFC）体例不承载新语义面 + 冻结面修订，判档向上取严为 Full RFC。**RFC-0025**（[`0025-world-and-specialty-renderers.md`](0025-world-and-specialty-renderers.md)，G9 D4 伞形：世界分区/流送预算契约/HLOD/大气 Froxel/水体双管线/地形 chunk≈cell/贴花 DBuffer + 显示管线 view transform/后处理骨架/OIT benchmark + 毛发 Marschner/皮肤 Burley，含 §4.L 🔒 MaterialClosure 32B 显式修订行〔资产化侧表扩展通道，32B 布局 0-byte〕）经 D-409 第 1 轮对抗性评审（评审 `Assisted-by: Kimi:Kimi-K3 rfc0025-adversarial-reviewer`；**单实例偏差如实登记**——单模型子代理会话无法派生跨工具/跨模型独立评审实例，偏差大于 RFC-0024「同工具族独立实例」先例，效力自限声明见 RFC-0025 §9.1）后 **Agent Approved**，4 findings（1 major + 3 minor）全部 disposition。Agent Approved 只表示语义评审完成，不解锁任何实现。当前下一个未用 Full RFC 为 **RFC-0026**。
>
> **RFC 台账校准（2026-08-15，G10.1 治理波）**：**RFC-0026**（[`0026-visual-comparison-metrics.md`](0026-visual-comparison-metrics.md)，G10 画面对标与图像度量语义：帧捕获 HDR 格式面（EXR float32 基准、色彩空间/位深/元数据闭集、往返无损）/ FLIP·SSIM·PSNR 口径冻结（NVlabs/flip 参考实现 pin 策略、Wang 2004 参数闭集、恒等图对极值断言）/ 逐像素 diff 报告 schema / 差距清单 schema（UE5 Renderer 模块归属枚举闭集，2026-08-15 真实源码树实测）/ 双端确定性契约（二进制 canonical + SHA-256，digest 不等不得出 A/B 报告门序硬约束））为 **Draft**——D-409 独立 provenance 对抗性评审后由主会话翻 Agent Approved（本追加登记不翻转状态）。编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free=26` 领取；`reserved_in_flight[G10]` 登记只含 0026（本行起草 agent 已落 ledger 登记），RFC-0027 由并行登记条目自行 claim 防同波撞号。Draft 不构成实现许可：G10.2+ 仍由 [`../milestones/g10/G10_CONTRACT.md`](../milestones/g10/G10_CONTRACT.md) G-G10-3 实现互锁硬门约束。此后自由号以 ledger 实测 `next_free` 为准。
>
> **RFC 台账校准（2026-08-15，G10.1 治理波）**：**RFC-0027**（[`0027-external-reference-harness-license.md`](0027-external-reference-harness-license.md)，G10 外部参照 harness 与压测资产许可边界：UE 出图编排边界零 vendoring / 压测资产许可白名单 SPDX/attribution/digest 登记面 / 资产外部缓存 K: 盘 + 场景清单冻结），**Draft 2026-08-15**——D-409 独立 provenance 对抗性评审后由主会话翻 Agent Approved；ledger 由主会话统一核对（本行起草 agent 不改 `registry/number_ledger.json`）。
>
> **RFC 台账校准（2026-08-15，G10.1 治理波收口，number_ledger v1.102）**：**RFC-0026 与 RFC-0027 均翻 Agent Approved（2026-08-15）**——D-409 对抗性评审完成：0026 独立隔离评审会话 17 findings（3 blocker〔世界系约定缺失/门序可旁路/LDR 臂 UE 路径无效〕+ 7 major + 7 minor）、0027 独立隔离评审会话 18 findings（3 high〔provenance 冲突/程序生成资产五元组不自洽/BMW 许可双重标准〕+ 9 med + 6 low），全部 disposition 落实（两 RFC v0.2 修法批，§9.1 评审记录段均已回填）；同环境单一模型 provenance 偏差不静默处理，按 RFC-0015 §9.1/v1.29/v1.73/v1.90 先例如实登记于两 RFC §9.1 并留 G10.8b 终审复核锚。主会话核对契约/MAP/CI_GATES 三面一致（`ci/check_g10_acceptance_map.py` PASS）后翻 Agent Approved。Agent Approved 只表示语义/治理评审完成，不解锁任何实现：G10.2+ 仍由 [`../milestones/g10/G10_CONTRACT.md`](../milestones/g10/G10_CONTRACT.md) G-G10-3 与 `ci/check_g10_implementation_interlock.py` 硬门约束。`reserved_in_flight[G10]` 两号 claim 兑现且 claim 行 0-byte；当前下一个未用 Full RFC 为 **RFC-0028**。
>
> **RFC 台账校准（2026-08-16，G11.1 治理波收口，number_ledger v1.113）**：**RFC-0028**（[`0028-g11-gi-quality-closure.md`](0028-g11-gi-quality-closure.md)，G11 GI 与光照画质闭环伞形：R4 多反弹 GI 修复语义〔屏幕探针近场 + 世界缓存远场兜底双级〕/ M99-clipmap 世界辐射缓存世界级承接语义〔空间哈希世界缓存 + 距离自适应辐射 LOD〔clipmap 级语义〕+ 屏幕缓存失效回落 + 远场能量回归双锚判定〕/ R3 灯种子集表达〔光源集五元闭集 + cornell 契约灯面 0-byte〕/ spec/global_illumination.md RXS-0360 世界级 not-triggered 登记翻转显式修订行〔既有字面 0-byte〕/ C1 口径对齐 GI·天光语义面〔不拟合、只对齐 + 残余口径差登记〕/ 修复闭环判据语义〔锁定基线锚 + delta 收敛 measured + 不设绝对通过线〕）经 D-409 第 1 轮对抗性评审（独立评审会话零共享上下文；**同环境单一模型 provenance 偏差不静默处理**——评审者与起草者同模型、独立性 = 评审轮次隔离，按 RFC-0015 §9.1/v1.29/v1.73/v1.90/v1.102 先例如实登记于 §9.1 并留 G11.7b 终审复核锚）后 **Agent Approved**，12 findings（3 high〔host 消费面形态悬空/收敛判定方向性缺陷/世界级判定锚不可机核〕+ 5 med + 4 low）全部 disposition（v0.2 修法批，§9.1 评审记录段已回填）。编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free=28` 领取；`reserved_in_flight[G11]` claim 兑现且 claim 行 0-byte。Agent Approved 只表示语义/治理评审完成，不解锁任何实现：G11.2+ 仍由 [`../milestones/g11/G11_CONTRACT.md`](../milestones/g11/G11_CONTRACT.md) G-G11-3 与 `ci/check_g11_implementation_interlock.py` 硬门约束。此后自由号以 ledger 实测 `next_free` 为准（当前 RFC-0029）。
>
> **RFC 台账校准（2026-08-24，G16plus，number_ledger v1.161）**：**RFC-0031**（[`0031-g16plus-gi-expression-quality-closure.md`](0031-g16plus-gi-expression-quality-closure.md)，G16plus 强制收口画质：生产加性 `--gi on` / cornell 面光反弹 / bistro 填光 / M-g 18/18 程序产阈）经 D-409 对抗性评审后 **Agent Approved**（评审 `milestones/g16/design/rfc0031_adversarial_review.md`）。编号按立项时实测 `namespaces.RFC next_free=31` 领取。此后自由号以 ledger 实测为准（当前 RFC-0032）。
>
> **RFC 台账补登（2026-08-24，登记滞后修复——两号早已 materialize 且在 ledger 登记，仅本索引漏登；按只追加纪律补行不改上方既有行）**：**RFC-0029**（[`0029-g12-path-tracer-productionization.md`](0029-g12-path-tracer-productionization.md)，G12 路径追踪生产化语义伞形单章，2026-08-17 经 D-409 对抗性评审〔10 findings 全 disposition，评审 `milestones/g12/design/rfc0029_adversarial_review.md`〕后 **Agent Approved**；G12 已于 2026-08-17 收口）；**RFC-0030**（[`0030-g14plus-pipeline-structural-optimization.md`](0030-g14plus-pipeline-structural-optimization.md)，G14plus 渲染管线结构性优化语义〔G14.8~G14.12 延续波伞形〕，2026-08-22 经 D-409 对抗性评审〔评审 `milestones/g14/design/rfc0030_adversarial_review.md`〕后 **Agent Approved**；G14 已于 2026-08-23 收口）。两号编号均按当时实测 ledger `next_free` 领取（v1.14x/v1.15x 系列校准注可查）。
>
> **RFC 台账校准（2026-08-24，G17.1 治理波，number_ledger v1.162）**：**RFC-0032**（[`0032-d3d12-host-ngx-lane.md`](0032-d3d12-host-ngx-lane.md)，D3D12 宿主 NGX 车道评估与实现语义：NGXCubinD3D12 宿主形态 / 跨 device 资源桥接与同步 / 单 device 化评估 / 决策树终态程序）经 D-409 对抗性评审（评审 `milestones/g17/design/rfc0032_adversarial_review.md`，findings 逐条 disposition）后 **Agent Approved（决策程序 + 实现语义）**——**终态 disposition（approved-implement / no-go / defer 三态均为合法终态）由 G17.4 M-c 按 RFC §5 决策树以 M-a/M-b 实测为输入程序产出**，终态字面以 RFC 正文修订行与 `evidence/g17_m_c_d3d12_host_lane_disposition_*.json` 为准。编号按立项时实测 `namespaces.RFC next_free=32` 领取（ledger v1.162：on_tree_max 31→32、next_free 32→33）。此后自由号以 ledger 实测为准（当前 RFC-0033）。
>
> **RFC 台账补登 + 校准（2026-08-24，G19.1 治理波——0033~0035 为 G18.1 领取但本索引漏登，按只追加纪律补行不改上方既有行）**：**RFC-0033**（[`0033-g18-light-quality-presentation-dual-profile.md`](0033-g18-light-quality-presentation-dual-profile.md)，G18 光线画质 presentation 双 profile，**Agent Approved**，评审 `milestones/g18/design/rfc0033_adversarial_review.md`）；**RFC-0034**（[`0034-virtualized-geometry-p3-mesh-shader.md`](0034-virtualized-geometry-p3-mesh-shader.md)，mesh shader VisBuffer 第三光栅路径，终态 **no-go**，评审 `milestones/g18/design/rfc0034_adversarial_review.md`）；**RFC-0035**（[`0035-frame-generation-independent-layer.md`](0035-frame-generation-independent-layer.md)，帧生成独立层评估，终态 **defer-to-G19+**，评审 `milestones/g18/design/rfc0035_adversarial_review.md`）——三号按 G18.1 实测 `next_free=33` 起领取。**RFC-0036**（[`0036-frame-generation-realization.md`](0036-frame-generation-realization.md)，G19 帧生成独立层兑现：host 参考臂 + MFG 多档 + vendor 三臂 disposition）经 D-409 对抗性评审（评审 `milestones/g19/design/rfc0036_adversarial_review.md`）后 **Agent Approved**——编号按 G19.1 实测 `namespaces.RFC next_free=36` 领取（ledger 校准：on_tree_max 35→36、next_free 36→37）。此后自由号以 ledger 实测为准（当前 RFC-0037）。
>
> **RFC 台账校准（2026-08-24，G20.1 治理波）**：**RFC-0037**（[`0037-virtualized-geometry-p4-hzb.md`](0037-virtualized-geometry-p4-hzb.md)，G20 虚拟化几何 P4：HZB 遮挡剔除 host 参考臂 + cluster 流送 P4 评估 + M61/M98-l4 重判程序）经 D-409 对抗性评审（评审 `milestones/g20/design/rfc0037_adversarial_review.md`）后 **Agent Approved**——编号按 G20.1 实测 `namespaces.RFC next_free=37` 领取（ledger 校准：on_tree_max 36→37、next_free 37→38）。此后自由号以 ledger 实测为准（当前 RFC-0038）。
>
> **RFC 台账校准（2026-08-24，G21.1 治理波）**：**RFC-0038**（[`0038-lighting-p3-deepening.md`](0038-lighting-p3-deepening.md)，G21 光照 P3+ 深化：ReSTIR 高档 reservoir host 参考臂 + SER 两半实测重判 + RD-040 五分项处置 + RD-034 上游复查程序）经 D-409 对抗性评审（评审 `milestones/g21/design/rfc0038_adversarial_review.md`）后 **Agent Approved**——编号按 G21.1 实测 `namespaces.RFC next_free=38` 领取（ledger 校准：on_tree_max 37→38、next_free 38→39）。此后自由号以 ledger 实测为准（当前 RFC-0039）。
>
> spec 条款号 `RXS-####`、错误码、deferred `RD-###`、spike-gating `SG-###` 的台账各自在 `spec/`、`registry/error_codes.json`、`registry/deferred.json`、`registry/spike_gating.json` 维护，均永不复用。
>
> **跨分支/off-tree 编号消费**（如 GRX 影子分支对 `MR-0006/0007`、`RXS-0181~0184` 的 claim + 私有 `GRX-0xx`/`patch-00xx`/`D-GRX`/`G-GRX` 段）登记于 [`../registry/number_ledger.json`](../registry/number_ledger.json)（MR-0010），并由守卫 [`../ci/check_number_ledger.py`](../ci/check_number_ledger.py) 强制『树内同号异义碰撞 + 已登记保留号被尊重』（10 §9.5 跨分支执行面）。守卫能力边界：CI 只见当前树，无法枚举未合分支，新影子工作流登记为人工/agent 前置动作。

## 6. 模板

- Full RFC：[`TEMPLATE-RFC.md`](TEMPLATE-RFC.md)
- Mini-RFC：[`TEMPLATE-MINI-RFC.md`](TEMPLATE-MINI-RFC.md)
