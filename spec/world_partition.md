# world_partition.md — 大世界分区与场景专项渲染器语义面（G9.5 M110/M111/M112/M113/M116/M117）

> **地位**：D4 大世界×专项渲染器「场景数据模型与场景专项」语义轴事实源——
> M110 世界分区数据模型与流送预算契约（单一持久世界 schema + 2D cell + 三项
> 流送预算契约逐帧 evidence + 预算违约注入必排队降级 + cell 四事件序列逐字
> golden + Data Layer 掩码位只预留不接线 + 代表性大世界 soak hitch p99 ≤
> measured 阈值）、M111 HLOD 烘焙管线（离线按 Component 分发 + 产物即资产 +
> 双构建 hash 相等 + 运行时零合并）、M112 大气体渲染器前端（Froxel 统一基础
> 设施 + 雾/云双前端）、M113 水体（大洋 Tessendorf IFFT 与浅水波方程双管线
> 分离）、M116 地形（chunk ≡ cell + 全 compute LOD/剔除/缝合 + 零 SVT 依赖）、
> M117 贴花（DBuffer 三通道帧图设计期占位 + screen-space cluster 化）
> （RFC-0025 §4.A~§4.D/§4.G/§4.H，Agent Approved 2026-08-12；
> G9_ACCEPTANCE_MAP §2 M110 行 + §3 M111/M112/M113/M116/M117 行〔G9.5 波 P1
> 全进裁决登记，G9_CONTRACT §8.1 裁决①〕）。G8 已冻结的页格式 ABI
> （spec/geometry_pages.md RXS-0328~0344）、几何页 streamer（M44）、磁盘异步
> I/O 链（M37/M38）、单源 gfx submit（RD-037/M89）、时域底座（RFC-0019 §4.6）
> **字面 0-byte 不动**；本文件只承载 G9.5 大世界×专项波新增语义。
>
> **档位**：Full RFC / RFC-0025。
>
> **编号**：RXS-0363~0368（G9.5 spec-first，自合入时实测
> `registry/number_ledger.json` `RXS.next_free = 363` 顺位领取，0363~0368
> 连续不跳号；编号永不复用，10 §9.5）。
>
> **新建裁决留痕（G9.5 spec PR）**：RFC-0025 §5 条款映射表裁定——D4 十一行
> 分两个独立语义轴新建两卷：本卷承载「大世界数据模型与场景专项」轴
> （M110/M111/M112/M113/M116/M117，RXS-0363~0368），`spec/display_pipeline.md`
> 承载「帧图输出与材质着色专项」轴（M118/M119/M120/M114/M115，
> RXS-0369~0373）。候选既有卷（rendering_platform.md 的 reflection/capability/
> 时域面、shader_stages.md 的语言类型面、geometry_pages.md 的页 ABI 面）与
> 两轴均不同轴，本体 0-byte。新建裁决沿 G9.2 virtual_geometry.md / G9.4
> global_illumination.md 新建先例（spec/README.md §4 登记 + 本头注留痕）。
>
> **SVT/RVT 排除**：M40/41/42 G8 close-out no-go 维持——地形与贴花语义面
> **不得依赖虚拟纹理**（D4 D17/R-D4-7；RXS-0367 零 SVT 依赖断言为硬条款）。
>
> **浮力边界**：水体浮力 gameplay 联动（M77→M124）归 G9.6 物理波 Field 通道；
> 本卷只冻结浮力查询接口面的**预留不实现**纪律（RXS-0366 L5）。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——分区流送、
  烘焙、大气、水体、地形与贴花所有失败均为 typed `Err` / 确定性拒绝
  （fail-closed），不设未定义行为。
- 实现锚定（实现期命名，`src/rurix-render` 维持 `forbid(unsafe_code)` 纪律）：
  M110 世界 schema 加载器 + 流送调度器（距离环 target/resident diff）+ 预算
  计数面 + cell 事件总线；M111 HLOD 离线 Builder（`src/rurix-asset` cook 面
  扩写）与运行时互斥切换面；M112 Froxel 基础设施与雾/云前端 compute pass；
  M113 大洋 IFFT 与浅水波方程双管线；M116 地形全 compute LOD/剔除/缝合与
  toroidal 更新；M117 DBuffer 合成与贴花 cluster 化。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定（traceability 矩阵全锚定，10 §4）。

## 2. 术语

- **单一持久世界（single persistent world）**：UE World Partition 范式——全
  世界一份持久资产，运行时按 2D 网格 cell 流送；schema 层显式区分
  `always_loaded`（全局/gameplay 关键对象）与 `spatially_loaded`（空间分格
  对象）（RFC-0025 §4.A）。
- **cell**：正方形 2D 网格单元，边长为资产属性（非代码常量）；cell 元数据
  v1 = cell id、包围盒、资产页引用（M04 ABI）、HLOD 层级引用；**Data Layer
  掩码位只预留不接线**（v2 才实现激活语义，D4 D4）。
- **三项流送预算契约**：`MaxStreamingCellsPerFrame` /
  `MaxActorsToSpawnPerFrame` / `MemoryBudgetMB` 为一等契约字段；超预算请求
  **排队而非抢占**；预算计数器逐帧落 evidence（RFC-0025 §4.A）。
- **cell 四事件**：`CellLoadBegin / CellResident / CellUnloadBegin /
  CellEvicted`——渲染器唯一消费面；地形 chunk、HLOD、贴花 cluster 重算、
  流送光源集均挂事件，不反向查询分区状态（D4 D1）。
- **HLOD**：离线烘焙的分层代理几何；Builder 按 Component 分发，**产物即资产，
  禁止运行时合并**（D4 D3）。
- **Froxel**：视锥体素网格（froxel volume）+ 密度/光照累积 + 深度切片分布
  + 帧图合成节点；云与雾共用同一基础设施、两个前端（D4 D5）。
- **双管线分离（水体）**：大洋 Tessendorf IFFT 谱管线和浅水波方程管线不共享
  几何路径，仅共享水面着色 closure 输入面（D4 D8）。
- **chunk ≡ cell**：地形 chunk 与世界分区 cell 对齐同一网格族，禁止第二套
  分格（D4 D11）。
- **DBuffer**：贴花三通道（法线 + 材质属性 + 可选第三通道）在 G-buffer pass
  内合成，帧图设计期即占位（D4 D12）。
- **toroidal 更新**：相机移动时地形数据环形窗口滚动复用 ring buffer，避免
  全量重传。

---

## 3. 条款（RXS-0363，G9.5 M110 世界分区）

### RXS-0363 单一持久世界 schema、2D cell、三项流送预算契约与 cell 四事件序列

**Legality**

1. **单一持久世界 schema 与 2D cell 冻结**（RFC-0025 §4.A 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M110 行）：单一 persistent world 资产；schema 层显式
   区分 `always_loaded` 与 `spatially_loaded`；每个 spatially-loaded 对象
   携带 cell 归属；cell 为正方形 2D 网格，**边长为资产属性，非代码常量**；
   cell 元数据 v1 字段闭集 = cell id、包围盒、资产页引用（M04 ABI，页格式
   只消费不重定）、HLOD 层级引用（烘焙工具语义锚 RXS-0364）；**Data Layer
   掩码位只预留不接线**——v1 只预留位、不实现激活语义，避免 schema 二次
   迁移（D4 D4）。
2. **流送运行时**（RFC-0025 §4.A 逐字）：streaming source（相机/玩家/自定义
   探针）携带距离环（loading radius / 内环常驻）；每帧由距离环求 target
   cell 集合，与 resident 集合 diff 出 load/unload 队列。
3. **三项预算契约逐帧 evidence**（判据逐字引 G9_ACCEPTANCE_MAP §2 M110 行）：
   `MaxStreamingCellsPerFrame`、`MaxActorsToSpawnPerFrame`、`MemoryBudgetMB`
   为一等契约字段；超预算请求**排队而非抢占**；**预算计数器逐帧落
   evidence**（hitch 审计的数据源），三项逐帧 evidence 非空。
4. **预算违约注入必排队降级（RED 臂）**（判据逐字引 G9_ACCEPTANCE_MAP §2
   M110 行）：**预算违约注入必须排队降级、不得静默超帧**——注入违约（如
   `MaxStreamingCellsPerFrame=0`）必须触发可见降级且计数器报警，出现静默
   超帧即 RED；该负例臂独立于正例臂成立，臂失效（违约注入不红）即漏检，
   本条款整体 FAIL。
5. **cell 四事件序列逐字 golden**（判据逐字引 G9_ACCEPTANCE_MAP §2 M110
   行）：`CellLoadBegin / CellResident / CellUnloadBegin / CellEvicted` 四
   事件为渲染器唯一消费面；固定相机轨迹的 cell 四事件（load/unload/
   activate/deactivate 类）序列与 golden **逐字相等**（确定性）；事件乱序
   variant 必须被判 RED。
6. **soak hitch p99 ≤ measured 阈值**（判据逐字引 G9_ACCEPTANCE_MAP §2 M110
   行）：代表性大世界 soak（≥ G7 量级：≥30min / ≥10000 帧，G8.8a 口径继承）
   hitch p99 ≤ measured 阈值——**阈值来自 `g9_budget.json` 实测标定，禁
   手写**（先 measured 后冻结，P-09；本条款不预造数字，阈值条目随实现波
   measured 追加进 `g9_budget.json`）。
7. **底座消费纪律**（RFC-0025 §3 逐字）：cell 资产打包页格式复用 M04 ABI；
   cell 流送 I/O 腿复用 M37 磁盘异步 I/O + M38 解压链；cell 内几何按需驻留
   复用 M44 几何页 streamer——三者只消费不重定，字面 0-byte。

**Implementation Requirements**

- 实现锚定（实现期命名，`forbid(unsafe_code)` 纪律维持）：世界 schema
  加载器 + 流送调度器（距离环 target/resident diff）+ 三项预算计数面 +
  cell 事件总线（四事件唯一消费面）；device 侧 FFI 确需时按当时
  `U.next_free` 实测顺位登记 unsafe-audit。
- RED 锚定计划（实现 PR 落）：预算违约注入未排队降级 → RED；cell 事件
  乱序注入 → RED；固定相机轨迹四事件序列逐字 golden；soak hitch p99 实测
  阈值核验（阈值经 `g9_budget.json` measured 标定后生效）。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/world_partition/reject/partition_budget_overrun_no_demote.rx`、
  `conformance/world_partition/reject/cell_event_sequence_out_of_order.rx`
  与正例 `conformance/world_partition/accept/cell_event_sequence_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_world_partition_smoke.py` 门（symbolic key
  `g9.p0.m110.world_partition`，G9.1 冻结字面 0-byte 不动）。

---

## 4. 条款（RXS-0364，G9.5 M111 HLOD 烘焙管线）

### RXS-0364 HLOD 离线烘焙按 Component 分发、产物即资产、双构建 hash 相等与运行时零合并

**Legality**

1. **离线 Builder 按 Component 分发**（RFC-0025 §4.B 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M111 行）：离线 Builder 输入 cell/Component 划分，
   逐 Component 生成代理几何（简化 + 合批 + 材质合并）；产物经 M01/M04
   管线落成普通资产（**产物即资产**），走同一 cook/DDC/页格式通道——不
   私定磁盘格式。
2. **双构建 hash 相等**（判据逐字引 G9_ACCEPTANCE_MAP §3 M111 行）：同输入
   两次独立烘焙产物 digest 逐位一致（沿 M79 判据形态）；烘焙输入扰动
   （路径/声明序/线程数）不得影响产物 hash。
3. **运行时零合并断言（RED 臂）**（判据逐字引 G9_ACCEPTANCE_MAP §3 M111
   行）：HLOD 代理与 cell 全量内容按 screen-size 阈值互斥切换（cell 级
   HLOD 树，层数为烘焙属性）；**运行时合并调用尝试即断言/装配期拒绝**
   （RED 臂独立有效）。
4. **代理质量 golden**（判据逐字引 G9_ACCEPTANCE_MAP §3 M111 行）：代理
   相对原始的屏幕空间误差上界作为 golden 断言；切换距离表 golden。
5. **M98 L4 依赖面衔接**（RFC-0025 §4.B 逐字）：本条款即 RFC-0022 §4.7
   M98 L4 Far Field 所依赖的 HLOD 接口；M98 行「L4 依赖 HLOD 接口未就绪
   时登记 SKIP=not-triggered 不充绿」字面 0-byte 维持，本条款不反向改写。

**Implementation Requirements**

- 实现锚定（实现期命名，纯 safe 方向维持）：HLOD 离线 Builder
  （`src/rurix-asset` cook 面扩写）+ 运行时 screen-size 互斥切换面 +
  零合并结构性断言面。
- RED 锚定计划（实现 PR 落）：运行时合并调用尝试 → RED；双构建 hash
  漂移 → RED；代理误差上界 golden 与切换距离表 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/world_partition/reject/hlod_runtime_merge_forbidden.rx`
  与正例 `conformance/world_partition/accept/hlod_baking_double_build_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_hlod_baking_smoke.py` 门（symbolic key
  `g9.p1.m111.hlod_baking`，G9.5 波 P1 登记字面不动）。

---

## 5. 条款（RXS-0365，G9.5 M112 大气体渲染器前端）

### RXS-0365 Froxel 统一基础设施与雾/云双前端、weather map 资产化与时序上采样默认路径

**Legality**

1. **Froxel 统一基础设施**（RFC-0025 §4.C 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M112 行）：视锥体素网格（froxel volume）+ 密度/
   光照累积 + 深度切片分布 + 与帧图的合成节点一次性建造；**云与雾共用
   同一 Froxel 基础设施、两个前端**（D4 D5）——各自独立体渲染器即 RED
   （云雾共用同一基础设施断言）。
2. **雾前端**（RFC-0025 §4.C 逐字）：高度雾/分层介质直接写 Froxel 密度场，
   解析项为主，预算极小。
3. **云前端**（RFC-0025 §4.C 逐字；判据逐字引 G9_ACCEPTANCE_MAP §3 M112
   行）：Perlin-Worley 低频塑形 + Worley 高频侵蚀（噪声 baker 离线产 3D
   纹理，M01 资产）+ 2D weather map（覆盖度/湿度/类型）**资产化走
   M01/M85 通道**（禁硬编码参数，D4 D7）；低分辨率 ray-march + temporal
   reprojection **时序上采样为默认路径**（D4 D6），全分辨率列为高端档。
4. **预算契约**（RFC-0025 §4.C 逐字；判据逐字引 G9_ACCEPTANCE_MAP §3 M112
   行）：ray-march 最大步数、froxel 分辨率档、上采样开关均为预算字段，
   逐帧 evidence 非空；每档有 measured 帧时证据（measured 写 evidence，
   阈值先 measured 后冻结，P-09）。
5. **资产完整性与时序初始化（RED 臂）**（判据逐字引 G9_ACCEPTANCE_MAP §3
   M112 行）：**篡改 weather map 资产签名即拒录**（RED 臂独立有效）；
   时序链断裂（首帧无历史）必须正确初始化，不得复用脏帧。

**Implementation Requirements**

- 实现锚定（实现期命名）：Froxel 基础设施（体素分配/密度/光照累积/深度
  切片/合成节点）+ 雾前端解析项写入 + 云前端噪声 baker 与 ray-march
  compute kernel + 时序上采样 pass（temporal 公共底座消费面，禁私写重
  投影）；AVBOIT 体素结构评估时复用同一体素内存布局族（D4 D16，评估项
  不承诺）。
- RED 锚定计划（实现 PR 落）：weather map 篡改签名 → 拒录 RED；首帧无
  历史复用脏帧 → RED；云雾各自独立体渲染器 variant → RED；逐档预算
  字段逐帧 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/world_partition/reject/atmosphere_weather_map_signature_tampered.rx`
  与正例 `conformance/world_partition/accept/atmosphere_froxel_fog_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_atmosphere_froxel_smoke.py` 门（symbolic key
  `g9.p1.m112.atmosphere_froxel`，G9.5 波 P1 登记字面不动）。

---

## 6. 条款（RXS-0366，G9.5 M113 水体双管线）

### RXS-0366 大洋 Tessendorf IFFT 与浅水波方程双管线分离、浮力接口面预留不实现

**Legality**

1. **大洋管线**（RFC-0025 §4.D 逐字；判据逐字引 G9_ACCEPTANCE_MAP §3 M113
   行）：Tessendorf IFFT 谱离线参数化（风向/风速/涌浪为资产属性）+ 运行时
   compute IFFT（或周期谱表寻址档）；位移/梯度/Jacobian 三贴图，Jacobian
   负值驱动泡沫；CDLOD 距离分档 mesh；多尺度谱 tiling-and-blending 防周期
   重复感；大洋 compute IFFT 位移/梯度/Jacobian 与 host FFT 参考**逐值
   对拍**（容差域经本条款面明示冻结，禁手写掩盖，P-09）。
2. **浅水管线**（RFC-0025 §4.D 逐字）：局部波方程（高度场 + 速度场
   ping-pong compute）服务池塘/河流/交互波纹。
3. **双管线分离断言**（判据逐字引 G9_ACCEPTANCE_MAP §3 M113 行）：大洋与
   浅水**不共享几何路径，仅共享水面着色 closure 输入面**（D4 D8）——
   几何路径互斥机核断言，互斥违反即 RED。
4. **非法谱参数资产拒录（RED 臂）**（判据逐字引 G9_ACCEPTANCE_MAP §3 M113
   行）：负风速/非法谱参数资产 → 装配期拒录（RED 臂独立有效）；浅水域
   越界写检测（越界写即 RED）。
5. **浮力接口面预留不实现**（RFC-0025 §4.D 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M113 行）：浮力查询接口面**预留不实现**——
   M77→M124 归 G9.6 物理波 Field 通道（RFC-0024 §4.D），本条款不授权任何
   浮力实现或旁路 API。

**Implementation Requirements**

- 实现锚定（实现期命名）：大洋 IFFT 谱 baker（离线）+ compute IFFT 运行时
  + CDLOD 分档 + 浅水波方程 ping-pong compute + 水面着色 closure 输入
  共享面；两管线几何路径互斥结构性断言。
- RED 锚定计划（实现 PR 落）：负风速/非法谱参数资产 → 拒录 RED；浅水
  域越界写 → RED；双管线几何路径互斥违反 → RED；IFFT 三贴图与 host
  FFT 参考逐值 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/world_partition/reject/water_spectrum_param_invalid.rx`
  与正例 `conformance/world_partition/accept/water_dual_pipeline_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_water_dual_pipeline_smoke.py` 门（symbolic key
  `g9.p1.m113.water_dual_pipeline`，G9.5 波 P1 登记字面不动）。

---

## 7. 条款（RXS-0367，G9.5 M116 地形）

### RXS-0367 地形 chunk ≡ cell、全 compute LOD/剔除/缝合、toroidal 更新与零 SVT 依赖断言

**Legality**

1. **chunk ≡ cell**（RFC-0025 §4.G 逐字；判据逐字引 G9_ACCEPTANCE_MAP §3
   M116 行）：heightfield 数据为 M04 页格式资产；**chunk ≡ M110 cell**
   （尺寸对齐同一网格族，**禁第二套分格**，D4 D11）——出现独立地形分格
   即 RED（chunk ≡ cell 断言）。
2. **全 compute LOD/剔除/缝合**（RFC-0025 §4.G 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M116 行）：LOD 选择/视锥剔除/邻级缝合（stitch
   skirt 或指数网格 morph）全部进 compute，产出 indirect draw；**CPU 侧零
   逐 chunk 提交**断言。
3. **toroidal 更新**（RFC-0025 §4.G 逐字）：相机移动时环形窗口滚动复用
   ring buffer，避免全量重传；与 M37 I/O 链直接对接——chunk 页迟到 →
   父级 LOD 占位（沿 M44 迟到页降级语义不重定，字面 0-byte）。
4. **零 SVT 依赖断言**（判据逐字引 G9_ACCEPTANCE_MAP §3 M116 行）：地形
   **不得依赖 SVT/RVT/sampler feedback**（D4 D17；M40/41/42 G8 close-out
   no-go 维持）——出现虚拟纹理依赖即 RED；真实大纹理资产需求走独立判档，
   不搭 D4 便车（R-D4-7）。
5. **缝合裂缝 RED 臂**（判据逐字引 G9_ACCEPTANCE_MAP §3 M116 行）：**相邻
   chunk LOD 差 >1 注入必须触发缝合路径，出现裂缝像素即 RED**（RED 臂
   独立有效）；邻级缝合处顶点位置连续性 golden（裂缝=0）。

**Implementation Requirements**

- 实现锚定（实现期命名）：地形 heightfield 页资产消费面 + 全 compute
  LOD/剔除/缝合 kernel 族 + indirect draw 产出面 + toroidal ring buffer
  更新面；与 RXS-0363 cell 事件面挂接（cell 加载事件消费方）。
- RED 锚定计划（实现 PR 落）：第二套分格注入 → RED；SVT 依赖注入 →
  RED；邻级 LOD 差 >1 未缝合出裂缝像素 → RED；缝合处顶点位置连续性
  golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/world_partition/reject/terrain_lod_gap_crack.rx`
  与正例 `conformance/world_partition/accept/terrain_chunk_cell_aligned_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_terrain_chunk_cell_smoke.py` 门（symbolic key
  `g9.p1.m116.terrain_chunk_cell`，G9.5 波 P1 登记字面不动）。

---

## 8. 条款（RXS-0368，G9.5 M117 贴花 DBuffer）

### RXS-0368 贴花 DBuffer 三通道帧图设计期占位、screen-space cluster 化与前向回退档

**Legality**

1. **DBuffer 三通道帧图设计期占位**（RFC-0025 §4.H 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M117 行）：DBuffer（法线 + 材质属性 + 可选第三
   通道）在 G-buffer pass 内合成，**帧图设计期即占位**——即使 v1 贴花
   数量为零，通道与 barrier 布局先行冻结（D4 D12），避免后期插 pass 改
   全局帧图；**缺占位即 RED**（帧图占位断言）。
2. **screen-space cluster 化**（RFC-0025 §4.H 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M117 行）：screen-space cluster（复用光照 cluster
   结构）对贴花体求交，限制逐像素贴花评估数上界；过绘制计数器落
   evidence 非空。
3. **前向回退档**（RFC-0025 §4.H 逐字；判据逐字引 G9_ACCEPTANCE_MAP §3
   M117 行）：无 DBuffer 的低端 profile 走 decal-forward pass，v1 即定义
   **两档语义等价性判据** golden。
4. **超界受界降级（RED 臂）**（判据逐字引 G9_ACCEPTANCE_MAP §3 M117 行）：
   **超 cluster 上界贴花密度注入必须受界降级，过绘制计数越界即 RED**
   （RED 臂独立有效）。
5. **零 SVT 依赖**：贴花语义面同样不得依赖 SVT/RVT（D4 D17 同口径，本
   条款不重引 RXS-0367 L4 字面，断言同构）。

**Implementation Requirements**

- 实现锚定（实现期命名）：DBuffer 三通道合成 pass（帧图设计期占位）+
  贴花 cluster 求交与逐像素评估数受界面 + 过绘制计数面 + 前向回退档
  pass；barrier 布局沿 RFC-0016 §4.A EB 三轴推导面 0-byte。
- RED 锚定计划（实现 PR 落）：DBuffer 通道占位缺失 → RED；超 cluster
  上界密度注入未受界降级 → RED；两档语义等价 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/world_partition/reject/decal_overdraw_budget_exceeded.rx`
  与正例 `conformance/world_partition/accept/decal_dbuffer_placeholder_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_decal_dbuffer_smoke.py` 门（symbolic key
  `g9.p1.m117.decal_dbuffer`，G9.5 波 P1 登记字面不动）。

---

## 9. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-12 | 新建（G9.5 spec-first，大世界×专项波场景数据模型轴，硬规则 7 条款先行）：RXS-0363（M110 单一持久世界 schema + 2D cell + 三项流送预算契约逐帧 evidence + 预算违约注入必排队降级 RED + cell 四事件序列逐字 golden + Data Layer 掩码位只预留不接线 + soak hitch p99 ≤ measured 阈值〔g9_budget.json 实测标定禁手写〕）/ RXS-0364（M111 HLOD 离线烘焙按 Component 分发 + 产物即资产 + 双构建 hash 相等 + 运行时零合并断言 + screen-size 互斥切换 golden）/ RXS-0365（M112 Froxel 统一基础设施云雾共用 + 雾/云双前端 + weather map 资产化走 M01/M85 + 时序上采样默认路径 + 篡改签名拒录 RED）/ RXS-0366（M113 大洋 Tessendorf IFFT 与浅水波方程双管线分离 + 非法谱参数拒录 RED + 浮力接口面预留不实现）/ RXS-0367（M116 地形 chunk ≡ cell 禁第二套分格 + 全 compute LOD/剔除/缝合 + toroidal 更新 + 零 SVT 依赖断言 + 邻级 LOD 差>1 注入裂缝 RED）/ RXS-0368（M117 贴花 DBuffer 三通道帧图设计期占位 + screen-space cluster 化受界 + 前向回退档语义等价 golden + 超界注入降级 RED）。**目标 spec 新建裁决**：RFC-0025 §5 映射表裁定 D4 两轴新建两卷——本卷承载场景数据模型轴（M110/M111/M112/M113/M116/M117），display_pipeline.md 承载帧图输出与着色专项轴（M118/M119/M120/M114/M115）；既有卷本体 0-byte（头注留痕，沿 G9.2/G9.4 新建先例）。条款号自 ledger 实测 `RXS.next_free=363` 顺位领取（0363~0368 连续不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。conformance 最小锚定语料同 PR 落（conformance/world_partition/{accept,reject}/，inert + `//@ spec` 锚定 + 预期诊断注释 + 转正路径旁注，G9.2~G9.4 spec 波先例）；symbolic key `g9.p0.m110.world_partition`（G9.1 冻结字面）与 `g9.p1.m111/m112/m113/m116/m117.*`（G9.5 波 P1 全进裁决登记，G9_ACCEPTANCE_MAP §3 / CI_GATES §4A）0-byte 不动。零新 RX 码（诊断码实现期按实际可达类别领取不预造）、零新 U/RD/SG、零 src/ 改动、零 workflow 步骤。依据 [RFC-0025](../rfcs/0025-world-and-specialty-renderers.md)（Agent Approved 2026-08-12）§4.A~§4.D/§4.G/§4.H + G9_ACCEPTANCE_MAP §2 M110 行 + §3 M111/M112/M113/M116/M117 行（判据逐字）+ G9_CANDIDATE_DECISIONS §6 M43/M48/M49 行与 v1.4 校准注 | **Full RFC**（RFC-0025） |
