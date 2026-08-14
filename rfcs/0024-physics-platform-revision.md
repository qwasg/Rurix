# RFC-0024 — 物理平台修订（G9 伞形三章之三，RFC-0021 修订）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0024（4 位制，编号永不复用，10 §9.5；2026-08-09 实测 `registry/number_ledger.json` namespaces.RFC next_free=22 顺序领取：RFC-0022=虚拟几何与 GI 语义、RFC-0023=GPU-driven 提交与着色系统、RFC-0024=本文） |
| 标题 | 物理平台修订（G9 伞形三章之三，RFC-0021 修订） |
| 档位 | **Full RFC**（10 §3：对既有 Full RFC（RFC-0021）的运行时语义与冻结面做显式修订；新增 Field 语义面、多时间域扩展、确定性画像扩展与 NN 权威禁止线） |
| 状态 | **Agent Approved**（2026-08-09） |
| 修订对象 | [RFC-0021](0021-physics-platform.md)（v1.3，Agent Approved 2026-08-02）；本文是**修订 RFC**——只追加修订语义，不回写 0021 原文；0021 未被修订部分 0-byte 继续有效（体例见 §1.1） |
| 承接里程碑 | G9 D5 物理建造期：M121（统一 physics particle view）· M122（Gameplay Field）· M123（双通道确定性判档）· M124（解析浮力）· M125（Jolt 5.3→5.6 A/B）· M126（Rapier 对标基准）· M127（神经变形研究子轨）；P0 门 `g9.p0.m121.physics_particle_view` / `g9.p0.m122.gameplay_field`（G9.2 骨架 → G9.6 完整，见 §6.4） |
| 关联条款 | **预期零新语言 spec 条款**（物理仍是引擎库，06 §8.3 / RFC-0017 五纪律 / RFC-0021 §5.2 不变）；资产 schema 与运行时契约见 §5，实现互锁开放后随首个实现 PR 落结构化事实源 |
| 依据决策 | D-406 v2.0（agent 完全自主）· D-409（独立 provenance 对抗性评审）· P-01/P-05/P-09/P-11/P-12/P-13 · RFC-0017 · RFC-0021（v1.3）· G9_PLAN · G9_CAPABILITY_MATRIX §5（M121~M127 行）· G9_D5_PHYSICS v0.1（G9.0 不可变基线附件）· R8_PHYSICS |
| Provenance | `Assisted-by: Kimi Code CLI (Kimi) rfc0024-drafter`（独立起草会话；只起草，不批准） |
| Agent 批准 | **Agent Approved 2026-08-09**；批准范围含 §1.1 全部 🔒 修订行；记录方式 = §9.1 评审记录 + 修订记录 v1.0；**批准不解锁实现**，实现仍由 G9 契约与实现互锁决定 |
| 对抗性评审 | **完成**（D-409 第 1 轮，2026-08-09）：评审 provenance `Assisted-by: Kimi Code CLI (Kimi) rfc0024-adversarial-reviewer`（独立实例）≠ 起草 provenance `Assisted-by: Kimi Code CLI (Kimi) rfc0024-drafter`；7 findings（3 major + 4 minor）全部 disposition（全部采纳并修）；同工具族偏差如实登记，详见 §9.1 |

---

## 1. 摘要

G8 已 closed（2026-08-06），G8.6 交付了 replay-first 物理平台底座：Jolt 5.3 capture/replay corpus、网络回滚链、破坏生产链、CharacterVirtual、PhysicsAsset/ragdoll、布料、载具与 Physics→GpuScene 单向桥。G8 同时以 defer/no-go 留档了四个物理面：M74 Physics Field（defer「G9+ gameplay Field」）、M75 异步物理 tick（no-go「异步调度须独立判档」）、M77 水体/浮力（no-go「ApplyBuoyancyImpulse 未包装」）、M65b Rapier 深造（no-go 条件制）。G9 立项后，D5 物理建造期把前三个面从留档转为正式建造（Rapier 维持条件制，仅做对标基准），并延续 G8.6a 的 Jolt 升级纪律评估 5.3→5.6。

本文是 RFC-0021 的**修订 RFC**，产出形态为：逐条 🔒 修订行（§1.1）+ 新增冻结语义（§4）+ 两道 P0 symbolic gate（§6.4），**零实现、零数字、零编号预消费**。核心裁决：

1. **M74 → go**：Gameplay Field 系统（M122）按三层解耦 + 三生命周期 + 一等过滤 + World-Field 通道建造；前置抽象 = 统一 physics particle view（M121）。
2. **M75 → 独立判档移交 G9/D5**：双通道确定性架构（lockstep-deterministic 永不异步化 vs async-decorative 零回写）；**判档硬前置 = Jolt 单线程成本 measured，测量不足维持 no-go**——「异步调度须独立判档」为 `G8_P2_DECISIONS.md` M75 行理由列字面（RFC-0021 §2.4 行 123 作「异步调度仍需独立判档」）；Q6 唯一字面以 R-6 所引为准。
3. **M77 → go**：解析浮力模型（浸入体积/浸没质心 → 浮力 + 浮力矩 + 阻力 impulse），**走 Field 通道、禁旁路 API**，确定性内置入 corpus；真双向流体耦合排除主线。
4. **Jolt 5.3→5.6**：§4.A4 七步程序逐字执行不变；新摩擦模型为重点；GPU compute 接口只评估不接权威。
5. **神经变形**：维持 `rfcs/0021:122` 无归属留痕、不新设 RD；M127 研究子轨、无主线门；**NN 权威禁止线**（NN 输出不得替代权威状态）随本 RFC 冻结。

本 RFC 可在 G9.1 governance-only 内起草与评审；**RFC 批准不解锁实现**，实现互锁与波次由 G9 契约决定。

### 1.1 修订 RFC 体例与修订行清单

**体例声明**：本文是 RFC-0021 的修订 RFC。凡对 RFC-0021 既有节/行/字面构成修订的语义，一律以本节 🔒 修订行逐条登记；修订行只**追加修订语义**，不回写 0021 原文（0021 文件 0-byte 改动）；RFC-0021 未被下列修订行覆盖的部分 **0-byte 继续有效**。0021 的修订记录不由本文代写。

| # | 对 RFC-0021 的修订点 | 修订内容（只追加修订语义） |
|---|---|---|
| R-1 🔒 | §2.3 in-scope 表 | 追加三行 G9 承接关系：M74 Physics Field（G9 承接 = M121/M122，主线建造）、M75 异步物理 tick（G9 承接 = M123，独立判档）、M77 水体/浮力（G9 承接 = M124，主线建造）；G8 既有行字面 0-byte，波次归属 G9 契约 |
| R-2 🔒 | §2.4 out-of-scope 行 122 神经变形项 | 修订为「G9 D5 研究子轨承接（M127），无主线门」；**维持无 RD 归属留痕、不新设 RD**；同表的 GPU 主刚体、可微物理（RD-042 观察维持）两项 0-byte 重申、不重开 |
| R-3 🔒 | Field 语义面（0021 无对应节，属节级追加） | 新增 Gameplay Field 语义：三层解耦（定义层/作用对象层/目标语义层）、`FieldPhysicsType` 首期八枚举、三生命周期（persistent 可显式注销）、过滤默认空匹配 = 零影响、World-Field 通道（GpuScene 只读 buffer）；本 RFC Approved 即为冻结面，冻结内容见 §4.B |
| R-4 🔒 | §3.3 多时间域 identity 表 | 追加 `DecorativePhysicsTickId`（**仅 M123 判档 go 时生效**；async-decorative 通道自有时间域，禁止冒充 `PhysicsTickId`）；追加 `WorldFieldSampleSet` 时间域归属 `RenderFrameId`、经 `FrameDomainMap` 显式映射的规则；既有五个 identity 字面 0-byte |
| R-5 🔒 | §4.A1 determinism 画像 | 加性扩展 `deterministic_profile` 字段：fixed_dt 锁死、substepping off、sleep 策略钉值、BVH-refit 策略钉值、`job_threads` 与画像一致；并要求负例 RED 臂（画像外运行必须 fail-closed）；既有画像字段 0-byte |
| R-6 🔒 | §9 Q6 行（M75 异步 tick） | Q6 字面「**提议**本 RFC 只冻结不同时间域 identity 和桥接，异步 physics thread 调度保持 P2，不能成为 M66~M72 前置」（rfcs/0021 §9 Q6 行；Q6 属 0021 的**提议性未决项**，非已裁决冻结句）**不变**；本行登记**判档移交**：异步调度的独立判档由 G9/D5 承担（M123），判档程序与硬前置见 §4.C——**Jolt 单线程成本 measured 是硬前置，测量不足维持 no-go**。注：「异步调度须独立判档」是 `G8_P2_DECISIONS.md` M75 行理由列字面（0021 §2.4 行 123 作「异步调度仍需独立判档」），全文不以此冒充 Q6 字面 |
| R-7 🔒 | §6.2 runtime feature 名冻结面 | 追加 `physics-field` / `physics-buoyancy` /（M123 判档 go 时）`physics-async-decorative`；名称即日冻结，改名须修订行；既有六个 feature 名 0-byte |
| R-8 🔒 | §6.5 冻结 bound 三步程序 | 浮力模型的数值 bound（若实现期确需）沿用同一三步程序（采样 → 本 RFC 加性修订行冻结 → 生效）；程序本身 0-byte |
| R-9 🔒 | §3.2 CPU 权威刚体与 GPU 禁止线（同构新增约束登记） | 新增 **NN 权威禁止线**：NN 输出不得替代权威状态（§4.F）；与 GPU 主刚体禁止线同构登记；GPU 禁止线本身 0-byte |
| R-10 🔒 | §2.4 G5 渲染冻结面行 | World-Field 通道（§4.B5）仅经既有 Physics→GpuScene 桥把场采样参数按 tick 提交为 GpuScene 只读 buffer，**渲染侧零回写**；若实现期需要扩展 `GpuScene` 冻结面（新 buffer 面），必须经渲染侧 RFC（RFC-0019 面）的显式修订行，本 RFC 不隐式扩展 G5 渲染冻结面 |

## 2. 动机、基线与边界

### 2.1 承接锚（G8 终态字面）

| 锚 | G8 终态 | 字面依据 | 本文处置 |
|---|---|---|---|
| M74 Physics Field | defer-to-G9+，目标 =「G9+ gameplay Field」 | 「M68 damage/field journal 已覆盖 G8 最小面；统一 Field 属建造期」（G8_P2_DECISIONS M74 行；矩阵 M74） | **go**：M121 前置抽象 + M122 主线建造（§4.A/§4.B） |
| M75 异步物理 tick | no-go，open-留档 | 「本期只冻结时间域 identity；异步调度须独立判档」（`G8_P2_DECISIONS.md` M75 行理由列字面；RFC-0021 §2.4 行 123 作「异步调度仍需独立判档」；矩阵 M75） | **判档移交 G9/D5**：双通道架构（§4.C），硬前置 = Jolt 单线程成本 measured（R-6 🔒） |
| M77 水体/浮力 | no-go，open-留档 | 「未包装且无 gameplay 需求；联动 M49 defer」（`G8_P2_DECISIONS.md` M77 行理由列字面；G8 矩阵 M77 行字面为「Jolt `ApplyBuoyancyImpulse` 原语未包装」） | **go**：解析浮力模型，走 Field 通道（§4.D）；M49 联动维持 defer（权威浮力不经 Taichi） |
| M65b Rapier 深造 | no-go，open-留档 | RD-044「快路径被真实 workload 采用时」（registry/deferred.json；矩阵 M65b） | **维持条件制**：对标基准先行（§4.E2）；RD-044 字面不变 |

### 2.2 既有基础（G8.6 交付物，直接复用、不得重建）

- Jolt 5.3 capture/replay corpus（`g8.p0.m66.physics_replay` 全绿）；恢复层 = `semantic_journal_rebuild_v1`（RFC-0021 §2.1.1.1 全线 (c)）；
- 网络回滚链（M67）、`RurixCharacter`（M71）、PhysicsAsset/ragdoll（M69）、破坏生产链（M68，含 damage/field journal 最小面）、自有 XPBD 布料（M72）、自研悬挂载具（M70）；
- Physics→GpuScene 单向变换桥（RFC-0017 纪律 1）与五时间域 identity + `FrameDomainMap`（RFC-0021 §3.3）。

所有 D5 新面的确定性断言一律挂接 M66 corpus 设施；任何新面不得反向要求改写 M66 capture 格式而不经显式 schema migration（承 RFC-0021 §6.3）。

### 2.3 为何需要 Full RFC

本文对一份已 Agent Approved 的 Full RFC 做运行时语义修订：新增 Field 目标语义与生命周期规则、扩展多时间域 identity 与 determinism 画像冻结面、登记 NN 权威禁止线、冻结两道 P0 门判据。按 10 §3 向上取严为 Full RFC。物理仍是引擎库，不借机进入 Rurix 语言核心。

### 2.4 in-scope

| 能力 | 编号 | 冻结内容 | 性质 |
|---|---|---|---|
| 统一 physics particle view | M121 | 五域 `ParticleAdapter`、`PhysicsParticleRef` 名义类型、写路径仅 impulse/force；M68 damage journal 迁移为首个 consumer | 主线（P0） |
| Gameplay Field 系统 | M122 | 三层解耦 + 首期 `FieldPhysicsType` 八枚举 + 三生命周期 + 过滤默认空匹配 + World-Field 通道 | 主线（P0） |
| 双通道确定性架构 | M123 | lockstep-deterministic vs async-decorative + `deterministic_profile` 运行时断言 | 条件制（判档后定档） |
| 解析浮力模型 | M124 | 浸入体积/浸没质心 → 浮力+浮力矩+阻力 impulse；走 Field 通道、确定性内置入 corpus | 主线（P1） |
| Jolt 5.3→5.6 升级 A/B | M125 | §4.A4 七步逐字执行；新摩擦模型重点；采纳臂三件事/失败臂钉 5.3 | 评估门（P1） |
| Rapier 深造对标基准 | M126 | 新 BVH / sparse voxel / persistent islands / 摩擦模型 / glam 迁移 A/B；不作 replay oracle | 基准（P2） |
| 神经变形研究子轨 | M127 | 混合架构优先、离线工具链、PhysicsAsset residual 通道预留 | 研究子轨（无主线门） |

### 2.5 out-of-scope

见 §8 范围红线。先重申两条 0-byte 线：**GPU 主刚体禁止线维持**（G6 裁决、RFC-0017、RFC-0021 §1 评审 F-06、矩阵 §12——包括「经预算隔离的可选副求解器」在内一律禁止；任何 GPU 刚体提案须 RD-043 触发 + 矩阵 §12 五项重审 + 独立 Full RFC）；**可微物理排除**（RD-042 观察维持，RFC-0021 §2.4 F-18 归属，不进本文任何面）。

## 3. 跨章不变量

### 3.1 RFC-0017 五纪律 0-byte 延续

RFC-0021 §3.1 已逐字引用 RFC-0017 §4.B1 五条纪律；本文**不重引、不改写**，五纪律全文 0-byte 继续有效。本文新增面在其约束内设计：单向事实源（纪律 1）约束 particle view 写路径与 World-Field 通道；Taichi AOT 出参面（纪律 4）约束浮力权威不经 Taichi；库不进语言（纪律 5）约束全部新 FFI 只留绑定 crate。

### 3.2 CPU 权威刚体、GPU 禁止线与 NN 权威禁止线

- CPU Jolt 权威主线不变（RFC-0021 §3.2 0-byte）：Field、浮力、lockstep 通道全部运行在 CPU 权威世界内。
- **GPU 主刚体禁止线 0-byte**：Jolt 5.6 新增的 GPU compute shader 接口在 D5 **只许评估、不许接入权威求解**（§4.E1）；评估报告留档，接入须 RD-043 + 矩阵 §12 + 独立 Full RFC。
- **NN 权威禁止线（R-9 🔒，新增冻结）**：任何 NN 输出不得替代权威状态；NN 只做加速近似或表现层输出，权威物理骨架兜底；违反即研究子轨红、不得进任何主线面。

### 3.3 多时间域 identity 扩展（R-4 🔒）

RFC-0021 §3.3 五时间域 0-byte。追加：

- `DecorativePhysicsTickId`：async-decorative 通道的独立固定步 identity（**仅 M123 判档 go 时生效**）；与 `PhysicsTickId` 不同名义类型，禁止裸整数互转或冒充；lockstep → async 单向经事件队列表达，跨域关系仍只经 `FrameDomainMap` 显式记录。
- `WorldFieldSampleSet`：World-Field 采样的时间域归属 `RenderFrameId`；每次采样记录消费的 `PhysicsTickId`（场参数提交 tick）与映射的 render frame，禁止隐式「当前帧」读取。

## 4. 参考级设计

### 4.A M121 — 统一 physics particle view

**依据**：UE Chaos Field System 的作用对象层是唯一大规模生产验证的统一抽象——刚体/布料顶点/碎块/RBA 节点统一为 Chaos Particles（R8 §2）。

- `PhysicsParticleRef` = `(domain, stable_id, element_index)` 名义类型：`domain ∈ {RigidBody, ClothVertex, DestructionChunk, RagdollNode, CharacterInner}`；`stable_id` 复用 generation 语义（RFC-0021 §3.4），绝不暴露 arena index。
- 每个域实现 `ParticleAdapter` trait：`mass() / position() / velocity() / set_force_impulse() / sleep_state()`；**写路径只允许 impulse/force 语义，不允许直接改写 transform**——纪律 1 单向事实源 0-byte，桥仍只读已提交变换。
- 视图是 Field 求值的唯一作用对象面；M68 destruction damage journal（G8 最小面）**迁移为该视图的第一个 consumer**，保持 journal 兼容（显式迁移器 + golden，迁移前后 digest 一致）。
- 视图只覆盖 CPU 权威世界内的对象；GPU 副轨粒子不进入该抽象。

### 4.B M122 — Gameplay Field 系统（R-3 🔒）

**依据**：Chaos Field System 三层解耦（R8 §2）；「场默认影响所有 Actor 是 UE 社区大坑」（R8 §2，[S04]）。

**B1. 三层解耦**：

1. **场定义层（Field Nodes）**：场 = 空间标量/向量函数 + 元数据。基元集：radial falloff / box / sphere / noise / curve-driven / analytic-surface（为浮力水面函数预留，§4.D）；节点图组合，但图 schema 版本化、canonical 序列化、cook 确定性（承 RFC-0021 §5.1 共同头纪律）；首期不接通用可视化 node graph 编辑器。
2. **作用对象层**：经 M121 的 `PhysicsParticleRef` 寻址（§4.A）。
3. **目标语义层 `FieldPhysicsType`**（对标 `EFieldPhysicsType`）：首期枚举冻结为八项——`LinearForce / Strain / Velocity / Torque / Sleeping / Disabled / CollisionGroup / Buoyancy`。`Buoyancy` 是 RuriX 相对 Chaos 的加性扩展（M124 共用求值管线）；扩枚举须先经两个真实用户（destruction damage + 浮力）验证。

**B2. 三生命周期**：

| 生命周期 | 语义 | 确定性规则 |
|---|---|---|
| Transient | 单 tick 内求值即弃 | 不进 journal；结果经命令规范化进 journal |
| Construction | cook/关卡构建期烘焙（如 anchor strain 预置） | 进 cooked artifact digest |
| Persistent | 跨 tick 存活 | **必须可显式注销**；**注册/注销/参数变更全部写 command journal，参与 `semantic_state_hash`，replay 逐 tick hash 一致为硬门** |

**B3. 过滤一等公民（零影响不变量）**：`FieldFilter = (object_state_mask × domain_mask × layer_mask × explicit_include/exclude)`；**默认空集匹配 = 无影响**，拒绝「默认全影响」语义；filter 是场定义的一部分，进 digest。过滤负例（默认空匹配 = 零影响）是 P0 门的显式断言项（§6.4）。

**B4. World-Field 通道（物理→VFX 反向通信，R-10 🔒）**：渲染/VFX 采样物理场的只读出口。GPU 求值**唯一合法路径 = Physics→GpuScene 桥**：场采样参数按 tick 提交为 GpuScene 只读 buffer（纪律 1 不变：物理→渲染单向；渲染不回写物理）；VFX/材质在 GPU 侧消费该 buffer 自行求值。`WorldFieldSampleSet` 归属 `RenderFrameId`，经 `FrameDomainMap` 显式映射（R-4 🔒）。**触 G5 冻结面纪律**：实现期若需扩展 `GpuScene` 面，须渲染侧 RFC 显式修订行，本文不隐式扩展。

### 4.C M123 — 双通道确定性架构（判档建议稿，R-6 🔒）

**依据**（R8 §3）：固定步长是确定性的必要非充分条件——还须 substepping off、锁死 dt、关 sleep/BVH-refit 等打包项，否则回放必分叉（Bugnet 2026 UE 事故）；异步线程与 lockstep 根本冲突；生产实践 = 双通道。

**通道划分**：

| 通道 | 内容 | 确定性承诺 | 调度 |
|---|---|---|---|
| lockstep-deterministic | gameplay-critical 刚体、Character、ragdoll、destruction cluster、vehicle、cloth 权威求解、Field（persistent/含 gameplay 语义）、浮力 | 承 RFC-0021 全集：capture/replay、逐 tick hash、网络 rollback | 维持固定步 `PhysicsTickId` 主线，**永不异步化**（与 rollback/corpus 根本冲突） |
| async-decorative | 碎块视觉、表现层布料副轨、次级刚体、VFX 联动粒子 | **无**；只接收单向力/事件，**绝不回写 gameplay 状态**；对象可丢弃可重建 | 独立线程，自有 `DecorativePhysicsTickId`（R-4 🔒） |

**硬边界**：async-decorative → lockstep 方向**零写路径**（类型系统层无 API）；lockstep → async 单向经事件队列（装饰事件允许丢弃）。

**`deterministic_profile` 运行时断言（R-5 🔒）**：启动与 corpus 运行前断言 `fixed_dt 锁死 && substepping==off && sleep_policy==profile值 && job_threads 与画像一致 && bvh_refit_policy==profile值`；任一不符 fail-closed。该画像加性并入 capture header 的 determinism 画像（RFC-0021 §4.A1 扩展）；**负例 RED 臂**：变步长运行、可变 substep 运行、画像外 `job_threads` 运行必须被 corpus 拒绝（fail-closed 断言三个独立布尔）。

**异步化范围决策程序（判档硬前置，R-6 🔒）**：先测 Jolt 单线程成本（D5 先行任务），只有 measured 证据显示主线程物理超预算才把装饰对象迁入 async 通道；**测量不足则维持 M75 no-go 留档**——「异步调度须独立判档」为 `G8_P2_DECISIONS.md` M75 行理由列字面（RFC-0021 §2.4 行 123 作「异步调度仍需独立判档」），Q6 唯一字面以 R-6 所引为准；本判档即该判档的承担。判档结论以本 RFC 修订行落定。

### 4.D M124 — 解析浮力模型

**依据**（R8 §4）：生产路径 = 解析近似（浸入体积/浸没质心 → 浮力 + 浮力矩 + 水阻力 impulse；Eurographics 2020 模型在细长/翻滚物体上稳定性优于 UE 采样点 Pontoon 方案——该对比结论引自 D5 草案转述，R8 标注未能独立复核原始出处）；真双向耦合经 CGF 2025 STAR 判定仍研究级，排除（§8）。

- **走 Field 通道、禁旁路 API**：水体区域 = persistent field（解析水面函数为场定义层 `analytic-surface` 基元）；`FieldPhysicsType::Buoyancy` 语义。水体是 Field 统一抽象的**第二个真实用户**（第一个是 destruction damage）。浮力不得长成第二套空间影响管线——「走 Field 通道」是硬判据，旁路即红。
- **求值**：每 tick 对落入 filter 的 `PhysicsParticleRef`（首期 RigidBody 域）计算 clipped 浸入体积与浸没质心 → `buoyancy impulse + buoyancy torque + linear/angular drag impulse`，经 `AddForceAtPoint` 类既有 FFI 施加（消费既有导出符号纪律不变）。
- **确定性内置**：固定 dt + 解析水面函数（**禁帧率相关插值、禁墙钟相位**）；全部输入/输出进 command journal；fixture 入 capture/replay corpus（M66 设施挂接点）。变帧率输入同 tick 结果逐位一致为断言项。
- **形状支持分层**：首期 convex/primitive 解析 clip；任意 mesh 走离线预计算 voxelized volume table（cooked artifact，版本化）。
- **M49 联动维持 defer**：Taichi AOT 只产出粒子/体积场（纪律 4 字面不变），水面视觉可消费 World-Field 通道，但权威浮力不经 Taichi。

### 4.E M125/M126 — Jolt 5.3→5.6 升级路径与 Rapier 对标基准

#### E1. Jolt 5.3→5.6 A/B（M125）

**纪律承 G8.6a**：「corpus 先在当前版本建成再评估升级」——G8 已建成，D5/G9 评估窗正式开启。程序**逐字承 RFC-0021 §4.A4 七步，不变**：① 冻结 5.3 基线（corpus/资产 cook digest/CCD/contact/query 结果/measured baseline）→ ② 5.6 独立 vendor/ABI 构建、不覆盖 5.3 基线 → ③ 两版本各自证明同版本 capture/replay 逐 tick 一致 → ④ 相同 canonical source asset/input journal A/B → ⑤ 性能阈值只从真实采样写入 budget、版本锚按实测 tag/commit 登记 → ⑥ **失败臂**：任一硬门失败正式钉住 5.3、记录失败证据、不得伪写 5.6 PASS → ⑦ **采纳臂三件事**：corpus 显式迁移并保留 5.3 基线 artifact + replay 门在新版本重跑落 evidence + 判据字面经修订后才改版本号。**两臂诚实登记**：采纳与失败都是正式终态；G9 契约判据字面若再钉「Jolt 5.3」，同样须修订后才可改字面。

**分项处置**：

| 5.6 特性 | 处置 |
|---|---|
| 新摩擦模型（平均接触点；官方口径 Pyramid 测试快 15%/省 40% 内存/消除接触点序偏向） | **A/B 重点项**：消除接触点序偏向对确定性 corpus 有直接价值；但求解器语义变化 → 逐字段 exact/tolerance/invariant 分类（§4.A4 程序） |
| GPU compute shader 接口 | **只评估不接权威**（GPU 主刚体禁止线 0-byte）；评估报告留档，接入须 RD-043 + 矩阵 §12 + 独立 Full RFC |
| GPU strand 毛发（Cosserat 杆，官方自标 work in progress） | 非权威装饰副轨候选（async-decorative 通道），非主线 |
| HeightField 16bit | 与流送/地形页联动评估，独立分项判档 |
| glTF `KHR_physics_rigid_bodies` 马达 | 资产管线候选增强，进 RFC-0020 面而非本文 |
| `Ragdoll::DriveToPoseUsingMotors` | 与 G8 M69 约束五件套路线对照评估；采纳需 JoltC C 面审计（当前 pin 无该导出） |
| **layout 探针工具化** | layout_hinge 类探针升级为 vendor 升级/新 FFI 检查单固定项：所有 `*Settings` 结构 sizeof/offsetof 静态断言重跑；探针源码入库，不再散落工作树 |

#### E2. Rapier 深造对标基准（M126）

- **对标基准先行**：新 Dynamic BVH / sparse voxel collider / persistent islands / manifold ≤4 / 简化摩擦模型大堆叠场景建 A/B benchmark 夹具（与 Jolt 同场景、同输入、同 determinism 画像）；产出 measured 报告，**不作 replay oracle**（RFC-0021 §7 备选 D：跨 solver 不承诺逐位，只作不变量/容差对拍）。
- **RD-044 字面不变**：「快路径被真实 workload 采用时」才深造。D5 建造期 workload（Field 高频查询、async-decorative 次级刚体）是首个真实候选消费方；基准显示 measured 优势才按 RD-044 程序申请判档，否则维持 no-go 留档。
- **glam 迁移兼容**：Rapier 0.32+ glam 化对既有快路径封装的 API 冲击评估，兼容层设计留档；不承诺 bitwise 不变。
- Rapier 的 rust-gpu 刚体方向仅观察（GPU 主刚体禁止线）；Dimforge 定锚 rust-gpu 只作「Rust→GPU」路线佐证留档，不构成任何接入承诺。

### 4.F M127 — 神经变形研究子轨边界（R-2/R-9 🔒）

**边界冻结**（维持 `rfcs/0021:122` 无归属留痕、**不新设 RD**；本子轨无主线门、无 P0/P1 判据、不进 G9 收口硬门）：

- **混合架构优先**（Hybrid Neural-MPM 同构）：NN 只做加速近似，权威物理骨架兜底；**NN 权威禁止线（R-9 🔒）：任何 NN 输出不得替代权威状态**。
- **骨骼驱动神经服装先做离线工具链形态**：离线仿真 → 神经压缩 → 运行时回放，复用 G8 capture/replay 设施——corpus 即训练语料生产器，replay 设施即回放校验器。
- **PhysicsAsset 变形格式预留 residual 通道**：schema 加性扩展位（版本化、首期字段可空），避免日后 breaking 修订；schema 面走 RFC-0020。
- **产出形态**：研究报告 + 离线工具链原型 + PhysicsAsset schema 加性修订建议；成果另行判档，不记主线绿。
- **红线**：不承诺碰撞保证、不承诺双向耦合（R8 §7 共性短板）；可微物理不进本子轨（RD-042 观察维持，0-byte）。

## 5. 资产 schema、版本化与下游事实源

**预期零新 RXS 条款**（承 RFC-0021 §5.2）：物理仍是引擎库；若实现发现必须改变语言类型、内存模型、FFI ABI 或 unsafe 安全包络，必须另立/修订 Full RFC 并先落条款；本文不预留 RXS 号，也不以「引擎库」名义绕过规范先行。

| 条款（拟） | 标题 | 测试锚定计划（每条 ≥1） |
|---|---|---|
| —（零新 RXS） | 预期零新语言 spec 条款 | —（若实现期出现真实需求，另立/修订 Full RFC 后按 actual next-free 领取） |

实现互锁开放后，以下 schema/测试资产应与对应首个实现 PR 同落；此处只冻结职责，不创建空壳：

| 事实源 | 内容 | 消费方 |
|---|---|---|
| Field schema | 场定义层基元/节点图、`FieldPhysicsType`、filter、三生命周期 | M122/M124 |
| buoyancy cooked artifact | voxelized volume table（任意 mesh 分层支持）、解析水面函数资产 | M124 |
| PhysicsAsset residual 通道（加性） | 变形格式预留扩展位（版本化、首期可空） | M127（schema 面走 RFC-0020） |
| determinism profile 扩展字段 | `deterministic_profile` 并入 capture header determinism 画像（R-5 🔒） | M123/M66 corpus |

- **错误码策略**：资产/运行时失败优先使用结构化 `PhysicsError`/tool error，不预造 RX 号；只有出现真实可达的编译器/工具诊断类别时，按合入时 actual next-free 领取，registry 只追加 + en/zh message-key。
- **unsafe**：新增 FFI（若有）只留绑定 crate，每块附 `// SAFETY:` 并按实现时 actual next-free 登记 U；本文零 U claim。
- 本文不登记 RD；M127 维持无归属留痕（R-2 🔒）；实现期出现新阻塞才按 actual next-free 追加。

## 6. feature、tracking、实现序与验收

### 6.1 治理/实现硬互锁

当前合法动作只有 G9.1 governance-only：本 RFC 可被起草、对抗性评审与批准，但不得 materialize 实现。`src/`、`spec/`、`conformance/`、实现脚本、数字 CI、RD/U/RX 在实现互锁开放前均 0-byte；本 RFC 从 Draft 变 Approved 不构成任何实现许可。

### 6.2 runtime feature 边界（R-7 🔒）

承 RFC-0021 §6.2 既有六名 0-byte，追加（名称即日冻结，改名须修订行）：

- `physics-field`：Field 定义/求值/生命周期/过滤/World-Field 出口；
- `physics-buoyancy`：解析浮力模型（Field 通道消费方）；
- `physics-async-decorative`：async-decorative 通道（**仅 M123 判档 go 时启用**）。

功能未编译时返回明确 `FeatureNotCompiled` 类错误，不静默退化成视觉-only 成功。

### 6.3 实现序建议（波次细节由 G9 契约定）

1. 判档与治理：本 RFC 评审/批准 + Jolt 单线程成本测量（M123 判档硬前置）；
2. M121 particle view → M68 damage journal 迁移为首个 consumer（迁移器 + golden）；
3. M122 Field 系统核心：定义层基元/schema → 语义层 → 生命周期/journal → 过滤 → corpus 断言；
4. M124 浮力（消费 M121/M122）→ Field 通道接入 → corpus fixture；
5. World-Field 通道 + `deterministic_profile` 断言；async-decorative 通道依判档结论启停；
6. M125 Jolt A/B（独立 vendor 线，∥；失败不影响 Field/浮力在 5.3 上继续）；
7. M126 Rapier 对标基准（基准专用线，∥）；
8. M127 研究子轨全程伴随，无硬门。

依赖纪律：后波不得反向要求改写 M121 视图或 M66 capture 格式而不经显式 schema migration（承 RFC-0021 §6.3）。

### 6.4 symbolic gates（零数字 claim）

P0 key 命名空间单一事实源为 `G9_ACCEPTANCE_MAP.md` §2：key 一律小写点分 `g9.p0.m<##>.<slug>`，脚本一律 `ci/g9_<slug>_smoke.py`，evidence schema 一律 `milestones/g9/g9_m<##>_<slug>_evidence_schema.json`；**三向一致当事方 = `G9_ACCEPTANCE_MAP.md` / `G9_CONTRACT.md` 验收章 / `CI_GATES.md` §4**（由 `ci/check_g9_acceptance_map.py` 三向比对强制，任一处漂移即 FAIL）。本文**只冻结 key/脚本名/evidence schema 目标路径**，不创建空 schema 壳、不创建空脚本占位、不猜数字 CI 步骤号（承 RFC-0021 §6.4 纪律）。以下两行判据**逐字照抄 `G9_ACCEPTANCE_MAP.md` M121/M122 行**（判据事实源 = MAP 对应行）。

| Gate | 脚本 | evidence schema 目标路径 | 独立硬判据 |
|---|---|---|---|
| `g9.p0.m121.physics_particle_view` | `py -3 ci/g9_physics_particle_view_smoke.py --gate g9.p0.m121.physics_particle_view --phase g9.2`<br>`py -3 ci/g9_physics_particle_view_smoke.py --gate g9.p0.m121.physics_particle_view --phase g9.6` | `milestones/g9/g9_m121_physics_particle_view_evidence_schema.json` | 五域 `ParticleAdapter` 全部实现；写路径仅 impulse/force，旁路写注入即 RED；`PhysicsParticleRef` 名义类型编译期隔离断言全真；M68 damage journal 迁移为首个 consumer 后，迁移前后逐 tick digest 与 golden 一致、journal 全消费无损；单向事实源纪律 0-byte。schema 同时要求 `phase_g9_2_pass=true` 与 `phase_g9_6_pass=true`；骨架期绿色不能替完整期充绿。 |
| `g9.p0.m122.gameplay_field` | `py -3 ci/g9_gameplay_field_smoke.py --gate g9.p0.m122.gameplay_field --phase g9.2`<br>`py -3 ci/g9_gameplay_field_smoke.py --gate g9.p0.m122.gameplay_field --phase g9.6` | `milestones/g9/g9_m122_gameplay_field_evidence_schema.json` | 三层解耦 schema 冻结；首期 `FieldPhysicsType` 八枚举逐项 accept GREEN 与非法枚举 RED；过滤默认空匹配 = 零影响显式断言（field 注册但零匹配时世界状态 hash 与无 field 基线逐位一致）；persistent 注册/注销/变更全 journal 化且 replay 逐 tick hash 一致；World-Field 唯一出口 = GpuScene 只读 buffer、渲染侧零回写断言全真。schema 同 M121 双 phase 要求；任一阶段绿色不能替另一阶段充绿。 |

**双 phase 纪律（承 MAP）**：M121/M122 均为 `--phase g9.2`（骨架期）与 `--phase g9.6`（完整期）两条独立调用；evidence schema 强制 `phase_g9_2_pass` 与 `phase_g9_6_pass` 同真，任一阶段绿不替另一阶段充绿。

**非 P0 面**：M123（条件制）、M125、M126 不新造 key；其闭环证据由 G9 波次聚合门按 RFC-0021 §6.4 F-14 模式汇总（波次 evidence 中登记独立 subject 行，不产 P0/P1 PASS）。**M124 已判 go（P1，拟 G9.6）**：MAP 当前已 go P1 集合为空集，M124 go 的硬门落点走 G9.6 开工前 `G9_ACCEPTANCE_MAP.md` §1 只追加程序或波次聚合 subject（字面 G9.6 开工时冻结），当前不预造 key；其四项判据（解析模型 fixture 含细长/翻滚回归 / 走 Field 通道非旁路 / capture→replay 逐 tick hash 一致 / 禁帧率相关插值断言）随该落点校准。M127 无主线门、无 subject（研究子轨）。

## 7. 备选方案

- **A. Field 直接照搬 Chaos 全集**——否决；语义枚举膨胀而无真实 consumer。首期枚举冻结八项，只有两个真实用户（destruction damage + 浮力）验证后才扩。
- **B. 浮力走独立 API、不经 Field**——否决；会形成第二套空间影响管线，丧失对 M74 抽象正确性的生产检验；「走 Field 通道」为硬判据。
- **C. 全异步物理（单通道异步化）**——否决；异步时间轴与 lockstep 权威时间轴双源事实，与 capture/replay、网络回滚根本冲突（R8 §3）。生产可行解是双通道，且 lockstep 通道永不异步化。
- **D. 不测量直接拆 async 通道**——否决；为不存在的瓶颈付架构税。Jolt 单线程成本 measured 是判档硬前置。
- **E. 真双向流体-刚体耦合进主线**——否决；CGF 2025 STAR 判定仍研究级、无商业 gameplay-critical 使用证据（R8 §4）。
- **F. 神经变形进主线验收**——否决；四条共性短板（语料依赖/泛化退化/无碰撞保证/无法双向耦合）决定其只能是无主线门的研究子轨。
- **G. 借 Jolt 5.6 GPU compute 接口上 GPU 刚体**——否决；GPU 主刚体禁止线 0-byte，任何提案须 RD-043 + 矩阵 §12 + 独立 Full RFC。

## 8. 不做（范围红线）

- **GPU 主刚体**：含「经预算隔离的可选副求解器」在内一律禁止（RFC-0021 §1 评审 F-06 字面延续）；Jolt 5.6 GPU compute 接口只评估不接权威。
- **可微物理**：RD-042 观察维持（RFC-0021 §2.4 F-18 归属），不进本文任何面。
- **真双向流体-刚体耦合**（FLIP/MPM/SPH）：排除主线；浮力只做解析近似。
- **通用软体 MPM/FLIP/Flesh**：RD-044 Continuum/Fluid 观察面维持。
- **跨平台/跨编译器 bitwise lockstep 承诺**：RFC-0021 §2.4 放弃口径延续；async-decorative 通道本就不承诺确定性。
- **神经变形主线化**：不占主线验收门、不承诺碰撞保证、不承诺双向耦合、不新设 RD。
- 不改写 RFC-0017 五纪律、G6/G8 closed 契约字面、G5 渲染冻结面（World-Field 面的扩展须经渲染侧 RFC 显式修订行，R-10 🔒）、RFC-0021 未被 §1.1 修订行覆盖的任何部分。
- 编辑器 GUI、通用 node graph 编辑器、cache farm。
- 不预先消费 RXS、CI、RD、U、RX、SG 或 D 编号；数字阈值只从 `measured_local` evidence 写入，本文零预造数字。
- 不把 `Draft`/`Agent Approved` 当作任何 M121~M126 门 PASS。

## 9. 未决问题 / 关键裁决

- **Q1 — M123 异步判档（待裁）**：判档硬前置 = Jolt 单线程成本 measured；测量显示主线程物理超预算 → 采纳双通道（async-decorative 通道启用，`physics-async-decorative` feature 生效，`DecorativePhysicsTickId` 生效）；测量不足 → 维持 M75 no-go 留档。判档结论以本 RFC 修订行落定，并引判档证据。
- **Q2 — 浮力数值 bound（若需）**：浮力模型的数值判据（若实现期确需）沿用 RFC-0021 §6.5 三步冻结程序（R-8 🔒），本文不预造数字。
- **Q3 — M126 Rapier 深造判档（条件制）**：基准报告显示 D5 真实 workload 上 measured 优势 → 按 RD-044 程序申请深造判档；否则维持 no-go 留档。RD-044 字面不变。
- **Q4 — M127 成果判档**：研究子轨产出（离线工具链原型/PhysicsAsset residual schema 建议）不自动进主线；任何主线化提案另行判档，且不得越过 NN 权威禁止线。

## 9.1 对抗性评审记录（10 §3 / §7 · D-409）

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: Kimi Code CLI (Kimi) rfc0024-adversarial-reviewer`（独立实例，≠ 起草 `Assisted-by: Kimi Code CLI (Kimi) rfc0024-drafter`） |
| 评审轮次 | 第 1 轮（2026-08-09） |
| 评审镜头 | ① 字面归属核对（Q6 字面 vs `G8_P2_DECISIONS.md` M75/M77 行理由列字面、0021 §2.4 行 123、G8 矩阵 M77 行）② 与 `G9_ACCEPTANCE_MAP.md` 已冻结面一致性（M121/M122 双 phase 调用、判据逐字、三向比对当事方）③ 修订 RFC 体例完整性（🔒 修订行、非 P0 面衔接） |
| 结论 | **有条件通过**：3 major + 4 minor = 7 findings，全部**采纳并修**（正文实改），翻 Agent Approved |

**Findings 与 disposition**：

| # | Finding（评审者提出） | 严重度 | Disposition |
|---|---|---|---|
| F-1 | §1 摘要第 2 条/§2.1 M75 行/§4.C 末段三处称「异步调度须独立判档」为 RFC-0021 Q6 字面——实际 Q6 字面（rfcs/0021 §9 Q6 行）是「提议本 RFC 只冻结不同时间域 identity 和桥接，异步 physics thread 调度保持 P2，不能成为 M66~M72 前置」；「异步调度须独立判档」实为 `G8_P2_DECISIONS.md` M75 行理由列字面（0021 §2.4 行 123 为「异步调度仍需独立判档」）；同一文档 R-6 又引了近真 Q6 字面，出现两种「Q6 字面」 | major | **采纳并修**：三处归属改为「`G8_P2_DECISIONS.md` M75 行字面 / 0021 §2.4 行 123」，全文统一以 R-6 所引为 Q6 唯一字面（§1 摘要第 2 条、§2.1 M75 行、§4.C 末段实改） |
| F-2 | §6.4 M121/M122 脚本引用缺 `--phase g9.2`/`--phase g9.6` 双腿（`G9_ACCEPTANCE_MAP.md` M121/M122 行冻结为两条独立调用且 schema 强制 `phase_g9_2_pass` 与 `phase_g9_6_pass` 同真、任一阶段绿不替另一阶段） | major | **采纳并修**：§6.4 脚本列按 MAP 逐字补齐两条带 `--phase` 调用，并新增「双 phase 纪律」段登记 schema 双 phase 要求 |
| F-3 | §6.4 判据列弱于 MAP 已冻结判据（MAP M121 含「旁路写注入即 RED」「journal 全消费无损」「单向事实源纪律 0-byte」；M122 含「八枚举逐项 accept GREEN 与非法枚举 RED」「World-Field 唯一出口 = GpuScene 只读 buffer、渲染侧零回写断言全真」） | major | **采纳并修**（取推荐项）：§6.4 判据列**逐字照抄** `G9_ACCEPTANCE_MAP.md` M121/M122 行，并显式声明判据事实源 = MAP 对应行 |
| F-4 | §1.1 R-6 引 Q6 脱漏句首「提议」二字且 Q6 属未决提议项 | minor | **采纳并修**：R-6 补全「**提议**」句首字面，并标注 Q6 属 0021 的提议性未决项、非已裁决冻结句 |
| F-5 | §6.4「G9 三方（PLAN/矩阵/CONTRACT）」误述三向比对当事方——MAP §1 定义的三向是 ACCEPTANCE_MAP / G9_CONTRACT 验收章 / CI_GATES §4 | minor | **采纳并修**：§6.4 首段更正三向一致当事方 = `G9_ACCEPTANCE_MAP.md` / `G9_CONTRACT.md` 验收章 / `CI_GATES.md` §4（`ci/check_g9_acceptance_map.py` 强制） |
| F-6 | §2.1 M77 行「ApplyBuoyancyImpulse 未包装且无 gameplay 需求；联动 M49 defer」归属标错（实为 `G8_P2_DECISIONS.md` M77 行拼接；G8 矩阵 M77 行字面为「Jolt `ApplyBuoyancyImpulse` 原语未包装」） | minor | **采纳并修**：§2.1 M77 行字面归属改标 `G8_P2_DECISIONS.md` M77 行理由列，并并列登记 G8 矩阵 M77 行字面 |
| F-7 | §6.4 非 P0 面 M124/M125/M126 未像 0021 F-14 那样冻结波次 subject 字面；且 M124 判 go（P1，G9.6）与 MAP 已 go P1 空集之间的衔接未写 | minor | **采纳并修**：§6.4 补衔接句——「M124 go 的硬门落点走 G9.6 开工前 `G9_ACCEPTANCE_MAP.md` §1 只追加程序或波次聚合 subject（字面 G9.6 开工时冻结），当前不预造 key」 |

**偏差说明（如实登记）**：首选跨工具评审者在本环境不可得，本轮评审由同工具族独立实例执行（评审 provenance `Kimi Code CLI (Kimi) rfc0024-adversarial-reviewer` ≠ 起草 provenance `Kimi Code CLI (Kimi) rfc0024-drafter`），按 RFC-0015 §9.1 / `registry/number_ledger.json` revision_log v1.29 先例如实登记，不构成对 D-409 字面之外效力的声称。

## 10. 稳定化与 provenance

- RFC Approved 后仍须等 G9 实现互锁真实开放；批准本身不改变任何实现阻塞状态。
- 实现采用 schema-first/RED-first：结构化 schema + invalid/migration/replay fixture 先落，再实现；每个数字来自真实命令输出（`measured_local`）。
- 冻结面：§1.1 全部 🔒 修订行、§4.B Field 语义（含八枚举与过滤默认空匹配）、§4.C 通道边界与判档程序、§4.D「走 Field 通道」硬判据、NN 权威禁止线；feature 名即日冻结（R-7 🔒）。
- 起草 provenance：`Assisted-by: Kimi Code CLI (Kimi) rfc0024-drafter`。
- 未来评审 provenance 必须不同，并在 §9.1 逐条 disposition 后方可追加 Agent approval 修订行。

## 11. 规范与实现依据

- [RFC-0021](0021-physics-platform.md)（v1.3）：本文修订对象——replay-first 平台、capture/replay、§4.A4 Jolt 升级七步程序、§6.5/§6.5.1 冻结 bound 程序、Q6 异步判档留档、行 122 神经变形无归属留痕、五纪律逐字引用。
- [RFC-0017](0017-engine-physics.md)：Jolt 5.3 主物理、FFI/unsafe 边界、五条同步纪律、GPU 主刚体否决。
- [G9_PLAN](../milestones/g9/G9_PLAN.md)：G9 立项与 D5 模块定位；G9.0 不可变 ref = `1d9460a1`。
- [G9_CAPABILITY_MATRIX](../milestones/g9/G9_CAPABILITY_MATRIX.md) §5：M121~M127 行（承接锚、backfill 字面、P 级、拟承接波次）。
- [G9_D5_PHYSICS](../milestones/g9/design/G9_D5_PHYSICS.md)（v0.1，G9.0 冻结引用）：D5 设计事实源——§4.1~4.7 模块分解、§5 决策表 D5-1~D5-14、§9.1 RFC-0021 修订面清单（本文 §1.1 修订行的直接依据）。
- [R8_PHYSICS](../milestones/g9/research/R8_PHYSICS.md)：六条调研线索的正式化与来源落盘（Chaos Field 三层抽象 / 异步与确定性 / 浮力解析路径 / Jolt 5.6 / Rapier 2025–2026 / 神经变形）。
- [04_DESIGN_PRINCIPLES](../04_DESIGN_PRINCIPLES.md) P-01/P-05/P-09/P-11/P-12/P-13；[10_GOVERNANCE](../10_GOVERNANCE.md) §3/§7；[13_DECISION_LOG](../13_DECISION_LOG.md) D-406 v2.0 / D-409。

---

## 12. 章 F — G9.6 判档落定与 World-Field GpuScene 扩面修订行（G9.6 spec-first 修订增补）

> **增补性质**（修订行体例；先例 = RFC-0018 v1.1 §E 纯加性章增补 / RFC-0025 §4.L 对 G5 冻结面显式修订行）：本章为 **v1.1 纯加性增补**，§1~§11 既有冻结文本 **0-byte 不动**。兑现两处前置：① §9 Q1 字面「判档结论以本 RFC 修订行落定，并引判档证据」（M123 判档）；② R-10 🔒 字面「若实现期需要扩展 `GpuScene` 冻结面（新 buffer 面），必须经渲染侧 RFC（RFC-0019 面）的显式修订行，本 RFC 不隐式扩展 G5 渲染冻结面」（M121/M122 完整期 World-Field GpuScene 扩面）。裁定事实源 = G9_ACCEPTANCE_MAP v1.4 / G9_CANDIDATE_DECISIONS v1.5 / 树内 evidence 与 budget 实测。

### F1 🔒 M123 双通道判档落定（Q1 discharged：no-go 不充绿）

- **判档结论 = no-go，维持 M75 no-go 留档**（Q1「测量不足 → 维持 M75 no-go 留档」字面兑现；「异步调度须独立判档」为 `G8_P2_DECISIONS.md` M75 行理由列字面，Q6 唯一字面以 R-6 所引为准，均 0-byte）。
- **判档证据**（Q1「引判档证据」字面；2026-08-13 树内实测）：判档硬前置 = Jolt 单线程成本 measured——**树内零对应 measured artifact**：`evidence/` 物理相关件（G6 physics_core/physics_bridge/physics_rapier_parity、G8 M66/M67、G9.2 M121/M122 骨架 evidence）零 Jolt 单线程成本字段；`milestones/g9/g9_budget.json` 无物理段 counter（grep 实测零匹配）；测量任务 = D5 先行任务 P-6 / §6.3 步 1，归实现波真跑（measured_local 纪律，禁 estimated），G9.6 治理登记 + spec-first 波零 cargo 构建不产测量。**测量不足 → 判档不成立**。
- **生效面**（R-4/R-7 🔒 字面维持）：lockstep-deterministic 维持唯一通道；`physics-async-decorative` feature 与 `DecorativePhysicsTickId` 维持「仅 M123 判档 go 时生效」字面**不启用**；async-decorative 通道不建造。M123 不产 P0/P1 key、不充绿；**no-go 项不入 G9_ACCEPTANCE_MAP §3**（§3「no-go/defer 项不入本表」纪律）。
- **承接锚**：G9.7 P2 穷举（G9_PLAN §2 G9.7 候选行集已列「M123/M126（若判档不成立）」；`ci/g9_p2_decisions_check.py` 候选行含 M123）；实现波 Jolt 单线程成本 measured 数据落地后只追加重判，本行字面不改写。`registry/deferred.json` 0-byte（不新设 RD——承接锚为 G9.7 穷举既有候选行字面，沿 G9_CANDIDATE_DECISIONS v1.3/v1.4 先例）。
- **登记联动**：G9_CANDIDATE_DECISIONS v1.5 校准注（47 行裁决字面 0-byte）；G9_ACCEPTANCE_MAP §1 只追加 M123 no-go 登记句。

### F2 🔒 World-Field GpuScene 只读扩面显式修订行（RFC-0019 §8 面）

- **修订对象**：RFC-0019 §8 冻结面清单 `GpuScene` 项（G5 冻结面；RFC-0016 §4 语义面，RFC-0022 §8 / RFC-0025 §8 重申 0-byte）。
- **修订内容（加性，单行面）**：`GpuScene` 加性扩展 **World-Field 只读 buffer 面**——物理侧经既有 Physics→GpuScene 桥按 tick（`PhysicsTickId`）把场采样参数提交为 GpuScene 承载的只读 buffer；渲染/VFX/材质侧**只读消费**该 buffer 自行求值；**渲染侧零回写**（纪律 1 单向事实源 0-byte；旁路写/回写通道即 RED）；时间域归属 `WorldFieldSampleSet` → `RenderFrameId` 经 `FrameDomainMap` 显式映射（R-4 🔒 字面不变）。语义面条款化落 [spec/physics.md](../spec/physics.md) **RXS-0374**（G9.6 spec-first；字段级布局归实现波，本行只冻结面语义与纪律）。
- **0-byte 边界**：`GpuScene` 既有面（实例变换表/更新口 `update_transform`/`flush_dirty` 等）与 RFC-0019 §8 清单其余项（`MaterialClosure` 32B〔RFC-0025 §4.L 修订行面除外〕/Barrier EB 三轴/`PageRequest`/VisBuffer 位格式/物理五纪律）**字面 0-byte**；本行不授权任何既有字段重排/重释。
- **R-10 衔接声明**：R-10 🔒「本 RFC 不隐式扩展 G5 渲染冻结面」字面维持——本行为**显式修订行**而非隐式扩展，程序先例 = RFC-0025 §4.L（G9 波 Full RFC 承载对 G5 冻结面〔MaterialClosure 32B〕的显式 🔒 修订行）；「渲染侧 RFC（RFC-0019 面）的显式修订行」经本行对 RFC-0019 §8 `GpuScene` 项的逐字点名修订兑现。差距裁定：扩面 = 单个只读 buffer 面（与 RFC-0025 §4.L 单侧表通道同量级），判档不另起新 RFC（RFC next_free=26 维持不消费）。
- **RED 臂**（实现波接线）：渲染侧对 World-Field buffer 的任何写/回写通道注入即 RED；绕过 Physics→GpuScene 桥的旁路提交即 RED；骨架期「GpuScene 0-byte 扩面」断言面（`apps/g9-physics-gates` m122.rs 机验）在完整期改按本修订行面核验（world_field buffer 仅经本行授权面出现）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-09 | G9.1 governance-only 初稿：RFC-0021 修订 RFC——§1.1 十条 🔒 修订行（in-scope 承接三行、行 122 神经变形、Field 语义面追加、多时间域扩展、determinism 画像扩展、Q6 判档移交、feature 名追加、bound 程序沿用、NN 权威禁止线、G5 冻结面纪律）；§4 冻结 M121 particle view / M122 Field（八枚举+默认空匹配+persistent 全 journal）/ M123 双通道判档（硬前置=Jolt 单线程成本 measured）/ M124 解析浮力（走 Field 通道、禁旁路）/ M125 Jolt A/B（七步逐字、两臂诚实）/ M127 研究子轨边界；§6.4 冻结两道 P0 key/脚本/schema 路径（不建空壳）；§8 范围红线；§9.1 第 1 轮评审待进行。零 RXS/CI/RD/U/RX 数字 claim；Draft 不构成任何许可。起草 provenance `Kimi Code CLI (Kimi) rfc0024-drafter`。 | Full RFC（Draft） |
| v1.0 | 2026-08-09 | **Agent Approved**：D-409 第 1 轮独立实例对抗性评审（评审 provenance `Kimi Code CLI (Kimi) rfc0024-adversarial-reviewer` ≠ 起草 `Kimi Code CLI (Kimi) rfc0024-drafter`；同工具族偏差按 RFC-0015 §9.1 / number_ledger v1.29 先例如实登记于 §9.1）完成，7 findings（3 major + 4 minor）全部**采纳并修**：F-1 Q6 字面归属统一（「异步调度须独立判档」= `G8_P2_DECISIONS.md` M75 行字面 / 0021 §2.4 行 123；Q6 唯一字面以 R-6 所引为准，三处实改）、F-2 §6.4 补 `--phase g9.2`/`--phase g9.6` 双腿与双 phase schema 要求、F-3 §6.4 判据逐字照抄 `G9_ACCEPTANCE_MAP.md` M121/M122 行并声明判据事实源、F-4 R-6 补全 Q6「提议」句首并标注提议性质、F-5 三向比对当事方更正为 ACCEPTANCE_MAP/G9_CONTRACT 验收章/CI_GATES §4、F-6 §2.1 M77 行字面归属改标 `G8_P2_DECISIONS.md` M77 行、F-7 §6.4 补 M124 已 go P1 衔接句（G9.6 开工前 MAP §1 只追加程序或波次聚合 subject，不预造 key）。零 RXS/CI/RD/U/RX 数字 claim；批准不解锁实现。 | Full RFC（Agent Approved） |
| v1.1 | 2026-08-13 | **增补 §12（章 F）：G9.6 判档落定与 World-Field GpuScene 扩面修订行（G9.6 spec-first，纯加性，§1~§11 既有冻结文本 0-byte）**——F1 🔒 M123 Q1 判档落定：**no-go 不充绿**（判档硬前置 Jolt 单线程成本 measured 未满足——树内零 measured artifact：evidence/ 物理件零单线程成本字段、g9_budget.json 无物理段 counter；测量归实现波 D5 先行任务 P-6/§6.3 步 1，治理/spec 波零 cargo 构建不产测量，禁 estimated；维持 M75 no-go 留档，`physics-async-decorative`/`DecorativePhysicsTickId` 维持「仅判档 go 时生效」字面不启用；承接锚 G9.7 穷举；deferred.json 0-byte 不新设 RD；联动 G9_CANDIDATE_DECISIONS v1.5 / G9_ACCEPTANCE_MAP v1.4）；F2 🔒 World-Field GpuScene 只读扩面显式修订行（RFC-0019 §8 `GpuScene` 冻结面加性扩展 World-Field 只读 buffer 面——按 tick 经 Physics→GpuScene 桥提交、渲染侧只读消费零回写、旁路写/旁路提交即 RED、时间域 R-4 字面不变；语义面条款化落 spec/physics.md RXS-0374；GpuScene 既有面与 §8 清单其余项 0-byte；R-10「不隐式扩展」字面维持——本行为显式修订行，程序先例 RFC-0025 §4.L；差距裁定 = 单个只读 buffer 面同 §4.L 量级，不另起新 RFC，RFC next_free=26 维持不消费）。零 RXS/CI/RD/U/RX/MR/D 数字 claim 随本章（spec-first 条款批 RXS-0374~0379 消费登记于 number_ledger v1.93，随 spec PR 同批落）；本章不构成任何实现许可。 | Full RFC（Agent Approved 增补） |
