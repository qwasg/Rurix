# G9-R8 — 物理引擎建造期（深度调研）

> **所属**：G9 文档集（`milestones/g9/`）——本文是 [G9_PLAN.md](../G9_PLAN.md)（待立项）、[G9_CAPABILITY_MATRIX.md](../G9_CAPABILITY_MATRIX.md)（待立项）与 [design/G9_D5_PHYSICS.md](../design/G9_D5_PHYSICS.md)（D5 物理引擎建造期设计草案，v0.1 DRAFT）的调研输入之一。G9 未立项，本文不构成契约/验收承诺。
>
> **与 G8-R2 的关系（增量边界）**：G8 已有 [R2_PHYSICS_CHAOS_JOLT.md](../../g8/research/R2_PHYSICS_CHAOS_JOLT.md) = UE5 Chaos/Jolt 全景基线（Chaos 能力矩阵、Jolt 5.x 缺口、2023–2026 物理前沿通览）。本文 **R8 = R2 之后的增量**，只覆盖五路联网调研中「物理引擎建造期」一路的六条专向线索：Chaos Field System 三层抽象、异步物理与确定性、解析浮力与流体耦合边界、Jolt 5.6、Rapier 2025–2026、神经变形研究轨。**本文不重复 R2 已覆盖的 Chaos 全景、Jolt 5.5 及以前版本能力、XPBD/MPM/可微物理基础结论**（可微物理 R2 已判定观察维持，本文不重开）。
>
> **事实基线**：本文结论以 [design/G9_D5_PHYSICS.md](../design/G9_D5_PHYSICS.md) 草案正文引用的「调研 1~6」为准；本文是这些调研结论的正式化与来源落盘，不回写、不修订草案；凡草案陈述与来源细节有出入处，以草案为准并在文中标注。
>
> **调研基准日与访问日期**：2026-08-08；**调研方式**：联网检索（一手来源优先：Epic 官方文档、GitHub/dimforge 官方发布页、arXiv/Eurographics/CGF 论文页），全部结论附来源 URL；个别未能独立复核的陈述在对应小节与参考来源条目中显式标注。
>
> **纪律**：零编号占用——本文不新设任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用既有编号（RFC-0017/0021、RD-042/043/044、M74/M75/M77/M65b 等）；G8 已 closed，其契约与判据字面 **0-byte 改动**，本文一切处置建议均须待 G9 立项程序与 RFC 修订后才生效。

---

## 目录

1. 结论摘要
2. Chaos Field System 三层抽象（调研 1）
3. 异步物理与确定性工程（调研 2）
4. 浮力解析路径与流体耦合边界（调研 3）
5. Jolt 5.6 升级面（调研 4）
6. Rapier 2025–2026 演进（调研 5）
7. 神经变形研究轨（调研 6）
8. 对 G9-D5 的判定清单
9. 附：与 G8-R2 的增量边界对照
10. 参考来源
11. 修订记录

---

## 1. 结论摘要

六条调研线索共同回答一个问题：G9 物理引擎建造期（D5）把 G8 defer/no-go 的四锚（M74 Field / M75 异步 / M77 浮力 / M65b Rapier）从留档转成建造时，工业界与学术界各自提供了什么可照搬的架构与什么必须避开的坑。

六条线索与 D5 草案决策表一一对应：调研 1 → D5-1/2/3/13，调研 2 → D5-4/5，调研 3 → D5-6/7，调研 4 → D5-9/10/14，调研 5 → D5-8，调研 6 → D5-12；汇总判定见 §8。

- **Field 系统**：UE Chaos Field 是唯一大规模生产验证的「空间函数 → 物理求解」统一抽象，其三层解耦（场定义/作用对象/目标语义）、三生命周期（Transient/Construction/Persistent）与 Chaos Particles 统一作用对象面可直接对标；但 UE 社区公认「场默认影响所有 Actor」是大坑，过滤必须一等公民、默认无影响。[S01][S02][S03][S04]
- **异步与确定性**：固定步长是确定性的必要非充分条件；异步物理与 lockstep 根本冲突，生产答案是双通道而非全异步。[S07][S08][S09]
- **浮力**：解析近似（浸入体积/浸没质心 → 浮力 + 浮力矩 + 阻力 impulse）是唯一生产可行路径；真双向流体-刚体耦合经 CGF 2025 STAR 判定仍研究级。[S10][S11][S12]
- **Jolt 5.6**（2026-07）：新摩擦模型（平均接触点）对 RuriX 的确定性 corpus 有直接价值；GPU compute 接口只许评估、不碰 GPU 主刚体禁止线。[S13][S14]
- **Rapier 2025–2026**：性能面（新 BVH/persistent islands/manifold ≤4/大堆叠 25% 提速）值得对标基准；glam 迁移定锚 rust-gpu 是「Rust→GPU」路线的行业佐证，但 RuriX 侧 GPU 主刚体禁止线不变。[S15][S16]
- **神经变形**：全部路线仍是研究轨；Hybrid Neural-MPM 的「NN 只做加速近似、物理骨架兜底」与 RuriX 哲学同构、风险最低，但共性短板（训练语料依赖/泛化退化/无碰撞保证/无法双向耦合）决定其不占主线门。[S17][S18][S19][S20][S21][S22]

---

## 2. Chaos Field System 三层抽象（调研 1）

### 2.1 调研内容

**三层解耦**。UE Chaos 的 Physics Field System 在结构上把「场」拆为三层：场定义层（空间标量/向量函数节点图 + 元数据）、作用对象层（场作用于谁）、目标语义层（场对对象做什么——`EFieldPhysicsType` 枚举：LinearForce / Velocity / Torque / Strain / Sleeping / Disabled / CollisionGroup 等）。官方参考指南明确列出三类主字段类型与各自语义。[S01][S02][S06]

**Chaos Particles 统一抽象**。Chaos Particle 是带位置/速度/质量（可扩展方向、角速度、惯量、几何）的空间点，刚体、Geometry Collection 碎块、布料顶点、Rigid Body Animation（RBA）节点在求解器内统一表现为 Chaos Particles——这是 Field 系统作用对象层得以统一的前提，也是目前唯一大规模生产验证的跨域物理对象统一抽象。[S02]

**三生命周期**。官方参考指南定义三类字段：Transient（函数/事件调用内创建-执行-销毁）、Construction（构造脚本创建、编译后存储，典型 = Anchor Field）、Persistent（跨时间存活）。[S02] Persistent 语义经 Blueprint API 文档进一步确认：`AddPersistentField` 派遣的命令「在时间上持久存活，直到组件销毁或 `RemovePersistentFields` 被调用」——即 persistent 字段**必须可显式注销**；注销点是状态演化的一个事件，对任何 replay/journal 体系而言注册与注销都必须进入可回放命令流，否则回放必分叉。[S03]

**过滤一等公民——「场默认影响所有 Actor 是 UE 社区大坑」**。UE 社区论坛的反复反馈证实：Chaos 预置字段（如默认 Sleep/Disable 蓝图）默认影响一切刚体，开发者必须显式克隆并改造过滤逻辑才能限定作用范围。[S04] 这不是个别用户误用，而是默认语义的结构性问题——场定义若不把「作用于谁」作为定义的一等部分，就会在大型关卡里产生难以排查的隐性耦合。

**World-Field 通道（物理→VFX 反向通信）**。UE 的 World Physics Fields 是全局 int/float/vector 通道容器，其数据可被 Chaos、蓝图与 Niagara 发射器查询——物理世界状态以只读容器形式暴露给 VFX 侧消费，构成物理→VFX 的反向通信通道。[S05]

### 2.2 对 G9/D5 的判定含义

- **D5-1 / D5-2 / D5-3 成立**：M74 go，按三层解耦建造；过滤默认空集匹配 = 无影响（拒绝「默认全影响」语义）；persistent 注册/注销/变更全进 command journal 且可显式注销。调研 1 是这三条裁决建议的直接依据。
- **D5-PV（统一 particle view）以 Chaos Particles 为对标模板**：刚体/布料顶点/碎块/ragdoll 节点统一寻址，是 Field 作用对象层的唯一现实模板。
- **World-Field 通道在 RuriX 的唯一合法出口 = Physics→GpuScene 桥只读 buffer**（RFC-0017 纪律 1 不变）：UE 的 Niagara 查询模式在 RuriX 须翻译为「场采样参数按 tick 提交 GpuScene 只读 buffer，渲染侧自求值、不回写」（D5-13）。
- RuriX 加性扩展 `FieldPhysicsType::Buoyancy`（见 §4）：Chaos 没有浮力语义位，D5 借用同一求值管线加枚举位，是对抽象正确性的第二个生产检验。

---

## 3. 异步物理与确定性工程（调研 2）

### 3.1 调研内容

**固定步长是确定性的必要非充分条件**。Replay 体系下的物理非确定性有四个主要来源：可变步长、未播种随机、异步加载时序、多线程乱序；只锁步长不足以保证回放一致。[S08] UE 侧的实战事故（Bugnet 2026）进一步证实：async physics tick 的 step override 破坏确定性——substepping 用可变 delta 切分时，两次运行间回放必分叉，修法是锁死固定步长并关掉可变 substep 路径；类 Enhanced Determinism 的打包项（关 sleep 随机性、钉死 BVH-refit 策略、钉死线程数等）缺一即分叉。[S07]

**异步线程与 lockstep 根本冲突**。UE 官方文档把 Asynchronous Physics 模式定位为「改进模拟确定性、使结果可预测」的运行模式——但其本质是物理在独立线程/独立时间轴上推进，与 gameplay 帧解耦；一旦要求「逐 tick 可回放、可网络回滚」的 lockstep 语义，异步时间轴与权威时间轴就必须显式同步，否则双源事实。[S09]

**生产实践 = 双通道**。综合 UE async 模式的存在理由（装饰性、可丢弃对象吃满多核）与 lockstep 的硬性需求（gameplay-critical 对象必须回放逐位一致），工业界可行解不是「全异步」也不是「全同步」，而是按对象语义划分 lockstep-deterministic 通道与 async-decorative 通道，后者零回写承诺。

**「先测 Jolt 单线程成本再定异步化范围」**：异步化的动机只能是 measured 的主线程物理超预算证据；没有测量就拆通道，是在为不存在的瓶颈付架构税。

### 3.2 对 G9/D5 的判定含义

- **D5-4（M75 有条件 go）成立**：双通道架构；lockstep 通道永不异步化；async-decorative 零回写（类型层无 API + 运行时不变量双断言）；采纳以 P-6 单线程成本测量为硬前置，测量不足维持 G8 no-go 留档。
- **D5-5（确定性打包为画像）成立**：`deterministic_profile` 运行时断言（固定 dt 锁死 + substepping off + sleep/BVH 策略钉值 + 线程数与画像一致），并入 capture header determinism 画像（RFC-0021 §4.A1 扩展须走修订）；验收门含负例 RED 臂（变步长/可变 substep/画像外线程数运行必须 fail-closed）。
- 时间域纪律：async 通道自有 `DecorativePhysicsTickId`，禁止冒充 `PhysicsTickId`（RFC-0021 §3.3 多时间域扩展）。

---

## 4. 浮力解析路径与流体耦合边界（调研 3）

### 4.1 调研内容

**生产路径 = 解析近似**。Bajo 等人的「Realistic Buoyancy Model for Real-Time Applications」（Computer Graphics Forum 39(6), 2020，Eurographics 系出版面）给出实时应用浮力模型：按阿基米德原理，由浸入体积计算浮力、由浸没质心计算浮力矩，并叠加线性/角水阻力 impulse，全部以逐 tick 力/冲量形式施加入既有刚体求解。[S10] 该模型对细长物体与自由翻滚物体的稳定性表现，优于 UE 官方 Water/Buoyancy 的 Pontoon 采样点方案（Pontoon = 在船体上放置若干球形采样点、逐点测浸没深度求合力，采样点布局敏感、对翻滚姿态容易失稳）。（注：「优于 Pontoon」的对比结论引自 D5 草案转述，本轮检索未能独立复核该对比的原始出处；两个方案的各自存在与定义分别由 [S10][S11] 证实。）

**浮力应走 Field 通道**。水体区域本质是「空间函数（水面高度场）→ 对落入区域的物理对象施加语义化力」——与 Field 系统的三层结构天然同构：水面函数是场定义层基元，浸入体是作用对象，Buoyancy 是目标语义位。这与调研 1 的判定（Field 统一抽象需要第二个真实用户）互为印证。

**真双向流体-刚体耦合仍研究级**。Holz 等人的 CGF 2025 STAR「Multiphysics Simulation Methods in Computer Graphics」（Computer Graphics Forum 44(2)）系统梳理了多物理模拟（含流体-固体双向耦合）的方法谱系：FLIP/MPM/SPH 路径上的真双向耦合（流体与刚体互施力/冲量）在方法上已成熟于离线/影视管线，但尚无商业引擎 gameplay-critical 场景的生产使用证据。[S12]（「无商业 gameplay-critical 使用」为 D5 草案基于该 STAR 与生态观察的判定转述。）

### 4.2 对 G9/D5 的判定含义

- **D5-6（M77 go）成立**：解析浮力模型，走 Field 通道（persistent field + `analytic-surface` 基元 + `FieldPhysicsType::Buoyancy`），确定性内置（固定 dt + 解析水面函数、禁帧率相关插值/墙钟相位、全部输入输出进 journal），fixture 入 capture/replay corpus。
- **D5-7（真双向耦合排除主线）成立**：CGF 2025 STAR 的研究级判定写入契约 out-of-scope；浮力只做解析近似。
- 验收门「走 Field 通道（非旁路 API）」是硬判据——防止浮力长成第二套空间影响管线。
- 形状支持分层：convex/primitive 解析 clip；任意 mesh 走离线预计算 voxelized volume table（cooked artifact 版本化）。

---

## 5. Jolt 5.6 升级面（调研 4）

### 5.1 调研内容

Jolt Physics 5.6.0 于 2026-07 发布，属重大版本，官方 Release Notes 与发布报道确认的变更包括：[S13][S14]

- **GPU compute shader 接口**：新增在 GPU 上运行 compute shader 的接口，提供 DX12 / Vulkan / Metal 实现（`JPH_USE_DX12`/`JPH_USE_VK`/`JPH_USE_MTL`/`JPH_USE_CPU_COMPUTE` 可关）。
- **GPU strand 毛发模拟**：基于 Cosserat 杆的 strand hair，跑在 GPU 上；支持长程 attachment 约束、guide/render 毛发、栅格化平均速度处理 hair-hair 碰撞、环境碰撞（限 ConvexHull/CompoundShape）、发根蒙皮；官方自标 work in progress。
- **新摩擦模型（平均接触点）**：不再对每个接触点施加摩擦，而是计算平均接触点并在该点施加 2 个线性约束 + 1 个角约束；官方数字 = Pyramid 测试 **快 15%、省 40% 内存**，且**不再偏向接触 manifold 的第一个点**（消除接触点序偏向）。
- **HeightField 16bit**：`HeightFieldShape::mBitsPerSample` 上限提至 16，heightfield 更贴近未压缩高度值。
- **glTF `KHR_physics_rigid_bodies` 约束马达支持**：新增 `ESpringMode::MassNormalizedStiffnessAndDamping` 与 `EMotorState::PositionAndVelocity`。
- **`Ragdoll::DriveToPoseUsingMotors` 新变体**：用位置 + 速度双通道驱动到目标姿态。
- **整体**：官方口径 up to **40% 性能提升、70% 内存削减**（均 scene dependent）。

### 5.2 对 G9/D5 的判定含义

- **D5-9（评估门）成立**：按 RFC-0021 §4.A4 七步程序做 5.3→5.6 A/B；G8 已建成 corpus，评估窗正式开启；采纳臂三件事（corpus 显式迁移 + replay 门重跑 + 判据字面经修订才改版本号），失败臂钉 5.3。
- **新摩擦模型是 A/B 重点项**：消除接触点序偏向直接消除一类确定性噪声源，对 corpus 有实测价值；但求解器语义变化，须逐字段 exact/tolerance/invariant 分类。
- **GPU compute 接口只评估不接权威**（GPU 主刚体禁止线，D5-10，0-byte 重申）：接入须 RD-043 触发 + 矩阵 §12 重审 + 独立 Full RFC。GPU strand 毛发只作 async-decorative 副轨候选观察，非 D5 主线。
- **layout 探针工具化（D5-14）**：5.6 结构面变动（新 Settings 字段、`mBitsPerSample` 语义）要求所有 `*Settings` 结构 sizeof/offsetof 静态断言重跑；`DriveToPoseUsingMotors` 采纳前须 JoltC C 面审计（当前 pin 无该导出）。

---

## 6. Rapier 2025–2026 演进（调研 5）

### 6.1 调研内容

Dimforge 官方年度复盘（2026-01-09）确认的 Rapier 2025 主线变更：[S15]

- **新 Dynamic BVH**（parry#361）：支持高效自动再平衡与 SIMD 加速遍历，同时用于场景查询与 broad-phase，取代原 Hierarchical Sweep-and-Prune，消除双加速结构维护。
- **sparse voxel collider**（parry#336）：自称首个显式支持 voxel 的通用刚体引擎；单 voxel 接近 1 字节内存、无 ghost collision、自动块合并加速碰撞检测。
- **persistent islands**（rapier#895）：模拟岛跨帧持久化，免除每帧重建碰撞图连通分量。
- **manifold 缩减 ≤4 接触**（rapier#895）：约束求解器处理的 contact manifold 永不超过 4 个接触点，求解器代码简化。
- **简化 3D 摩擦模型**（rapier#876）：≥2 接触的 manifold 减少求解约束数，**大堆叠等高接触场景 25% 提速**。
- **nalgebra→glam 迁移（rapier 0.32 / parry 0.26 起）**：99% 公开 API 转为 glam；官方明述动因二条 = 游戏/图形社区采纳度 + **rust-gpu 编译器后端对 glam 一等支持**（nalgebra 内部复杂度 rust-gpu 编不动）；性能上 release 不变、debug 约快 20%。multibody 与求解器 AoSoA 两处仍留 nalgebra。
- **GPU 路线定锚 rust-gpu**：官方复盘记录 WGSL（wgmath/wgrapier）→ Slang（Slosh）→ rust-gpu 的探索路径，最终定锚 rust-gpu 做跨平台 GPU 刚体物理；2026 年 Q2 技术报告进一步宣布 Nexus（rust-gpu 跨平台 GPU 物理引擎）已跑起来。[S15][S16]

**战略信号**：Dimforge 把「Rust→GPU」从实验转为正式路线，佐证 RuriX「Rust 物理 → GPU」技术路线的行业合理性——仅作路线佐证留档，不构成任何接入承诺。

### 6.2 对 G9/D5 的判定含义

- **D5-8（M65b 维持条件制）成立**：对标基准先行（新 BVH / voxel collider / 摩擦模型 / 大堆叠，与 Jolt 同场景同输入同 determinism 画像）；产出 measured 报告，不作 replay oracle（跨 solver 只作不变量/容差对拍，RFC-0021 §7 备选 D）。
- **RD-044 字面不变**：「快路径被真实 workload 采用时」才深造；D5 的 Field 高频查询与 async-decorative 次级刚体是首个真实候选消费方，基准有 measured 优势才按程序申请判档，否则维持 no-go。
- **glam 迁移兼容**：Rapier 0.32+ 对 `src/rurix-physics/src/rapier.rs` 快路径封装的 API 冲击须评估并留档兼容层设计；不承诺 bitwise 不变。
- Rapier 的 rust-gpu 刚体方向在 RuriX 受 GPU 主刚体禁止线约束，仅观察。

---

## 7. 神经变形研究轨（调研 6）

### 7.1 调研内容

2026 年前后骨骼驱动/物理化神经变形三条代表线：

- **PhysSkin**（CVPR 2026 Highlight；D5 草案写作 "PhySkin"）：自监督神经蒙皮框架，从静态 3D 几何直接学习连续 skinning weight 场，handle 变换子空间 → 全空间变形，mesh-free、离散无关，实时且跨形状泛化。[S17]
- **HyperBones**（2026）：hypernetwork 条件化的骨骼驱动神经服装模拟，训练 reduced-space 神经动力学模拟器，实时且保持物理合理。[S18]
- **UNIC**（2026）：实例专属神经变形场驱动服装动画，按动作序列实时变形；实例专属训练换取变形质量，不要求泛化到新服装。[S19]

更早与混合路线：

- **NeuralClothSim**（NeurIPS 2024）：准静态布料模拟重铸为 Kirchhoff-Love 薄壳理论监督下的神经变形场（NDF），连续坐标表示、显存高效。[S20]
- **Subspace Neural Physics**：NN 学习子空间动力学，含外力与碰撞的变形模拟比标准离线仿真**快 300×–5000×**。[S21]
- **Hybrid Neural-MPM**（2025）：NN 与经典 MPM 求解器混合——NN 承担求解的高耗近似段，误差越界即 fallback 到经典数值求解器兜底；同时取得低延迟与高物理保真。[S22]

**共性短板**（四者一致）：训练语料依赖（语料外输入质量退化）、泛化退化（UNIC 甚至明示不泛化）、无碰撞保证（Subspace Neural Physics 的碰撞是学出来的近似，无可证保证）、无法双向耦合（NN 变形不反作用于权威物理世界）。

### 7.2 对 G9/D5 的判定含义

- **D5-12（研究子轨，无主线门）成立**：混合架构优先（Hybrid Neural-MPM 同构：NN 只做加速近似，权威物理骨架兜底）；「NN 权威禁止线」建议与 GPU 禁止线同构进 RFC 修订面——任何 NN 输出不得替代权威状态。
- **骨骼驱动神经服装先做离线工具链形态**：离线仿真 → 神经压缩 → 运行时回放，复用 G8 capture/replay 设施（corpus 即训练语料生产器、replay 设施即回放校验器）——D5 与 G8 设施的最大协同点。
- **PhysicsAsset 变形格式预留 residual 通道**：schema 加性扩展位（版本化、首期可空），避免日后 breaking 修订。
- 红线写明：不承诺碰撞保证、不承诺双向耦合、不占主线验收门；可微物理不进本子轨（RD-042 观察维持，0-byte）。

---

## 8. 对 G9-D5 的判定清单

| # | 调研线索 | D5 裁决建议 | 判定状态 | 关键依据 |
|---|---|---|---|---|
| 1 | Chaos Field 三层抽象 | D5-1/2/3：M74 go；过滤默认空匹配；persistent 全 journal 化可显式注销 | 调研证实，建议维持 | [S01][S02][S03][S04] |
| 2 | 异步物理与确定性 | D5-4/5：双通道 + `deterministic_profile` 断言；P-6 测量为判档硬前置 | 调研证实，建议维持 | [S07][S08][S09] |
| 3 | 浮力与流体耦合 | D5-6/7：解析浮力走 Field 通道 go；真双向耦合排除主线 | 方案各自证实；「优于 Pontoon」对比与「无商业使用」判定引自草案转述（§4.1 标注） | [S10][S11][S12] |
| 4 | Jolt 5.6 | D5-9/10/14：A/B 评估门；GPU 只评估不接权威；layout 探针工具化 | 调研证实，建议维持；官方数值（15%/40%、40%/70%）已复核 | [S13][S14] |
| 5 | Rapier 2025–2026 | D5-8：基准先行、RD-044 条件制不变 | 调研证实，建议维持；25% 提速数值已复核 | [S15][S16] |
| 6 | 神经变形 | D5-12：研究子轨，混合架构优先，不占主线门 | 调研证实，建议维持；300–5000× 数值已复核 | [S17][S18][S19][S20][S21][S22] |

**横切红线（0-byte 重申，无需裁决）**：GPU 主刚体禁止线（D5-10）、可微物理排除（D5-11）、G8 已 closed 契约与判据字面不改动、本文不回写 D5 草案。

---

## 9. 附：与 G8-R2 的增量边界对照

为避免与 R2 内容重叠，读者按下表定位：

| 主题 | R2（G8，2026-08-02） | R8（本文） |
|---|---|---|
| UE Chaos 全景能力矩阵 | §2 完整覆盖 | 不重复 |
| Chaos Field System 三层抽象/生命周期/过滤/World-Field | 未专节展开 | §2 |
| Jolt 5.5 及以前能力、确定性构建、StateRecorder | §3、S38/S39 覆盖 | 不重复 |
| Jolt 5.6.0（2026-07） | 仅版本说明行提及 | §5 全特性展开 |
| 异步物理与 replay 确定性工程 | 未专节展开 | §3 |
| 浮力解析模型与流体耦合边界 | 仅列 Jolt ApplyBuoyancyImpulse 符号（S37） | §4 |
| Rapier 2025–2026（BVH/voxel/islands/glam/rust-gpu） | 未覆盖 | §6 |
| 神经变形（PhysSkin/HyperBones/UNIC/Hybrid Neural-MPM 等） | 仅 NeuralClothSim（S56）一点 | §7 全线 |
| XPBD/布料、MPM、可微物理、Newton 等前沿通览 | §4 完整覆盖 | 不重复（可微物理维持 R2 观察判定） |
| G6 裁决一致性（Jolt 生产默认/Rapier 快路径/Taichi 副轨/GPU 主刚体否决） | §5 判定「维持」 | 0-byte 重申，不重开 |

---

## 10. 参考来源

> 全部 URL 经 2026-08-08 当日检索/抓取定位；访问日期均为 2026-08-08。

- **[S01]** Chaos Fields User Guide in Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-fields-user-guide-in-unreal-engine （访问日期 2026-08-08）
- **[S02]** Reference Guide for Physics Field in Unreal Engine（Epic 官方文档：Chaos Particle 定义 + Transient/Construction/Persistent 三类字段）：https://dev.epicgames.com/documentation/unreal-engine/reference-guide-for-physics-field-in-unreal-engine （访问日期 2026-08-08）
- **[S03]** AddPersistentField（Epic 官方 Blueprint API 文档：persistent 命令存活至组件销毁或 RemovePersistentFields 调用）：https://dev.epicgames.com/documentation/en-us/unreal-engine/BlueprintAPI/Field/AddPersistentField （访问日期 2026-08-08）
- **[S04]** Chaos fields affect all physics actors（Epic Developer Community Forums 讨论串：默认字段影响一切刚体的社区反馈）：https://forums.unrealengine.com/t/chaos-fields-affect-all-physics-actors/651778 （访问日期 2026-08-08）
- **[S05]** World Physics Fields（Epic Developer Community 官方教程：全局 int/float/vector 通道容器，Chaos/蓝图/Niagara 可查询）：https://dev.epicgames.com/community/learning/tutorials/Y5p7/unreal-engine-world-physics-fields （访问日期 2026-08-08）
- **[S06]** Overview of Physics Fields in Unreal Engine（Epic 官方文档：运行时作用于指定空间区域的 Chaos 物理场系统）：https://dev.epicgames.com/documentation/unreal-engine/overview-of-physics-fields-in-unreal-engine?lang=en-US （访问日期 2026-08-08）
- **[S07]** Fix: Unreal Async Physics Tick Step Override Violates Determinism（Bugnet，2026-07-30：可变 delta substepping 破坏回放确定性，须锁固定步长）：https://bugnet.io/blog/fix-unreal-async-physics-tick-stepoverride-violates-determinism （访问日期 2026-08-08）
- **[S08]** How to Debug Non-Deterministic Physics in Replay（Bugnet，2026-04-10：非确定性四来源——可变步长/未播种随机/异步加载/多线程）：https://bugnet.io/blog/how-to-debug-non-deterministic-physics-in-replay （访问日期 2026-08-08）
- **[S09]** Physics in Unreal Engine（Epic 官方文档：Asynchronous Physics 模式定位）：https://dev.epicgames.com/documentation/unreal-engine/physics-in-unreal-engine （访问日期 2026-08-08）
- **[S10]** J. M. Bajo et al., Realistic Buoyancy Model for Real-Time Applications, Computer Graphics Forum 39(6), 2020（Eurographics 数字图书馆）：https://diglib.eg.org/bitstream/handle/10.1111/cgf14013/v39i6pp217-231.pdf?isAllowed=y&sequence=1 （访问日期 2026-08-08）
- **[S11]** Water Buoyancy Component in Unreal Engine（Epic 官方文档：Pontoon 采样点浮力方案）：https://dev.epicgames.com/documentation/unreal-engine/water-buoyancy-component-in-unreal-engine?lang=en-US （访问日期 2026-08-08）
- **[S12]** D. Holz et al., Multiphysics Simulation Methods in Computer Graphics, Computer Graphics Forum 44(2), 2025（STAR；RWTH Aachen 作者版 PDF）：https://animation.rwth-aachen.de/media/papers/93/2025-CGF-STAR-Multiphysics.pdf （访问日期 2026-08-08）
- **[S13]** Jolt Physics Release Notes（GitHub jrouwe/JoltPhysics，master 分支 ReleaseNotes.md）：https://github.com/jrouwe/JoltPhysics/blob/master/Docs/ReleaseNotes.md （访问日期 2026-08-08）
- **[S14]** Jolt Physics 5.6 Released（GameFromScratch，2026-07-13：5.6 全特性清单，含 GPU compute 接口/GPU strand 毛发/新摩擦模型 15%-40%/HeightField 16bit/glTF 马达/DriveToPoseUsingMotors/40%-70%）：https://gamefromscratch.com/jolt-physics-5-6-released/ （访问日期 2026-08-08）
- **[S15]** The Rapier physics engine 2025 review and 2026 goals（Dimforge 官方博客，2026-01-09：新 Dynamic BVH/sparse voxel collider/persistent islands/manifold ≤4/简化摩擦模型大堆叠 25% 提速/glam 迁移/rust-gpu 定锚）：https://dimforge.com/blog/2026/01/09/the-year-2025-in-dimforge/ （访问日期 2026-08-08）
- **[S16]** Dimforge Blog 索引（含「Dimforge Q2 2026 technical report − Nexus cross-platform GPU physics engine with rust-gpu」，2026-07-04 条目；该文直链抓取被 403 拒绝，经博客索引页确认条目存在与标题）：https://dimforge.com/blog/ （访问日期 2026-08-08）
- **[S17]** PhysSkin: Real-Time and Generalizable Physics-Based Animation via Self-Supervised Neural Skinning（arXiv，2026）：https://arxiv.org/abs/2603.23194 （访问日期 2026-08-08）
- **[S18]** HyperBones: Realtime Bone-driven Neural Garment Simulation with Hypernetwork Conditioning（arXiv，2026）：https://arxiv.org/html/2605.20460 （访问日期 2026-08-08）
- **[S19]** UNIC: Neural Garment Deformation Field for Real-time Clothed Character Animation（arXiv，2026）：https://arxiv.org/abs/2603.25580 （访问日期 2026-08-08）
- **[S20]** NeuralClothSim: Neural Deformation Fields Meet the Thin Shell Theory（NeurIPS 2024，MPI 项目页）：https://4dqv.mpi-inf.mpg.de/NeuralClothSim/ （访问日期 2026-08-08）
- **[S21]** Subspace Neural Physics: Fast Data-Driven Interactive Simulation（作者版 PDF：300×–5000× 加速声明）：https://theorangeduck.com/media/uploads/other_stuff/deep-cloth-paper.pdf （访问日期 2026-08-08）
- **[S22]** Hybrid Neural-MPM for Interactive Fluid Simulations in Real-Time（arXiv，2025：fallback safeguard 到经典数值求解器）：https://arxiv.org/html/2505.18926v1 （访问日期 2026-08-08）

---

## 11. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-08 | 初版：五路调研之「物理建造期」路正式化落盘（G9.0 文档集输入材料；R2 后增量）。 |
