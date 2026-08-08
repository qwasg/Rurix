# G9-D5 — 物理引擎建造期设计草案

> **DRAFT 设计提案——G9 未立项，不构成契约/验收承诺。**
> 本文仅为 G9 立项时的设计输入；所有裁决建议须经 G9 立项程序与 RFC 修订才生效。
> 版本：v0.1 · 日期：2026-08（G8 收口后）· 依据：G8_PLAN §2 G8.6、G8_P2_DECISIONS（M74/M75/M77/M65b 行）、RFC-0021（v1.3）、调研 R-D5（UE Chaos Field / 异步物理 / 浮力 / Jolt 5.6 / Rapier 2025-2026 / 神经变形）。
> **G9.0 冻结引用**：2026-08-08 起，本文作为 G9.0 文档集不可变基线附件被 [G9_PLAN.md](../G9_PLAN.md) 冻结引用；正文 0-byte，后续变更只追加修订记录（追加于文末）。

---

## 1. 定位与承接锚

**D5 = G9 内「UE5 级物理引擎正式建造期」模块。** G8.6 交付的是 replay-first 物理平台底座（capture/replay corpus、网络回滚、破坏生产链、CharacterVirtual、PhysicsAsset/ragdoll、fracture、布料、载具六真腿、Physics→GpuScene 桥）；D5 在其上建造 G8 明确 deferred/no-go 留档的四个面与升级面。

**法定承接锚（G8_P2_DECISIONS 字面）：**

| 锚 | G8 终态 | 字面依据 | D5 处置方向 |
|---|---|---|---|
| **M74 Physics Field** | defer-to-G9+，目标 =「G9+ gameplay Field」 | 「M68 damage/field journal 已覆盖 G8 最小面；统一 Field 属建造期」（G8_P2_DECISIONS M74 行；RFC-0021；矩阵 M74） | D5 主线交付物（§4.2） |
| **M75 异步物理 tick** | no-go，open-留档 | 「本期只冻结时间域 identity；异步调度须独立判档」（RFC-0021 Q6；矩阵 M75） | D5 承担独立判档：双通道确定性架构（§4.3） |
| **M77 水体/浮力** | no-go，open-留档 | 「ApplyBuoyancyImpulse 未包装且无 gameplay 需求；联动 M49 defer」（矩阵 M77） | D5 交付解析浮力模型，走 Field 通道（§4.4）；M49 联动重新评估 |
| **M65b Rapier 深造** | no-go，open-留档 | RD-044「快路径被真实 workload 采用时」（registry/deferred.json；矩阵 M65b） | D5 建造期 workload 为首个真实候选消费方；深造以对标基准先行（§4.6） |

**研究轨承接（不进主线交付物）：**

- 神经变形 → G9+ 研究轨（RFC-0021 §2.4 行 122：「无 RD 归属，属 G9+ 研究轨」）→ D5 内挂研究子轨 §4.7，**不占主线验收门**。
- 可微物理 → RD-042 观察维持，**不进 D5**（RFC-0021 §2.4 F-18 归属）。
- GPU 主刚体禁止线维持（G6 裁决 + RFC-0017；矩阵 §12；R-G8-7）。

**既有基础（G8.6 交付物，D5 直接复用，不得重建）：**

- Jolt 5.3 capture/replay corpus（`g8.p0.m66.physics_replay`，`src/rurix-physics/src/capture/`：header/journal/canonical/divergence/inject）；
- 恢复层 = `semantic_journal_rebuild_v1`（RFC-0021 §2.1.1.1 全线 (c)）；
- 网络回滚链（M67，`src/rurix-physics/src/net/`）、`RurixCharacter`（M71）、PhysicsAsset/ragdoll（M69）；
- 破坏生产链（M68，`destruction/`：cook/runtime/cache/vfx，damage/field journal 最小面）；
- 自有 XPBD 布料（M72）、自研悬挂载具（M70）；
- Physics→GpuScene 单向变换桥（RFC-0017 纪律 1）、五时间域 identity（`PhysicsTickId`/`NetworkPhysicsFrameId`/`ClothTickId`/`GameFrameId`/`RenderFrameId`，RFC-0021 §3.3）。

**先兆线索：** 工作树未提交的 `src/rurix-physics-sys/tools/layout_hinge.cpp` / `layout_hinge2.cpp` 是 JoltC `JPC_HingeConstraintSettings` 布局探针（含 `MotorSettings`、`LimitsSpring`、`MaxFrictionTorque`、per-constraint `NumVelocityStepsOverride` 偏移审计）——方向与 D5 一致：更复杂的 constraint/motor 组合（Field 驱动的 joint 目标、ragdoll `DriveToPoseUsingMotors` 对照面、Jolt 5.6 新摩擦模型迁移时的 layout 复核）将持续需要此类 build 期 `LAYOUT_COMPATIBLE` 探针。D5 应把「JoltC 布局探针」从临时工具升级为 vendor 升级检查单的固定一项（§4.5）。

---

## 2. 范围 in/out

### 2.1 In-scope

| 能力 | 锚 | 内容 | 性质 |
|---|---|---|---|
| 统一 physics particle view | M74 前置抽象 | 刚体/布料顶点/碎块 chunk/ragdoll 节点的统一可寻址视图（对标 Chaos Particles，调研 1） | 主线 |
| Gameplay Field 系统 | M74 | 场定义/作用对象/目标语义三层解耦 + 三生命周期 + 一等过滤 + World-Field 通道 | 主线 |
| 双通道确定性架构 | M75 独立判档 | lockstep-deterministic 通道 vs async-decorative 通道 + `deterministic_profile` 运行时断言 | 主线（判档后定档） |
| 解析浮力模型 | M77 | 浸入体积/浸没质心解析近似 → buoyancy + buoyancy torque + drag impulse，走 Field 通道 | 主线 |
| Jolt 5.3→5.6 升级评估 | G8.6a 纪律延续 | corpus A/B；新摩擦模型/HeightField 16bit/glTF 物理马达/DriveToPoseUsingMotors | 主线（评估门） |
| Rapier 深造基准 | M65b | 新 Dynamic BVH / sparse voxel collider / persistent islands / manifold ≤4 / glam 迁移对标 | 基准先行，深造仍条件制 |
| 神经变形研究轨 | RFC-0021 G9+ 研究轨行 | 混合架构优先；骨骼驱动神经服装离线工具链；PhysicsAsset residual 通道预留 | 研究子轨，无主线门 |

### 2.2 Out-of-scope（红线写明）

- **GPU 主刚体禁止线维持**（G6 裁决、RFC-0017、矩阵 §12）：GPU 不承载任何权威刚体求解，包括「经预算隔离的可选副求解器」（RFC-0021 §1 F-06 字面延续）；任何 GPU 刚体提案须 RD-043 触发 + 矩阵 §12 五项重审 + 独立 Full RFC。Jolt 5.6 的 GPU compute shader 接口在 D5 **只许评估、不许接入权威求解**。
- **可微物理排除**（RD-042 观察维持，RFC-0021 §2.4）：Warp/MJX/Newton 类机器人训练回路不进 G9。
- 真双向流体-刚体耦合（FLIP/MPM/SPH）：CGF 2025 STAR 判定仍研究级、无商业 gameplay-critical 使用（调研 3）→ 不进主线；浮力只做解析近似。
- 通用软体/Flesh/MPM/FLIP：RD-044 Continuum/Fluid 观察面维持。
- 跨平台/跨编译器 bitwise lockstep：RFC-0021 §2.4 放弃口径延续；async-decorative 通道本就不承诺确定性。
- 编辑器 GUI、通用 node graph 编辑器、cache farm。
- 神经变形不作为主线验收项；不承诺碰撞保证、不承诺双向耦合（调研 6 共性短板）。
- 不改写 RFC-0017 五纪律、G6/G8 closed 契约字面、G5 渲染冻结面。

---

## 3. 依赖前置

| # | 前置 | 来源 | 说明 |
|---|---|---|---|
| P-1 | G9 立项完成（契约 + 编号校准） | 10_GOVERNANCE | 本文一切裁决建议生效的硬前置 |
| P-2 | RFC-0021 修订（§9 列面） | D5 首个 RFC 工作项 | Field 语义、异步判档、浮力、5.6 采纳面必须先进 RFC |
| P-3 | `g8.p0.m66.physics_replay` 全绿 corpus 可复用 | G8.6a 已交付 | 所有 D5 新面的确定性断言挂接点 |
| P-4 | Physics→GpuScene 桥 + `GpuScene` 冻结面只读 | RFC-0017 纪律 1 / G5 | World-Field 通道 GPU 求值的唯一合法出口 |
| P-5 | 五时间域 identity + `FrameDomainMap` | RFC-0021 §3.3 | Field/浮力/异步通道必须显式挂域，禁止隐式「当前帧」 |
| P-6 | Jolt 单线程成本测量 | D5 内先行任务 | 异步化范围决策的实测输入（调研 2：「先测 Jolt 单线程成本再定」） |
| P-7 | M68 damage/field journal 最小面 | G8.6c 已交付 | Field 系统求值管线的雏形与兼容约束 |
| P-8 | JoltC 布局探针工具化 | 工作树 layout_hinge 先兆 | vendor 升级/新 FFI 的固定检查项 |

---

## 4. 模块分解

### 4.1 统一 physics particle view（D5-PV）

**依据**：UE Chaos Field System 的作用对象层是唯一大规模生产验证的统一抽象——刚体/布料顶点/碎块/RBA 节点统一为 Chaos Particles（调研 1）。

**设计**：

- `PhysicsParticleRef` = `(domain, stable_id, element_index)` 名义类型：`domain ∈ {RigidBody, ClothVertex, DestructionChunk, RagdollNode, CharacterInner}`；`stable_id` 复用 generation 语义（RFC-0021 §3.4），绝不暴露 arena index。
- 每个域实现 `ParticleAdapter` trait：`mass() / position() / velocity() / set_force_impulse() / sleep_state()`；写路径只允许 impulse/force 语义，**不允许直接改写 transform**（纪律 1 单向事实源不变——桥仍只读已提交变换）。
- 视图是 Field 求值的唯一作用对象面；destruction damage journal（M68 最小面）迁移为该视图的第一个 consumer，保持 journal 兼容（迁移器 + golden）。
- 视图只覆盖 CPU 权威世界内的对象；GPU 副轨粒子不进入该抽象。

### 4.2 Gameplay Field 系统（M74 主线交付）

**依据**：Chaos Field System 三层解耦（调研 1）。

**场定义层（Field Nodes）**：场 = 空间标量/向量函数 + 元数据。基元集：radial falloff / box / sphere / noise / curve-driven / analytic-surface（为浮力水面函数预留，§4.4）；节点图组合，但**图 schema 版本化、canonical 序列化、cook 确定性**（承 §5 共同头纪律）；首期不接通用可视化 node graph 编辑器（out-of-scope）。

**作用对象层**：经 D5-PV 的 `PhysicsParticleRef` 寻址（§4.1）。

**目标语义层 `FieldPhysicsType`**（对标 `EFieldPhysicsType`）：首期枚举 `LinearForce / Strain / Velocity / Torque / Sleeping / Disabled / CollisionGroup / Buoyancy`；`Buoyancy` 是 RuriX 相对 Chaos 的加性扩展（M77 共用求值管线，见 §4.4）。

**三生命周期**：

| 生命周期 | 语义 | 确定性规则 |
|---|---|---|
| Transient | 单 tick 内求值即弃 | 不进 journal，结果经命令规范化进 journal |
| Construction | cook/关卡构建期烘焙（如 anchor strain 预置） | 进 cooked artifact digest |
| **Persistent** | 跨 tick 存活 | **必须可显式注销**（调研 1：persistent 显式注销保持 replay 确定性）；注册/注销/参数变更全部写 command journal，参与 `semantic_state_hash` |

**过滤一等公民**（调研 1 核心教训：「场默认影响所有 Actor 是 UE 社区大坑」）：`FieldFilter = (object_state_mask × domain_mask × layer_mask × explicit_include/exclude)`；**默认空集匹配 = 无影响**，拒绝「默认全影响」语义；filter 是场定义的一部分，进 digest。

**World-Field 通道（物理→VFX 反向通信）**：渲染/VFX 采样物理场的只读出口。GPU 求值**唯一合法路径 = Physics→GpuScene 桥**：场采样参数按 tick 提交为 GpuScene 只读 buffer（纪律 1 不变：物理→渲染单向；渲染不回写物理）；VFX/材质在 GPU 侧消费该 buffer 自行求值。新增 `WorldFieldSampleSet` 时间域归属 `RenderFrameId`，经 `FrameDomainMap` 显式映射（P-5）。

### 4.3 双通道确定性架构（M75 独立判档建议稿）

**依据**（调研 2）：固定步长是确定性的必要非充分条件——还须 substepping off、锁死 dt、关 sleep/BVH-refit 等 Enhanced Determinism 项，否则回放必分叉（Bugnet 2026 UE 事故）；异步线程与 lockstep 根本冲突；生产实践 = 双通道。

**通道划分**：

| 通道 | 内容 | 确定性承诺 | 调度 |
|---|---|---|---|
| **lockstep-deterministic** | gameplay-critical 刚体、Character、ragdoll、destruction cluster、vehicle、cloth 权威求解、Field（persistent/含 gameplay 语义）、浮力 | 承 RFC-0021 全集：capture/replay、逐 tick hash、网络 rollback | 维持固定步 `PhysicsTickId` 主线，**不变** |
| **async-decorative** | 碎块视觉、表现层布料副轨、次级刚体、VFX 联动粒子 | **无**；只接收单向力/事件，**绝不回写 gameplay 状态**；对象可丢弃可重建 | 独立线程，自有 tick identity（新 `DecorativePhysicsTickId`，禁止冒充 `PhysicsTickId`） |

**硬边界**：async-decorative → lockstep 方向零写路径（类型系统层无 API）；lockstep → async 单向经事件队列（`PhysicsEventId` 去重语义无需求，装饰事件允许丢弃）。

**`deterministic_profile` 运行时断言**（调研 2：确定性打包为画像断言）：启动与 corpus 运行前断言 `fixed_dt 锁死 && substepping==off && sleep_policy==profile值 && job_threads 与画像一致 && bvh_refit_policy==profile值`；任一不符 fail-closed。该画像并入 capture header 的 determinism 画像（RFC-0021 §4.A1 已有画像面，D5 扩展字段须走修订）。

**异步化范围决策程序**：先测 Jolt 单线程成本（P-6），只有 measured 证据显示主线程物理超预算才把装饰对象迁入 async 通道；lockstep 通道**永不**异步化（与 rollback/corpus 根本冲突）。本判档结论须以 RFC-0021 Q6 修订行落定。

### 4.4 解析浮力模型（M77）

**依据**（调研 3）：生产路径 = 解析近似（浸入体积/浸没质心 → 浮力 + 浮力矩 + 水阻力 impulse；Eurographics 2020 模型在细长/翻滚物体上稳定性优于 UE 采样点 Pontoon 方案）；真双向耦合研究级，排除（§2.2）。

**设计**：

- **走 Field 通道**（调研 3：浮力应走 Field 通道，与 M74 共用求值管线）：水体区域 = persistent field（解析水面函数为场定义的 `analytic-surface` 基元）；`FieldPhysicsType::Buoyancy` 语义。水体是 Field 统一抽象的**第二个真实用户**（第一个是 destruction damage），这是对 M74 抽象正确性的生产检验。
- 求值：每 tick 对落入 filter 的 `PhysicsParticleRef`（首期 RigidBody 域）计算 clipped 浸入体积与浸没质心 → `buoyancy impulse + buoyancy torque + linear/angular drag impulse`，经 `AddForceAtPoint` 类既有 FFI 施加（消费既有导出符号纪律不变）。
- **确定性内置**：固定 dt + 解析水面函数（禁帧率相关插值、禁墙钟相位）；全部输入/输出进 command journal；fixture 入 capture/replay corpus（P-3 挂接点）。
- 形状支持：首期 convex/primitive 解析 clip；任意 mesh 走离线预计算 voxelized volume table（cooked artifact，版本化）。
- M49（Taichi）联动：维持 defer——Taichi AOT 只产出粒子/体积场（纪律 4），水面视觉可消费 World-Field 通道，但权威浮力不经 Taichi。

### 4.5 Jolt 5.3→5.6 升级 A/B（G8.6a 纪律的建造期延续）

**依据**（调研 4）：Jolt 5.6（2026-07）重大版本——GPU compute shader 接口、GPU strand 毛发、**新摩擦模型**（平均接触点：快 15%/省 40% 内存/消除接触点序偏向——对确定性 corpus 有直接价值）、HeightField 16bit、glTF `KHR_physics_rigid_bodies` 马达、`Ragdoll::DriveToPoseUsingMotors`、整场景 up to 40% 提速/70% 省内存。

**纪律承 G8.6a**：「corpus 先在当前版本建成再评估升级」——G8 已建成，D5 可正式评估 5.3→5.6 A/B，程序逐字承 RFC-0021 §4.A4 七步（冻结 5.3 基线 → 独立 vendor/ABI → 各自同版本 replay → canonical A/B → 实测阈值 → 失败臂钉 5.3 → 采纳臂三件事：corpus 显式迁移 + replay 门在新版本重跑 + 判据字面经修订后才改版本号）。G9 契约判据字面若再钉「Jolt 5.3」，同样须修订后才可改字面。

**分项处置表**：

| 5.6 特性 | D5 处置 |
|---|---|
| 新摩擦模型（平均接触点） | A/B 重点项：消除接触点序偏向对 corpus 有直接价值；但求解器语义变化 → 逐字段 exact/tolerance/invariant 分类（§4.A4 程序） |
| GPU compute shader 接口 | **只评估不接权威**（GPU 主刚体禁止线 §2.2）；评估报告留档，接入须 RD-043 + 矩阵 §12 + 独立 Full RFC |
| GPU strand 毛发（Cosserat 杆） | 非权威装饰副轨候选（async-decorative 通道），非 D5 主线 |
| HeightField 16bit | 与流送/地形页联动评估，独立分项判档 |
| glTF KHR_physics_rigid_bodies 马达 | 资产管线候选增强，进 RFC-0020 面而非本 RFC |
| `Ragdoll::DriveToPoseUsingMotors` | 与 G8 M69 约束五件套路线对照评估；采纳需 JoltC C 面审计（当前 pin 无该导出） |
| layout 审计 | layout_hinge 类探针扩展为 5.6 升级检查单固定项（P-8）：所有 `*Settings` 结构 sizeof/offsetof 静态断言重跑 |

### 4.6 Rapier 深造基准（M65b）

**依据**（调研 5）：Rapier 2025-2026——新 Dynamic BVH、sparse voxel collider、persistent islands、manifold ≤4 接触、大堆叠 25% 提速、0.32 起 nalgebra→glam 迁移（动因：rust-gpu 一等支持）、GPU 路线定锚 rust-gpu。**战略信号：Dimforge 定锚 rust-gpu 验证 RuriX「Rust→GPU」路线的行业合理性**（只作路线佐证留档，不构成任何接入承诺）。

**处置**：

1. **对标基准先行**：新 BVH / voxel collider / 摩擦模型 / 大堆叠场景建 A/B benchmark 夹具（与 Jolt 同场景、同输入、同 determinism 画像）；产出 measured 报告，**不作 replay oracle**（RFC-0021 §7 备选 D：跨 solver 不承诺逐位，只作不变量/容差对拍）。
2. **RD-044 字面不变**：「快路径被真实 workload 采用时」才深造。D5 建造期 workload（Field 高频查询、async-decorative 次级刚体）是首个真实候选消费方；若基准显示 Rapier 在该面有 measured 优势，再按 RD-044 程序申请判档，否则维持 no-go 留档。
3. **glam 迁移兼容**：Rapier 0.32+ glam 化对 `src/rurix-physics/src/rapier.rs` 快路径封装的 API 冲击评估，兼容层设计留档；不承诺 bitwise 变化。
4. GPU 路线：Rapier 的 rust-gpu 刚体方向在 RuriX 同样受 GPU 主刚体禁止线约束，仅观察。

### 4.7 神经变形研究轨（不占主线验收门）

**依据**（调研 6）：PhySkin/HyperBones/UNIC（2026 骨骼驱动神经服装实时）、NeuralClothSim（NeurIPS 2024）、Subspace Neural Physics（300-5000× 加速）、Hybrid Neural-MPM（2025，NN 替换求解器最贵一步、物理骨架兜底——**与 RuriX 哲学同构，风险最低**）；共性短板 = 训练语料依赖/泛化退化/无碰撞保证/无法双向耦合。

**子轨设计**：

- **混合架构优先**（Hybrid Neural-MPM 同构）：NN 只做加速近似，权威物理骨架兜底；任何 NN 输出不得替代权威状态（与 GPU 禁止线同构的「NN 权威禁止线」建议——进 RFC 修订面 §9）。
- **骨骼驱动神经服装先做离线工具链形态**：离线仿真 → 神经压缩 → 运行时回放，**复用 G8 capture/replay 设施**（corpus 即训练语料生产器，replay 设施即回放校验器——D5 与 G8 设施的最大协同点）。
- **PhysicsAsset 变形格式预留 residual 通道**：schema 加性扩展位（版本化，首期字段可空），避免日后 breaking 修订。
- 产出形态：研究报告 + 离线工具链原型 + PhysicsAsset schema 加性修订建议；**无主线 gate，无 P0/P1 判据**，不进 G9 收口硬门。
- 可微物理不进本子轨（§2.2）。

---

## 5. 关键设计决策表（含承接锚处置裁决建议）

| # | 决策 | 建议 | 依据 | 裁决程序 |
|---|---|---|---|---|
| D5-1 | M74 处置 | **go（G9 主线）**：统一 Field 系统按 §4.2 三层 + 三生命周期 + 一等过滤 + World-Field 通道建造 | G8 defer 字面「统一 Field 属建造期」；调研 1（Chaos 唯一大规模生产验证抽象） | G9 契约 + RFC-0021 修订 |
| D5-2 | Field 过滤默认语义 | **默认无影响（空集匹配）**，显式 opt-in | 调研 1：「场默认影响所有 Actor 是 UE 社区大坑，过滤必须一等公民」 | RFC-0021 修订冻结 |
| D5-3 | Persistent field 确定性 | 注册/注销/变更全进 command journal，可显式注销，参与 semantic hash | 调研 1：persistent 显式注销保持 replay 确定性 | RFC-0021 修订冻结 |
| D5-4 | M75 处置 | **有条件 go**：双通道架构（§4.3）；lockstep 通道永不异步化；async-decorative 零回写；采纳与否以 P-6 单线程成本测量为前置 | G8 no-go「异步调度须独立判档」（RFC-0021 Q6）；调研 2 | 判档报告 + RFC-0021 Q6 修订行；测量不足则维持 no-go |
| D5-5 | 确定性打包 | `deterministic_profile` 运行时断言（固定 dt + substep off + sleep/BVH 策略钉死），并入 capture header determinism 画像 | 调研 2：固定步长必要非充分（Bugnet 2026 事故） | RFC-0021 §4.A1 画像扩展修订 |
| D5-6 | M77 处置 | **go**：解析浮力模型，走 Field 通道（persistent field + `Buoyancy` 语义），确定性内置入 corpus | G8 no-go「ApplyBuoyancyImpulse 未包装」；调研 3（Eurographics 2020 优于 Pontoon 采样点） | G9 契约 + RFC-0021 修订 |
| D5-7 | 真双向流体耦合 | **排除主线** | 调研 3：CGF 2025 STAR 仍研究级，无商业 gameplay-critical 使用 | 本设计即裁决建议，契约写明 |
| D5-8 | M65b 处置 | **维持条件制**：对标基准先行（§4.6）；RD-044 字面不变；仅当 D5 真实 workload + measured 优势成立才申请深造判档 | G8 no-go；调研 5 | 基准报告 → RD-044 程序 |
| D5-9 | Jolt 5.6 | **评估门**：按 RFC-0021 §4.A4 七步程序 A/B；新摩擦模型为重点；GPU compute 接口只评估不接权威 | 调研 4；G8.6a 纪律（corpus 已建成，评估窗开启） | 升级判档 + RFC/契约字面修订 |
| D5-10 | GPU 主刚体 | **禁止线维持，全文 0-byte** | G6 裁决；RFC-0017；矩阵 §12；R-G8-7 | 无需裁决，重申 |
| D5-11 | 可微物理 | **排除，RD-042 观察维持** | RFC-0021 §2.4（F-18 归属）；调研 6（价值在机器人训练回路） | 无需裁决，重申 |
| D5-12 | 神经变形 | **研究子轨**：混合架构优先、离线工具链形态、PhysicsAsset residual 通道预留；不占主线门 | RFC-0021 §2.4 G9+ 研究轨行；调研 6 | G9 契约登记子轨，成果另行判档 |
| D5-13 | World-Field GPU 求值 | 唯一路径 = Physics→GpuScene 桥只读 buffer；渲染侧自求值，不回写 | RFC-0017 纪律 1（0-byte）+ 调研 1（物理→VFX 反向通信） | RFC-0021 修订冻结 |
| D5-14 | layout 探针工具化 | layout_hinge 类探针升级为 vendor 升级/新 FFI 固定检查项 | 工作树先兆线索；JoltC ABI 缺口审计先例（VENDOR.md §3） | 工程纪律条目（14 章面） |

---

## 6. 波次建议

```text
D5-W0 判档与治理
  → RFC-0021 修订（§9 列面）+ G9 契约登记 + P-6 Jolt 单线程成本测量
D5-W1 统一 physics particle view（D5-PV）
  → M68 damage/field journal 迁移为首个 consumer（兼容迁移器 + golden）
D5-W2 Field 系统核心
  → 定义层基元/schema → 语义层 → 生命周期/journal → 过滤 → corpus 断言
D5-W3 浮力（消费 W1/W2）
  → 解析模型 → Field 通道接入 → corpus fixture
D5-W4 World-Field 通道 + 双通道架构（∥ W3）
  → GpuScene 只读 buffer 出口；deterministic_profile 断言；async-decorative 通道（依 D5-W0 判档结论启停）
D5-W5 Jolt 5.3→5.6 A/B（∥ W3/W4，独立 vendor 线）
  → 采纳臂/失败臂按 §4.A4 七步
D5-W6 Rapier 对标基准（∥，基准专用线）
  → 报告 → RD-044 判档申请（若成立）
D5-RT 神经变形研究子轨（全程伴随，无硬门）
  → 离线工具链原型 + PhysicsAsset residual schema 建议
```

依赖纪律：W2/W3 不得反向要求改写 W1 视图或 M66 capture 格式而不经显式 schema migration（承 RFC-0021 §6.3）；W5 失败不影响 W2~W4 在 5.3 上继续。

---

## 7. 验收门草案

> 全部门为草案；symbolic key、evidence schema 名、数字阈值均待 G9 立项后按 actual next-free 校准，本文不猜 CI 步骤号（承 RFC-0021 §6.4 纪律）。

| 门（草案 key） | 独立硬判据 | evidence schema（草案名） |
|---|---|---|
| `g9.d5.particle_view` | 五域 adapter 全实现；M68 damage journal 经迁移器无损迁移（迁移前后 digest 一致 + golden 对拍） | `g9_d5_particle_view_evidence` |
| `g9.d5.field_system` | ① 三层 schema canonical roundtrip；② persistent field 注册/注销/变更全 journal 化且 replay 逐 tick hash 一致；③ 过滤负例：默认空匹配 = 零影响（显式断言）；④ Transient/Construction/Persistent 三生命周期语义各自独立断言 | `g9_d5_field_system_evidence` |
| `g9.d5.determinism_profile` | profile 断言在 corpus 全 fixture 前置通过；**负例 RED 臂**：变步长运行、可变 substep 运行、画像外 job_threads 运行**必须被 corpus 拒绝**（fail-closed 断言三个独立布尔） | `g9_d5_determinism_evidence` |
| `g9.d5.buoyancy` | ① 解析模型 fixture（含细长/翻滚物体回归，对标 Eurographics 2020 稳定性场景）；② 走 Field 通道求值（非旁路 API）；③ capture→replay 逐 tick hash 一致；④ 禁帧率相关插值断言（变帧率输入同 tick 结果逐位一致） | `g9_d5_buoyancy_evidence` |
| `g9.d5.world_field` | 场采样经 GpuScene 只读 buffer 提交；渲染侧零回写断言；`FrameDomainMap` 映射完整 | `g9_d5_world_field_evidence` |
| `g9.d5.async_channel`（判档 go 才启用） | async-decorative 对象零回写（类型层 + 运行时不变量双重断言）；lockstep 通道 hash 在 async 通道启/停两态下逐位一致；装饰对象可丢弃重建 | `g9_d5_async_channel_evidence` |
| `g9.d5.jolt_ab` | 波次聚合 subject（不产独立 PASS，承 RFC-0021 §6.4 F-14 模式）：七步程序记录完整；采纳臂三件事/失败臂钉版证据 | `g9_d5_jolt_ab_evidence` |
| `g9.d5.rapier_benchmark` | 波次聚合 subject：基准报告 + RD-044 判档申请/维持记录 | `g9_d5_rapier_bench_evidence` |

**通用断言纪律**（承 G8）：spec-first + RED 先行；schema + invalid/migration/replay fixture 先于实现；数字阈值只从 `measured_local` 写入，本文零预造数字；确定性 replay 对拍一律挂接 M66 corpus 设施（P-3）。

---

## 8. 风险与止损

| 风险 | 预警 | 止损 |
|---|---|---|
| Field 抽象过度设计（Chaos 全集照搬） | 语义枚举膨胀、无真实 consumer | 只有两个真实用户（destruction damage + 浮力）验证后才扩枚举；首期枚举冻结为 §4.2 八项 |
| Persistent field 破坏 replay 确定性 | replay hash 分叉 | 注册/注销/变更全 journal 化是硬门；分叉即门红，禁止放宽 hash |
| 双通道边界渗漏（装饰物理回写 gameplay） | lockstep hash 受 async 态影响 | 类型层无写路径 + 运行时不变量双断言；渗漏即通道关闭回退单线程 |
| 异步化范围误判 | 无 measured 依据就拆通道 | P-6 测量是 D5-4 判档硬前置；测量不足维持 M75 no-go |
| 浮力走旁路 API 绕开 Field | 出现第二套空间影响管线 | 验收门硬判据「走 Field 通道」；旁路即红 |
| Jolt 5.6 升级倒置重演 | corpus 未绿先换 vendor、改 golden 迎合新 solver | 承 RFC-0021 §4.A4：恢复 5.3 基线、判失败钉版、禁写 5.6 PASS |
| 5.6 GPU compute 接口诱惑越线 | 提案绕过 RD-043/矩阵 §12 | 禁止线重申（D5-10）；越线提案直接拒绝 |
| Rapier 基准被当 replay oracle | 用跨 solver 容差充 Jolt correctness | RFC-0021 §7 备选 D 字面：只作不变量/容差对拍 |
| 神经变形挤占主线资源 | 研究子轨产出被记主线绿 | 子轨无主线 gate；成果须另行判档，不进 G9 收口硬门 |
| layout 探针腐蚀（临时工具失维护） | 升级时 layout 断言缺失 | 工具化进 vendor 检查单（D5-14），探针源码入库不再散落工作树 |

---

## 9. spec/RFC 需求

### 9.1 RFC-0021 修订面（D5 首个 RFC 工作项，全部走加性修订行）

| 面 | 修订内容 |
|---|---|
| §2.3 in-scope 表 | 追加 M74（Field 系统）、M75（双通道判档）、M77（浮力）三行，标 G9 波次 |
| §2.4 out-of-scope | 神经变形行改写为「G9 D5 研究子轨承接，无主线门」；GPU 主刚体、可微物理两行字面 0-byte 重申 |
| 新增 Field 语义节 | 三层解耦（定义/作用对象/目标语义）、`FieldPhysicsType` 首期枚举、三生命周期（persistent 显式注销）、过滤默认空匹配语义、World-Field 通道（GpuScene 只读 buffer 面）——以上均即批准即冻结 |
| §3.3 多时间域 | 追加 `DecorativePhysicsTickId`（判档 go 时）；World-Field 采样的域映射规则 |
| §4.A1 determinism 画像 | 扩展 `deterministic_profile` 字段（substepping/sleep/BVH 策略钉值）+ 负例 RED 臂要求 |
| §9 Q6 行 | 改写为「D5 已判档：双通道（或维持 no-go）」+ 判档证据引用 |
| §6.2 feature 名 | 追加 `physics-field` / `physics-buoyancy` /（判档 go 时）`physics-async-decorative`；名称即日冻结 |
| §6.5 冻结 bound 程序 | 浮力模型的数值 bound（若需）沿用三步程序 |
| NN 权威禁止线（新） | 神经变形子轨的「NN 输出不得替代权威状态」约束，与 GPU 禁止线同构登记 |

### 9.2 其他 RFC

- **RFC-0020（资产管线）**：Field schema、水体资产、PhysicsAsset residual 通道（加性）、glTF `KHR_physics_rigid_bodies` 马达（若采纳）的 schema/版本化面。
- **RFC-0019（渲染）**：World-Field 只读 buffer 若触 `GpuScene` 冻结面扩展，必须显式修订行（承 G8 §2 纪律）。
- **语言 spec**：预期零新 RXS 条款（物理仍是引擎库，承 RFC-0021 §5.2）；若实现发现须改语言类型/内存模型/FFI ABI，另立/修订 Full RFC 先落条款。

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08 | 首版 DRAFT：承接 M74/M75/M77/M65b 四锚 + Jolt 5.6/Rapier/神经变形三调研轨；14 项决策建议、7 波次、8 门草案、RFC-0021 修订面。G9 未立项，不构成契约/验收承诺。 |
