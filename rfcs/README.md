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
| Full RFC `RFC-####` | RFC-0001（[`0001-cuda-d3d12-interop.md`](0001-cuda-d3d12-interop.md)，G1.1）· RFC-0002（[`0002-shader-stages.md`](0002-shader-stages.md)，G2.1，**Approved 2026-06-23**）· RFC-0003（[`0003-dxil-backend.md`](0003-dxil-backend.md)，G2.2 MIR→DXIL 第二后端，**Approved 2026-06-23**）· RFC-0004（[`0004-spirv-dxil-graphics-backend.md`](0004-spirv-dxil-graphics-backend.md)，G2.2 图形=B，**Approved 2026-06-25**）· RFC-0005（[`0005-binding-layout-inference.md`](0005-binding-layout-inference.md)，G2.3 绑定布局推导，**Approved 2026-06-28**）· RFC-0006（[`0006-uc04-deferred-renderer.md`](0006-uc04-deferred-renderer.md)，G2.4 UC-04 deferred 渲染器 / 原生 D3D12 运行时出图路径，**Approved 2026-06-28**）· RFC-0007（[`0007-texture-sampling-memory-model.md`](0007-texture-sampling-memory-model.md)，G2.4 纹理采样内存模型，**Approved 2026-06-30**）· RFC-0008（[`0008-edition-stabilization.md`](0008-edition-stabilization.md)，G2.5 edition 机制与 stabilization，**Approved 2026-06-30**）· RFC-0009（[`0009-host-gpu-orchestration.md`](0009-host-gpu-orchestration.md)，MS1.2 single-source 宿主 GPU 编排 std::gpu + present typestate 面 + 宿主图像落盘桥，**Approved 2026-07-14**）· RFC-0010（[`0010-uc07-sim-renderer.md`](0010-uc07-sim-renderer.md)，MS1.3 UC-07 ruridrop 主语言渲染器/仿真二合一应用 + 主语言判据操作化，**Approved 2026-07-14**）· RFC-0011（[`0011-vulkan-spirv-backend.md`](0011-vulkan-spirv-backend.md)，mb1 Vulkan/SPIR-V 跨端第三后端 AMD 桌面 + Android compute+graphics,**Owner Approved 2026-07-15**——owner 明确指示解除红线 3 并继续）· RFC-0012（[`0012-toolchain-real-distribution.md`](0012-toolchain-real-distribution.md)，EA1 rurixup 工具链真实分发:FS 物化 + 活跃切换 + GitHub Releases 四级校验拉取 + 发布资产自动化,RD-025 兑现,**Draft 2026-07-16**——§9 Q-A~Q-D 经 milestones/ea1/OWNER_DECISION_PACKAGE.md 呈 owner,裁决 A/B 落地前不翻 Approved） | RFC-0013 |
| Mini-RFC `MR-####` | MR-0001（[`mini-0001-async-buffer.md`](mini-0001-async-buffer.md)，G1.2）· MR-0002（[`mini-0002-engine-integration.md`](mini-0002-engine-integration.md)，G1.3）· MR-0003（[`mini-0003-oss-community.md`](mini-0003-oss-community.md)，G1.4）· MR-0004（[`mini-0004-geometry.md`](mini-0004-geometry.md)，G1.4 生态二梯队）· MR-0005（[`mini-0005-fatbin-distribution.md`](mini-0005-fatbin-distribution.md)，G1.5 生产分发 fatbin;**台账滞后修正**:文件 2026-06-22 已落,本行随 MR-0008 PR 补登）· **MR-0006 / MR-0007 已被 GRX 影子分支(`codex/grx-godot-dxil-workspace`,closed,未合 main)claim**,main 侧跳号避撞(编号永不复用 10 §9.5,对齐 MR-0005 避撞 MR-0003/0004 教训;结构化登记见 [`../registry/number_ledger.json`](../registry/number_ledger.json) `namespaces.MR` + `off_tree_workflows[grx]`,守卫 [`../ci/check_number_ledger.py`](../ci/check_number_ledger.py))· MR-0008（[`mini-0008-stable-channel-manifest.md`](mini-0008-stable-channel-manifest.md)，V1.2 语言 1.0 stable channel 最小清单，**Approved 2026-07-14**）· MR-0009（[`mini-0009-toolchain-frontend.md`](mini-0009-toolchain-frontend.md)，post-V1 rurixup 工具链前端首切片:install/list/default 消费 stable channel，**Approved 2026-07-14**）· MR-0010（[`mini-0010-shadow-workflow-ledger.md`](mini-0010-shadow-workflow-ledger.md)，影子/off-tree 编号工作流登记机制:`registry/number_ledger.json` + `ci/check_number_ledger.py` 跨分支保留号守卫，**Approved 2026-07-17**） | MR-0011 |

> spec 条款号 `RXS-####`、错误码、deferred `RD-###`、spike-gating `SG-###` 的台账各自在 `spec/`、`registry/error_codes.json`、`registry/deferred.json`、`registry/spike_gating.json` 维护，均永不复用。
>
> **跨分支/off-tree 编号消费**（如 GRX 影子分支对 `MR-0006/0007`、`RXS-0181~0184` 的 claim + 私有 `GRX-0xx`/`patch-00xx`/`D-GRX`/`G-GRX` 段）登记于 [`../registry/number_ledger.json`](../registry/number_ledger.json)（MR-0010），并由守卫 [`../ci/check_number_ledger.py`](../ci/check_number_ledger.py) 强制『树内同号异义碰撞 + 已登记保留号被尊重』（10 §9.5 跨分支执行面）。守卫能力边界：CI 只见当前树，无法枚举未合分支，新影子工作流登记为人工/agent 前置动作。

## 6. 模板

- Full RFC：[`TEMPLATE-RFC.md`](TEMPLATE-RFC.md)
- Mini-RFC：[`TEMPLATE-MINI-RFC.md`](TEMPLATE-MINI-RFC.md)
