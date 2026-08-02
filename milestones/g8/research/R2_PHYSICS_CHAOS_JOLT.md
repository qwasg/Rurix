# G8-R2 — UE5 Chaos 物理全景、Jolt 缺口与物理前沿（深度调研）

> **所属**：G8 计划定稿档（`milestones/g8/`）——本文是 [G8_CAPABILITY_MATRIX.md](../G8_CAPABILITY_MATRIX.md) 与 [G8_PLAN.md](../G8_PLAN.md) 的调研输入之一。
>
> **定位**：**项目首份系统性物理调研报告**（G6 物理选型为会话级调研 + Cursor canvas，未沉淀正式报告，见 [G6_PLAN.md](../../g6/G6_PLAN.md) 上游行）；本文同时服务 RD-044（物理 P3+）的拆分裁决输入。
>
> **与既有裁决的关系**：不改写 [RFC-0017](../../../rfcs/0017-engine-physics.md) 与 G6 已收口裁决（Jolt 生产默认 / Rapier 快路径 / Taichi 副轨 / GPU 主刚体否决）；§5 一致性论证结论为「维持」。
>
> **调研基准日**：2026-08-02；**调研方式**：联网深度调研（多源交叉验证，全部结论附来源 URL）。
>
> **纪律**：零编号占用——本文不新设任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用既有编号（RD-042/043/044 等）。

---

核心对照范围：UE 5.4/5.5/5.6；补充 2026 年中最新 UE 5.8 与 Jolt 5.6.0。

> 版本说明：UE 5.8 于 2026-06-23 发布，是 UE5 路线图中最后一个计划内大版本；Jolt 5.6.0 于 2026-07-11 发布，严格说晚于“2026 年中”，但本报告纳入其最新变化。[S1][S25]

---

## 目录

1. 结论摘要
2. UE5 Chaos 完整能力矩阵
3. Jolt 5.x 能力与 Chaos 缺口
4. 2023–2026 物理前沿
5. 与 G6 已锁定裁决的一致性
6. 附：调研侧分期建议（输入性质）
7. UE5 物理能力 → Rurix 前置要求总表
8. 调研侧最终裁决建议（输入性质）
9. 参考来源

---

## 1. 结论摘要

### 1.1 总判断

Rurix 当前的 Jolt CPU 多核主物理路线没有选错。Jolt 已被《Horizon Forbidden West》和《Death Stranding 2》使用，能够支撑 AAA 级刚体、碰撞、角色、布娃娃和流式世界；Guerrilla 迁移后将物理频率从 30 Hz 提升到 60 Hz，同时报告运行时内存下降 25%、物理资产下降 30%、可执行文件下降 12%。[S17][S18]

但“UE5 级别物理引擎”不等于“拥有一个性能足够好的刚体库”。Chaos 的真正壁垒是：

1. Geometry Collection、Fracture Mode、Dataflow、Cache、Field、Niagara 联动构成的破坏生产链。
2. Panel Cloth、USD/CLO/Marvelous Designer 导入、LOD、权重图、碰撞、缓存、ML Deformer 构成的布料资产链。
3. Network Physics 的输入历史、状态快照、预测、回滚、重演、平滑与调试工具。
4. Physics Asset、ragdoll、physical animation、车辆、角色控制器的编辑器和资产化工作流。
5. 统一的物理资产图、烘焙、版本迁移、可视化调试和性能预算系统。

因此 G8 的优先级应是：

- **P0：统一物理资产与调试基础设施。**
- **P0：网络物理快照/回滚层。**
- **P0：预破碎资产 + 层级聚类破坏运行时。**
- **P1：布料资产管线和独立 CPU/GPU 副求解器。**
- **P1：车辆、ragdoll、physical animation 产品化。**
- **P2：Flesh/MPM/FLIP/神经变形研究副轨。**

不建议重启 GPU 主刚体项目。

---

## 2. UE5 Chaos 完整能力矩阵

### 2.1 版本演进

| 版本 | 关键物理变化 |
|---|---|
| UE 5.4 | Chaos Destruction 标记 Production Ready；Panel Cloth Editor 从 Experimental 升为 Beta；Networked Physics 为 Beta，其中 Predictive Interpolation 为 Beta、Physics Resimulation 仍为 Experimental。[S2] |
| UE 5.5 | Panel Cloth/Dataflow 重构、质量改进；Chaos Modular Vehicles 作为 Experimental 系统出现；Chaos Visual Debugger 增强。[S3][S34] |
| UE 5.6 | Outfit Asset、服装 resizing/refitting、cloth-to-cloth constraints、Simulation Morph Target、Unified Dataflow Editor；异步创建/销毁 physics state；Physics Replication LOD 仍为 Experimental。[S4][S35] |
| UE 5.8 | Dataflow 和 Chaos Cloth 正式 Production Ready；Dataflow 成为 Cloth、Destruction、Flesh、Hair 的统一物理资产生成框架。[S1] |

这意味着以 UE 5.6 为唯一目标会低估 Chaos 工具链：截至 2026 年中，UE5 的物理资产生产基准应按 **5.8 Dataflow/Cloth 成熟度**衡量。

---

### 2.2 刚体求解器

Chaos 是基于约束迭代的 PBD 系物理架构。公开资料明确区分：

- position iterations：消除穿透和关节位置误差；
- velocity iterations：处理摩擦、恢复系数和动量；
- projection：求解后非物理修正；
- substep：以更多 CPU 成本改善碰撞稳定性；
- UE 5.5 起支持对象级迭代预算，同一 island 采用参与对象中的最大值。[S5]

能力面包括：

- dynamic/static/kinematic body；
- island 构建与 sleeping；
- 摩擦、恢复、contact modification；
- 离散碰撞和逐 body CCD；
- fixed timestep、substep、async physics thread；
- 碰撞事件、sleep/wake、constraint break；
- 多线程 collision detection、island generation 与 solving（5.6 持续优化）。[S6][S35]

**CCD 口径：**可按对象启用，但官方 API 明确提示其昂贵；它不是所有对象默认开启的“完全连续求解”。[S7]

**Sleeping：**按 body/island 静止状态休眠，提供 sleep/wake 事件。5.6 进一步引入 partial sleeping islands 的优化信息，但公开算法细节有限。[S6][S35]

**限制：**

- projection 会注入能量，超过 tolerance 时甚至直接 teleport，仅适合错误恢复。
- Async Physics 虽改善同一执行环境中的可重复性，但不等于跨 CPU 架构的位级确定性。[S8][S9]
- Chaos 的具体 island 排序、约束着色和内部迭代实现并未在稳定官方文档中形成完整算法规范，部分细节只能从源码核实。

---

### 2.3 约束与 Joint

Chaos 的主接口以 6-DOF Constraint Instance 为中心，可组合得到：

- fixed；
- ball/socket；
- hinge；
- prismatic/slider；
- cone、swing/twist；
- 每轴 locked/limited/free；
- 线性与角度 limit；
- position/velocity motor、spring、drive；
- break force/torque；
- collision enable；
- mass conditioning；
- projection、shock propagation；
- constraint profile。[S10]

对 UE 级能力而言，关键不只是 joint 数量，而是：

- Physics Asset 中批量编辑骨骼约束；
- profile/drive 参数资产化；
- constraint break 事件；
- ragdoll 与动画姿态之间的映射；
- hero object 的每 island 迭代覆盖。

---

### 2.4 场景查询

Chaos/UE 提供完整的：

- raycast；
- shape sweep/cast；
- overlap；
- single/multi hit；
- channel、object type、response、ignore list 过滤；
- query-only、physics-only、query-and-physics、probe 等碰撞参与模式；
- closest blocking hit、touching hit、MTD；
- Physics Thread 查询和 Chaos Visual Debugger 场景查询记录。[S11][S12]

Rurix 已有并发查询基础，但 UE 级要求还包括查询批处理、历史帧查询、调试录制、稳定的碰撞过滤资产和异步查询生命周期。

---

### 2.5 Chaos Destruction

这是 Jolt 与 Chaos 最大的产品级差距。

#### 资产模型

Chaos 使用 Geometry Collection：

- 静态或骨骼网格转换为 collection；
- 预先 fracture；
- 多层 cluster；
- 每碎块质量、碰撞、材质、damage threshold；
- connection graph 表示碎块间结构连接；
- collision impulse 或 Physics Field strain 超阈值后断裂。[S13]

#### Fracture 工具链

Fracture Mode/Dataflow 支持：

- Uniform/Voronoi；
- planar；
- brick；
- mesh cutter；
- noise；
- 多级 recursive fracture；
- selection；
- auto cluster、magnet/adjacency cluster；
- anchor/kinematic；
- 清理过小碎片；
- 碰撞几何生成；
- interior material；
- 可视化层级和 connection graph。[S14]

5.8 的 Dataflow 允许非破坏式修改 fracture pattern、scatter points、cluster 和输入网格，而不是每次永久修改资产。[S1][S15]

#### 运行时

- 层级 cluster 初始作为少数刚体存在，受损后逐层释放碎块；
- size-specific damage threshold；
- break、crumble、collision 事件；
- removal/decay；
- Physics Field 控制 external/internal strain、dynamic state、kill、force、velocity、sleep threshold；
- Niagara Data Channel 可消费破坏事件生成尘埃和碎屑。[S16][S34]

#### Cache

Chaos Cache 可记录并回放复杂破坏，减少运行时求解负担；支持多个 component 的 record/playback 和时间控制。[S13]

#### 生产工作流本质

Chaos 并非默认做运行时 FEM 裂纹扩展，而是：

> 离线/编辑期预破碎 + 运行时层级刚体聚类 + strain 断键 + 缓存/事件/VFX。

这是 Rurix 最现实的 G8 路线。

#### 对 Rurix 的前置能力要求

- **A 语言/编译器：**稳定的并行 graph/cluster kernel；结构化 buffer、SoA、原子操作；离线工具可复用 Rurix kernel。
- **B 运行时：**层级 body 激活/失活、批量碎块注册、事件有界队列、物理状态流送、cache playback。
- **C 引擎库：**GeometryCollection 等价资产、connection graph、strain/damage、cluster proxy、碎块 LOD、field evaluator。
- **D 工具链：**Voronoi/plane/mesh fracture、interior face、collision cooking、cluster 编辑、anchor、可视化、缓存录制、Niagara 等价 VFX 事件桥。

---

### 2.6 Chaos Cloth、Panel Cloth 与 ML

#### Chaos Cloth

Chaos Cloth 是独立于主刚体求解的粒子/约束布料系统，提供：

- PBD/XPBD；
- stretch、bend、area、tether、skinning；
- self/environment collision；
- weight map；
- simulation mesh 与 render mesh 分离；
- LOD、skinning/deformer mapping；
- skeletal collision；
- cloth-to-cloth constraints；
- Outfit Asset 和 garment fitting。[S3][S4][S19]

XPBD 改善了普通 PBD 的时间步和迭代数依赖，但单迭代成本更高。[S5][S41]

#### Panel Cloth

Panel Cloth 是**服装授权/制作工作流**，不是单纯求解器：

- 2D pattern/panel；
- seam；
- fabric properties；
- USD 导入；
- 从 Marvelous Designer/CLO 导入 garment 和部分仿真参数；
- 自动 graph、skinning、LOD；
- Dataflow 非破坏编辑；
- resizing/refitting。[S2][S4]

UE 提供导入器不代表附带 Marvelous Designer/CLO 的商业授权。Rurix 若要达到同等生产力，需要：

1. 购买并支持外部 DCC；
2. 或建立开放格式的 panel/seam/fabric 资产标准；
3. 或投入自研服装编辑器，成本最高。

#### ML Cloth 与 ML Deformer

二者不应混为实时物理：

- Chaos ML Cloth Generation 用 Chaos Cloth 批量生成 geometry cache 训练数据；
- ML Deformer 在运行时根据姿态/输入推断顶点偏移；
- 主要作用是近似高质量褶皱、肌肉和复杂变形；
- 它是动画/网格 deformation accelerator，不负责刚体接触权威状态、网络碰撞或通用布料拓扑演化。[S20][S21]

#### 对 Rurix 的前置能力要求

- **A：**GPU scatter/gather、prefix sum、邻域和约束着色；跨 Vulkan/DXIL/PTX 一致的粒子计算；可选 autodiff/训练数据导出。
- **B：**独立 cloth timeline、固定步、skin pose 输入、cloth→render buffer 互操作、异步 compute 和预算调度。
- **C：**XPBD cloth、self-collision、tether、LOD、collision proxy、sim/render mesh mapping、cache。
- **D：**panel/seam/fabric schema、USD 导入、CLO/Marvelous Designer 验证、权重绘制、drape/preview、LOD 烘焙、训练缓存生成。

---

### 2.7 载具

Chaos Vehicles 提供：

- 任意轮数；
- wheel、axle、suspension；
- engine、transmission、differential、steering、brake；
- tire friction；
- wheel animation；
- Blueprint/Pawn movement component；
- 网络复制集成。[S22]

UE 5.5 的 Chaos Modular Vehicles 进一步将发动机、悬挂、轮、空气动力等做成可组合模块，并允许运行时组装和破坏，但当时仍是 Experimental。[S3]

UE 级载具差距主要不在 Jolt 是否能算轮胎，而在：

- 调参资产；
- telemetry；
- wheel/suspension debug；
- animation binding；
- surface material；
- damage；
- input/network prediction；
- AI 和 replay。

#### 对 Rurix 的前置能力要求

- **A：**无新增硬要求。
- **B：**车辆固定步输入、telemetry ring buffer、回滚状态、轮胎接触批查询。
- **C：**Jolt VehicleConstraint 包装、轮式/履带式模型、差速器、动力总成、车辆状态序列化。
- **D：**车辆 prefab、曲线编辑、悬挂/摩擦可视化、自动骨骼绑定、标准测试场。

---

### 2.8 角色、Ragdoll 与 Physical Animation

Chaos 角色物理包含：

- Physics Asset：骨骼对应刚体和 joint；
- full/partial ragdoll；
- RBAN；
- Physical Animation Component；
- motor 驱动至动画姿态；
- simulation space；
- profile 和 limb set；
- animation ↔ physics blend；
- Physics Control/Control Rig Physics。[S23]

Physical Animation 的核心不是简单“布娃娃开关”，而是给骨骼刚体设置目标姿态、强度和阻尼，通过 motor 追随动画，同时保留碰撞反馈。

#### 对 Rurix 的前置能力要求

- **A：**无新增核心要求。
- **B：**动画/物理双缓冲、固定步姿态采样、骨骼结果插值、重演安全事件。
- **C：**PhysicsAsset、ragdoll mapping、pose motor、partial simulation、physical-animation profile。
- **D：**骨骼 collider 自动生成、joint limit gizmo、ragdoll preview、profile 编辑和稳定性测试。

---

### 2.9 软体/Flesh

Chaos Flesh 面向四面体软体，尤其是肌肉、脂肪和角色形变：

- 低分辨率 tetrahedral runtime simulation；
- 高质量离线结果缓存；
- simulation mesh 驱动 render/skeletal mesh；
- collision、skinning/binding；
- Dataflow 资产生成。[S24]

它不是通用 BeamNG 式永久塑性车体破坏，也不是 Chaos Destruction 的替代品。公开资料仍把其主要定位放在角色 muscle deformation。

#### 对 Rurix 的前置能力要求

- **A：**稀疏邻接、tet/FEM/XPBD kernel；可选 GPU 副轨。
- **B：**低分辨率 soft state 与 render deformation bridge。
- **C：**tet mesh、material、binding、collision、cache。
- **D：**tetrahedralization、surface binding、muscle/fat authoring；建议 P3+。

---

### 2.10 Niagara Fluids 与 Water

Niagara Fluids 是 GPU VFX 仿真体系，典型能力包括：

- Grid3D gas：烟、火、密度、速度、温度；
- Grid2D/3D 数据接口；
- particle/grid stage；
- render target；
- shallow-water/液体效果模板和 Niagara/Water 联动。[S26]

定位必须明确：

- 它主要服务视觉效果；
- 不应默认作为 gameplay-authoritative 流体；
- 通常不提供刚体主求解、网络回滚或确定性保证；
- UE Water 是另一个系统，负责河、湖、海、水面网格、波浪、浮力与 gameplay ripple/wake。[S27]

这与 Rurix 的 Taichi Vulkan AOT 副轨非常接近。

#### 对 Rurix 的前置能力要求

- **A：**Vulkan compute、3D texture/storage image、原子、barrier、scan/sort、可编程 simulation stages。
- **B：**render graph external resource、async compute、预算与降级、GPU capture。
- **C：**Grid2D/3D、advection、pressure projection、PBF/MPM/浅水模块、刚体事件输入。
- **D：**VFX graph、场可视化、cache/bake、参数模板；不进入主刚体里程碑验收。

---

### 2.11 异步物理 Tick

UE 支持：

- physics thread；
- async fixed timestep；
- Event Async Physics Tick；
- substepping；
- 延后碰撞 callback 至最后一个 substep；
- physics state 异步创建/销毁。[S6]

其工程含义是：Gameplay 代码必须区分 game frame、render frame、physics frame，并处理同一显示帧中的多次接触 callback。

---

### 2.12 网络物理

UE 5.4 起 Networked Physics 提供：

- server authority；
- Default replication；
- Predictive Interpolation；
- Resimulation；
- input/state history；
- packet-loss 冗余输入和状态；
- error threshold；
- rewind、replay；
- soft snap/hard snap；
- NetworkPhysicsComponent；
- fixed-tick cue rollback；
- 5.6 Physics Replication LOD：按距离在 Predictive Interpolation 和 Resimulation 间切换，仍为 Experimental。[S2][S28][S35]

网络物理不要求完全跨平台位级确定，但要求：

1. 相同起点与输入尽量收敛；
2. 保存足够内部状态；
3. 副作用可撤销或去重；
4. 权威修正后可重演；
5. 用平滑隐藏残差。

---

### 2.13 确定性口径

Chaos 官方只称 Async Physics 可“improve determinism”，没有公开承诺跨平台、跨编译器位级一致。[S9]

因此应区分：

- **可重复固定步：**同机器、同构建、同输入；
- **rollback sufficient determinism：**误差小且可权威修正；
- **bitwise cross-platform determinism：**Chaos 未公开保证。

Rurix 不应把“UE5 级”错误解释为必须固定点 lockstep；更实际目标是固定步、状态哈希、重演验证和 server correction。

---

### 2.14 Physics Field

Field System 能以标量/向量场作用于：

- linear force/velocity；
- angular torque/velocity；
- dynamic state；
- external/internal cluster strain；
- kill；
- collision group；
- sleep/disable threshold。[S16]

它是破坏、粒子、布料和 gameplay 之间的统一空间影响描述。

#### 本章综合前置能力要求

- **A：**固定步数学约定、跨后端资源模型、并行图算法、GPU 粒子/网格原语、状态序列化布局。
- **B：**Physics Thread、frame history、回滚、事件去重、流式 body 生命周期、统一 CPU/GPU 时间线。
- **C：**PhysicsAsset、Field、Destruction、Cloth、Vehicle、Character、Network Physics 子系统。
- **D：**Dataflow 等价图、碰撞/破碎/布料 cook、调试录制、性能分析、资产迁移和版本化。

---

## 3. Jolt 5.x 能力与 Chaos 缺口

### 3.1 Jolt 现有能力全景

Jolt 是 MIT 许可、面向游戏和 VR 的多核刚体/碰撞库。[S18]

#### 刚体与并发

- sequential impulse solver + warm starting；
- island 并行；
- JobSystem；
- lock-free broadphase 设计；
- simulation、查询、body 批插入/移除可并发；
- sphere、box、capsule、cylinder、convex、compound、mesh、heightfield；
- sensor、layer filter、contact listener；
- sleeping；
- friction/restitution；
- 大世界双精度可选。[S17][S18]

#### CCD

`EMotionQuality::LinearCast` 在离散步骤后追加 shape cast，防止快速小物体穿过薄几何，但：

- 使用起始旋转做线性 cast；
- 高角速度长物体仍可能漏碰；
- 发生碰撞时采用 time stealing；
- 不会完整回退并重新积分整个 simulation step。[S29]

这是实用游戏 CCD，不是高成本的全局连续动力学。

#### Joint

Jolt 内建：

- fixed、point、distance；
- hinge、slider、cone、swing-twist；
- six-DOF；
- path；
- gear、rack-and-pinion、pulley；
- vehicle；
- motor、spring、limit；
- 每 constraint 迭代覆盖和 priority。[S30]

#### 查询

- broadphase AABB 查询；
- raycast；
- collide shape；
- shape cast；
- point query；
- collectors；
- broadphase/object/body/shape filter；
- 查询与 simulation、body update 并发。[S31]

#### Character

- Character：刚体角色；
- CharacterVirtual：基于查询的高级虚拟角色；
- stair、slope、ground state、moving platform；
- CharacterVirtual 可并行更新；
- 可加 inner body，使 sensor/普通查询可检测到角色；
- CharacterVirtual 状态需单独 SaveState。[S32]

#### Vehicle

Jolt VehicleConstraint 支持 wheel、track、engine、transmission、differential、suspension 和 tire friction。已具备技术底座，但公开文档也记录了轻物体夹在车轮与地面之间时的迭代求解伪影，通常需要更多 velocity/position steps 或质量策略。[S33]

#### Ragdoll

- 高低细节 skeleton mapping；
- hard/soft keying；
- constraint motor drive-to-pose；
- 5.5 增加根部优先 constraint priority；
- 5.6 增加 position+velocity motor 驱动和 glTF 物理 motor 支持。[S18][S25]

#### Soft Body

Jolt soft body 使用 XPBD 风格约束，支持：

- distance、bend、volume；
- skinning；
- long-range attachment；
- rigid-soft collision；
- soft-body contact listener；
- 多线程约束组；
- 可用作软球或基础布料。[S36]

当前官方架构仍列出的主要限制包括：

- 无 soft-soft/self-collision；
- 碰撞查询成本较高；
- 普通刚体 joint 不能直接约束 soft body；
- 部分 body API 语义不同；
- 不是完整服装生产系统。[S36]

5.6 新增的是基于 Cosserat rods 的 GPU strand hair WIP，而不是 GPU Cloth；其 GPU compute 抽象已有 DX12/Vulkan/Metal 后端。[S25]

#### 浮力

`BodyInterface::ApplyBuoyancyImpulse` 支持给刚体施加平面水面浮力、线性/角阻力和流速影响；它是浮力原语，不是海洋、波浪或体积水系统。[S37]

#### 确定性与状态

Jolt 提供：

- SaveState/RestoreState；
- StateRecorderFilter；
- determinism validation；
- `JPH_CROSS_PLATFORM_DETERMINISTIC`；
- determinism log；
- 多架构 CI 修复记录。[S38]

但使用者仍必须确保：

- body ID、constraint order/priority 稳定；
- 输入和 dt 相同；
- body 创建/销毁也进入历史；
- friction、配置等非 simulation-modified state 由上层保存；
- CharacterVirtual 单独保存。[S38][S39]

这仍不是现成的网络复制框架。

---

### 3.2 5.3 之后进展

#### 5.4/5.5

公开 release notes 显示重点包括：

- soft-body 约束并行与性能；
- skinning、tether、bend 和 contact listener；
- per-body simulation stats；
- ragdoll priority；
- soft-body scaled mesh collision修复；
- Windows/编译器适配。[S40]

#### 5.6.0

2026-07-11 发布：

- DX12/Vulkan/Metal GPU compute abstraction；
- GPU strand hair WIP；
- 新 friction model：官方 Pyramid test 声称快 15%、少 40% 内存；
- 场景相关最高 40% 性能提升、最高 70% 内存下降；
- 16-bit heightfield samples；
- glTF rigid-body motor；
- ragdoll position+velocity motor；
- character、determinism、island 修复。[S25]

这些数字是版本发布者的场景相关数据，不应外推到 Rurix workload。

---

### 3.3 Chaos 对照缺口

| 能力 | Jolt | 相对 Chaos 缺口 |
|---|---|---|
| 主刚体 | 成熟、AAA 已验证 | 无根本缺口；需要继续扩展 JoltC FFI 与诊断 |
| 多核/流送 | 强项 | 与 Rurix G6 已较好对齐 |
| Joint | 类型丰富 | 缺少 Physics Asset/profile/editor 产品层 |
| 查询 | 成熟且并发友好 | 缺少 UE 式 channel preset、录制和查询调试 |
| Character | CharacterVirtual 很强 | 缺完整 animation/network/workflow |
| Ragdoll | 求解原语齐全 | 缺 pose profile、编辑器、physical animation 产品层 |
| Vehicle | 有轮式/履带式底座 | 缺模块化资产、调参、动画、网络与完整工作流 |
| Destruction | 无官方内建 fracture/Geometry Collection 管线 | **最大缺口**：资产、层级聚类、strain、cache、field、VFX 联动均需上层实现 |
| Cloth | 基础 CPU soft-body cloth | 缺 panel/seam/fabric、self-collision 成熟度、LOD/cook、GPU cloth、DCC 导入 |
| Soft body | WIP 但持续增强 | 不等价 Chaos Flesh；缺 tet/FEM 角色绑定工具 |
| 浮力/Water | 只有刚体浮力原语 | 缺 water body、wave、surface query、wake/ripple |
| Network Physics | 有 snapshot/determinism 原语 | 无 prediction/replication/rollback/smoothing 体系 |
| Field | 可由 force/query API搭建 | 无统一资产化 Field graph |
| Cache | SaveState 偏回滚 | 无 Chaos Cache 式物理动画录制/回放资产 |
| GPU | 5.6 有 compute 与 GPU hair | 无 GPU 主刚体、无成熟 GPU cloth；对 G6 不构成重审理由 |

---

### 3.4 AAA 公开案例

Guerrilla 的 GDC 2022 报告说明，Jolt 是为解决旧商业引擎在数据流送和多线程游戏对象更新中的全局锁争用而设计。关键架构是 lock-free broadphase 与并行 island building。[S17]

公开 README 还列出《Death Stranding 2: On the Beach》。[S18]

这证明：

- Jolt 足以承载大型开放世界的主刚体和查询；
- 但 Horizon 的布料使用独立 Verlet 粒子/棒约束系统，而不是 Jolt 内建 cloth；
- AAA 使用 Jolt 并不意味着只集成 Jolt 即得到完整物理工具链。[S42]

#### 本章对 Rurix 的前置能力要求

- **A：**保持 ABI 稳定、C FFI 完整覆盖、确定性构建选项、状态哈希工具。
- **B：**升级至 Jolt 5.6 需单独做 replay/perf/CCD 回归；建立 snapshot delta、body lifecycle journal、CharacterVirtual 状态保存。
- **C：**不要 fork Jolt 实现全部 UE 功能；在 `rurix-physics` 上层增加 Destruction、PhysicsAsset、Vehicle、Network Physics。
- **D：**建立 Jolt 版本升级资产重烘焙规则、golden scene、性能基线和 release-note 审计。

---

## 4. 2023–2026 物理前沿

### 4.1 XPBD 与 Small Steps

基础工作《Small Steps in Physics Simulation》（SCA 2019）指出，相同总迭代预算下，更多小步、每步少量迭代常比单大步多迭代更稳定。它虽早于时间窗，但仍是 2023–2026 XPBD 工程的基线。[S43]

近年成果：

1. **Primal Extended Position Based Dynamics for Hyperelasticity**，MIG 2023：修正 XPBD 对任意 hyperelasticity 的残差与收敛问题。[S44]
2. **A Multi-layer Solver for XPBD**，SCA 2024：引入粗到细、多层自由度，加速长距离信息传播。[S45]
3. **XPBI: Position-Based Dynamics with Smoothing Kernels Handles Continuum Inelasticity**，SIGGRAPH Asia 2024：将 XPBD 扩展到 elastoplastic、viscoplastic、granular continuum。[S46]
4. **A Nonconforming Formulation of Cloth**，SIGGRAPH Asia 2025：用非协调高阶表面有限元降低 cloth mesh dependence 和 locking。[S47]

对游戏的价值是更稳定的 cloth/soft-body 与更物理化材料参数，不代表应替换 Jolt 刚体主环。

---

### 4.2 GPU 刚体

#### PhysX 5.4+

PhysX 5.4.2 CUDA GPU simulation 覆盖：

- broadphase；
- contact generation；
- shape/body management；
- constraint solver；
- articulation；
- PBD particle；
- FEM soft body；
- Direct GPU API。[S48]

但限制同样明确：

- CUDA-only，要求 Pascal/SM 6.0+；
- CCD、trigger、joint projection 不在 GPU 加速范围；
- scene query 在 CPU；
- contact modification 回退 CPU；
- 非 D6 joint 部分在 CPU；
- 需预分配 GPU buffer；
- OOM 可令 scene 进入 corrupt 状态；
- 通常数千 active actors 后才可能获得显著优势。[S48]

这进一步支持 Rurix 的裁决：GPU 主刚体不是“免费加速”，会争夺 Vulkan 渲染预算并引入跨 API、同步和容错复杂度。

#### Warp/Newton

Newton 是 NVIDIA、Google DeepMind、Disney Research 合作、Linux Foundation 管理的开源 GPU 物理引擎，基于 Warp/OpenUSD，主要面向 robotics、RL 和 differentiable simulation。其 solver 后端包括 XPBD、VBD、MuJoCo Warp、Featherstone、Kamino、Implicit MPM、Style3D。[S49][S50]

**Kamino: GPU-based Massively Parallel Simulation of Multi-Body Systems with Challenging Topologies**，2026 预印本：面向大批量机器人环境、闭环机构和异构 world，不是单场景开放世界游戏主物理的直接证据。[S51]

#### Genesis

**Genesis: A Generative and Universal Physics Engine for Robotics and Beyond**，2024，项目技术报告：统一 rigid、MPM、SPH、FEM、PBD 和 stable fluid，主要定位 Physical AI/机器人批量仿真。[S52]

其“4300 万 FPS”是 RTX 4090 上大批量简单 Franka 环境的聚合帧数，不可与单个复杂游戏世界帧率直接比较。[S52]

---

### 4.3 MPM 与连续体

1. **CK-MPM: A Compact-Kernel Material Point Method**，SIGGRAPH 2025：双网格紧支撑 kernel，在减少数值扩散的同时提高 GPU MPM 效率。[S53]
2. 经典 **MLS-MPM with CPIC**，SIGGRAPH 2018：仍是 Taichi MPM、切割和刚体双向耦合的基础。[S54]
3. XPBI 2024 展示 XPBD 与 MPM 观念的融合，可处理雪、沙、塑性体等。[S46]

游戏近期最可能落地的是：

- 雪、泥、沙、软土的局部特效；
- 离线破坏/变形烘焙；
- 玩家附近受限区域；
- 低频副轨，而非全世界连续体。

---

### 4.4 GPU/神经布料

1. **ContourCraft: Learning to Resolve Intersections in Neural Multi-Garment Simulations**，SIGGRAPH 2024：用 intersection contour loss 处理神经多层服装穿插恢复。[S55]
2. **NeuralClothSim: Neural Deformation Fields Meet the Thin Shell Theory**，NeurIPS 2024：连续 neural deformation field + Kirchhoff–Love shell，支持任意分辨率查询，但属于准静态/研究型求解。[S56]
3. **A Nonconforming Formulation of Cloth**，SIGGRAPH Asia 2025：高质量非协调 FEM cloth。[S47]

神经布料短期更适合作为：

- 高质量 cloth cache 的 runtime surrogate；
- hero character deformation；
- LOD；
- 特定服装/姿态域加速。

它不宜取代具备拓扑、碰撞、网络和任意输入能力的通用实时 cloth。

---

### 4.5 破坏与 VDB

1. **Remeshing-free Graph-based Finite Element Method for Fracture Simulation**，Computer Graphics Forum 2023：在 tetrahedral graph 上重标记受损边，避免 fracture 后系统矩阵规模爆炸。[S57]
2. **Fast Remeshing-Free Methods for Complex Cutting and Fracture Simulation**，MIG 2023 Doctoral Symposium：加入 anisotropy、随机 damage 和 multigrid 交互求解。[S58]
3. **CD-MPM: Continuum Damage Material Point Methods for Dynamic Fracture Animation**，SIGGRAPH 2019：虽超出时间窗，仍是 phase-field/continuum-damage MPM fracture 基础。[S59]

OpenVDB/NanoVDB 的作用主要是稀疏体素、SDF、level set、切割和碰撞数据表示，不是完整 fracture solver。[S60]
对 Rurix，短期应优先预破碎 Geometry Collection 路线；FEM/MPM/VDB runtime fracture 作为 P3+ 研究项。

---

### 4.6 FLIP/APIC 流体

1. **Fluid Implicit Particles on Coadjoint Orbits (CO-FLIP)**，SIGGRAPH Asia 2024：改善 FLIP 的能量和环量保持，并在较低分辨率下维持质量。[S61]
2. **Building a Real-Time System on GPUs for Simulation and Rendering of Realistic 3D Liquid in Video Games**，Web3D 2023：讨论游戏实时 GPU 液体系统。[S62]
3. **GPU-Accelerated FLIP Fluid Simulation Based on Spatial Hashing Index and Thread Block-Level Cooperation**，2026：报告相对 CPU-FLIP 近 50×、P2G 超 30% 优化，但属于特定 CUDA 实现和测试场景。[S63]

游戏中大规模水体仍多采用 shallow water、FFT/ocean、局部粒子和视觉欺骗；完整 3D FLIP/APIC 应放在特效副轨。

---

### 4.7 可微物理

**A Review of Differentiable Simulators**，IEEE Access 2024，总结了可微物理在：

- system identification；
- trajectory optimization；
- policy optimization；
- material/shape estimation；
- morphology optimization

中的主要价值与速度、通用性、梯度精度权衡。[S64]

**Differentiable Solver for Time-dependent Deformation Problems with Contact**，ACM TOG/SIGGRAPH 2024，使用 FEM、IPC contact 和 adjoint differentiation，报告非线性问题反向求导开销通常低于 forward 的 10%。[S65]

对游戏引擎最现实的“溢出”是离线用途：

- 自动拟合摩擦、悬挂、布料参数；
- 角色/车辆控制器优化；
- 物理 LOD 训练；
- 生成 ML Deformer 数据；
- 自动匹配实拍或高质量离线缓存。

#### 本章对 Rurix 的前置能力要求

- **A：**GPU kernel 泛型、稀疏/邻域原语、可选 autodiff、跨后端计算图和精确 barrier。
- **B：**GPU 副求解器与 Vulkan 渲染的时间/显存预算隔离；cache、离线 batch、deterministic capture。
- **C：**实验 solver plugin ABI；cloth/MPM/FLIP 不得侵入主刚体状态机。
- **D：**论文 benchmark scenes、资产转换、参数拟合、训练和回归数据集。

---

## 5. 与 G6 已锁定裁决的一致性

### 5.1 GPU 主刚体禁令

**结论：维持，不重审。**

理由：

- PhysX GPU 刚体确有完整 pipeline 优势，但仍有 CPU scene query、CCD、trigger、projection、contact modification 等回退和限制。[S48]
- Newton/Genesis 的优势主要是机器人批量环境和可微训练，不是 Vulkan 主渲染单世界的已验证替代。
- RTX 4070 Ti 同时承担渲染和物理，会产生显存、异步队列、同步和帧尾延迟竞争。
- Jolt 已证明 CPU 多核足以支撑 AAA 开放世界。

**仅在以下条件同时成立时重审：**

1. 明确场景存在超过 CPU Jolt 能力的数千活跃刚体；
2. Vulkan 渲染 profiling 证明有稳定 GPU headroom；
3. GPU 方案覆盖查询、CCD、回滚和容错；
4. 实际 end-to-end 帧时间优于增加 CPU 核心/物理 LOD；
5. 跨 NVIDIA/AMD 的后端策略成立。

当前证据不满足。

### 5.2 Taichi 副轨

**结论：定位正确，应扩展而非升级为主环。**

适合：

- Niagara 类粒子/烟火；
- 局部 MPM；
- 浅水/FLIP 特效；
- cloth 或 hair compute spike；
- 离线缓存生成。

必须保留单向或弱耦合边界：

- 主刚体可向副轨输出 collider/impulse；
- 副轨默认不反向决定 gameplay-authoritative 刚体；
- 需要反作用时，使用限频、汇总后的 force/impulse，而非逐粒子强耦合。

### 5.3 CPU 多核主物理

**结论：继续作为 G8 基础。**

Jolt 的并发查询、批插体、lock-free broadphase 与 Rurix 流送页架构高度匹配。升级 5.6 可能带来性能和新 GPU compute 设施，但必须先经过 ABI、资产、CCD、determinism、vehicle 和 soft-body 回归。

### 5.4 需要调整的观察项

- RD-042 Newton/Genesis/MuJoCo Warp：保持研究隔离，重点观察 solver plugin、GPU batch 与参数拟合，不观察“替代主刚体”。
- RD-043 wgrapier：继续观察，但除非出现大型单世界、完整 joint/query/network 的生产案例，否则不进入 G8 主线。
- RD-044 P3+：建议拆成 Cloth、Continuum、Fluid、Differentiable Physics 四个独立研究条目。

---

## 6. 附：调研侧分期建议（输入性质）

*本章仅为调研输入，其分期编号（G8.0~G8.4）不构成计划裁决；正式波次划分以 [G8_PLAN.md](../G8_PLAN.md) 为准。*

### G8.0：基线与统一资产

- Jolt 5.3→5.6 升级评估；
- PhysicsAsset schema；
- collision material/filter preset；
- physics capture/replay；
- state hash；
- benchmark scenes；
- query/contact/constraint 可视化。

### G8.1：网络与角色

- physics frame ID；
- input/state history；
- delta snapshot；
- body create/remove journal；
- rollback/resimulation；
- event deduplication；
- CharacterVirtual state；
- ragdoll/physical animation；
- server correction 和 smoothing。

### G8.2：破坏 MVP

- 预破碎资产；
- hierarchy/cluster；
- connection graph；
- strain/damage；
- anchor；
- batch activation；
- fragment removal；
- cache；
- VFX event bridge。

### G8.3：布料与车辆

- CPU XPBD cloth 或独立成熟库；
- panel/seam/fabric schema；
- USD/DCC import；
- cloth collision/LOD/cache；
- Jolt vehicle 产品层；
- telemetry、调试、网络输入。

### G8.4：高级工具链

- Dataflow 等价图；
- 非破坏 fracture；
- cloth graph；
- unified field；
- physics asset versioning；
- offline cache farm。

### P3+

- Chaos Flesh 等价软体；
- GPU hair/cloth；
- MPM/FLIP；
- neural deformation；
- differentiable parameter fitting。

---

## 7. UE5 物理能力 → Rurix 前置要求总表

| UE5 能力 | A 语言/编译器 | B 运行时 | C 引擎库 | D 工具/资产 |
|---|---|---|---|---|
| 多核刚体 | SIMD、稳定 ABI | Job、固定步 | JoltC 完整面 | benchmark/capture |
| Sleeping/CCD | 数学一致性 | island 状态、CCD 预算 | per-body policy | CCD 可视化 |
| Joint | 无重大新增 | break/event | joint/profile/motor | gizmo、profile editor |
| 场景查询 | 泛型 filter | 并发批查询 | ray/sweep/overlap API | query recorder |
| 破坏 | graph/SoA | cluster 激活、cache | damage/strain/field | fracture/cluster/cook |
| Chaos Cache | 序列化布局 | 时间轴、压缩 | cache player | record/bake |
| Cloth | GPU 邻域原语 | cloth timeline | XPBD/self-collision/LOD | panel/seam/fabric/USD |
| ML Deformer | 推理后端 | deformation buffer | model runtime | cache/training pipeline |
| Vehicles | 无重大新增 | fixed input/history | drivetrain/tire/network | tuning/telemetry |
| Character | 无重大新增 | character fixed tick | CharacterVirtual 包装 | controller test scenes |
| Ragdoll | 无重大新增 | anim/physics 双缓冲 | PhysicsAsset/pose drive | collider/joint authoring |
| Soft body/Flesh | sparse/tet kernel | deformation bridge | soft/FEM plugin | tet/binding/cache |
| Niagara Fluids | compute/grid primitives | async GPU budget | Grid/MPM/FLIP 副轨 | VFX graph/cache |
| Async Tick | frame-domain 类型约束可选 | physics thread | callback contract | timeline debugger |
| Network Physics | 可序列化状态 | history/rollback/resim | replication modes | lag simulator/replay |
| Determinism | build/math policy | hash/validation | snapshot filter | cross-machine CI |
| Physics Field | kernel graph | field dispatch | field evaluator | node editor/gizmo |
| Dataflow | 编译计算图 | asset evaluation | node/plugin ABI | 通用物理资产图 |
| Water/Buoyancy | grid/FFT 可选 | water query | buoyancy/wave | water-body authoring |

---

## 8. 调研侧最终裁决建议（输入性质）

*本章仅为调研输入，其裁决建议与五项门槛不构成计划裁决；正式验收门槛与波次划分以 [G8_PLAN.md](../G8_PLAN.md) 为准。*

G8 不应定义为“重写 Chaos”或“给 Jolt 加更多 FFI”。更合理的验收定义是：

> 以 Jolt CPU 多核为权威刚体核心，建立与 Chaos 类似的资产化、网络化、可调试、可流送、可扩展物理平台，并补齐预破碎、布料、车辆和角色物理生产链。

建议设定五个必须完成的 G8 门槛：

1. **物理 capture 能完整重演固定步世界并定位首个 divergence。**
2. **网络物理支持预测、权威修正、rollback/resimulation 和事件去重。**
3. **预破碎资产可完成 fracture cook、层级 cluster、strain 断裂、cache 和 VFX 联动。**
4. **PhysicsAsset 能完成 ragdoll、physical animation、vehicle/character collider authoring。**
5. **Cloth 至少具备开放资产 schema、DCC 导入、碰撞、LOD 和独立求解时间线。**

达成这五项后，Rurix 才能合理宣称具备“UE5 级物理引擎的前置能力”，而不只是拥有 AAA 级刚体后端。

---

## 9. 参考来源

- [S1 UE 5.8 发布与 Dataflow/Cloth Production Ready](https://www.unrealengine.com/news/unreal-engine-5-8-is-now-available)
- [S2 UE 5.4 Release Notes](https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5.4-release-notes?application_version=5.4)
- [S3 Chaos Character Physics / Panel Cloth 5.5](https://dev.epicgames.com/community/learning/paths/QX/unreal-engine-welcome-to-chaos-character-physics)
- [S4 Chaos Cloth 5.6 更新](https://forums.unrealengine.com/t/tutorial-chaos-cloth-updates-5-6/2555686)
- [S5 Chaos 迭代与性能说明](https://dev.epicgames.com/community/learning/tutorials/KWeX/unreal-engine-a-tech-artist-s-playbook-for-chaos-performance)
- [S6 Physics Sub-Stepping](https://dev.epicgames.com/documentation/en-us/unreal-engine/physics-sub-stepping-in-unreal-engine)
- [S7 Chaos CCD API](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/PhysicsControlModifierData)
- [S8 UE Physics Settings 5.6](http://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/PhysicsSettings?application_version=5.6)
- [S9 Physics in Unreal Engine](https://dev.epicgames.com/documentation/unreal-engine/physics-in-unreal-engine?lang=en-US)
- [S10 Physics Constraint Component](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/UPhysicsConstraintComponent)
- [S11 Chaos Runtime API](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Chaos)
- [S12 ComponentSweepMulti](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/UWorld/ComponentSweepMulti)
- [S13 Destruction Overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/destruction-overview)
- [S14 Geometry Collection Dataflow Nodes](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Plugins/GeometryCollectionNodes)
- [S15 Dataflow for Destruction Quickstart](https://dev.epicgames.com/documentation/en-us/unreal-engine/dataflow-for-destruction-quickstart)
- [S16 Field Physics Type](http://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/FieldPhysicsType?application_version=5.0)
- [S17 Architecting Jolt Physics for Horizon Forbidden West](https://jrouwe.nl/architectingjolt/ArchitectingJoltPhysics_Rouwe_Jorrit_Notes.pdf)
- [S18 Jolt Physics README](https://github.com/jrouwe/JoltPhysics)
- [S19 Chaos Cloth Asset API](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Plugins/ChaosClothAssetEngine/UChaosClothAsset)
- [S20 Chaos ML Cloth Generation](https://dev.epicgames.com/community/learning/paths/QX/unreal-engine-welcome-to-chaos-character-physics)
- [S21 ML Deformer API](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/MLDeformerGeomCacheModel?application_version=5.6)
- [S22 Chaos Vehicles](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/ChaosVehicleMovementComponent.html?application_version=5.7)
- [S23 Physical Animation Component](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/PhysicalAnimationComponent)
- [S24 Chaos Flesh](https://forums.unrealengine.com/t/community-tutorial-chaos-flesh/1134001)
- [S25 Jolt 5.6.0 Release](https://github.com/jrouwe/JoltPhysics/releases/tag/v5.6.0)
- [S26 Niagara Grid3D](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/NiagaraDataInterfaceGrid3D?application_version=5.6)
- [S27 UE Water System](https://dev.epicgames.com/documentation/en-us/unreal-engine/water-system-in-unreal-engine)
- [S28 Network Physics Predictive Interpolation](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/NetworkPhysicsSettingsPredictiveInterpolation?application_version=5.5)
- [S29 Jolt Motion Quality/CCD](https://jrouwe.github.io/JoltPhysics/_motion_quality_8h.html)
- [S30 Jolt Constraints](https://jrouwe.github.io/JoltPhysics/_constraint_8h.html)
- [S31 Jolt NarrowPhaseQuery](https://jrouwe.github.io/JoltPhysics/class_narrow_phase_query.html)
- [S32 Jolt CharacterVirtual](https://jrouwe.github.io/JoltPhysics/class_character_virtual.html)
- [S33 Jolt VehicleConstraint](https://jrouwe.github.io/JoltPhysics/class_vehicle_constraint.html)
- [S34 GDC 2025 Chaos Destruction](https://www.gdcvault.com/play/1035357/Dynamic-Destruction-in-UE5-with)
- [S35 UE 5.6 Performance/Physics Highlights](https://tomlooman.com/unreal-engine-5-6-performance-highlights/)
- [S36 Jolt Soft Body Architecture](https://github.com/jrouwe/JoltPhysics/blob/master/Docs/Architecture.md)
- [S37 Jolt ApplyBuoyancyImpulse](https://jrouwe.github.io/JoltPhysics/class_body_interface.html)
- [S38 Jolt Deterministic Build](https://jrouwe.github.io/JoltPhysicsDocs/5.1.0/md__build__r_e_a_d_m_e.html)
- [S39 Jolt StateRecorder](https://github.com/jrouwe/JoltPhysics/blob/master/Jolt/Physics/StateRecorder.h)
- [S40 Jolt Release Notes](https://github.com/jrouwe/JoltPhysics/blob/master/Docs/ReleaseNotes.md)
- [S41 XPBD 原论文](https://matthias-research.github.io/pages/publications/XPBD.pdf)
- [S42 Horizon 布料讨论](https://github.com/jrouwe/JoltPhysics/discussions/303)
- [S43 Small Steps in Physics Simulation](https://matthias-research.github.io/pages/publications/smallsteps.pdf)
- [S44 Primal XPBD for Hyperelasticity，MIG 2023](https://doi.org/10.1145/3623264.3624437)
- [S45 A Multi-layer Solver for XPBD，SCA 2024](https://www.alexandremercieraubin.com/Work/papers/SCA2024MultiLayerXPBD.pdf)
- [S46 XPBI，SIGGRAPH Asia 2024](https://dl.acm.org/doi/10.1145/3680528.3687577)
- [S47 A Nonconforming Formulation of Cloth，SIGGRAPH Asia 2025](https://dl.acm.org/doi/10.1145/3757377.3763989)
- [S48 PhysX 5.4.2 GPU Simulation](https://nvidia-omniverse.github.io/PhysX/physx/5.4.2/docs/GPURigidBodies.html)
- [S49 Newton Physics](https://developer.nvidia.com/newton-physics)
- [S50 Newton Solver API](https://newton-physics.github.io/newton/stable/api/newton_solvers.html)
- [S51 Kamino，2026](https://doi.org/10.48550/arxiv.2603.16536)
- [S52 Genesis](https://github.com/Genesis-Embodied-AI/genesis-world)
- [S53 CK-MPM，SIGGRAPH 2025](https://dl.acm.org/doi/10.1145/3731155)
- [S54 MLS-MPM/CPIC，SIGGRAPH 2018](https://yuanming.taichi.graphics/publication/2018-mlsmpm/)
- [S55 ContourCraft，SIGGRAPH 2024](https://doi.org/10.1145/3641519.3657408)
- [S56 NeuralClothSim，NeurIPS 2024](https://4dqv.mpi-inf.mpg.de/NeuralClothSim/)
- [S57 Remeshing-free Graph-based FEM Fracture](https://doi.org/10.1111/cgf.14725)
- [S58 Fast Remeshing-Free Fracture Methods](https://doi.org/10.1145/3623053.3623366)
- [S59 CD-MPM Fracture](https://dl.acm.org/doi/10.1145/3306346.3322949)
- [S60 OpenVDB](https://www.openvdb.org/)
- [S61 CO-FLIP，SIGGRAPH Asia 2024](https://today.ucsd.edu/story/this-new-advanced-method-produces-highly-realistic-simulations-of-fluid-dynamics)
- [S62 Real-Time GPU Liquid，Web3D 2023](https://doi.org/10.1145/3587423.3595537)
- [S63 GPU FLIP，2026](https://doi.org/10.3390/modelling7010027)
- [S64 A Review of Differentiable Simulators，IEEE Access 2024](https://doi.org/10.48550/arxiv.2407.05560)
- [S65 Differentiable Deformation with Contact，SIGGRAPH 2024](https://huangzizhou.github.io/research/diffipc.html)

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-02 | 初版：物理引擎深度调研成果落盘（项目首份系统性物理调研；G8 计划定稿档输入材料） |
