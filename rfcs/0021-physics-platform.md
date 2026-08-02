# RFC-0021 — G8 replay-first 物理平台：capture/replay、网络物理、破坏与角色资产链

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0021（4 位制，编号永不复用，10 §9.5） |
| 标题 | G8 replay-first 物理平台：capture/replay、网络物理、破坏与角色资产链 |
| 档位 | **Full RFC**（10 §3：新增引擎运行时语义、持久化资产 ABI、网络 rollback 边界与多时间域身份；Jolt FFI/unsafe 消费只能在实现期按真实顺位登记） |
| 状态 | **Agent Approved**（2026-08-02）；§9.1 独立 provenance 对抗性评审完成，20 findings 逐条 disposition（4 blocker + 12 major 正文实改）。**批准不开放实现** |
| RFC 身份 | 本 RFC 即 `G8_ACCEPTANCE_MAP.md` 中所称的 **RFC-γ**（M67 smoothing bound、M72 碰撞穿透 bound 的冻结载体，冻结程序见 §6.5） |
| 承接里程碑 | G8.1 治理交付 D-G8-4；未来实现波次 G8.6a~G8.6d，均受 G-G8-3 事实互锁 |
| 关联条款 | **预期零新语言 spec 条款**（物理仍是引擎库，06 §8.3 / RFC-0017 五纪律不变）；资产 schema 与运行时契约见 §5，待实现互锁开放后落结构化事实源 |
| 依据决策 | D-406（agent 完全自主）· D-409（独立 provenance 对抗性评审）· P-01/P-05/P-09/P-11/P-12/P-13 · RFC-0017 · G8_PLAN v1.2 · G8_CONTRACT G-G8-3/G-G8-8A~8D |
| Provenance | `Assisted-by: Codex:gpt-5 rfc21-drafter-session`（独立起草会话；只起草，不批准） |
| Agent 批准 | **Agent Approved 2026-08-02**；只表示语义评审完成，实现仍由 G-G8-3 互锁与 `ci/check_g8_implementation_interlock.py` 决定 |
| 对抗性评审 | **完成**（D-409）：评审 provenance `Assisted-by: Kiro:claude-opus-5 rfc-review-session` ≠ 起草 provenance `Codex:gpt-5 rfc21-drafter-session`；findings 与 disposition 见 §9.1 |

---

## 1. 摘要

本 RFC 冻结 G8 物理平台的 **replay-first** 路线。G6 已交付 Jolt 5.3.0 CPU 刚体底座，但 capture/replay、网络回滚、破坏生产链、CharacterVirtual、PhysicsAsset/ragdoll/physical animation、载具和服装资产链仍不存在。G8 不先升级求解器再补可观测性，而是按以下顺序推进：

```text
Jolt 5.3 authoritative CPU rigid body
  └─ G8.6a capture + lifecycle journal + per-tick semantic hash
       ├─ frozen replay corpus + first-divergence locator
       ├─ only then: Jolt 5.3 ↔ 5.6 A/B  (M73)
       │    ├─ all correctness/migration/CCD/determinism/budget gates pass
       │    │    → may adopt 5.6 (corpus migrated, M66 re-run on 5.6,
       │    │      CI_GATES/MAP 判据经修订后才改版本字面)
       │    └─ any hard failure → formally pin 5.3 (记失败证据，不写 5.6 PASS)
       └─ downstream waves run on whichever version the A/B pinned:
            ├─ G8.6b network rollback + CharacterVirtual + PhysicsAsset
            ├─ G8.6c pre-fracture/destruction/cache/VFX
            └─ G8.6d vehicle + cloth product chain
```

核心裁决如下：

1. **M66 capture/replay-first**：先在 Jolt 5.3 建可重复 corpus、完整生命周期 journal、逐 tick 语义状态 hash 与首 divergence 定位器；G6 的 N=100 固定步重跑不是 capture/replay 的替代品。
2. **M73 升级后置**：仅在 M66 corpus 全绿后做 5.3↔5.6 A/B；5.6 升级失败时钉住 5.3 是正式成功止损，不得伪写 5.6 PASS。
3. **M67 网络物理**：server authority、input/state history、prediction、authoritative correction、rollback/resimulation、事件去重与表现层 smoothing 形成一条可重放链。
4. **M68 破坏生产链**：预破碎 cook、connection graph、层级 cluster、strain 断键、cache 与 VFX 事件桥共用同一版本化资产和时间域。
5. **M69/M70/M71/M72 产品层**：PhysicsAsset/ragdoll/physical animation、vehicle、CharacterVirtual、cloth 均接入 capture/network/资产版本化，而不是形成旁路状态机。
6. **权威主线不变**：CPU Jolt 刚体始终是权威主物理；**GPU 主刚体禁止**，GPU 只能承担**非权威特效副轨**（粒子/体积场/表现层布料视觉），不得作为任何刚体求解器——包括「经预算隔离的可选副求解器」在内（评审 F-06）。任何 GPU 刚体求解提案必须先由 RD-043 触发、满足 `G8_CAPABILITY_MATRIX` §12 五项重审条件，并另立 Full RFC；本 RFC 不提供任何隐式出口。

本 RFC 可在 G8.1 governance-only 内起草、评审与批准；**RFC 批准不解锁实现**。任何 `src/`、`spec/`、`conformance/`、数字 CI 步骤、RD/U/RX 消费都必须等 G-G8-3 真实变绿。

## 2. 动机、基线与边界

### 2.1 已有基线

RFC-0017 / G6 已冻结并实现以下底座：

- Jolt 5.3.0 生产默认、Rapier 可选快路径；
- `PhysicsWorld` 固定步、generation 句柄、批插/移除、并发查询、接触事件与 `SyncBudget`；
- 同机器、同构建、同输入的 N=100 固定步逐位重跑；
- 物理到 `GpuScene` 的单向变换桥；
- FFI/unsafe 集中在绑定 crate；
- CPU 主刚体与 Vulkan 渲染车道正交。

这些事实说明“刚体后端能运行”，但不等于“物理平台可生产化”。当前缺口是：没有可长期保存和迁移的完整 capture，没有 body create/remove journal，没有逐 tick divergence 定位，没有网络 history/rollback，没有 CharacterVirtual 与 ragdoll 状态保存，没有破坏/vehicle/cloth 资产闭环。

**在树 N=100 重跑的确切覆盖面（评审 F-17）**：该逐位断言只覆盖 `active_transforms`，且成立前提是 `job_threads=Some(1)`；接触事件的 `impulse` 因 JoltC 缺求解后 impulse 回调而恒 `0.0`（`src/rurix-physics-sys/VENDOR.md` §3 子缺口，G6 已按 (c) 收窄登记）。语义层 hash 的字段覆盖面属本 RFC §4.A1 新冻结面，不是既有断言的延伸。

#### 2.1.1 JoltC ABI 缺口与处置（评审 F-01 blocker）

在树 pin 为 **JoltPhysics 5.3.0 + JoltC `29820043`**（`src/rurix-physics-sys/VENDOR.md` §1）。该 pin 的 C ABI **不含**本 RFC 恢复层与 M71 所需的两个面：

| 缺口 | 实测 | 影响 | 处置（RFC-0017 §4.C1 三选一，实现前必须落定） |
|---|---|---|---|
| `StateRecorder` / `SaveState` / `RestoreState` C 面 | `JoltC/Functions.h` 无对应导出 | §4.A1 的 `PhysicsSnapshotBlob` 恢复层无法实现 | **M66 前置任务**：(a) 受审计的 vendor 补丁补 C 面（沿 U33~U42 审计模式）；(b) 换用另一 C ABI（如 amer-koleci/joltc）并重做许可/SBOM 审计；(c) 收窄——首期恢复层改为「由 canonical 语义层 + journal 完整重建」，放弃不透明 blob。三者必须在 G8.6a 首个实现 PR 前书面选定并记入本 RFC 修订行 |
| CharacterVirtual 函数 | `Enums.h` 仅剩 `JPC_GroundState` 枚举，`Functions.h` 无 CharacterVirtual 导出；soft-body create 在上游被注释 | §4.B2 的 safe wrapper 无底层可包 | **M71 前置任务**：同上三选一；未落定前 M71 不得开工，也不得以「wrapper 已设计」记任何绿 |

纪律：无论选 (a)(b)(c)，都不得在 `rurix-physics` safe 层泄露原生指针，不得绕过绑定 crate 集中 unsafe 的纪律；(a)(b) 均须先更新 `VENDOR.md` 的 pin/许可/SBOM/补丁 digest。

### 2.2 为何需要 Full RFC

本设计同时冻结：

- capture blob 与 canonical semantic state 的职责分界；
- 网络权威状态、rollback 副作用和事件 exactly-once 边界；
- PhysicsAsset、DestructionAsset、VehicleAsset、ClothAsset 的版本化持久化 ABI；
- game/render/rigid/network/cloth 多时间域的不可混用 identity；
- Jolt 5.3→5.6 迁移与失败回退规则；
- CPU 权威刚体和 GPU 非权威副轨的安全边界。

这些是运行时语义、资产 ABI 与安全包络的组合变更，按 10 §3 向上取严为 Full RFC。物理仍是引擎库，不借机进入 Rurix 语言核心。

### 2.3 in-scope

| 能力 | 编号 | 冻结内容 | 波次 |
|---|---|---|---|
| capture/replay 与 divergence | M66 | snapshot envelope、lifecycle journal、canonical state hash、故障注入定位 | G8.6a |
| Jolt 升级 A/B | M73 | M66 前置、5.3↔5.6 correctness/perf/CCD/迁移矩阵、失败钉 5.3 | G8.6a |
| 网络物理 | M67 | physics frame、history、prediction、correction、rollback/resim、事件去重、smoothing | G8.6b |
| PhysicsAsset/ragdoll | M69 | 骨骼体/constraint 映射、pose motor、partial simulation、physical animation | G8.6b |
| CharacterVirtual | M71 | safe wrapper、独立 SaveState、moving-platform/ground state、网络接入 | G8.6b |
| 破坏生产链 | M68 | fracture cook、connection graph、cluster/strain、cache、VFX bridge | G8.6c |
| 载具产品层 | M70 | drivetrain/tire/suspension asset、输入历史、状态保存、telemetry | G8.6d |
| 布料产品层 | M72 | panel/seam/fabric schema、DCC 导入、碰撞、LOD、独立求解时间线 | G8.6d |

### 2.4 out-of-scope

- GPU 主刚体、GPU 权威 network physics、以 GPU 求解结果替代 Jolt 权威状态；
- 把 Rapier 改成生产默认或用跨引擎容差对拍代替 Jolt replay correctness；
- 在 M66 corpus 之前升级 Jolt、改写 corpus 来迎合 5.6、把升级失败记为 PASS；
- 跨平台/跨编译器 bitwise lockstep 承诺；网络正确性走 server correction + rollback sufficient determinism；
- runtime 任意拓扑 fracture；通用软体/Flesh/流体与 MPM/FLIP（**RD-044** 的 Continuum/Fluid 观察面）；神经变形（无 RD 归属，属 G9+ 研究轨）；**可微物理（观察归 RD-042，明确不进 RD-044 四拆**，评审 F-18）；
- M75 异步 physics thread 的完整调度实现；本 RFC 只冻结时间域 identity，异步调度仍需独立判档；
- 编辑器 GUI、通用 node graph、cache farm、任意第三方脚本执行；
- 改写 RFC-0017、G6 closed 契约、G5 渲染冻结面或 G7 在途事实。

## 3. 跨章不变量

### 3.1 RFC-0017 五条纪律 0-byte 延续（逐字引用）

以下五条是 RFC-0017 §4.B1 的**逐字冻结句**，不得在本 RFC 内改写（评审 F-02 blocker：初稿曾删去纪律 2 的「同帧可读上一拍变换」并把纪律 4 的「Taichi AOT 只产出粒子/体积场」扩为含 cloth，属未经 0017 修订行的冻结面改写）：

1. **单向事实源**：动态/运动学变换仅由 `rurix-physics` → `GpuScene`；渲染器不回写物理。
2. **查询并行**：角色/AI/拾取走并发 query；与 `render_exec` **同帧可读上一拍变换**。
3. **流送同构**：几何页驻留/卸载驱动 body 批插入/移除；物理只订阅「页驻留/卸载」通知，不重新实现 `StreamingBudget`。
4. **特效隔离**：**Taichi AOT 只产出粒子/体积场**，经 external import 进 graph；不进刚体求解、不承担确定性联网。
5. **库不进语言**：FFI 集中在 `rurix-physics-sys`；`rurix-render` 维持 `#![forbid(unsafe_code)]`。

### 3.1.1 本 RFC 的附加约束（不改写 0017，只在其之上加严）

这些是 G8 新增约束，与上表分离登记，便于审计谁在什么时候加了什么：

- **A1（承纪律 2）**：rollback / resimulation 不得旁路「同帧只读上一拍变换」——重演期间渲染侧仍只读已提交 buffer，禁止把 resim 中间态暴露给同帧渲染。
- **A2（承纪律 3）**：卸载路径仍遵守 `RemovalReceipt` 先卸 body 后放页；capture/journal 必须记录该次序。
- **A3（承纪律 4）**：G8 的 **cloth 表现层** GPU 副轨若出现，只能是非权威可丢弃视觉，且**不经 Taichi AOT 通道**（纪律 4 的 Taichi 出参面字面不变）；权威 cloth 求解见 §4.D2，恒在 CPU。

### 3.2 CPU 权威刚体与 GPU 禁止线

- 权威 world、server correction、rollback/resimulation、destruction cluster 激活、CharacterVirtual inner body、ragdoll 和 vehicle 刚体均运行在 CPU Jolt 主线。
- GPU 结果不得写回或替换权威 rigid-body snapshot；表现层 cloth、粒子、碎片视觉 cache 可消费权威事件，但必须可丢弃和重建。
- 任何未来 GPU rigid-body 提案必须满足 G8_CAPABILITY_MATRIX §12 五项重审条件并另立 Full RFC；本 RFC 不提供隐式出口。
- GPU 能力缺失、队列拥塞或 device loss 时只允许显式禁用非权威副轨；不得静默切换权威求解器。

### 3.3 多时间域 identity

以下 ID 是不同名义类型，禁止裸整数互转或以“同一帧”暗示相等：

| Identity | 权威方 | 含义 |
|---|---|---|
| `GameFrameId` | gameplay loop | 输入采样与游戏逻辑显示帧 |
| `RenderFrameId` | renderer | graph submit/present 序列 |
| `PhysicsTickId` | authoritative rigid world | 固定步刚体状态边界；capture 的主索引 |
| `NetworkPhysicsFrameId` | replication layer | 权威修正、输入 history 与 rollback 的网络序列 |
| `ClothTickId` | cloth solver | cloth 独立固定步/子步序列，不与 rigid tick 冒充同域 |

跨域关系只能通过 `FrameDomainMap` 的显式记录表达：

```text
FrameDomainMap {
  game_frame,
  render_frame,
  rigid_tick_range,
  network_frame?,
  cloth_tick_range?,
  interpolation_alpha,
}
```

`FrameDomainMap` 是 capture/evidence 的一部分。网络修正定位到 `NetworkPhysicsFrameId` 后必须解析为明确的 `PhysicsTickId`；cloth 输入引用 rigid pose 时记录消费的 rigid tick；VFX 事件由 physics tick 映射到 render frame。任何隐式“当前帧”读取均为错误。

### 3.4 标识与 exactly-once

- `BodyId` / `ShapeId` 继续使用 generation 语义；capture 和 network 消息不得只保存 arena index。
- 所有可回滚副作用使用稳定 `PhysicsEventId`，由事件类型、权威 `PhysicsTickId`、参与对象 generation ID 和该 tick 内 canonical ordinal 共同派生。
- rollback 可以多次重新产生同一内部事件；提交到 audio/VFX/gameplay 的桥以 `PhysicsEventId` 去重，只在状态越过不可回滚提交边界后对外发布一次。
- smoothing 只修改 presentation transform，不改变权威 body state、hash 或碰撞查询结果。

## 4. 参考级设计

### 4.A M66 — capture/replay-first

#### A1. capture envelope

每份 capture 由不可混用的三层组成：

1. **环境头** `PhysicsCaptureHeader`：schema version、Jolt version/commit、JoltC ABI digest、Rurix build fingerprint、platform/architecture、fixed-step rational、world/cook/profile digests、初始 ID domain，外加两组必需画像：
   - **determinism 画像（评审 F-11 具体化）**：`job_threads` 实际取值（在树逐位断言只在 `Some(1)` 成立）、job system 种类（ThreadPool / SingleThreaded）、vendor `CROSS_PLATFORM_DETERMINISTIC` 取值（在树为 `OFF`）、`DOUBLE_PRECISION`、`OBJECT_LAYER_BITS`。corpus 必须固定线程画像，跨画像的 hash 不可比。
   - **预算画像（评审 F-10）**：`SyncBudget` 与接触事件 ring 容量。在树语义是接触事件受预算截断、未消费条目跨 tick 留在 ring，因此换预算即改变事件 digest；journal 另须记每 tick 的饱和计数与 tick 末 ring backlog。
2. **恢复层** `PhysicsSnapshotBlob`：Jolt `SaveState`/`RestoreState` 所需的同版本不透明 payload，加 CharacterVirtual/上层组件的独立状态块。该 blob 只承诺**同版本恢复**，不得作为跨 Jolt 版本 ABI。**可得性前置见 §2.1.1**：当前 pin 的 JoltC 无该 C 面，必须先按 (a)(b)(c) 落定处置；若选 (c) 收窄，则本层由「canonical 语义层 + journal 完整重建」替代，且该替代必须在 corpus 中显式登记，不得假装 blob 存在。
3. **语义层** `CanonicalPhysicsState`：按 generation ID 与稳定字段顺序序列化的 body/constraint/character/destruction/vehicle 状态，用于逐 tick hash、diff 和跨实现可读诊断。

`snapshot_blob_digest` 验证同版本 blob 完整性；`semantic_state_hash` 验证规范化可观察状态。两者不得混为一个“state hash”。浮点以 canonical bit pattern 编码（统一 `-0`、拒绝非规范 NaN），排序以稳定 ID 为主，不使用地址、hash-map 迭代序或 job 完成顺序。

#### A2. command 与 lifecycle journal

每个 `PhysicsTickId` 的 journal 至少记录：

- tick 前输入/命令：力、冲量、kinematic target、constraint/field/character/vehicle input；
- body/shape/constraint create、remove、enable、disable 与 generation 变化；
- 流送页驻留/卸载及 `RemovalReceipt`；
- CharacterVirtual 独立状态变更、destruction cluster 激活、vehicle/cloth 输入边界；
- normalized contact/break/fracture 事件及 canonical ordinal；
- tick 后 `semantic_state_hash`、事件 digest 与 journal-consumed 标志。

重演完成时所有 journal 条目必须恰好消费一次；多余、遗漏、乱序或引用已失效 generation 均 fail-closed。不可重演的外部 callback 必须先规范化成 journal command，禁止 replay 时再次访问墙钟、线程 ID、随机全局状态或未记录 I/O。

#### A3. Jolt 5.3 corpus 与 divergence locator

G8.6a 首件事是在当前 Jolt 5.3.0 上建立 checked-in fixture corpus，覆盖至少：

- create/remove/reuse generation、睡眠/唤醒、CCD、批插/流送卸载；
- contact begin/persist/end、joint/motor、查询与 moving platform；
- CharacterVirtual 独立保存、ragdoll/vehicle/destruction 状态（对应子系统落地后追加）；
- deterministic replay：原始运行与 restore+replay 的每 tick hash、事件 digest、最终状态全等；
- 故障注入（**注入点与字段白名单冻结，评审 F-12**）：注入作用于 fixture 指定 tick 的**活世界 step 之前**的状态，被注入字段必须取自「参与该 tick `semantic_state_hash` 的字段白名单」（否则被求解器吸收或延后可见时定位器会误红）；定位器返回的 `first_divergence_tick` 精确等于注入 tick，并输出字段路径/稳定 ID/expected/actual。

M66 的硬判据沿 `g8.p0.m66.physics_replay`：只有完整 capture→restore→replay、全 journal 消费和首 divergence 定位均通过才是 PASS。仅重复跑 N=100、仅比较最终 transform、仅保存 Jolt blob、或只报“hash 不同”均不满足。

#### A4. Jolt 5.3 ↔ 5.6 A/B 与止损

M73 必须在 A3 corpus 全绿后执行，顺序固定：

1. 在 5.3 上冻结 corpus、资产 cook digest、CCD/contact/query 结果和 measured baseline；
2. 5.6 走独立 vendor/ABI 构建，不覆盖 5.3 基线；
3. 两版本分别证明各自同版本 capture/replay 逐 tick一致；
4. 对相同 canonical source asset/input journal 做 A/B，核对 lifecycle/event 拓扑、无效状态、CCD 漏碰、query/contact/constraint 不变量、CharacterVirtual/ragdoll/vehicle 状态和资产迁移；
5. 性能阈值只从真实采样写入 budget，RFC 不预造数字。目标版本锚：Jolt **v5.6.0**（R2 §3.2 记 2026-07-11 发布）；实现期必须按实测 tag/commit 登记，不以版本号字面代替 vendor 可得性核实（评审 F-19）；
6. **失败臂**：若 correctness、迁移、CCD、determinism 或预算任一硬门失败，**正式钉住 5.3**，记录失败证据并继续 G8.6b~d；不得修改 corpus、放宽 hash 或把 pin 5.3 写成 5.6 PASS；
7. **采纳臂（评审 F-15 补齐）**：若全部硬门通过并采纳 5.6，必须同一 PR 内完成三件事——① corpus 按显式迁移器迁到 5.6 并保留 5.3 基线 artifact；② `g8.p0.m66.physics_replay` 在 5.6 上重跑并重新落 evidence；③ `CI_GATES.md` §4.1 的 m66 GREEN 判据字面（当前为「**Jolt 5.3** capture/replay hash 一致」）与 `G8_ACCEPTANCE_MAP.md` M66 行经修订后才可改版本字面。缺任一项即视为未采纳，继续钉 5.3。

跨版本不要求 Jolt 私有 blob 可互相 Restore；升级载体是 canonical asset + input journal + 显式迁移器。若 solver 变化导致语义状态不可能逐 bit 同值，A/B 必须逐字段分类为 exact/tolerance/invariant，容差由 corpus 实测后在独立追加记录中冻结；对象集合、生命周期、错误类别和离散事件不允许浮点容差。

### 4.B M67/M69/M71 — 网络、CharacterVirtual 与 PhysicsAsset

#### B1. network physics 状态机

```text
sample input(GameFrameId)
  → assign NetworkPhysicsFrameId
  → predict on PhysicsTickId range
  → receive authoritative snapshot/hash
      ├─ within accepted state → keep simulation; presentation smoothing only
      └─ mismatch → restore authoritative tick
                     → replay recorded inputs/journal
                     → deduplicate side effects
                     → converge to server semantic_state_hash
```

**hash 收敛的适用画像（评审 F-13）**：`g8.p0.m67.network_physics` 要求「resimulation 后最终 canonical state hash 等于服务端」，该判据只在 fixture 的两端为**同 build、同平台、同 determinism 画像**时成立；本 RFC §2.4 放弃的是跨平台/跨编译器 bitwise lockstep 承诺，二者不冲突。跨画像场景只做 tolerance/invariant 对照，且必须在 evidence 中标明画像差异，不得用跨画像结果充该门绿。

网络层至少包含：

- 有界 input/state/snapshot history ring；容量不足时显式 hard correction，不静默丢历史继续预测；
- server authority 与 correction 的 schema/version/build/cook digest 校验；不兼容直接拒绝连接或进入明确的迁移路径；
- rollback 起点、重放输入序列和最终 server hash 可写入 evidence；
- contact/fracture/gameplay cue 的 `PhysicsEventId` 去重；
- soft snap/hard snap 或 predictive interpolation 只作用于 presentation transform；
- packet loss/latency trace 可重复，测试必须先产生预测偏差，再在 golden frame correction 后收敛。

网络正确性不以跨平台 bitwise lockstep 为前提。服务端 canonical state 是权威边界；客户端必须能恢复并重演足够内部状态，不能只修 transform 而遗漏 sleep/island/constraint/CharacterVirtual 状态。

#### B2. M71 CharacterVirtual

`CharacterVirtualState` 是独立版本化状态块，至少覆盖 position/rotation、linear velocity、ground state、ground body generation、support normal、stair/slope 状态、moving-platform relative transform、inner-body identity 与 user state。它必须：

- 由 safe wrapper 暴露，不泄露 Jolt 原生指针；
- 与 rigid `PhysicsSnapshotBlob` 一起 capture，但保持独立 schema/version；
- 参与 semantic hash、network correction 和 rollback；
- 在 moving platform 被流送移除时通过 generation/receipt 规则 fail-closed；
- 并行 update 的输出在 tick 边界按稳定 character ID 归一化提交。

#### B3. M69 PhysicsAsset / ragdoll / physical animation

`PhysicsAsset` 是 canonical source asset，不是编辑器私有内存快照。它至少包含：

- skeleton/bone stable ID 到 body/collider/constraint 的映射；
- collision layer/material/filter preset；
- joint frame、limit、mass/inertia policy 与 motor profile；
- ragdoll LOD、partial-simulation mask、kinematic/physical blend profile；
- physical-animation pose target profile 与 per-bone drive parameters；
- cook profile、source skeleton digest、schema version 和迁移记录。

动画域与物理域通过显式双缓冲桥接：某个 `PhysicsTickId` 消费带 source pose identity 的目标姿态，求解后产出带同一 tick 的 physical pose；render interpolation 只能读取已提交 buffer。ragdoll 进入/退出、partial simulation 和 motor profile 切换都写入 command journal，才能在 rollback 后重现。

### 4.C M68 — 破坏生产链

#### C1. 资产与 cook

`DestructionSourceAsset` 与 `DestructionCookedArtifact` 分离。source 记录原 mesh/material、fracture recipe、anchor/field 配置与 cook profile；cooked artifact 至少包含：

- chunk stable ID、父子 cluster 层级、connection graph；
- interior face/material 映射、collision shape 与 mass properties；
- anchor、edge strength/strain policy、activation layer；
- source/tool/profile/version digests 与 canonical serialization。

同输入独立双 cook 必须逐字节相等；运行时只消费受版本校验的 cooked artifact。未知 schema、digest 不匹配、悬空 graph edge、非树 cluster 或非法 anchor 均 fail-closed。

#### C2. 运行时、cache 与 VFX

- runtime 以预破碎 graph 为主；damage/field 只改变 edge strain 与 cluster activation，不在主帧执行任意拓扑 fracture；
- 指定阈值下不破坏，越阈值时在指定 `PhysicsTickId` 断开指定 edge 并激活稳定 chunk/cluster ID；
- cluster 激活进入 authoritative CPU world，body 生命周期写 journal 并参与网络 snapshot；
- `DestructionCache` 记录 command/event/state，不存依赖地址的 Jolt 私有指针；cache roundtrip 后事件序列与 semantic hash 不变；
- VFX bridge 仅消费 `FractureEvent`，按 `PhysicsEventId` 恰好提交一次；视觉碎片可非权威并可重建。

M68 沿 `g8.p0.m68.fracture_pipeline` 独立过门，不能用“Jolt 能产生刚体碎片”或离线 fracture 单段成功代替全链。

### 4.D M70/M72 — 载具与布料产品层

#### D1. M70 vehicle

`VehicleAsset` 至少冻结 chassis body 引用、wheel/track 布局、suspension、engine/transmission/differential、tire/friction profile、control mapping、telemetry channels、schema/cook version。vehicle input 以 `NetworkPhysicsFrameId`/`PhysicsTickId` 进入 history；wheel contact、gear/engine state 和 constraint internals 必须纳入 capture 或可由已记录状态确定性重建。

载具仍是 CPU Jolt `VehicleConstraint` 产品包装，不启动 GPU vehicle 主环。acceptance 至少覆盖：资产 roundtrip、固定输入重演、rollback correction、轻物体/轮胎接触回归、状态序列化与 telemetry 可追溯。

#### D2. M72 cloth

`ClothAsset` 使用开放、版本化、可迁移的产品 schema，至少包含：

- panel topology、seam、fabric/material 参数与 stable vertex/constraint ID；
- sim mesh ↔ render mesh/skin pose 映射；
- collision proxy、self-collision policy、tether/bend/stretch constraints；
- LOD topology、跨 LOD state mapping 与 cook profile；
- DCC/source digest、import recipe、schema version 与 canonical artifact。

**授权来源（评审 F-05）**：cloth 进入 G8.6d 主线的依据是 `G8_CANDIDATE_DECISIONS.md` 的 RD-044 布料分项 **go** 行 + `G8_CAPABILITY_MATRIX` M72（「XPBD cloth 或 Jolt soft body 扩展」）。RD-044 该分项 backfill 字面为「Jolt 软体/布料在真实角色/资产需求出现时(rurix-physics crate 内扩 safe API，FFI 沿 U33~U42 审计模式续号，unsafe 集中绑定 crate 纪律不变)」——本 RFC **不改写该字面**；若首期选择不引入新 FFI 的自有 CPU solver，则「FFI 沿 U33~U42 续号」这一前提**不被消费**（零 U claim），而不是被否定。

首期权威产品路径为 **CPU cloth solver**（**裁决：自有可审计 XPBD**，见 §9 Q4 的最终裁决），与 rigid-body CPU 主线交换显式 pose/collider 边界。`ClothTickId` 与 `PhysicsTickId` 必须保持不同 identity：cloth 可对子步，但每次输入注明消费的 rigid tick，每次 render 输出注明 cloth tick 与映射的 render frame。未来 GPU cloth 只能是可选非权威 feature，须单独证明 Vulkan 预算隔离、device loss 恢复和 CPU fallback 的显式选择；不能放宽 GPU 主刚体禁令。

M72 沿 `g8.p1.m72.cloth_product_chain` 过门，五面逐字照抄 `G8_ACCEPTANCE_MAP.md`（评审 F-08：初稿只列四项）：**① schema、② 导入、③ 碰撞（含 seam 约束不断裂）、④ LOD state mapping、⑤ 独立求解 timeline**，缺一即 FAIL；evidence 必须为五个独立布尔字段。碰撞/穿透数值 bound 由 §6.5 程序在本 RFC 内冻结后该门才可判 GREEN，本节不预造数字。

## 5. 资产 schema、版本化与下游事实源

### 5.1 canonical schema 共同头

所有物理 source/cooked/capture/network/cache schema 共享：

```text
SchemaHeader {
  schema_id,
  schema_version,
  producer_tool_version,
  source_digest,
  dependency_digests[],
  cook_profile_digest?,
  payload_digest,
}
```

共同规则：

- canonical 字段顺序、稳定 ID 排序、固定 endian 与规范浮点编码；
- source 与 cooked artifact 分离，runtime 不静默重新 cook；
- 未知版本 fail-closed；迁移器必须显式声明 from/to version，并有 golden；
- DDC key 覆盖 source、依赖、工具版本、schema、Jolt ABI 和 cook profile；
- 资产身份不使用路径字符串作为唯一键，路径只作诊断元数据；
- vendor 私有 blob 只能置于带 vendor/version/ABI digest 的隔离字段，不能成为跨版本 canonical ABI。

### 5.2 预期结构化事实源

实现互锁开放后，以下 schema/测试资产应与对应首个实现 PR 同落；此处只冻结职责，不创建空壳：

| 事实源 | 内容 | 消费方 |
|---|---|---|
| physics capture schema | header/snapshot/semantic state/journal/divergence | M66/M73/M67 |
| PhysicsAsset schema | bone/body/collider/joint/motor/LOD/authoring | M69/M71 |
| Destruction schema | fracture recipe/chunk graph/cluster/anchor/cache | M68 |
| Vehicle schema | drivetrain/tire/suspension/input/telemetry | M70 |
| Cloth schema | panel/seam/fabric/mapping/collision/LOD | M72 |
| frame-domain schema | five IDs、range/map、event identity | M66/M67/M69~72 |

**下游 spec 映射**：预期零新 RXS 条款。若实现发现必须改变语言类型、内存模型、FFI ABI 或 unsafe 安全包络，必须另立/修订 Full RFC 并先落条款；本 RFC 不预留 RXS 号，也不以“引擎库”名义绕过规范先行。

### 5.3 错误码与 unsafe

- 资产/网络/capture 失败优先使用结构化 `PhysicsError`/tool error，不预造 RX 号；只有出现真实可达的编译器/工具诊断类别时，按合入时 actual next-free 领取。
- Jolt 5.6 ABI、CharacterVirtual、vehicle 或 soft-body FFI 新增 unsafe 时，只能留在绑定 crate，每块附 `// SAFETY:` 并按实现时 actual next-free 登记 U；本 RFC 零 U claim。
- 本 RFC 不登记 RD；实现期出现新阻塞才按 actual next-free 追加，不以预留债务代替设计。

## 6. feature、tracking、实现序与验收

### 6.1 治理/实现硬互锁

当前合法动作只有 G8.1 governance-only：本 RFC 可被起草、对抗性评审和批准，但不得 materialize 实现。G8.2+（包括 G8.6）只有在以下条件全部满足后才开放：

1. `milestones/g7/G7_CONTRACT.md` 字面 `status: closed`；G7 active 时不存在实现 override 出口；
2. RD-038 `status: closed`，**或** G7 closed 后 G8_PLAN §1.0 六行接入表以终态 evidence 全填，并在 RD-038 history 追加一条与其他 strategic override 分离的治理 override；
3. G8.1 治理门完成，且共享编号按当时 registry 的 actual next-free 重新校准。

RFC-0021 从 Draft 变 Agent Approved、M50 strategic override 或任何其他决策，均不能替代上述事实。互锁未绿时：`src/`、`spec/`、`conformance/`、实现脚本、数字 CI、RD/U/RX 均 0-byte。

### 6.2 runtime feature 边界

库级功能面**名称即日冻结**（评审 F-04 blocker：Full RFC 不得把冻结权推给评审者；下列名字自本 RFC Approved 起为冻结面，改名须走本 RFC 修订行）：

- `physics-capture`：snapshot/journal/hash/divergence；
- `network-physics`：history/rollback/correction/event dedup；
- `physics-character`：CharacterVirtual + PhysicsAsset/ragdoll；
- `physics-destruction`：cooked graph/cache/VFX event；
- `physics-vehicle`：vehicle product wrapper；
- `physics-cloth`：CPU cloth product chain。

功能未编译时返回明确 `BackendNotCompiled`/`FeatureNotCompiled` 类错误，不静默退化成无碰撞、无 rollback 或视觉-only 成功。Cargo feature 名不构成语言 stable 面承诺；对外 schema/version 才是冻结面。

### 6.3 严格实现序

1. **G8.6a / M66**：schema + RED fixture → Jolt 5.3 capture/journal/hash/divergence → corpus 全绿；
2. **G8.6a / M73**：独立 5.6 vendor/ABI → 5.3↔5.6 A/B → 采纳或钉 5.3；
3. **G8.6b / M67/M71/M69**：network history/rollback → CharacterVirtual state → PhysicsAsset/ragdoll/physical animation；
4. **G8.6c / M68**：deterministic fracture cook → runtime cluster/strain → cache/VFX/network；
5. **G8.6d / M70/M72**：vehicle product layer → cloth schema/import/collision/LOD/timeline；
6. 全量 replay corpus、packet-loss trace、asset migration、existing G6 tests 与 strict budget 回归。

任一后波不得反向要求改写 M66 capture 格式而不经显式 schema migration。一次进程可共享 world 初始化，但 M66/M67/M68 与 M72 必须各自产出独立 subject、独立 PASS/FAIL/SKIP。

### 6.4 symbolic gates（零数字 claim）

key/脚本的唯一事实源是 `G8_ACCEPTANCE_MAP.md` §2/§3 与 `CI_GATES.md` §4/§4.0（`g8.p{0,1}.m##.<slug>` + `ci/g8_<slug>_smoke.py`，由 `ci/check_g8_acceptance_map.py` 三向比对强制）。本 RFC 不新造 key，也不混用大小写两套（评审 F-07）。

| Gate | 独立硬判据 |
|---|---|
| `g8.p0.m66.physics_replay` | Jolt 5.3 capture 完整重演、journal 全消费、逐 tick hash 与注入 tick 首 divergence 精确定位 |
| `g8.p0.m67.network_physics` | prediction 偏差→权威修正→rollback/resim→server hash 收敛（同画像，§4.B1）、事件只提交一次、smoothing **不改权威状态且逐帧满足 §6.5 冻结 bound**（两项并列，评审 F-09） |
| `g8.p0.m68.fracture_pipeline` | deterministic cook→graph/cluster→strain 断键→cache roundtrip→VFX exactly-once 全链 |
| `g8.p1.m72.cloth_product_chain` | schema / 导入 / 碰撞（穿透 ≤ §6.5 bound）/ LOD mapping / 独立 cloth timeline 五项独立为真 |

**无 P0/P1 gate 的面（评审 F-14）**：M69（PhysicsAsset/ragdoll/physical animation）、M70（vehicle）、M71（CharacterVirtual）、M73（Jolt 5.3↔5.6 A/B）当前**没有** symbolic gate、没有独立 evidence subject，也不在已 go P1 集合 `{M25,M72,M83}` 内。它们的闭环证据由波次聚合门 `g8.wave.6b.exit` / `g8.wave.6d.exit` / `g8.wave.6a.exit` 汇总（`CI_GATES.md` §5），并须在该波 evidence 中各自登记独立 subject 行：`g8.wave6b.m69.physics_asset`、`g8.wave6b.m71.character_virtual`、`g8.wave6d.m70.vehicle`、`g8.wave6a.m73.jolt_ab`。这些 subject **不产 P0/P1 PASS**、不计入 G8.8a 的「已 go P1 回归」集合；若要升格为独立硬门，须先按 `G8_ACCEPTANCE_MAP.md` §6 修订覆盖集合。

数字 CI 步骤必须在 G7 close 后读取实际 next-free 分配；本文不猜号、不创建空 workflow 或空 evidence schema。

### 6.5 冻结 bound 的程序（RFC-γ 职责，评审 F-03 blocker）

`G8_ACCEPTANCE_MAP.md` 把 M67 的 smoothing bound 与 M72 的碰撞穿透 bound 指为「RFC-γ 冻结 bound」，而本 RFC 即 RFC-γ。为同时满足「零预造数字」（P-09）与「判据可求值」，冻结程序固定为三步：

1. **采样**：G8.6b/G8.6d 首批 measured corpus 在冻结 determinism 画像下采集 —— smoothing 面采 presentation transform 的逐帧位置/角度偏移与收敛帧数；cloth 面采穿透深度相对布料厚度与碰撞代理尺度的归一化值。
2. **冻结**：采样结果以**本 RFC 的加性修订行**写入（新增 §6.5.1 数值表 + 修订记录行），与 `g8_budget.json` 的对应 measured 条目同 PR；bound 是 correctness 判据，不是性能阈值，因此进 RFC 而非只进 budget。
3. **生效**：bound 冻结前，`g8.p0.m67.network_physics` 的 smoothing 项与 `g8.p1.m72.cloth_product_chain` 的碰撞项只能是 RED/未实现；**禁止**以「bound 待定」跳过该项或以「视觉上平滑」代替数值判据。

bound 一经冻结即为 0-byte 面，放宽须新修订行 + 说明理由；收紧允许追加。

## 7. 风险、止损与备选

| 风险 | 预警 | 止损 |
|---|---|---|
| capture 只包 Jolt blob | 无 canonical diff、CharacterVirtual/上层状态遗漏 | 阻断 M66；恢复层与语义层缺一不可 |
| 5.6 升级倒置 | corpus 未绿先换 vendor，或改 golden 迎合新 solver | 恢复 5.3 基线；升级判失败并钉 5.3 |
| network 只修 transform | correction 后 sleep/constraint/event hash 不收敛 | restore 完整 snapshot + journal resim；禁止 presentation 修正充权威绿 |
| rollback 副作用重复 | 同一 contact/fracture cue 多次对外提交 | `PhysicsEventId` + commit boundary 去重；失败即 M67 红 |
| 多时间域混同 | 以 frame `u64` 互传、cloth/render 读取“当前物理帧” | 不同 newtype + `FrameDomainMap`，无显式映射拒绝 |
| 破坏链只剩碎片特效 | 无 graph/strain/cache/network 状态 | 预破碎权威 graph 必须进 CPU world 与 capture；视觉桥不得代绿 |
| cloth 侵占 GPU 主线 | GPU solver 成为唯一实现或 device loss 破坏权威状态 | CPU 产品路径先行；GPU 仅可选副轨并独立预算 |
| FFI unsafe 外溢 | safe crate 持有原生指针或新增无审计 unsafe | 集中绑定 crate；unsafe-audit/SAFETY 门阻断 |

备选方案：

- **A. 先升级 Jolt 5.6，再做 replay**——否决；无 corpus 无法区分升级改善与状态漂移。
- **B. capture 直接持久化 Jolt 私有 blob**——否决为唯一格式；仅允许同版本恢复层，跨版本诊断/迁移依赖 canonical semantic state。
- **C. 固定点跨平台 lockstep**——不进 G8；成本和产品需求不匹配，server authority + correction + replay 是正式口径。
- **D. Rapier 作为 replay oracle**——否决；跨 solver 不承诺逐位，Rapier 可作不变量/容差对拍但不能判定 Jolt corpus truth。
- **E. runtime 动态 Voronoi fracture**——首期否决；离线 deterministic cook + runtime graph activation 更可测试且可网络化。
- **F. GPU rigid/cloth 统一求解器**——否决；违反 G6 禁止线并与 Vulkan 渲染争车道。

## 8. 不做与冻结面

- RFC-0017 五纪律、G6 CPU 主物理与 GPU 主刚体否决线全文不改；
- G5 renderer/resource/barrier/page ABI 冻结面不因 physics integration 改写；
- G7/RD-038 状态和 evidence 不由本文推导；
- 不承诺实现任意 editor、通用 dataflow、完整 Chaos parity、Flesh/Fluid/ML Deformer；
- 不预先消费 RXS、CI、RD、U、RX、MR、SG 或 D 编号；
- 不把 `Draft`/`Agent Approved` 当作 M66/M67/M68/M72 PASS；
- 性能叙述必须来自 `measured_local` evidence，未采样不写阈值。

## 9. 未决问题 / 关键裁决

- **Q1 — semantic state hash 的浮点 canonical 规则**：提议统一 `-0`、拒绝非规范 NaN、逐字段 bit encoding；跨 Jolt 版本只对 exact 字段用 hash，同一版本 replay 必须逐 tick exact。由 §9.1 评审核对是否足以避免误报。
- **Q2 — network correction 的提交边界（已裁决，评审 F-04）**：**裁决 = 采纳提议并冻结**——history ring 内事件均可撤回；不可逆副作用只在状态越过 `server_confirmed_frame` 后按 `PhysicsEventId` 恰好发布一次；ring 窗口耗尽走显式 hard correction（不静默丢历史继续预测）。API 具体形态（函数签名/类型名）属实现期 safe API 设计，但上述**提交边界语义**自本 RFC Approved 起为冻结面。
- **Q3 — destruction runtime fracture 范围**：提议 G8 只接受 deterministic offline fracture cook + runtime graph break，不接任意 runtime topology fracture。
- **Q4 — cloth 首期 CPU solver（已裁决，评审 F-04）**：**裁决 = 自有可审计 XPBD 为首期权威产品路径**。理由：① 当前 pin 的 JoltC 连 soft-body create 都被上游注释（§2.1.1），走 Jolt soft body 需先补 C 面，把 M72 绑到一个未落定的 vendor 处置上；② 自有 CPU solver 零新 FFI，因而 RD-044「FFI 沿 U33~U42 续号」前提不被消费；③ M72 的五项判据里有四项（schema/导入/LOD/独立 timeline）与 solver 选择无关，绑定 vendor 反而增加风险。Jolt soft body 保留为**候选后端**，若未来接入须独立判档并更新 `VENDOR.md`。开放 schema 与独立 `ClothTickId` 不随 solver 改变。
- **Q5 — Jolt 5.6 采纳规则**：提议任一 correctness/migration/CCD/determinism hard gate 失败即 pin 5.3；性能优势不能覆盖 correctness 红。由评审确认无“局部升级”静默出口。
- **Q6 — M75 异步 tick**：提议本 RFC 只冻结不同时间域 identity 和桥接，异步 physics thread 调度保持 P2，不能成为 M66~M72 前置。

### 9.1 对抗性评审记录（10 §3 / §7 · D-409）

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: Kiro:claude-opus-5 rfc-review-session`（≠ 起草 `Codex:gpt-5 rfc21-drafter-session`） |
| 评审轮次 | R1（独立会话只读评审，findings 由本会话逐条落改） |
| 日期 | 2026-08-02 |
| 评审镜头 | ① correctness（在树实况：`VENDOR.md` pin Jolt 5.3.0 / JoltC `29820043` 与七面缺口审计、`behavior.rs` 的 N=100 逐位断言与线程画像、接触事件预算截断语义、G6 closed）② redline（编号 claim、RD-042/043/044 字面、GPU 主刚体禁止线、0017 五纪律 0-byte、5.6 顺序倒置、Draft 冒充许可）③ implementability（capture/replay/divergence/rollback/cloth 能否被 MAP 断言精确求值） |
| 结论 | 4 blocker + 12 major + 4 minor；blocker 与全部 major 已在正文实改，minor 逐条 disposition 后翻 **Agent Approved** |

| # | Finding | 严重度 | Disposition |
|---|---|---|---|
| F-01 | 恢复层依赖 Jolt `SaveState`/`RestoreState`、M71 依赖 CharacterVirtual，但在树 pin 的 JoltC **无这两个 C 面**（soft-body create 亦被上游注释），且未按 RFC-0017 §4.C1 留任何缺口处置 | **blocker** | **采纳，正文实改**：新增 §2.1.1 逐面列缺口 + (a)(b)(c) 三选一处置，并定为 M66/M71 的开工前置；§4.A1 恢复层加可得性前置引用 |
| F-02 | 自称五纪律「0-byte 延续」却删去纪律 2 的「同帧可读上一拍变换」、把纪律 4 的「Taichi AOT 只产出粒子/体积场」扩为含 cloth | **blocker** | **采纳，正文实改**：§3.1 改为 0017 §4.B1 逐字引用；新增 §3.1.1「本 RFC 的附加约束」把 A1/A2/A3 与冻结句分离登记，cloth GPU 副轨明确不经 Taichi 通道 |
| F-03 | MAP 把 M67 smoothing bound 与 M72 穿透 bound 指为「RFC-γ 冻结 bound」，本 RFC 既不声明自身即 RFC-γ 也不给冻结程序 → 两门永不可判 | **blocker** | **采纳，正文实改**：头部新增「RFC 身份 = RFC-γ」行；新增 §6.5 三步冻结程序（采样 → 本 RFC 加性修订行冻结 → 生效），冻结前对应判据只能 RED |
| F-04 | 把 feature 名、提交边界 API 形态、cloth solver 选型的裁决权交给 D-409 评审 → Approved 时无冻结面 | **blocker** | **采纳，正文实改**：§6.2 feature 名即日冻结（改名走修订行）；§9 Q2 冻结提交边界语义；§9 Q4 裁决为自有可审计 XPBD 并给三条理由，Jolt soft body 降为候选后端 |
| F-05 | 全文零引 RD-044，cloth 授权来源缺失，且 backfill 的「FFI 沿 U33~U42 续号」前提未表态 | major | **采纳，正文实改**：§4.D2 增「授权来源」段，引 RD-044 go 行与 M72，明记 backfill 字面不改、自有 solver 下该 FFI 前提「不被消费」而非被否定 |
| F-06 | §1 第 6 条常态授权「经预算隔离的可选 GPU 副求解器」，与 §3.2 自相矛盾并越过 RD-043 与 0017 禁止线 | major | **采纳，正文实改**：删除该授权，改为「GPU 只承担非权威特效副轨」，任何 GPU 刚体求解须 RD-043 触发 + 矩阵 §12 + 独立 Full RFC |
| F-07 | §6.4 混用大小写两套 key（M66-68 小写、M72 大写），且 MAP 的 M68 key/脚本与 CI_GATES 不同 | major | **采纳，正文实改**：§6.4 统一为 canonical 小写 key 并声明事实源；上游三份文档已统一并加机器锁 |
| F-08 | §4.D2 只列四项，MAP 要求五项（schema/导入/碰撞/LOD/独立时间线） | major | **采纳，正文实改**：逐字照抄五项并要求五个独立布尔字段 |
| F-09 | §6.4 的 M67 判据以「smoothing 不改权威状态」替换 MAP 的「逐帧满足冻结 bound」 | major | **采纳，正文实改**：两项并列，bound 指向 §6.5 |
| F-10 | capture 身份未含 `SyncBudget`/接触 ring 容量，而在树语义是事件受预算截断、未消费条目跨 tick 留存 → 换预算即改事件 digest | major | **采纳，正文实改**：§4.A1 环境头增「预算画像」，journal 增饱和计数与 tick 末 ring backlog |
| F-11 | 「determinism flags」过笼统：在树逐位断言只在 `job_threads=Some(1)` 成立，vendor `CROSS_PLATFORM_DETERMINISTIC=OFF` | major | **采纳，正文实改**：§4.A1 增「determinism 画像」显式列 `job_threads`、job system 种类、该 CMake 选项与精度/层位宽，corpus 固定线程画像 |
| F-12 | 故障注入未定义注入点，也未限定被注入字段必须参与该 tick hash → 被吸收/延后可见时误红 | major | **采纳，正文实改**：§4.A3 明定注入作用于该 tick 活世界 step 前状态，字段取自参与 hash 的白名单 |
| F-13 | §4.B1 放弃跨平台 bitwise，但 MAP M67 要求最终 hash 等于服务端，未规定两端同 build/同平台 | major | **采纳，正文实改**：§4.B1 增「hash 收敛的适用画像」段，跨画像只做 tolerance/invariant 且不得充绿 |
| F-14 | M69/M70/M71/M73 无 gate key、无 evidence subject，而波次退出门与矩阵 §11.3 要求闭环证据；M73 亦不在已 go P1 三行内 | major | **采纳，正文实改**：§6.4 增「无 P0/P1 gate 的面」段，给四个波次级 subject 名，并明记不产 P0/P1 PASS、不计入 8a 已 go P1 回归 |
| F-15 | §4.A4 只写失败臂止损；采纳 5.6 臂缺 corpus 迁移与门再基线，而 CI_GATES m66 GREEN 判据字面钉「Jolt 5.3」 | major | **采纳，正文实改**：§4.A4 增第 7 条采纳臂三件事（corpus 迁移 + m66 在 5.6 重跑 + CI_GATES/MAP 判据修订后才改版本字面），缺一即视为未采纳 |
| F-16 | §1 ASCII 图把 6b/6c/6d 挂在「any hard failure → pin 5.3」分支下，成功臂无下游波次 | major | **采纳，正文实改**：图改为 A/B 两臂并列、下游波次与 A/B 结论同级共用 |
| F-17 | N=100 逐位重跑未限定只覆盖 `active_transforms`，未记 contact `impulse` 恒 0.0 的 JoltC 子缺口 | minor | **采纳，正文实改**：§2.1 增「在树 N=100 重跑的确切覆盖面」段并引 `VENDOR.md` 子缺口 |
| F-18 | out-of-scope 把 MPM/FLIP、神经变形、可微物理并列且无 RD 归属，而上游要求 Differentiable 归 RD-042 | minor | **采纳，正文实改**：§2.4 该行逐项标 RD 归属，可微物理明确归 RD-042 不进 RD-044 |
| F-19 | 「5.3↔5.6」无实测锚（tag/commit/发布日期/vendor 可得性） | minor | **采纳，正文实改**：§4.A4 第 5 条补 v5.6.0（R2 §3.2 记 2026-07-11 发布）并要求实现期按实测 tag/commit 登记 |
| F-20 | §5.1 `SchemaHeader` 与 §4.A1 capture header 字段重叠未声明包含关系；「Jolt ABI」与「JoltC ABI digest」同物异名 | minor | **留痕不改正文**：§4.A1 已明列 capture header 全部字段，§5.1 是所有物理 schema 的共同头；两者关系为「capture header ⊇ SchemaHeader + 环境/预算/determinism 扩展」。字段名统一为 `JoltC ABI digest` 属实现期 schema 落盘时的机械对齐项，记录于本表，不在 G8.1 扩正文 |

**评审者对跨文档矛盾的移交**：gate key/脚本双源 → 已统一并加机器锁；冻结 bound 位置 → 由本 RFC §6.5 承担；五纪律字面 → 已改为逐字引用 + 附加约束分离；恢复层可得性 → 已由 §2.1.1 显式化；GPU 副求解器 → 已删除；cloth solver 归属 → Q4 已裁决且不改 RD-044 字面；上游 Jolt 版本事实（0017 记「上游已至 5.5 系」vs R2 记 5.6.0 已发布）→ 本 RFC §4.A4 以 R2 为较新锚，0017 的历史陈述不回写。

## 10. 稳定化与 provenance

- RFC Agent Approved 后仍须等 G-G8-3 真实开放实现；批准本身不改变 `implementation_status: blocked`。
- 实现采用 schema-first/RED-first：结构化 schema + invalid/migration/replay fixture 先落，再实现；每个数字来自真实命令输出。
- stable 资产面须完成版本迁移 golden、两个里程碑无重大修订与 stabilization report；vendor 私有 blob 不进入 stable 跨版本承诺。
- 起草 provenance：`Assisted-by: Codex:gpt-5 rfc21-drafter-session`。
- 未来评审 provenance 必须不同，并在 §9.1 逐条 disposition 后方可追加 Agent approval 修订行。

## 11. 规范与实现依据

- [RFC-0017](0017-engine-physics.md)：Jolt 5.3 主物理、FFI/unsafe 边界、五条同步纪律、Rapier/特效副轨与 GPU 主刚体否决。
- [G8_PLAN](../milestones/g8/G8_PLAN.md) v1.2：governance/implementation 双门、G8.6a~d 波次和 replay-first 顺序。
- [G8_CONTRACT](../milestones/g8/G8_CONTRACT.md)：G-G8-3 实现互锁、G-G8-8A~8D 物理退出门。
- [G8_CAPABILITY_MATRIX](../milestones/g8/G8_CAPABILITY_MATRIX.md)：M66~M73 当前能力缺口与 P0/P1 分级。
- [G8_ACCEPTANCE_MAP](../milestones/g8/G8_ACCEPTANCE_MAP.md)：M66/M67/M68/M72 独立 symbolic gate 与 evidence 判据。
- [R2 Physics/Chaos/Jolt 调研](../milestones/g8/research/R2_PHYSICS_CHAOS_JOLT.md)：Chaos 产品层对照、Jolt 5.x 能力、网络/破坏/角色/布料缺口与五项验收建议。
- [04_DESIGN_PRINCIPLES](../04_DESIGN_PRINCIPLES.md) P-01/P-05/P-09/P-11/P-12/P-13；[10_GOVERNANCE](../10_GOVERNANCE.md) §3/§7；[14_ENGINEERING_DISCIPLINE](../14_ENGINEERING_DISCIPLINE.md) §1/§4/§5/§6。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-02 | G8.1 governance-only 初稿：冻结 M66 replay-first、M73 5.3↔5.6 A/B 止损、M67 network physics、M68 destruction、M69/M70/M71/M72 产品层、五条 G6 纪律、多时间域 identity 与 CPU 权威/GPU 禁止线；§9.1 留空待独立 provenance 评审；零实现编号 claim。起草 provenance `Codex:gpt-5 rfc21-drafter-session`。 | Full RFC（Draft） |
| v1.0 | 2026-08-02 | **Agent Approved**：D-409 独立 provenance（`Kiro:claude-opus-5` ≠ 起草 `Codex:gpt-5`）三镜头评审完成，20 findings 全 disposition。正文实改要点：§2.1.1 JoltC ABI 缺口与三选一处置（M66/M71 开工前置）、§3.1 五纪律逐字引用 + §3.1.1 附加约束分离、头部认领 RFC-γ 身份 + §6.5 bound 三步冻结程序、§6.2 feature 名即日冻结、Q2/Q4 由本 RFC 裁决（cloth = 自有 XPBD）、删除 GPU 可选副求解器授权、§4.A1 补 determinism/预算画像、§4.A3 注入点与字段白名单、§4.A4 补 5.6 采纳臂三件事与 v5.6.0 锚、§4.B1 hash 收敛画像、§6.4 统一 canonical key 并登记 M69/M70/M71/M73 的波次级 subject、§4.D2 五项逐字与 RD-044 授权来源、§2.4 out-of-scope RD 归属。零 RXS/CI/RD/U/RX 数字 claim；批准不解锁实现。 | Full RFC（Agent Approved） |
