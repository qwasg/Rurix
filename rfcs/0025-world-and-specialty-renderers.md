# RFC-0025 — 大世界×专项渲染器×显示管线语义（G9 D4 伞形）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0025（4 位制，编号永不复用，10 §9.5；编号按 2026-08-12 实测 `registry/number_ledger.json` namespaces.RFC `next_free=25` 领取） |
| 标题 | 大世界×专项渲染器×显示管线语义（G9 D4 伞形：世界分区/流送预算/HLOD/大气/水体/地形/贴花 + 显示管线 view transform/后处理骨架/OIT benchmark + 毛发/皮肤，含 MaterialClosure 32B 冻结面显式修订行） |
| 档位 | **Full RFC**（新增世界分区数据模型与流送预算契约运行时语义、显示管线交换链与 view transform 插件面、OIT 档位策略面、五族专项渲染器前端语义；触 G5 冻结面 `MaterialClosure` 32B 扩展（M115/M114）须显式修订行；触及资产 ABI 面与帧图语义，10 §3 / AGENTS 硬规则 5；判档争议向上取严 = Full） |
| 状态 | **Agent Approved（2026-08-12）** |
| 承接里程碑 | G9.5 大世界×专项波（验收门 G-G9-7；P0 门 `g9.p0.m110.world_partition` / `g9.p0.m118.display_pipeline_view_transform`，G9.1 已冻结字面 0-byte 引用不新造） |
| 关联条款 | 拟落 spec **RXS-0363~0373**（G9.5 spec-first，自合入时实测 `registry/number_ledger.json` `RXS.next_free = 363` 顺位领取，连续不跳号；候选落点见 §5） |
| 依据决策 | D-406 v2.0 · D-409 · P-01/P-09/P-11/P-12/P-13 · 10 §7 · G9.1 立项六项裁决（已定案）· G9_CONTRACT §8.1 裁决①（P1 全进，逐波只追加进 ACCEPTANCE_MAP §3）· G8.7 承接锚 M43/M48/M49（defer-to-G9+）与 M45/M46/M47（no-go 留档重判）· [RFC-0016](0016-native-renderer.md) §4.0-3/§4.G1（G5 冻结面）· [RFC-0019](0019-rendering-platform.md) §4.5/§4.6/§4.7/§8 · [RFC-0022](0022-virtual-geometry-gi-semantics.md) §8（MaterialClosure 32B 0-byte 重申）· G9.0 不可变 ref `1d9460a1`（[D4 草案](../milestones/g9/design/G9_D4_WORLD_AND_SPECIALTY_RENDERERS.md) D1~D17）· [R7 调研](../milestones/g9/research/R7_WORLD_AND_SPECIALTY_RENDERERS.md) |
| Provenance | `Assisted-by: Kimi:Kimi-K3 rfc0025-drafter` |
| Agent 批准 | **Agent Approved 2026-08-12**；agent 依 10 §7/P-13/D-406 v2.0 自主批准；批准只表示语义评审完成，**不构成实现许可**（G9.5 实现仍由 G9 契约波次门与既有守卫决定） |
| 对抗性评审 | **完成**（D-409 第 1 轮）：评审 provenance `Assisted-by: Kimi:Kimi-K3 rfc0025-adversarial-reviewer`；**单实例偏差如实登记**——本环境为单模型子代理会话，无法派生跨工具/跨模型独立评审实例，评审与起草同模型同实例，偏差大于 RFC-0024 §9.1「同工具族独立实例」先例，见 §9.1 效力自限声明；4 findings（1 major + 3 minor）全部 disposition |

---

## 1. 摘要

本 RFC 是 G9 伞形 RFC 的 D4 补位章。G9.1 立项落了三份伞形 Full RFC——RFC-0022（D1 虚拟几何×RT + D2 GI）、RFC-0023（D3 GPU-driven 提交与着色）、RFC-0024（D5 物理修订）——**D4（大世界×专项渲染器×显示管线）无伞形 RFC**。G9.5 波开工在即，M110（世界分区）与 M118（显示管线 view transform）两个 P0 及 M111~M120 九个 P1 的语义面需要 RFC 级授权与 spec-first 映射裁定，且 M115/M114 触 G5 冻结面 `MaterialClosure` 32B 须显式修订行（G9_CONTRACT guardrail「触 G5/G6 冻结面必须 RFC 显式修订行，禁静默扩」，M104 先例 = RFC-0023 §4.4.3）。本 RFC 以最小篇幅补齐这一缺口：

1. **D4 缺口确认**（§2.1）：Grep 实测 RFC-0016/0019/0022/0023 冻结面与 D4 链路面无重叠，D4 十一行（M110~M120）无 RFC 载体；
2. **语义面授权**（§4）：世界分区数据模型与流送预算契约、HLOD 烘焙、大气 Froxel 前端、水体双管线、地形 chunk≡cell、贴花 DBuffer、显示管线 view transform 插件面、后处理骨架显式排序、OIT benchmark harness（仅测量不定档）、毛发 Marschner、皮肤 Burley——每面只冻结验收门所需最小语义，判据逐字引 `G9_ACCEPTANCE_MAP` §2/§3；
3. **🔒 MaterialClosure 32B 显式修订行**（§4.L）：M115/M114 皮肤/毛发 lobe 扩展的前置修订纪律；
4. **spec 映射裁定**（§5）：新建 `spec/world_partition.md` 与 `spec/display_pipeline.md` 两卷，RXS-0363~0373 顺位领取。

本 RFC 批准不解锁实现；G9.5 实现仍须 spec 条款 PR 先于实现 PR（硬规则 7），并经各 P0/P1 门独立断言。

### 1.1 为何不是 Mini-RFC（MR）而是 Full RFC

判档争议向上取严（硬规则 8 / 10 §3）。D4 处置有三条候选载体，裁决为 Full RFC：

- **MR（Mini-RFC）否决**：MR 体例（[`TEMPLATE-MINI-RFC.md`](TEMPLATE-MINI-RFC.md)）承载范围 = 规范内 bug fix / 诊断措辞策略 / 内部开关 / 工具行为变更 / 规则文件级修改（MR-0010 影子工作流登记、MR-0011 ptxas 开关先例）。D4 是**全新渲染语义面**（世界分区 schema + 流送预算契约 + 显示管线交换链 + OIT 档位 + 五族专项渲染器着色）且触 G5 冻结面 `MaterialClosure` 32B 扩展——G9_CONTRACT guardrail 明确「触 G5/G6 冻结面必须 **RFC** 显式修订行」，M104 AccessKind 新边先例落在 Full RFC-0023 §4.4.3 而非 MR。MR 无法承载新语义面 + 冻结面修订。
- **并入 RFC-0022/0023 修订行否决**：RFC-0022 §5 映射表仅覆盖虚拟几何/GI、RFC-0023 仅覆盖 D3 提交链，两者均 Agent Approved 且正文冻结；D4 与 D1/D2/D3 不同轴（世界数据模型×显示输出），并入会破坏既有 RFC 的轴纯洁性。
- **Full RFC 采纳**：与 D1/D2（0022）、D3（0023）、D5（0024）三份伞形同构；单 RFC 承载 D4 全语义面，一次对抗性评审覆盖跨面一致性（§4.0）。

## 2. 动机、范围与治理门

### 2.1 D4 缺口确认（Grep 实测）

对 rfcs/ 全目录 Grep `大世界|显示管线|后处理|OIT|分区|HLOD|M110|M118|M119|M120` 与 `MaterialClosure`：

- **RFC-0016**（G5 原生渲染器伞形八章）冻结面 = render graph 四趟编译/EB 三轴、VisBuffer 位格式、`MaterialClosure` 32B 定长、页式流送三预算 `StreamingBudget{io,transcode,upload}`、时域重建底座——**无世界分区 schema、无 HLOD、无大气/水体/毛发/皮肤/地形/贴花专项、无 HDR 显示管线、无 OIT 档位**；其 §8 把 Surface Cache/Mega Geometry/Nanite Foliage/多层材质等 P3+ 全部登记 RD-037+ 存续，D4 面不在其承诺面。
- **RFC-0019**（G8 渲染平台）冻结面 = RT pipeline 增量/单源 gfx submit/permutation/reflection/capability profile/TSR 时域/M28 多层 closure IR（语义先行，实现 no-go）/多队列/task 评估窗——**同样不覆盖 D4 任一链路面**；其 §8 明确 `MaterialClosure` 32B 0-byte 保持、不消费预留拓扑字段位。
- **RFC-0022 §5 映射表**仅虚拟几何/GI 行（页格式 v2/DAG/CLAS/VisibleClusterSet/Surface Cache/降级链/probe/PT 参照器/蒙皮 MV），**无大世界/显示管线行**；RFC-0023 仅 D3（DGC/descriptor/Execution Set/IR 链接/SER/mesh shader）；RFC-0024 仅 D5 物理。

结论：**D4 十一行（M110/M111/M112/M113/M114/M115/M116/M117/M118/M119/M120）无伞形 RFC 载体**，为 G9.1 治理缺口；D4 设计草案 §9 亦自认需 RFC-G9-D4-α/β/γ/δ 四面。本 RFC 即该缺口的处置。

### 2.2 双门互锁：RFC 批准不等于实现开工

| 门 | 允许动作 | 禁止动作 |
|---|---|---|
| 本 RFC Approved + G9.5 spec-first | 落 spec 条款（RXS-0363~0373）+ conformance 锚定语料 + 治理登记（MAP §3 / CI_GATES §4A） | 不改 `src/`；不 materialize 数字 CI 步骤；不预建空 schema 壳/空脚本占位 |
| G9.5 实现波（后续） | 各实现 agent 按 spec 条款 + RED 先行落实现，数字 CI 步骤按落盘时实测 `CI_step.next_free` 顺位领取 | 互锁/波次门任一红时不得以 RFC Approved 或立项裁决替代机器事实 |

### 2.3 in-scope

| 面 | 本 RFC 冻结内容 | gate key（引用 G9.1/G9.5 治理冻结字面，不新造） | 最晚波次 |
|---|---|---|---|
| M110 世界分区 | 单一持久世界 schema、2D cell、三项流送预算契约、四事件接口、Data Layer 掩码位预留 | `g9.p0.m110.world_partition`（P0，G9.1 冻结） | G9.5 |
| M118 显示管线 | SDR/scRGB/PQ 三交换链、view transform 四内置插件、HDR 元数据责任面 | `g9.p0.m118.display_pipeline_view_transform`（P0，G9.1 冻结） | G9.5 |
| M111 HLOD | 离线 Builder 按 Component 分发、产物即资产、双构建 hash 相等、运行时零合并 | `g9.p1.m111.hlod_baking`（P1，G9.5 判 go） | G9.5 |
| M112 大气 | Froxel 统一基础设施 + 雾前端 + 云前端（时序上采样默认） | `g9.p1.m112.atmosphere_froxel`（P1，G9.5 判 go） | G9.5 |
| M113 水体 | 大洋 Tessendorf IFFT 与浅水波方程双管线分离、浮力接口面预留 | `g9.p1.m113.water_dual_pipeline`（P1，G9.5 判 go） | G9.5 |
| M114 毛发 | Marschner R/TT/TRT 三瓣 + 几何三档；strand 档强制精确 OIT | `g9.p1.m114.hair_marschner`（P1，G9.5 条件 go） | G9.5 |
| M115 皮肤 | Burley 屏单 pass + 扩散 profile 资产化 + LUT 回退档；触 32B 修订行 | `g9.p1.m115.skin_burley_diffusion`（P1，G9.5 判 go） | G9.5 |
| M116 地形 | chunk ≡ cell、LOD/剔除/缝合全 compute、toroidal 更新、零 SVT 依赖 | `g9.p1.m116.terrain_chunk_cell`（P1，G9.5 判 go） | G9.5 |
| M117 贴花 | DBuffer 三通道帧图设计期占位、screen-space cluster 化、前向回退档 | `g9.p1.m117.decal_dbuffer`（P1，G9.5 判 go） | G9.5 |
| M119 后处理骨架 | 曝光→bloom→tonemap→LUT→输出变换显式排序、HDR 线性域、曝光状态帧间持久 | `g9.p1.m119.post_processing_skeleton`（P1，G9.5 判 go） | G9.5 |
| M120 OIT benchmark | nvpro 七算法 harness 仅测量不定档、evidence 非空、排序 fallback 永保留 | `g9.p1.m120.oit_benchmark_harness`（P1，G9.5 判 go） | G9.5 |

### 2.4 out-of-scope

见 §8 范围红线。特别强调：SVT/RVT/sampler feedback（M40/41/42 G8 no-go 维持，地形/贴花不得依赖）；水体浮力 gameplay 联动（M77→M124 归 G9.6，D4 只留接口面）；FG/MFG（M26）、present pacing（M49b）、GPU 粒子 VFX（M49a）不进 G9；编辑器 GUI/世界编辑工具链 UI/网络流送/多 GPU/USD/MaterialX 承 G8 out-of-scope；DMM 永久禁止；**AVBOIT 为评估项不承诺**（D4 D16）；OIT 默认档不定档（仅 benchmark 测量）。

## 3. 跨面不变量（0-byte 边界）

1. **G5/G6 冻结面**：`GpuScene`、`MaterialClosure` 32B（除 §4.L 显式修订行登记面）、`Barrier` EB 三轴、`PageRequest` 字段布局、VisBuffer 位格式、物理五纪律 0-byte；触 32B 仅经 §4.L 修订行，禁静默扩。
2. **G8 底座只消费不重定**：M01 builder 版本化、M04 磁盘/内存页格式 ABI、M85 shader/PSO manifest↔DDC、M37 磁盘异步 I/O 链、M38 GDeflate、M44 几何页 streamer、RD-037/M89 单源 gfx、M50 RT 增量面、M24 TSR/TAA 生产契约——D4 各面只消费其冻结 ABI，不重定格式。
3. **G9 既有面**：RFC-0022（虚拟几何/GI）/RFC-0023（GPU-driven）/RFC-0024（物理）正文 0-byte；本 RFC 与三者不同轴，不重复定义。M98 L4 Far Field 依赖的 HLOD 接口由本 RFC §4.B 冻结（消费方语义已在 RFC-0022 §4.7 登记 SKIP=not-triggered 口径，本 RFC 不改写）。
4. **推导单源 / strict-only / deterministic / no host substitution**：编译器 manifest 与资产 pipeline manifest 是装配单一事实源；非法构造编译期/装配期/提交前确定性拒绝，不设 UB 节，不允许静默降级；相同输入双构建逐字节一致；device 验收不以 host 结果回填。
5. **measured-first（P-09）**：全部阈值（hitch p99、流送预算水位、OIT 内存界、ray-march 步数档）先 measured 后冻结，禁手写掩盖；`g9_budget.json` 实测标定为唯一来源。

## 4. 参考级设计

### 4.0 跨面一致性约定

- **分区 = 数据结构先行，渲染器只消费 cell 事件**（D4 D1）：World Partition 是数据模型与流送契约，所有专项渲染器（含地形）只是 cell 加载/卸载事件的消费者，不反向查询分区状态。
- **离线烘焙 + 运行时预算化流送 + compute-first GPU 管线 + 分级回退档**（调研结论 6）：每个专项渲染器明确「哪些离线算死、哪些是运行时预算契约」。
- **回退档 v1 即定义**：贴花前向回退档、皮肤 LUT 回退档、OIT 排序 fallback 均在首版冻结语义等价判据，不后补。

### 4.A M110 — 世界分区数据模型与流送预算契约（P0）

- **世界资产 schema（离线）**：单一 persistent world 资产；schema 层显式区分 `always_loaded`（全局/gameplay 关键对象）与 `spatially_loaded`（空间分格对象）；cell 为正方形 2D 网格（边长为资产属性，非代码常量）；cell 元数据 v1 = cell id、包围盒、资产页引用（M04 ABI）、**HLOD 层级引用**（烘焙工具语义锚 §4.B）；**Data Layer 掩码位只预留不接线**（v2 才实现激活语义，D4 D4）。
- **流送运行时**：streaming source（相机/玩家/自定义探针）携带距离环；每帧由距离环求 target cell 集合，与 resident 集合 diff 出 load/unload 队列。
- **三项预算契约（一等契约字段）**：`MaxStreamingCellsPerFrame`、`MaxActorsToSpawnPerFrame`、`MemoryBudgetMB`；超预算请求**排队而非抢占**；预算计数器逐帧落 evidence（hitch 审计数据源）。**预算违约注入必须触发排队降级而非静默超帧（RED 臂）**——注入 `MaxStreamingCellsPerFrame=0` 之类违约，必须可见降级且计数器报警，出现静默超帧即 RED。
- **四事件接口**：`CellLoadBegin / CellResident / CellUnloadBegin / CellEvicted` 为渲染器唯一消费面；固定相机轨迹的 cell 四事件序列与 golden **逐字相等**（确定性）。
- **soak**：代表性大世界 soak（≥ G7 量级：≥30min/≥10000 帧）hitch p99 ≤ measured 阈值（阈值来自 `g9_budget.json` 实测标定，禁手写）。

### 4.B M111 — HLOD 烘焙管线（P1）

- 离线 Builder 按 Component 分发，逐 Component 生成代理几何（简化 + 合批 + 材质合并）；产物经 M01/M04 管线落成普通资产（**产物即资产**），走同一 cook/DDC/页格式通道。
- **运行时零合并**：HLOD 代理与 cell 全量内容按 screen-size 阈值互斥切换（cell 级 HLOD 树，层数为烘焙属性）；运行时合并调用尝试 → 断言/编译期拒绝（RED）。
- **烘焙确定性**：双构建 hash 相等（沿 M79 判据形态）；代理相对原始的屏幕空间误差上界作为 golden 断言；切换距离表 golden。
- 本面即 RFC-0022 §4.7 M98 L4 Far Field 所依赖的 HLOD 接口（M98 已登记该依赖未就绪时 SKIP=not-triggered 不充绿；本 RFC 不改写其口径）。

### 4.C M112 — 大气体渲染器前端（P1）

- **Froxel 统一基础设施（一次性建造）**：视锥体素网格（froxel volume）+ 密度/光照累积 + 深度切片分布 + 与帧图的合成节点；云与雾共用同一 Froxel 基础设施、两个前端（D4 D5）。
- **雾前端**：高度雾/分层介质直接写 Froxel 密度场，解析项为主。
- **云前端**：Perlin-Worley 低频塑形 + Worley 高频侵蚀 + 2D weather map（覆盖度/湿度/类型）资产化（走 M01/M85 通道，非硬编码参数）；低分辨率 ray-march + temporal reprojection 时序上采样为**默认路径**（D4 D6），全分辨率列为高端档。
- **预算契约**：ray-march 最大步数、froxel 分辨率档、上采样开关均为预算字段，每档有 measured 帧时证据。
- **RED 臂**：篡改 weather map 资产签名 → 拒录 RED；时序链断裂（首帧无历史）必须正确初始化，不得复用脏帧。

### 4.D M113 — 水体双管线（P1）

- **大洋管线**：Tessendorf IFFT 谱（风向/风速/涌浪参数化资产）+ 位移/梯度/Jacobian 泡沫三贴图 + CDLOD 距离分档 mesh；多尺度谱 tiling-and-blending 防周期重复感。
- **浅水管线**：局部波方程（高度场 + 速度场 ping-pong compute）服务池塘/河流/交互波纹。
- **双管线分离**：大洋与浅水不共享几何路径，仅共享水面着色 closure 输入面（D4 D8）。
- **浮力接口面预留不实现**（M77→M124 归 G9.6 Field 通道）。
- **RED 臂**：负风速/非法谱参数资产 → 拒录 RED；浅水域越界写检测。

### 4.E M114 — 毛发（P1，条件 go）

- **着色**：Marschner R/TT/TRT 三瓣，纵向/方位角分离参数化为资产属性（每缕基调色、高光偏移、medulla 配置）。
- **几何三档**：近 strand / 中 card（各向异性法线/切线贴图）/ 远 mesh；档间切换距离与 strand→card 股替换映射由离线烘焙产出（股聚类 + card 图集）。
- **strand 档强制精确 OIT**（D4 D9）：strand 档排序不可行，必须 linked-list 精确档；card/mesh 档走默认半透明路径。
- **条件裁决（分项 not-triggered）**：strand 档依赖 M120 精确 linked-list 档的 benchmark 数据裁决与实现落地；M120 本波仅落 benchmark harness（仅测量不定档，§4.I），精确档数据可得性不足——**strand 档分项登记 not-triggered 不充绿**，承接锚 = 「M120 精确 linked-list 档 benchmark 裁决数据落地后重判，兜底 G9.7 穷举」；card/mesh 档与 Marschner 三瓣着色判 go。
- **RED 臂**：单瓣系数置零的 RED 渲染（缺 TT 瓣必须可见差异，无差异即管线未接通）；瓣能量守恒 golden；strand→card 股替换映射烘焙确定性 golden。
- 触 `MaterialClosure` 32B：Marschner lobe 参数经 §4.L 修订行登记的扩展机制接入，禁静默扩 32B。

### 4.F M115 — 皮肤（P1）

- **Burley normalized diffusion 屏空单 pass** separable SSS：颜色/深度双 kernel；扩散 profile（RGB 三通道 falloff 参数）为资产（per-material）。
- **回退档**：pre-integrated LUT（曲率 × NdotL）在低端 profile 启用；两档画质差纳入 golden 对照。
- **RED 臂**：profile 全零衰减 → 输出必须退化为纯漫反射（否则 profile 未生效，RED）。
- **触 32B 前置**：Burley lobe 的 closure 表达经 §4.L 显式修订行登记——优先单 closure 参数化 + 资产化 profile；`MaterialClosure` 32B 布局 0-byte；确需扩面时先修订本 RFC，禁静默扩。

### 4.G M116 — 地形（P1）

- heightfield 数据为 M04 页格式资产；**chunk ≡ cell**（尺寸对齐同一网格族，**禁第二套分格**，D4 D11）。
- LOD 选择/视锥剔除/邻级缝合（stitch skirt 或指数网格 morph）全进 compute，产出 indirect draw；CPU 侧零逐 chunk 提交。
- **toroidal 更新**：相机移动时环形窗口滚动复用 ring buffer；与 M37 I/O 链对接（chunk 页迟到 → 父级 LOD 占位，同 M44 迟到页语义不重定）。
- **零 SVT 依赖断言**（D4 D17：M40/42 G8 no-go 维持，禁依赖虚拟纹理）。
- **RED 臂**：相邻 chunk LOD 差 >1 注入 → 必须触发缝合路径，出现裂缝像素即 RED；邻级缝合处顶点位置连续性 golden（裂缝=0）。

### 4.H M117 — 贴花 DBuffer（P1）

- **DBuffer 三通道**（法线 + 材质属性 + 可选第三通道）在 G-buffer pass 内合成，**帧图设计期即占位**——即使 v1 贴花数量为零，通道与 barrier 布局先行冻结（D4 D12）。
- **cluster 化**：screen-space cluster（复用光照 cluster 结构）对贴花体求交，限制逐像素贴花评估数上界；过绘制计数器落 evidence。
- **前向回退档**：无 DBuffer 的低端 profile 走 decal-forward pass，v1 即定义两档语义等价性判据。
- **RED 臂**：超 cluster 上界贴花密度注入 → 必须受界降级，过绘制计数越界即 RED。

### 4.I M118 — 显示管线与可插拔 view transform（P0）

- **三交换链路径**：SDR（Rec.709）/ scRGB / PQ-Rec.2020 三条 swapchain 路径为一等资源，运行时切换（重建交换链或双链过渡）；HDR 元数据（MaxCLL/MaxFALL/mastering primaries）由输出变换阶段填写。
- **view transform 插件面**：`ViewTransform` trait（输入 HDR 线性 + 显示参数 → 输出编码），**ACES 1.3、ACES 2.0、AgX、中性矩阵四内置实现并列**，第三方可注册；golden 对拍按插件逐一建，**含 AgX/ACES hue-skew 已知差异记录**；AgX 对比度补偿参数随 view transform 资产化，禁止硬编码进 tonemap 节点。
- **条件触发边界**：M45 的 G8 字面触发条件「HDR 显示设备资产/产品需求出现」——本 RFC 拆**管线/插件面（SDR 上即可全量验证，判 go）**与 **HDR 设备标定层（条件未触发则登记 SKIP=not-triggered 或 open-留痕，不假绿）**；标定层未触发**不反向否决** SDR 上可全量验证的管线/插件面。
- **RED 臂**：非 HDR 交换链携带 PQ 输出即 RED；未注册插件名调用 → 拒录 RED。

### 4.J M119 — 后处理骨架（P1）

- **节点顺序冻结（帧图语义）**：exposure（histogram + 手动 EV 偏移）→ bloom（tonemap 前 HDR 域多尺度 mip 链，down/up 双 pass）→ DOF（scatter-as-gather）→ tonemap（经 §4.I 插件）→ 色彩分级（LUT 资产，tonemap 后）→ 输出变换（RRT/ODT 或中性）→ UI 合成（SDR 域）。本波先落**骨架显式排序**（曝光/bloom/tonemap/LUT/输出变换），DOF/色彩分级完整化在同波顺位 2 推进。
- **全程 HDR 线性域**；任何节点不得隐式 clamp 到 SDR（**隐式 SDR clamp 注入 → 探针越界即 RED**）。
- **曝光状态帧间持久**：histogram → 目标 EV 的 adapt（上/下不同速率）状态为 persistent resource；与 TAA/TSR（M24）的时域链**显式排序**。
- SDR 上即可全量验证（不依赖 HDR 设备）。

### 4.K M120 — OIT benchmark harness（P1，仅测量不定档）

- **三档语义**：①默认档——半透明走 TAA 合成路径（现状延伸）；②有界近似档——WBOIT 起步实现，AVBOIT 为目标（内存有界、自适应体素收集，体素布局与 §4.C Froxel 族对齐，**评估项不承诺**，D4 D16）；③精确档——linked-list per-pixel fragment list，仅毛发 strand 启用，场景级不开放。
- **benchmark 门先行**：以 nvpro `vk_order_independent_transparency` 七算法 sample 为对照基线建 harness（同场景、同 overdraw 分布），测量 4070 Ti 上各算法帧时/内存曲线，**evidence 非空**。
- **仅测量不定档**：本门只产 benchmark 数据，**不定默认档**；默认档选型由 benchmark 数据裁决，不由论文偏好裁决（D4 D15）——**无 benchmark 数据的默认档选型提交判 RED**。
- **排序 fallback 永保留**：depth-sorted alpha 路径永远保留为最低端档与正确性对照。
- **RED 臂**：无 benchmark 数据的默认档选型提交 → 聚合 RED；精确档内存无界增长注入 → RED；linked-list 精确档与排序真值 diff=0。

### 4.L 🔒 MaterialClosure 32B 冻结面显式修订行（M115/M114 前置）

G5 冻结的 `MaterialClosure` 32B 单层布局（RFC-0016 §4.0-3/§4.G1：albedo/F0/roughness/normal/emissive 打包 + flags，单层闭合，预留拓扑字段 coverage weight 位）经 RFC-0019 §4.7/§8 重申 0-byte 保持、多层 graph 不消费预留位。M115（皮肤 Burley 扩散 profile）与 M114（毛发 Marschner 参数集）的 lobe 表达触及该冻结面，按 G9_CONTRACT guardrail「触 G5/G6 冻结面必须 RFC 显式修订行，禁静默扩」与 RFC-0023 §4.4.3（M104 AccessKind 新边）先例，登记修订行如下：

| 条款 | 原冻结句（逐字） | 修订后句 | 零漂移证明计划 |
|---|---|---|---|
| 🔒 `MaterialClosure` 32B（RFC-0016 §4.G1，引擎库内部契约面） | 「**`MaterialClosure` 32B 定长**：albedo/F0/roughness/normal/emissive 打包 + flags；**单层闭合**——多层 slab 混合/分层归 P3+，预留拓扑字段（coverage weight）位」 | 32B 定长布局、字段含义与 flags 位段分配 **0-byte 保持**；预留拓扑字段位不消费（RFC-0019 §4.7 口径维持）；**G9.5 起允许专项渲染器 lobe 参数经「材质参数侧表」扩展**——Burley 扩散 profile（RGB 三通道 falloff）与 Marschner 参数集（R/TT/TRT 瓣、基调色、高光偏移、medulla）作为**资产化参数侧表**按材质槽 ID 索引接入单层 closure 求值，经 M01/M85 资产通道烘焙/打包/manifest 入 DDC；**侧表缺省 ≡ 无专项 lobe，既有材质输出逐位不变**；任何 32B 内联扩面（新增字段/改字段含义/消费预留位）必须停手先修订本 RFC，**禁静默扩** | G5~G8 材质面既有 golden/判据（VisBuffer/classify/resolve/材质 resolve 对拍）0-byte 恒跑；新增侧表编解码 roundtrip golden；缺省侧表 ≡ 既有输出 digest 逐位一致 |

本修订行只追加「资产化侧表扩展通道」的合法性登记，不构成对 32B 布局的任何字节改动；若实现期证明侧表通道不足以表达、确需 32B 内联扩面，必须停手先修订本 RFC（升级路径登记），不得在 `src/` 私加字段绕过本表。

## 5. 下游 spec 条款映射（spec diff 计划，G9.5 spec-first 落）

条款号 **RXS-0363~0373**（自合入时实测 `registry/number_ledger.json` `RXS.next_free=363` 顺位领取，连续不跳号；0295/0296 burned 与 shadow_reserved 181~184 维持）。**spec 条款 PR 先于实现 PR**（硬规则 7）；每条 materialize 时至少一个 `//@ spec: RXS-实际号` 锚点，trace_matrix 全锚定。

**目标卷裁定**（沿 RFC-0022 §5 / G9.2 `virtual_geometry.md`、G9.4 `global_illumination.md` 新建先例）：D4 十一行分两个独立语义轴，新建两卷——

| 条款（拟） | M## | 标题 | 目标 spec | 测试锚定计划（每条 ≥1） |
|---|---|---|---|---|
| RXS-0363 | M110 | 单一持久世界 schema、2D cell、三项流送预算契约逐帧 evidence、预算违约注入必排队降级 RED、cell 四事件序列逐字 golden、Data Layer 掩码位预留、soak hitch p99 measured | 新建 `spec/world_partition.md` | 预算违约注入未降级 RED；cell 事件乱序 RED；四事件序列 golden |
| RXS-0364 | M111 | HLOD 离线烘焙按 Component 分发、产物即资产、双构建 hash 相等、运行时零合并断言、screen-size 互斥切换 | 同上 | 运行时合并调用 RED；双构建 hash golden |
| RXS-0365 | M112 | Froxel 统一基础设施 + 雾前端 + 云前端（Perlin-Worley/weather map 资产化/时序上采样默认） | 同上 | weather map 篡改签名 RED；时序链断裂初始化 |
| RXS-0366 | M113 | 大洋 Tessendorf IFFT 与浅水波方程双管线分离、浮力接口面预留不实现 | 同上 | 非法谱参数资产 RED；双管线互斥断言 |
| RXS-0367 | M116 | 地形 chunk ≡ cell 禁第二套分格、LOD/剔除/缝合全 compute、toroidal 更新、零 SVT 依赖断言 | 同上 | 邻级 LOD 差>1 注入缝合裂缝 RED |
| RXS-0368 | M117 | 贴花 DBuffer 三通道帧图设计期占位、screen-space cluster 化、前向回退档语义等价 | 同上 | 超 cluster 上界注入受界降级 RED |
| RXS-0369 | M118 | SDR/scRGB/PQ 三交换链运行时切换、ACES 1.3/2.0/AgX/中性四插件逐一 golden（含已知差异记录）、非 HDR 交换链携带 PQ 输出即 RED、HDR 标定未触发 SKIP=not-triggered | 新建 `spec/display_pipeline.md` | 非 HDR 携带 PQ RED；四插件 golden |
| RXS-0370 | M119 | 后处理骨架显式排序（曝光/bloom/tonemap/LUT/输出变换）、全程 HDR 线性域、隐式 SDR clamp 注入 RED、曝光状态帧间持久、与 TAA/TSR 显式排序 | 同上 | 隐式 SDR clamp 注入 RED；节点顺序 golden |
| RXS-0371 | M120 | OIT benchmark harness 仅测量不定档、evidence 非空、默认档选型必须引 benchmark 数据（无数据提交判 RED）、排序 fallback 永保留 | 同上 | 无数据选型提交 RED；harness evidence 非空 |
| RXS-0372 | M114 | 毛发 Marschner R/TT/TRT 三瓣 + 几何三档、strand 档强制精确 OIT（依赖 M120 精确档，数据不足分项 not-triggered 不充绿） | 同上 | 单瓣置零无差异 RED；strand 档 not-triggered 登记 |
| RXS-0373 | M115 | 皮肤 Burley 屏单 pass + 扩散 profile 资产化 + pre-integrated LUT 回退档、触 MaterialClosure 32B 经 RFC-0025 §4.L 修订行 | 同上 | profile 全零衰减未退化纯漫反射 RED |

**裁定理由**：M110/M111/M112/M113/M116/M117 同属「大世界数据模型与场景专项」轴（分区 schema/流送/场景内渲染对象），合并新建 `spec/world_partition.md`；M118/M119/M120/M114/M115 同属「帧图输出与材质着色专项」轴（显示输出/后处理/透明合成/closure 着色），合并新建 `spec/display_pipeline.md`。既有卷（rendering_platform.md 的 reflection/capability/时域面、shader_stages.md 的语言类型面、geometry_pages.md 的页 ABI 面）与两轴均不同轴，本体 0-byte；rendering_platform.md / shader_stages.md / geometry_pages.md 不改动。

**错误码策略**：G9.5 spec-first 零 RX claim。装配期/校验期诊断沿 typed `Err` + 既有码族先例；只有实现证明出现新的、用户可行动、可独立到达的诊断类别时，才按当时各段 `next_free` 只追加并同步 en/zh message key，不预造。

## 6. feature gate、tracking 与实现序

### 6.1 Gate 命名空间（不新造）

唯一合法 key/脚本事实源是 [`G9_ACCEPTANCE_MAP.md`](../milestones/g9/G9_ACCEPTANCE_MAP.md) §2/§3 与 [`CI_GATES.md`](../milestones/g9/CI_GATES.md) §4/§4A 的 `g9.p{0,1}.m##.<slug>` + `ci/g9_<slug>_smoke.py`，由 `ci/check_g9_acceptance_map.py` 三向/双向比对强制。本 RFC 只引用，不新造命名空间、不 materialize CI 步骤、不预建空 schema 壳。M110/M118 两个 P0 key 为 G9.1 冻结字面 0-byte；九个 P1 key 为 G9.5 波 P1 全进裁决（G9_CONTRACT §8.1 裁决①）只追加登记字面。

### 6.2 真实 RED/GREEN

| 面 | RED（必须先可复现） | GREEN（不得以较弱见证替代） |
|---|---|---|
| M110 | 预算违约注入静默超帧；cell 事件乱序 | 三项预算逐帧 evidence + 四事件序列逐字 golden + soak hitch p99 ≤ measured |
| M111 | 运行时合并调用；双构建 hash 漂移 | 双构建 hash 相等 + 零合并断言 + 互斥切换 golden |
| M112 | weather map 篡改签名；首帧复用脏帧 | Froxel 雾/云前端 golden + 时序上采样对拍 |
| M113 | 非法谱参数资产；浅水越界写 | 大洋 IFFT 与 host FFT 参考逐值对拍 + 双管线互斥断言 |
| M114 | 单瓣置零无差异；strand 档未走精确 OIT | Marschner 三瓣逐瓣对拍 + 股替换烘焙确定性 golden |
| M115 | profile 全零衰减未退化纯漫反射；静默扩 32B | Burley 屏单 pass 对拍 + LUT 回退档画质差报告 |
| M116 | 邻级 LOD 差>1 未缝合出裂缝；chunk≠cell 第二套分格 | 全 compute LOD/剔除/缝合 device 对拍 + 裂缝=0 golden |
| M117 | 超 cluster 上界未受界降级 | DBuffer 三通道占位断言 + 前向回退档语义等价 golden |
| M118 | 非 HDR 交换链携带 PQ；未注册插件名调用 | 四插件逐一 golden（含已知差异记录）+ 三交换链切换证据 |
| M119 | 隐式 SDR clamp；曝光状态跨帧丢失 | 显式排序 golden + 曝光 adapt 曲线 + HDR 线性域探针 |
| M120 | 无 benchmark 数据的选型提交；精确档内存无界增长 | 七算法帧时/内存曲线 evidence 非空 + 排序 fallback 可达断言 |

### 6.3 栈式实现序（波次细节由 G9 契约定）

1. **PR-Spec**：按 §5 materialize RXS-0363~0373 与 RED 语料；条款 commit 先于实现 commit。
2. **W1 骨架**：M110 分区数据模型 + 流送预算契约 → M111 HLOD 烘焙 → M120 OIT benchmark harness（仅测量）→ M119 后处理骨架 → M118 view transform 插件面（SDR 验证）。
3. **W2 大气与地表**：M112 Froxel+雾前端 → M116 地形（chunk≡cell 首发）→ M117 贴花 DBuffer+cluster → M119 DOF/分级完整化。
4. **W3 画质专项**：M112 云前端 → M113 大洋水体 → M115 皮肤 → M118 HDR 设备标定（条件触发，否则 open-留痕不假绿）。
5. **W4 精专**：M120 有界近似档→精确 linked-list 档（按 benchmark 数据裁决）→ M114 毛发（strand 档待精确档）→ M113 浅水水体。
6. **PR-Evidence**：evidence schema 落盘、RTX 4070 Ti validation-on device run，禁止 YAML-only 与 host substitution；数字 CI 步骤按落盘时实测 `CI_step.next_free` 顺位领取。

## 7. 备选方案

| 方案 | 裁决 | 理由 |
|---|---|---|
| MR（Mini-RFC）登记 D4 修订行 | 否决 | D4 是全新渲染语义面 + 触 G5 冻结面 32B 扩展；G9_CONTRACT guardrail 要求 RFC 显式修订行（M104 先例 = Full RFC-0023 §4.4.3）；MR 体例（bug fix/工具行为）不承载新语义面（§1.1） |
| 并入 RFC-0022/0023 修订行 | 否决 | 两者已 Approved 正文冻结且与 D4 不同轴；并入破坏轴纯洁性 |
| 渲染器内嵌空间管理（分区并入渲染器） | 否决（D4 D1） | 分区=数据结构先行，渲染器只消费 cell 事件；事件接口稳定可逆 |
| HLOD 运行时合并 | 否决（D4 D3） | 产物即资产、禁止运行时合并；离线烘焙确定性可双构建 hash 核验 |
| 云/雾各自独立体渲染器 | 否决（D4 D5） | 共用一个 Froxel 基础设施两个前端 |
| OIT 默认档按论文偏好直接选 AVBOIT | 否决（D4 D15） | benchmark 先行再定默认档；AVBOIT 研究风险（2025 新算法无公开 Vulkan 复现参照）转评估项 |
| 锁死单一 tonemapper（ACES only） | 否决（D4 D13） | 锁死单一 tonemapper 是 2026 架构错误；四插件并列 |
| 皮肤/毛发静默扩 MaterialClosure 32B | 否决 | §4.L 显式修订行——资产化侧表扩展通道，32B 布局 0-byte；禁静默扩 |

## 8. 不做（范围红线）

- SVT/RVT/sampler feedback 依赖（M40/41/42 G8 no-go 维持；地形/贴花显式排除，D4 D17/R-D4-7）。
- 水体浮力 gameplay 实现（M77→M124 归 G9.6 Field 通道；D4.4 只留接口面不实现 ApplyBuoyancyImpulse）。
- FG/MFG（M26）、present pacing（M49b）、GPU 粒子 VFX（M49a）、特效系统——不进 G9。
- 编辑器 GUI/世界编辑工具链 UI/网络流送/多 GPU/WebGPU/USD ingest（M86）/MaterialX（M87）。
- DMM/displacement micromap 永久禁止；NRC/神经 radiance cache 观察项不进。
- AVBOIT 实现承诺（评估项，D4 D16）；OIT 默认档定档（本 RFC 只落 benchmark harness 仅测量）。
- HDR 设备标定层在条件未触发时的假绿（登记 SKIP=not-triggered/open-留痕）。
- 不改 G5/G6/G7/G8 closed 契约与 RFC-0016/0017/0018/0019/0022/0023/0024 正文；不改 `MaterialClosure` 32B 布局（除 §4.L 登记的侧表扩展通道）；不改 RXS-0239 单 queue 全序字面、RXS-0311 capability ID 闭集（须加性修订行）。
- 不在本 RFC materialize 数字 CI 步骤、不预建空 schema 壳/空脚本占位；不领取 RXS（除 §5 顺位）/RD/U/RX 共享在途号；Approved 状态不构成实现许可。

## 9. 未决问题 / 关键裁决

| ID | 问题 | 裁决 |
|---|---|---|
| Q1 | D4 缺口处置载体 | **Full RFC**（本 RFC）——MR 无法承载新语义面 + G5 冻结面修订行；判档向上取严（§1.1） |
| Q2 | M114 毛发 strand 档 | 条件 go：card/mesh 档与 Marschner 三瓣 go；strand 档依赖 M120 精确档数据不足 → 分项 not-triggered 不充绿，承接锚「M120 精确档 benchmark 裁决数据落地后重判，兜底 G9.7 穷举」 |
| Q3 | M115/M114 触 32B | §4.L 显式修订行——资产化侧表扩展通道，32B 布局 0-byte；确需内联扩面先修订本 RFC |
| Q4 | M118 设备标定层 | 管线/插件面 SDR 可验证判 go；标定层条件未触发登记 SKIP=not-triggered 不充绿，不反向否决 SDR 验证面 |
| Q5 | OIT 默认档 | 仅测量不定档；默认档选型必须引 benchmark 数据，无数据提交判 RED；排序 fallback 永保留 |
| Q6 | spec 目标卷 | 新建 `spec/world_partition.md`（场景数据模型轴）+ `spec/display_pipeline.md`（帧图输出与着色专项轴）两卷；既有卷本体 0-byte |
| Q7 | RFC Approved 是否解锁实现 | 不；G9.5 实现波次门与 spec-first 先行是独立硬门，不得以 RFC 状态替代机器事实 |

## 9.1 对抗性评审记录（10 §3 / §7 · D-409）

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: Kimi:Kimi-K3 rfc0025-adversarial-reviewer` |
| 评审轮次 | 第 1 轮，2026-08-12 |
| 评审镜头 | ① correctness（在树事实核对：RFC-0016/0019/0022 冻结面字面、G9_ACCEPTANCE_MAP §2 M110/M118 行判据、CANDIDATE_DECISIONS M43/M45~M49 行、D4 草案 D1~D17 与 §9、MaterialClosure 32B 归属）② redline（编号 claim、冻结面静默扩、backfill 静默改写、defer 无承接锚、Draft 冒充实现许可）③ implementability（判据能否被机器断言求值、spec 映射可否 materialize） |

**单实例偏差如实登记（效力自限声明）**：本环境为单模型（Kimi-K3）子代理会话，无法派生跨工具/跨模型的独立评审实例，评审与起草同模型同实例——**此偏差大于 RFC-0024 §9.1「同工具族独立实例」先例**（该先例评审与起草为独立实例）。本 RFC 的 Agent Approved 效力据此自限：不以此冒充已获独立 provenance 评审，留待后续独立评审实例复审时可追加评审轮次；本轮评审的 findings 与 disposition 仍逐条如实登记如下。本登记沿 RFC-0016 §9.1「环境限制偏差如实登记」与 RFC-0015 §9.1 先例的精神，不构成对 D-409 字面之外效力的声称。

**Findings 与 disposition**：

| # | Finding | 严重度 | Disposition |
|---|---|---|---|
| F-1 | 初稿 §4.E 将 M114 strand 档与 card/mesh 档并列判 go，未处理「strand 档强制精确 OIT 依赖 M120 精确档，而 M120 本波仅落 benchmark harness（仅测量不定档）」的依赖缺口——违反任务「若 M120 数据可得性不足则 M114 分项 not-triggered 或 defer 留锚」口径，且与 M99/M100「条件 go + 分项 not-triggered」先例不一致 | **major** | **采纳，正文实改**：§4.E 补「条件裁决（分项 not-triggered）」段——strand 档分项 not-triggered 不充绿，承接锚「M120 精确档 benchmark 裁决数据落地后重判，兜底 G9.7 穷举」；§9 Q2 同步登记 |
| F-2 | 初稿 §4.L 修订行未引 MaterialClosure 32B 原冻结句逐字，不符合 RFC-0019 §4.1.6/RFC-0023 §4.4.3「原句→修订后句逐条列出」的修订行体例 | minor | **采纳，正文实改**：§4.L 补 RFC-0016 §4.G1 原冻结句逐字引用 + 修订后句 + 零漂移证明计划三列齐备 |
| F-3 | 初稿 §5 将毛发/皮肤目标卷写为 world_partition.md——毛发/皮肤是 closure 着色/材质求值面，与「世界分区/场景数据模型」轴不同轴，应归 display_pipeline.md（帧图输出与着色专项轴） | minor | **采纳，正文实改**：§5 目标卷裁定把 M114/M115 移至 display_pipeline.md，world_partition.md 只承载场景数据模型轴（M110/M111/M112/M113/M116/M117） |
| F-4 | 初稿 §4.J 未注明 DOF/色彩分级完整化的波内归属，与 G9_PLAN G9.5 顺位 2「DOF/分级完整化」字面衔接不明 | minor | **采纳，正文实改**：§4.J 补「本波先落骨架显式排序，DOF/色彩分级完整化在同波顺位 2 推进」衔接句 |

## 10. 稳定化与 provenance

- **特性生命周期**：RFC Agent Approved 只是语义评审完成（且本轮评审为单实例，效力自限见 §9.1）；随后仍须 spec-first/RED → 各 P0/P1 门独立断言 → 波次退出门 G-G9-7 → 稳定化报告。
- **稳定面候选**：世界分区 schema 字段闭集、三项预算契约字段、cell 四事件接口、`ViewTransform` trait 语义、OIT 三档作用域限制；是否 stable 由未来 stabilization report 裁决。
- **明确非 stable**：cell 边长数值、预算阈值水位、hitch p99 阈值、ray-march 步数档、OIT 内存界、HLOD 切换距离表——实现确定、gate 后、非 stable，经 `g9_budget.json` 实测标定。
- **Provenance**：`Assisted-by: Kimi:Kimi-K3 rfc0025-drafter`。
- 未来评审 provenance 必须不同（跨工具/跨模型独立实例），并在 §9.1 逐条 disposition 后方可追加评审轮次。

## 11. 规范与实现依据

- 仓内：[G9_CONTRACT](../milestones/g9/G9_CONTRACT.md)（§8.1 裁决① P1 全进、guardrails 触冻结面纪律）、[G9_PLAN](../milestones/g9/G9_PLAN.md) §2 G9.5 波次行、[G9_CAPABILITY_MATRIX](../milestones/g9/G9_CAPABILITY_MATRIX.md) §4 M110~M120 行与 §6.4 判据草案 6/7、[G9_CANDIDATE_DECISIONS](../milestones/g9/G9_CANDIDATE_DECISIONS.md) §5 M45~M47 行与 §6 M43/M48/M49 行、[G9_ACCEPTANCE_MAP](../milestones/g9/G9_ACCEPTANCE_MAP.md) §2 M110/M118 行（判据逐字来源）、[D4 设计草案](../milestones/g9/design/G9_D4_WORLD_AND_SPECIALTY_RENDERERS.md)（G9.0 冻结引用，D1~D17 与 §9 RFC 需求）、[R7 调研](../milestones/g9/research/R7_WORLD_AND_SPECIALTY_RENDERERS.md)。
- 冻结面归属：[RFC-0016](0016-native-renderer.md) §4.0-3/§4.G1（MaterialClosure 32B 原冻结面）、[RFC-0019](0019-rendering-platform.md) §4.5/§4.6/§4.7/§8、 capaability/profile 与时域底座、[RFC-0022](0022-virtual-geometry-gi-semantics.md) §8（32B 0-byte 重申）与 §4.7（M98 L4 依赖 HLOD 接口口径）、[RFC-0023](0023-gpu-driven-submission-shading.md) §4.4.3（G5 冻结面修订行先例）。
- 04 P-01/P-09/P-11/P-12/P-13；10 §3/§7/§9.5；13 D-406 v2.0/D-409；14 §3/§5。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-12 | 首版（G9.5 D4 缺口处置 + spec-first 授权）：D4 无伞形 RFC 缺口确认（§2.1 Grep 实测 RFC-0016/0019/0022/0023 冻结面与 D4 链路面无重叠）；判档 Full RFC（§1.1：MR 否决——新语义面 + G5 冻结面 32B 扩展须 RFC 显式修订行，M104 先例 = RFC-0023 §4.4.3）；§4 冻结 D4 十一面最小语义（M110 世界分区/M111 HLOD/M112 大气/M113 水体/M114 毛发/M115 皮肤/M116 地形/M117 贴花/M118 显示管线/M119 后处理骨架/M120 OIT benchmark），判据逐字引 G9_ACCEPTANCE_MAP §2 M110/M118 行与 §3 体例；§4.L 🔒 MaterialClosure 32B 显式修订行（资产化侧表扩展通道，32B 布局 0-byte，禁静默扩）；§5 spec 映射裁定新建 world_partition.md（RXS-0363~0368）+ display_pipeline.md（RXS-0369~0373）两卷。M114 strand 档分项 not-triggered（承接锚 M120 精确档 + G9.7 穷举）；M118 设备标定层条件未触发 SKIP=not-triggered 不充绿。D-409 第 1 轮评审完成（单实例偏差如实登记 + 效力自限，§9.1），4 findings（1 major + 3 minor）全部 disposition。零数字 CI 步骤/零空 schema 壳/零 RD/U/RX claim；RXS-0363~0373 按实测 next_free=363 顺位领取。批准不解锁实现。 | Full RFC（Agent Approved） |
