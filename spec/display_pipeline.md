# display_pipeline.md — 显示管线与材质着色专项语义面（G9.5 M118/M119/M120/M114/M115）

> **地位**：D4 大世界×专项渲染器×显示管线「帧图输出与材质着色专项」语义轴
> 事实源——M118 HDR 显示管线与可插拔 view transform（SDR/scRGB/PQ 三交换链
> 路径运行时切换 + ACES 1.3/2.0/AgX/中性四内置插件逐一 golden + 非 HDR 交换链
> 携带 PQ 输出即 RED + HDR 设备标定未触发 SKIP=not-triggered 不充绿）、M119
> 后处理骨架（histogram 曝光+EV → bloom → tonemap → LUT → 输出变换显式排序 +
> 全程 HDR 线性域 + 曝光状态帧间持久 + 与 TAA/TSR 显式排序）、M120 OIT
> benchmark harness（nvpro 七算法对照，仅测量不定档，evidence 非空）、M114
> 毛发（Marschner R/TT/TRT 三瓣 + 几何三档，strand 档强制精确 OIT 分项
> not-triggered）、M115 皮肤（Burley normalized diffusion 屏空单 pass + 扩散
> profile 资产化 + pre-integrated LUT 回退档）（RFC-0025 §4.E/§4.F/§4.I~§4.K，
> Agent Approved 2026-08-12；G9_ACCEPTANCE_MAP §2 M118 行 + §3
> M119/M120/M114/M115 行〔G9.5 波 P1 全进裁决登记，G9_CONTRACT §8.1 裁决①〕）。
> G8 已冻结的 TSR/TAA 生产契约（M24，RFC-0019 §4.6）、present 面
> （RXS-0220~0222）、shader/PSO manifest↔DDC（M85）与 `MaterialClosure` 32B
> 单层布局（RFC-0016 §4.G1）**字面 0-byte 不动**；本文件只承载 G9.5 大世界×
> 专项波新增语义。
>
> **档位**：Full RFC / RFC-0025。
>
> **编号**：RXS-0369~0373（G9.5 spec-first，自合入时实测
> `registry/number_ledger.json` `RXS.next_free = 363` 顺位领取——同批
> world_partition.md 领 0363~0368，本卷领 0369~0373，连续不跳号；编号永不
> 复用，10 §9.5）。
>
> **新建裁决留痕（G9.5 spec PR）**：RFC-0025 §5 条款映射表裁定——D4 十一行
> 分两个独立语义轴新建两卷：`spec/world_partition.md` 承载「大世界数据模型
> 与场景专项」轴（M110/M111/M112/M113/M116/M117，RXS-0363~0368），本卷承载
> 「帧图输出与材质着色专项」轴（M118 显示输出 / M119 后处理 / M120 透明合成
> benchmark / M114 毛发着色 / M115 皮肤着色——着色档面与 closure 扩展通道同
> 轴登记，RXS-0369~0373）。候选既有卷（rendering_platform.md 的 reflection/
> capability/时域面、shader_stages.md 的语言类型面）与本轴不同轴，本体
> 0-byte。新建裁决沿 G9.2 virtual_geometry.md / G9.4 global_illumination.md
> 新建先例（spec/README.md §4 登记 + 本头注留痕）。
>
> **MaterialClosure 32B 边界**：M114/M115 的 Marschner/Burley lobe 参数经
> RFC-0025 §4.L 🔒 显式修订行登记的**资产化侧表扩展通道**接入单层 closure
> 求值——32B 定长布局、字段含义与 flags 位段分配 0-byte，预留拓扑字段位不
> 消费（RFC-0019 §4.7 口径维持），**禁静默扩**；任何 32B 内联扩面必须停手
> 先修订 RFC-0025。
>
> **HDR 设备标定边界**：M118 拆两层——管线/插件面 SDR 上即可全量验证（本卷
> 冻结）；HDR 设备标定层条件未触发（缺 HDR 设备资产）登记 SKIP=not-triggered
> 不充绿，且**不反向否决** SDR 上可全量验证的管线/插件面（判据逐字引
> G9_ACCEPTANCE_MAP §2 M118 行）。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——交换链切换、
  view transform 插件、后处理排序、OIT 档位与毛发/皮肤着色所有失败均为
  typed `Err` / 确定性拒绝（fail-closed），不设未定义行为。
- 实现锚定（实现期命名，`src/rurix-render` 维持 `forbid(unsafe_code)` 纪律）：
  M118 三交换链路径与 `ViewTransform` 插件面；M119 后处理骨架 pass 链与曝光
  persistent resource；M120 OIT benchmark harness（nvpro 七算法对照）；M114
  Marschner 着色 kernel 与几何三档切换；M115 Burley 屏空 separable SSS 双
  kernel 与 LUT 回退档。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定（traceability 矩阵全锚定，10 §4）。

## 2. 术语

- **三交换链路径**：SDR（Rec.709）/ scRGB / PQ-Rec.2020 三条 swapchain 路径
  为一等资源，运行时切换（重建交换链或双链过渡）；HDR 元数据（MaxCLL/
  MaxFALL/mastering primaries）由输出变换阶段填写（RFC-0025 §4.I）。
- **view transform 插件面**：`ViewTransform` trait（输入 HDR 线性 + 显示参数
  → 输出编码）；ACES 1.3、ACES 2.0、AgX、中性矩阵四内置实现并列，第三方可
  注册（D4 D13——锁死单一 tonemapper 是 2026 架构错误）。
- **已知差异记录**：AgX/ACES hue-skew 等公认差异写进 golden 记录，不作 bug
  返工（D4 R-D4-5）；AgX 对比度补偿参数随 view transform 资产化，禁止硬
  编码进 tonemap 节点。
- **后处理骨架显式排序**：exposure（histogram + 手动 EV 偏移）→ bloom
  （tonemap 前 HDR 域多尺度 mip 链）→ DOF（scatter-as-gather）→ tonemap
  （经 view transform 插件）→ 色彩分级（LUT 资产，tonemap 后）→ 输出变换
  → UI 合成（SDR 域）；本波先落骨架（曝光/bloom/tonemap/LUT/输出变换），
  DOF/色彩分级完整化在同波顺位 2 推进（RFC-0025 §4.J）。
- **OIT 三档**：①默认档（TAA 半透明合成路径，现状延伸）②有界近似档
  （WBOIT 起步、AVBOIT 目标——**评估项不承诺**，D4 D16）③精确档
  （linked-list per-pixel fragment list，仅毛发 strand 启用，场景级不开放）；
  排序 fallback（depth-sorted alpha）永保留为最低端档与正确性对照。
- **仅测量不定档**：M120 本门只产 benchmark 数据，**不定默认档**；默认档
  选型由 benchmark 数据裁决，不由论文偏好裁决（D4 D15）。
- **Marschner 三瓣**：R/TT/TRT 三瓣，纵向/方位角分离参数化为资产属性（每缕
  基调色、高光偏移、medulla 配置）。
- **Burley normalized diffusion**：屏空单 pass separable SSS（颜色/深度双
  kernel）；扩散 profile（RGB 三通道 falloff 参数）为 per-material 资产；
  pre-integrated LUT（曲率 × NdotL）为低端回退档。
- **材质参数侧表**：RFC-0025 §4.L 登记的扩展通道——专项 lobe 参数（Burley
  扩散 profile / Marschner 参数集）作为资产化参数侧表按材质槽 ID 索引接入
  单层 closure 求值，经 M01/M85 资产通道烘焙/打包/manifest 入 DDC；侧表
  缺省 ≡ 无专项 lobe，既有材质输出逐位不变。

---

## 3. 条款（RXS-0369，G9.5 M118 显示管线 view transform）

### RXS-0369 SDR/scRGB/PQ 三交换链路径运行时切换与 ACES 1.3/2.0/AgX/中性四内置插件逐一 golden

**Legality**

1. **三交换链路径运行时切换**（RFC-0025 §4.I 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M118 行）：SDR（Rec.709）/ scRGB / PQ-Rec.2020 三条
   swapchain 路径为一等资源，运行时切换（重建交换链或双链过渡），三路径
   切换证据齐备；HDR 元数据（MaxCLL/MaxFALL/mastering primaries）由输出
   变换阶段填写。
2. **四内置插件逐一 golden**（判据逐字引 G9_ACCEPTANCE_MAP §2 M118 行）：
   `ViewTransform` trait（输入 HDR 线性 + 显示参数 → 输出编码）插件面，
   **ACES 1.3、ACES 2.0、AgX、中性矩阵四内置实现并列**且**逐一**对冻结
   golden（含 AgX/ACES hue-skew **已知差异记录**）；AgX 对比度补偿参数随
   view transform 资产化，禁止硬编码进 tonemap 节点；未注册插件名调用 →
   拒录 RED。
3. **非 HDR 交换链携带 PQ 输出即 RED**（判据逐字引 G9_ACCEPTANCE_MAP §2
   M118 行）：非 HDR 交换链（SDR/scRGB）携带 PQ 输出即 RED——该负例臂
   独立于正例臂成立，臂失效（违规不红）即漏检，本条款整体 FAIL。
4. **HDR 设备标定层条件未触发登记**（判据逐字引 G9_ACCEPTANCE_MAP §2 M118
   行）：HDR 设备标定层条件未触发（缺 HDR 设备资产）时登记
   **SKIP=not-triggered 或 open-留痕、不假绿**——条件未触发只表示决策已
   记录，不是成功；且标定层未触发**不得反向否决** SDR 上可全量验证的
   管线/插件面（L1/L2 判据照常在 SDR 验证）。

**Implementation Requirements**

- 实现锚定（实现期命名）：三交换链路径装配/切换面（present 面 RXS-0220~
  0222 语义 0-byte 复用）+ `ViewTransform` trait 与四内置插件实现 + HDR
  元数据填写面 + 插件注册核验；窗口腿维持 D-130 红线（C++ shim）0-byte。
- RED 锚定计划（实现 PR 落）：非 HDR 交换链携带 PQ 输出 → RED；未注册
  插件名调用 → 拒录 RED；四插件逐一输出 golden（含已知差异记录）。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/display_pipeline/reject/non_hdr_swapchain_pq_output.rx`
  与正例 `conformance/display_pipeline/accept/view_transform_four_plugins_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_display_pipeline_view_transform_smoke.py` 门
  （symbolic key `g9.p0.m118.display_pipeline_view_transform`，G9.1 冻结
  字面 0-byte 不动）。

---

## 4. 条款（RXS-0370，G9.5 M119 后处理骨架）

### RXS-0370 后处理骨架显式排序、全程 HDR 线性域与曝光状态帧间持久

**Legality**

1. **显式排序冻结**（RFC-0025 §4.J 逐字；判据逐字引 G9_ACCEPTANCE_MAP §3
   M119 行）：后处理骨架节点顺序冻结（帧图语义）——histogram 曝光+EV 偏移
   → bloom（tonemap 前 HDR 域多尺度 mip 链，down/up 双 pass）→ tonemap
   （经 RXS-0369 view transform 插件）→ 色彩分级（LUT 资产，tonemap 后）→
   输出变换；**SDR 上即可全量验证**（不依赖 HDR 设备）；DOF（scatter-as-
   gather）/色彩分级完整化在同波顺位 2 推进，其节点位置（bloom 后、
   tonemap 前）随本条款一并冻结。
2. **全程 HDR 线性域（RED 臂）**（判据逐字引 G9_ACCEPTANCE_MAP §3 M119
   行）：全链任何节点不得隐式 clamp 到 SDR——**隐式 SDR clamp 注入即探针
   越界 RED**（节点输出范围探针；RED 臂独立有效）。
3. **曝光状态帧间持久**（判据逐字引 G9_ACCEPTANCE_MAP §3 M119 行）：
   histogram → 目标 EV 的 adapt（上/下不同速率）状态为 persistent
   resource 帧间持久；**曝光状态跨帧丢失注入即 RED**（RED 臂独立有效）。
4. **与 TAA/TSR 时域链显式排序**（RFC-0025 §4.J 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M119 行）：bloom/DOF 输出与 TAA/TSR（M24）resolve
  顺序在帧图中显式声明；曝光状态供 TSR 消费；M24 时域底座字面 0-byte。

**Implementation Requirements**

- 实现锚定（实现期命名）：后处理骨架 pass 链（曝光 histogram/EV → bloom
  多尺度 mip → tonemap 插件 → LUT → 输出变换）+ 节点输出范围探针面 +
  曝光 persistent resource（外部资源双缓冲，沿 RFC-0016 §4.0-3 纪律）+
  与 TAA/TSR 时域链排序声明确认面。
- RED 锚定计划（实现 PR 落）：隐式 SDR clamp 注入 → 探针越界 RED；曝光
  状态跨帧丢失注入 → RED；节点显式排序 golden；曝光 adapt 曲线 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/display_pipeline/reject/post_stack_implicit_sdr_clamp.rx`
  与正例 `conformance/display_pipeline/accept/post_stack_explicit_order_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_post_processing_skeleton_smoke.py` 门（symbolic
  key `g9.p1.m119.post_processing_skeleton`，G9.5 波 P1 登记字面不动）。

---

## 5. 条款（RXS-0371，G9.5 M120 OIT benchmark harness）

### RXS-0371 OIT benchmark harness 仅测量不定档、默认档选型必须引 benchmark 数据与排序 fallback 永保留

**Legality**

1. **benchmark harness 与 evidence 非空**（RFC-0025 §4.K 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M120 行）：以 nvpro
   `vk_order_independent_transparency` 七算法 sample 为对照基线建 harness
   （同场景、同 overdraw 分布），测量 4070 Ti 上各算法帧时/内存曲线；
   **evidence 非空**（帧时/内存曲线 measured 落 evidence，P-09）。
2. **仅测量不定档**（判据逐字引 G9_ACCEPTANCE_MAP §3 M120 行）：本门只产
   benchmark 数据，**不定默认档**——三档语义（①默认档 TAA 半透明合成路径
   ②有界近似档 WBOIT 起步/AVBOIT 目标〔评估项不承诺，D4 D16；体素布局与
   Froxel 族对齐〕③精确档 linked-list per-pixel fragment list，仅毛发
   strand 启用、场景级不开放） frozen，但**默认档选型由 benchmark 数据
   裁决，不由论文偏好裁决**（D4 D15）。
3. **无数据选型提交判 RED**（判据逐字引 G9_ACCEPTANCE_MAP §3 M120 行）：
   **默认档选型必须引 benchmark 数据——无 benchmark 数据的默认档选型提交
   判 RED**（RED 臂独立有效）。
4. **排序 fallback 永保留**（判据逐字引 G9_ACCEPTANCE_MAP §3 M120 行）：
   depth-sorted alpha 路径**永远保留**为最低端档与正确性对照（排序
   fallback 可达断言）；**linked-list 精确档与排序真值 diff=0**；精确档
   内存无界增长注入即 RED。
5. **M114 strand 档依赖面**（RFC-0025 §4.E/§4.K 逐字）：精确 linked-list
   档的 benchmark 裁决数据是 RXS-0372 strand 档分项的前置；本门落地前
   strand 档维持 not-triggered 登记（不充绿），不以 harness 绿色冒充精确
   档已落地。

**Implementation Requirements**

- 实现锚定（实现期命名）：七算法对照 harness（同场景同 overdraw 分布，
  帧时/内存曲线采集面）+ 排序 fallback 路径 + 精确档 linked-list 面
  （仅毛发作用域）；测量数据经 evidence 落盘（measured_local）。
- RED 锚定计划（实现 PR 落）：无 benchmark 数据的默认档选型提交 → RED；
  精确档内存无界增长注入 → RED；七算法正确性 golden（排序参考真值）。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/display_pipeline/reject/oit_default_tier_without_benchmark_data.rx`
  与正例 `conformance/display_pipeline/accept/oit_benchmark_harness_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_oit_benchmark_harness_smoke.py` 门（symbolic
  key `g9.p1.m120.oit_benchmark_harness`，G9.5 波 P1 登记字面不动）。

---

## 6. 条款（RXS-0372，G9.5 M114 毛发）

### RXS-0372 毛发 Marschner R/TT/TRT 三瓣与几何三档、strand 档强制精确 OIT 分项 not-triggered 登记

**Legality**

1. **Marschner 三瓣着色**（RFC-0025 §4.E 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M114 行）：Marschner R/TT/TRT 三瓣，纵向/方位角
   分离参数化为资产属性（每缕基调色、高光偏移、medulla 配置）；与参考
   实现**逐瓣对拍 golden**（瓣能量守恒）。
2. **单瓣置零 RED 臂**（判据逐字引 G9_ACCEPTANCE_MAP §3 M114 行）：**单瓣
   系数置零的 RED 渲染独立有效**——缺 TT 瓣必须可见差异，无差异即管线
   未接通（RED 臂独立有效）。
3. **几何三档与烘焙确定性**（判据逐字引 G9_ACCEPTANCE_MAP §3 M114 行）：
   近 strand / 中 card（各向异性法线/切线贴图）/ 远 mesh 三档；档间切换
   距离与 strand→card 股替换映射由离线烘焙产出（股聚类 + card 图集），
   烘焙确定性 golden（双构建逐位一致）；card/mesh 档走默认半透明路径。
4. **strand 档强制精确 OIT——分项 not-triggered 登记**（判据逐字引
   G9_ACCEPTANCE_MAP §3 M114 行）：strand 档排序不可行，**必须 linked-list
   精确档**（RXS-0371 第三档）；strand 档依赖 M120 精确档 benchmark 裁决
   数据，**数据可得性不足——strand 档分项登记 not-triggered 不充绿**
   （承接锚「M120 精确档 benchmark 裁决数据落地后重判，兜底 G9.7 穷举」；
   条件未触发只表示决策已记录，不是成功；不得以 card/mesh 档绿色冒充
   strand 档已触发）。
5. **触 32B 修订行**（RFC-0025 §4.L 逐字；判据逐字引 G9_ACCEPTANCE_MAP
   §3 M114 行）：Marschner 参数集经**资产化侧表扩展通道**（材质槽 ID 索引，
   M01/M85 资产通道）接入单层 closure 求值；`MaterialClosure` 32B 定长
   布局/字段含义/flags 位段分配 0-byte，预留拓扑字段位不消费，**禁静默
   扩**；确需 32B 内联扩面必须停手先修订 RFC-0025。

**Implementation Requirements**

- 实现锚定（实现期命名）：Marschner 三瓣着色 kernel + 几何三档切换与股
  替换烘焙器（离线）+ 材质参数侧表消费面（RXS-0373 同通道）。
- RED 锚定计划（实现 PR 落）：单瓣系数置零无差异 → RED；strand 档未走
  精确 OIT（且精确档未落地）→ 分项 not-triggered 字段核验；逐瓣对拍
  golden 与股替换烘焙双构建 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/display_pipeline/reject/hair_lobe_tt_zeroed_no_diff.rx`
  与正例 `conformance/display_pipeline/accept/hair_marschner_lobes_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_hair_marschner_smoke.py` 门（symbolic key
  `g9.p1.m114.hair_marschner`，G9.5 波 P1 登记字面不动）。

---

## 7. 条款（RXS-0373，G9.5 M115 皮肤）

### RXS-0373 皮肤 Burley 屏单 pass、扩散 profile 资产化与 pre-integrated LUT 回退档（触 MaterialClosure 32B 经 RFC-0025 §4.L 修订行）

**Legality**

1. **Burley 屏单 pass**（RFC-0025 §4.F 逐字；判据逐字引 G9_ACCEPTANCE_MAP
   §3 M115 行）：Burley normalized diffusion 屏空单 pass separable SSS
   （颜色/深度双 kernel）与参考实现 device 对拍 golden。
2. **扩散 profile 资产化**（判据逐字引 G9_ACCEPTANCE_MAP §3 M115 行）：
   扩散 profile（RGB 三通道 falloff 参数）为 per-material 资产，经 M01/M85
   资产通道烘焙/打包/manifest 入 DDC；扩散 profile 参数 → 扩散半径响应
   golden。
3. **pre-integrated LUT 回退档**（判据逐字引 G9_ACCEPTANCE_MAP §3 M115
   行）：pre-integrated LUT（曲率 × NdotL）在低端 profile 启用；两档画质
   差纳入 golden 对照。
4. **profile 全零衰减 RED 臂**（判据逐字引 G9_ACCEPTANCE_MAP §3 M115 行）：
   **profile 全零衰减注入必须退化为纯漫反射**——否则 profile 未生效，
   RED（RED 臂独立有效）。
5. **触 MaterialClosure 32B 经 RFC-0025 §4.L 显式修订行**（判据逐字引
   G9_ACCEPTANCE_MAP §3 M115 行）：Burley 扩散 profile 经**资产化侧表扩展
   通道**（材质槽 ID 索引）接入单层 closure 求值——32B 定长布局/字段含义/
   flags 位段分配 0-byte，预留拓扑字段位不消费（RFC-0019 §4.7 口径维持），
   **禁静默扩**；**缺省侧表 ≡ 无专项 lobe，既有材质输出逐位不变**；任何
   32B 内联扩面（新增字段/改字段含义/消费预留位）必须停手先修订
   RFC-0025，不得在实现侧私加字段绕过修订行。

**Implementation Requirements**

- 实现锚定（实现期命名）：Burley 屏空 separable SSS 双 kernel + 扩散
  profile 资产化面（材质参数侧表，按材质槽 ID 索引）+ pre-integrated LUT
  回退档；材质分类/解析面（RFC-0016 §4.C4）只消费不重定。
- RED 锚定计划（实现 PR 落）：profile 全零衰减未退化纯漫反射 → RED；
  静默扩 32B variant（私加内联字段/改字段含义/消费预留位）→ RED；主档
  与 LUT 回退档画质差 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/display_pipeline/reject/skin_profile_zero_falloff_no_diffuse.rx`
  与正例 `conformance/display_pipeline/accept/skin_diffusion_profile_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_skin_burley_diffusion_smoke.py` 门（symbolic
  key `g9.p1.m115.skin_burley_diffusion`，G9.5 波 P1 登记字面不动）。

---

## 8. 条款（RXS-0404，G13.3 M-b(M168) 自研 TSR device 化）

### RXS-0404 自研 TSR device 化 kernel：resample/resolve 双腿公式面与 host 金标准逐字同源、temporal 底座 0-byte、确定性协议继承与 device vs host 逐帧对拍容差标定产

**Legality**

1. **TSR device 化形态**（判据逐字引 G13_CONTRACT §4.2 M-b 行 /
   G13_ACCEPTANCE_MAP §1 M-b 行）：tsr.rs host 金标准
   （`temporal::tsr::TsrUpscaler`）→ **.rx kernel device 面**——双腿纯图像
   空间 compute kernel `src/rurix-render/kernels/g13_tsr_resample.rx`
   （jitter 对齐 Catmull-Rom 重采样 × exposure 转显示域 + 抗振铃 4×4
   采集邻域 min/max 钳制 + 深度最近邻上采样 + YCoCg Y 亮度导出）与
   `src/rurix-render/kernels/g13_tsr_resolve.rx`（闪烁时域分析 EMA + MV
   最近邻上采样 + 历史双线性重投影 + 深度相对差历史验证 + 当前帧 YCoCg
   3×3 邻域 AABB 闪烁松弛钳制 + reactive 优先 alpha 调制混合 +
   YCoCg→RGB 显示域输出）；**公式面与 host 金标准逐字同源**（RXS-0357
   host oracle 纪律继承：仅 host 输出不能充绿，门绿由 device 腿承载）。
   编译链 = `rurixc --target vulkan` 产 SPV + spirv-val 通过（G12 PT
   megakernel 车道复用，`vk::run_compute` 同一 compute 派发面）。
2. **temporal 底座 0-byte 不接线**（G13 立项裁决 6 字面）：device 化只
   实现 host 金标准公式面的 device 同源兑现——UpscaleBackend trait 签名
   面与 temporal 底座历史接口面 0-byte（`src/rurix-render/src/temporal/`
   相对 G13.0 不可变 ref `8c5dc5ee` 目录级 git diff + 工作树双面机核）；
   device 后端经 UpscaleBackend 冻结接口面（RFC-0016 §4.0-3 三实现位
   预留位之自研 TSR 位）bin-local adapter 接入，不改底座任何语义面/代码
   面；**底座接线即 RED**。
3. **device vs host 金标准同输入逐帧对拍**（判据逐字引 G13_CONTRACT
   §4.2 M-b 行）：50%/67%/100% 三档内部分辨率（640×360 / 858×482 /
   1280×720 → 统一输出 1280×720）× 32 帧 Halton jitter 静态收敛序列，
   逐帧逐像素最大绝对差 p100 ≤ **标定容差**（标定程序产入
   `g13_budget.json`，threshold = measured × 2.0 冻结 k，禁手写 P-09）；
   **对拍超容差静默即 RED**。
4. **三档质量/帧时 measured 对照**（判据逐字引 G13_CONTRACT §4.2 M-b
   行）：质量 = 终帧 SSIM deficit（1−SSIM，RXS-0387 LDR 8×8 窗口径）
   对拍 4×4 超采样参照；帧时 = host Instant 墙钟 around 逐帧 device
   全链路（打包 + 双 dispatch + 回读同步 + 状态轮换），50×3 trimmed
   mean 协议沿 M141/M165 冻结统计口径（warmup 10 + timed 150 = 3 块
   × 50，逐块 IQR 去离群 → 块中位数 → 3 块均值）；全入 g13_budget
   measured_local **零 estimated**——**回归守护语义，不构成超分画质/
   帧率对标通过线**（G13 不设画质通过线归 G15；正式帧率对标锚定 G14）；
   **estimated 冒充 measured 即 RED**。
5. **固定 seed 位级确定性协议维持**（RXS-0357 L2 继承）：逐像素独立顺序
   求值，禁 atomic 顺序敏感累加；输出直写无跨像素交互；同档同参双跑
   digest 位级一致；**确定性协议漂移即 RED**。

**Implementation Requirements**

- 实现锚定（实现期命名，`src/rurix-render` 维持 `forbid(unsafe_code)`
  纪律）：device 后端 = 门 harness
  `src/rurix-render/src/bin/g13_tsr_device.rs` 内 bin-local
  `TsrDeviceBackend` 实现 UpscaleBackend 冻结面，逐帧经
  `rurix_rt::vk::run_compute` 双 dispatch 驱动双腿，历史状态（颜色/深度/
  亮度/翻转符号/闪烁分数，输出分辨率）host 侧双缓冲轮换（G12 PT
  megakernel 车道 host 簿记同模）。
- RED 锚定计划（实现 PR 落）：kernel 输出面加性偏置（kernel-bias 臂）
  → device vs host 对拍必超容差检出；jitter 序列相位偏移（seed-change
  臂）→ 终帧 digest 必异检出；estimated 注入 budget/evidence 面 → 必拒
  （CI 脚本 selftest 合成红臂承载）。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/display_pipeline/accept/tsr_device_kernel_minimal.rx` 与
  `conformance/display_pipeline/reject/tsr_device_temporal_base_rewire.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g13_tsr_device_kernel_smoke.py` 门（symbolic key
  `g13.p0.m_b.tsr_device_kernel`，G13.1 冻结字面不动）。

---

## 9. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.1 | 2026-08-18 | 追加（G13.3 M-b(M168) 自研 TSR device 化波 spec-first，硬规则 7 条款先行；G13 已解锁 implementation_status=unlocked，G13_CONTRACT §8.2 G-G13-3 互锁 READY）登记 **RXS-0404**（自研 TSR device 化 kernel：resample/resolve 双腿公式面与 host 金标准 `temporal::tsr::TsrUpscaler` 逐字同源〔RXS-0357 host oracle 纪律继承——仅 host 输出不能充绿，门绿由 device 腿承载〕+ `rurixc --target vulkan` 产 SPV + spirv-val 通过〔G12 PT megakernel 车道 `vk::run_compute` 复用〕+ temporal 底座 0-byte 不接线〔UpscaleBackend trait 签名面与历史接口面目录级 diff 机核 vs G13.0 不可变 ref 8c5dc5ee；底座接线即 RED〕+ device vs host 同输入逐帧对拍 p100 ≤ 标定容差〔threshold = measured × 2.0 冻结 k，标定程序产禁手写 P-09；超容差静默即 RED〕+ 50/67/100% 三档质量〔SSIM deficit，RXS-0387 口径〕/帧时〔50×3 trimmed mean，M141/M165 冻结统计口径〕measured 对照入 g13_budget 零 estimated〔回归守护语义不构成画质/帧率通过线；estimated 冒充 measured 即 RED〕+ 固定 seed 位级确定性协议维持〔漂移即 RED〕）。**落点裁决**：候选 global_illumination.md（RXS-0402 TSR 底座联动轴）与 display_pipeline.md（M24 TSR/TAA 生产契约引用轴，§1 头注）——裁定落本卷：TSR device 化 = 时域重建/超分显示链环节（M24 契约同轴），非 GI 语义面；候选 global_illumination.md 本体 0-byte。判档 = **加性 spec 条款**（G13_CONTRACT §7 裁决 4 Full RFC 触发面——UpscaleBackend trait 签名面/temporal 底座历史接口面/RXS-0357 参照器面/M137 scalars.flip 演进位——逐条未命中：本条款零冻结面消费，语义事实源 = G13_CONTRACT §4.2 M-b 行判据逐字 + RFC-0016 §4.0-3 冻结接口面，条款只登记不加语义）。条款号自 ledger 实测 `RXS.next_free=404` 顺位领取（0404 单号不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。零新 RX 码；零新 U/RD/SG；零 RFC 消费（RFC 命名空间 0-byte，实测 next_free=30 维持）；conformance 最小锚定语料两件（accept tsr_device_kernel_minimal.rx + reject tsr_device_temporal_base_rewire.rx；inert + `//@ spec` 锚定 + 预期 RED 注释 + 转正路径旁注，G9.2~G12.3 spec 波先例）同 PR 落；symbolic key `g13.p0.m_b.tsr_device_kernel`（G13.1 冻结字面，G13_ACCEPTANCE_MAP §1）0-byte 不动；trace_matrix 重生成（385→386 全锚定）；stable 快照因条款计数 385→386 同 PR 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。既有 spec 条款字面 0-byte（只追加新条款/修订记录行；§8 修订记录节号顺延 §9，节体 0-byte），不触红线/禁区。`Assisted-by: Kimi-K3（G13.3 TSR device 化波）` | **加性条款**（G13 治理波冻结判据 spec 面登记；零 RFC 触发面） |
| v1.0 | 2026-08-12 | 新建（G9.5 spec-first，大世界×专项波帧图输出与材质着色专项轴，硬规则 7 条款先行）：RXS-0369（M118 SDR/scRGB/PQ 三交换链路径运行时切换 + ACES 1.3/2.0/AgX/中性四内置插件逐一 golden〔含已知差异记录〕+ 非 HDR 交换链携带 PQ 输出即 RED + HDR 设备标定未触发 SKIP=not-triggered 不充绿且不反向否决 SDR 验证面）/ RXS-0370（M119 后处理骨架显式排序〔曝光/bloom/tonemap/LUT/输出变换〕+ 全程 HDR 线性域〔隐式 SDR clamp 注入 RED〕+ 曝光状态帧间持久 + 与 TAA/TSR 显式排序）/ RXS-0371（M120 OIT benchmark harness 七算法对照 evidence 非空 + 仅测量不定档 + 无 benchmark 数据选型提交判 RED + 排序 fallback 永保留）/ RXS-0372（M114 毛发 Marschner R/TT/TRT 三瓣逐瓣 golden + 几何三档 + strand 档强制精确 OIT 依赖 M120 精确档数据不足分项 not-triggered 不充绿〔承接锚 M120 精确档 + G9.7 穷举〕）/ RXS-0373（M115 皮肤 Burley 屏单 pass + 扩散 profile 资产化 + pre-integrated LUT 回退档 + profile 全零衰减未退化纯漫反射 RED + 触 MaterialClosure 32B 经 RFC-0025 §4.L 修订行〔资产化侧表扩展通道，32B 布局 0-byte，禁静默扩〕）。**目标 spec 新建裁决**：RFC-0025 §5 映射表裁定 D4 两轴新建两卷——world_partition.md 承载场景数据模型轴（RXS-0363~0368），本卷承载帧图输出与着色专项轴（M118/M119/M120/M114/M115，RXS-0369~0373）；既有卷本体 0-byte（头注留痕，沿 G9.2/G9.4 新建先例）。条款号自 ledger 实测 `RXS.next_free=363` 顺位领取（0369~0373 与 world_partition.md 0363~0368 连续不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。conformance 最小锚定语料同 PR 落（conformance/display_pipeline/{accept,reject}/，inert + `//@ spec` 锚定 + 预期诊断注释 + 转正路径旁注，G9.2~G9.4 spec 波先例）；symbolic key `g9.p0.m118.display_pipeline_view_transform`（G9.1 冻结字面）与 `g9.p1.m119/m120/m114/m115.*`（G9.5 波 P1 全进裁决登记，G9_ACCEPTANCE_MAP §3 / CI_GATES §4A）0-byte 不动。零新 RX 码（诊断码实现期按实际可达类别领取不预造）、零新 U/RD/SG、零 src/ 改动、零 workflow 步骤。依据 [RFC-0025](../rfcs/0025-world-and-specialty-renderers.md)（Agent Approved 2026-08-12）§4.E/§4.F/§4.I~§4.L + G9_ACCEPTANCE_MAP §2 M118 行 + §3 M119/M120/M114/M115 行（判据逐字）+ G9_CANDIDATE_DECISIONS §5 M45~M47 行、§6 M49 行与 v1.4 校准注 | **Full RFC**（RFC-0025） |
