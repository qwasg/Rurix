# G9_D4 — 大世界分区、专项渲染器族与显示管线 设计草案

> **DRAFT 设计提案——G9 未立项，不构成契约/验收承诺**
> 本文是 G9 模块 D4 的设计草案（agent 起草，2026-08）。G9 尚未立项，本文不产生任何契约、验收承诺、编号 claim 或实现义务；所有「验收门」均为建议形态，须经 G9 立项治理（契约四件套 + RFC + 候选决策表 + 验收映射）后方可转为硬门。
> **承接事实源**（只读引用，本文零改写）：[G8_PLAN](../g8/G8_PLAN.md)（G8 已于 2026-08-06 收口）· [G8_P2_DECISIONS](../g8/G8_P2_DECISIONS.md)（M43/M48/M49 行）· [G8_CANDIDATE_DECISIONS](../g8/G8_CANDIDATE_DECISIONS.md) §10（M45/M46/M47 行）· `src/rurix-render` / `apps/uc06-renderer` 现状结构。
> **G9.0 冻结引用**：2026-08-08 起，本文作为 G9.0 文档集不可变基线附件被 [G9_PLAN.md](../G9_PLAN.md) 冻结引用；正文 0-byte，后续变更只追加修订记录（追加于文末）。

---

## 1. 定位与承接锚

**D4 = G9 中「大世界 + 画质专项 + 显示输出」模块**：把 UE5 级渲染器从「单场景正确渲染」推进到「可持续流送的大世界 + 五族产品级专项渲染器 + 产品级显示管线」。D4 的定位是**数据结构先行、渲染器消费事件**：World Partition 是数据模型与流送契约，所有专项渲染器（含地形）只是 cell 加载/卸载事件的消费者。

G8 收口的法定承接锚（逐字摘自决策表，不得改写）：

| G8 锚 | 字面 | 承接 |
|---|---|---|
| M43 World Partition / HLOD | 「大世界资产面出现时」→ defer-to-G9+，承接锚「G9+ 大世界分区」 | D4.1 / D4.2 |
| M48 体积雾/云 | 「画质专项建造期」→ defer-to-G9+，承接锚「G9+ 大气特效」 | D4.3 |
| M49 水体/毛发/皮肤/地形/贴花族 | 「专项渲染器族建造期」→ defer-to-G9+，承接锚「G9+ 专项渲染器」 | D4.4~D4.8 |
| M45 HDR 管线 | no-go，「HDR 显示设备资产/产品需求出现时」，open-留 G8.7/G9+ | D4.9（条件触发，见 §6/§8） |
| M46 后处理栈 | no-go，「bloom/DOF/曝光分级产品需求随 G9+ 建造期出现」，open-留 G9+ | D4.10 |
| M47 透明/OIT | no-go，「OIT 策略选型需 measured 对照」，open-留 G8.7 | D4.11（benchmark 先行） |

**共同设计模式**（调研结论 6，全模块统一）：离线烘焙 + 运行时预算化流送 + compute-first GPU 管线 + 分级回退档。D4 内每个专项渲染器设计文档第一节必须是「**哪些离线算死、哪些是运行时预算契约**」。

---

## 2. 范围 in / out

### In（D4 模块面）

| 子面 | 内容 | 分期 |
|---|---|---|
| D4.1 World Partition 数据模型与流送 | 单一持久世界 schema、2D 网格 cell、streaming source 距离环、显式流送预算契约、always-loaded vs spatially-loaded 区分、cell 元数据 Data Layer 掩码位预留（v2） | 第一波 |
| D4.2 HLOD 烘焙管线 | 离线 Builder 按 Component 分发、代理网格烘焙、产物即资产、禁止运行时合并 | 第一波（烘焙）/ 后续（过渡质量） |
| D4.3 大气体渲染器 | 统一 Froxel 体积基础设施；雾（高度雾前端）+ 云（Schneider 前端）两前端；时序上采样默认路径；weather map 资产化 | 第一波 fog 骨架 / 第二波云 |
| D4.4 水体 | 开阔大洋 Tessendorf IFFT 谱（位移/梯度/Jacobian 泡沫三贴图 + CDLOD）与局部浅水波方程两条管线分离；tiling-and-blending 防重复感 | 第三波大洋 / 第四波浅水 |
| D4.5 毛发 | Marschner R/TT/TRT 三瓣着色；几何三层退化（近 strand / 中 card / 远 mesh）；strand 档精确 OIT | 第四波（依赖 D4.11 精确档） |
| D4.6 皮肤 | Burley normalized diffusion 屏空单 pass；扩散 profile 资产化；低端 pre-integrated LUT 回退 | 第三波 |
| D4.7 地形 | GPU-driven heightfield（geometry clipmaps 思想）；LOD/剔除/缝合全进 compute；chunk ≡ D4.1 cell；toroidal 环状更新 | 第二波（与 D4.1 对齐首发） |
| D4.8 贴花 | DBuffer 三通道（法线 + 材质属性）从帧图设计期占位；screen-space cluster 化防过绘制；前向回退档 v1 即定义 | 第二波 |
| D4.9 HDR 显示管线 | ACES RRT/ODT → scRGB 或 PQ/Rec.2020 双交换链路径；运行时切换一等资源；可插拔 view transform（ACES 1.3/2.0、AgX、中性矩阵并列） | 第一波（管线与插件面）/ 第三波（HDR 设备标定，条件触发） |
| D4.10 后处理栈 | histogram 自适应曝光 + 手动 EV → bloom（tonemap 前 HDR 域多尺度 mip）→ DOF（scatter-as-gather）→ tonemap → 色彩分级 LUT → 输出变换；全程 HDR 线性域 | 第一波骨架 / 第二波 DOF/分级完整化 |
| D4.11 OIT 三档 | 默认 TAA 半透明 → 有界近似（WBOIT 起步 / AVBOIT 目标）→ linked-list 精确档仅服务毛发；排序 fallback 永保留；benchmark 先行定默认档 | 第一波 benchmark / 后续按测量定档 |

### Out（明确不在 D4）

- GI 建造期主体（M12 Surface Cache / M16 档位）——归 G9 独立 GI 模块。
- GPU 粒子 VFX 渲染侧（M49a）、Niagara 类特效系统——RD-044/特效管线判档后独立模块。
- 水体浮力 gameplay 联动（M77，G8 no-go）——D4.4 只留物理接口面，不实现 ApplyBuoyancyImpulse。
- SVT/RVT/sampler feedback（M40/M41/M42，G8 close-out no-go）——地形与贴花设计**不得依赖虚拟纹理**。
- task shader（M62 不开放）、Work Graphs（M56）、mesh 第三光栅（M61）——维持 G8 裁决。
- 编辑器 GUI、世界编辑工具链 UI、网络流送、多 GPU、USD ingest（M86）、MaterialX（M87）。
- FG/MFG（M26）、present pacing（M49b）。

---

## 3. 依赖前置（G8 已兑现资产）

D4 只消费 G8 已冻结的 ABI 与证据，**不得重定格式、不得绕过冻结面**：

| 依赖 | G8 事实 | D4 消费方式 |
|---|---|---|
| M01 builder 版本化 | G8.3 已冻结，golden 硬门 | D4.2 HLOD 产物、weather map、扩散 profile、谱贴图烘焙全部走同一 builder/资产图式 |
| M04 磁盘/内存页格式 + 解码 ABI | G8.3 冻结（G8_PLAN：G8.4 只消费不重定） | cell 资产打包页格式；地形 chunk、HLOD 代理、噪声贴图同 ABI |
| M85 shader/PSO manifest ↔ DDC | G8.2 已绿（`evidence/g8_m85_shader_manifest_ddc_*.json`） | 大气体/水体/毛发/皮肤/贴花全部 compute/raster kernel 经 manifest 打包进 DDC，无例外通道 |
| M37 磁盘异步 I/O + 解压 + 上传分离 timeline | G8.4 门 | cell 流送 I/O 腿直接复用；迟到页降级路径即大世界 hitch 防线的第一级 |
| M38 GDeflate + CPU fallback | G8.4 门 | cell 页解压；GPU 解压预算计入流送预算契约 |
| M44 几何页 streamer | G8.4 门-GeomPage | cell 内几何按需驻留；HLOD 代理与全量几何同页族 |
| RD-037/M89 单源 gfx | G8.2 已绿 | 新增渲染器前端以 `.rx` 为主语言声明；禁止 Rust 宿主出图代码抢跑 |
| M50 RT 增量面 | G8.2 已绿 | D4 默认走 compute ray-march（体积）/ raster；RT 仅作为可选画质档，不作硬依赖 |
| M24 TSR/TAA 生产契约 | G8.5b 已绿 | 后处理栈时域集成面：bloom/DOF 输出必须与 TAA/TSR resolve 顺序在帧图中显式声明；曝光状态供 TSR 消费 |
| G5/G6 冻结面 | GpuScene / MaterialClosure 32B / Barrier EB 三轴 / PageRequest | Marschner/Burley lobe 需要 closure 扩展时**必须走 RFC 修订**（见 §9），不得静默扩 32B |
| measured baseline | `g8_budget.json`（RTX 4070 Ti） | G9 立项必须重测 G9 budget baseline；D4 所有预算阈值（hitch p99、ray-march 步数、OIT 内存界）禁止手写，须实测标定 |

---

## 4. 模块分解

### D4.1 World Partition 数据模型与流送

调研依据：UE World Partition 范式（调研结论 1）——单一持久世界 + 运行时 2D 网格 cell 流送 + 显式预算契约 + Data Layers 正交维度 + always-loaded vs spatially-loaded schema 层区分。**分区 = 数据结构先行，渲染器只是 cell 加载事件消费者。**

- **世界资产 schema（离线）**：单一 persistent world 资产；schema 层显式区分 `always_loaded`（全局/ gameplay 关键对象）与 `spatially_loaded`（空间分格对象）。每个 spatially-loaded 对象携带 cell 归属；cell 为正方形 2D 网格（边长为资产属性，非代码常量）。
- **cell 元数据**：v1 字段 = cell id、包围盒、资产页引用（M04 ABI）、HLOD 层级引用；**预留 Data Layer 掩码位（v2）**——Data Layers 是正交于空间分格的激活维度，v1 只预留位不实现激活语义，避免 schema 二次迁移。
- **流送运行时**：streaming source（相机/玩家/自定义探针）携带距离环（loading radius / 内环常驻）；每帧由距离环求 target cell 集合，与 resident 集合 diff 出 load/unload 队列。
- **显式预算契约（防 hitch，可测量可审计）**：`MaxStreamingCellsPerFrame`、`MaxActorsToSpawnPerFrame`、`MemoryBudgetMB` 三项为一等契约字段；超预算请求排队而非抢占；**预算计数器逐帧落 evidence**（hitch 审计的数据源）。预算违约注入测试必须触发可见降级而非静默超帧。
- **事件接口**：`CellLoadBegin / CellResident / CellUnloadBegin / CellEvicted` 四事件为渲染器唯一消费面；地形 chunk、HLOD、贴花 cluster 重算、流送光源集均挂事件，不反向查询分区状态。

### D4.2 HLOD 烘焙管线

调研依据：HLOD 离线烘焙代理，Builder 按 Component 分发，**产物即资产，禁止运行时合并**（调研结论 1）。

- 离线 Builder 输入 cell/Component 划分，逐 Component 生成代理几何（简化 + 合批 + 材质合并）；产物经 M01/M04 管线落成普通资产，走同一 cook/DDC/页格式通道。
- 运行时零合并：HLOD 代理与 cell 全量内容按 screen-size 阈值互斥切换（cell 级 HLOD 树，层数为烘焙属性）；过渡 popping 控制（dither/fade）列为后续子档。
- 烘焙确定性：双构建 hash 相等（沿用 M79 判据形态）；代理相对原始的屏幕空间误差上界作为 golden 断言。

### D4.3 大气体渲染器（云 + 雾，统一 Froxel）

调研依据：Schneider 范式（Perlin-Worley 低频塑形 + Worley 高频侵蚀 + 2D weather map + 高度梯度 + ray-march，SIGGRAPH 2015/2017 Nubis）至今是行业基线；Meteoros（UPenn）证明纯 Vulkan compute <3ms 可复现；时序上采样（低分辨率 ray-march + temporal reprojection，GDC 2022 Horizon Forbidden West）为默认路径；**云与雾共用 Froxel 基础设施——一个大气体渲染器两个前端**；weather map 作为资产而非硬编码参数（调研结论 2）。

- **Froxel 基础设施（一次性建造）**：视锥体素网格（froxel volume）+ 密度/光照累积 + 深度切片分布 + 与帧图的合成节点。AVBOIT 体素结构（D4.11）评估时复用同一体素内存布局族。
- **雾前端（第一波）**：高度雾/分层介质直接写 Froxel 密度场；解析项为主，预算极小。
- **云前端（第二波）**：噪声 baker 离线产出 Perlin-Worley 3D 纹理与 Worley 高频纹理（M01 资产）；2D weather map（覆盖度/湿度/类型）为资产；ray-march compute kernel 低分辨率执行 + temporal reprojection 上采样为默认路径，全分辨率列为高端档。
- **预算契约**：ray-march 最大步数、froxel 分辨率档、上采样开关均为预算字段；每档有 measured 帧时证据。

### D4.4 水体

调研依据：Tessendorf IFFT 谱大洋（位移/梯度/Jacobian 泡沫三贴图 + CDLOD）与局部浅水波方程**两条管线分离**；tiling-and-blending 防重复感（Ubisoft La Forge 2024）（调研结论 3）。

- **大洋管线**：IFFT 谱离线参数化（风向/风速/涌浪）+ 运行时 compute IFFT（或周期谱表寻址档）；位移/梯度/Jacobian 三贴图，Jacobian 负值驱动泡沫；CDLOD 距离分档 mesh；多尺度谱 tiling-and-blending 防周期重复感。
- **浅水管线**：局部波方程（高度场 + 速度场 ping-pong compute）服务池塘/河流/交互波纹；与大洋管线不共享几何路径，仅共享水面着色 closure 输入面。
- 物理接口面：浮力查询接口预留（M77 联动判档后启用），D4 不实现浮力。

### D4.5 毛发

调研依据：Marschner R/TT/TRT 三瓣（2003 至今唯一物理基线）+ 几何三层退化（近 strand / 中 card / 远 mesh，EGSR 2025 实时毛发 LOD 股替换 13× 加速背书）；**strand 必须精确 OIT**（调研结论 3）。

- 着色：Marschner R/TT/TRT 三瓣，纵向/方位角分离参数化为资产属性（每缕基调色、高光偏移、medulla 配置）。
- 几何三档：近 strand（tessellated/生成式发丝）、中 card（card + 各向异性法线/切线贴图）、远 mesh（头皮壳 mesh）；档间切换距离与 strand→card 股替换映射由离线烘焙产出（股聚类 + card 图集）。
- OIT 硬依赖：strand 档排序不可行，必须 linked-list 精确档（D4.11 第三档）；card/mesh 档走默认半透明路径。毛发因此排在 OIT 精确档落地之后（§6）。

### D4.6 皮肤

调研依据：Burley normalized diffusion 屏空单 pass（Activision SIGGRAPH 2018，UE/Unity 已收敛）；扩散 profile 做成资产；低端 pre-integrated LUT 回退（调研结论 3）。

- 屏空单 pass separable SSS：颜色/深度双 kernel，扩散 profile（RGB 三通道 falloff 参数）为资产（per-material）。
- 回退档：pre-integrated LUT（曲率 × NdotL）在低端 profile 启用；两档画质差纳入 golden 对照。
- 与 M28 边界：皮肤/毛发 lobe 若无法由单层 MaterialClosure 32B 表达，走 §9 RFC 修订，不得静默扩面（M28 G8 裁决 no-go，G9 可重新判档但不默认连带）。

### D4.7 地形

调研依据：GPU-driven heightfield（geometry clipmaps 思想 + LOD/剔除/缝合全进 compute，Far Cry 5 GDC 2018）；**cell = 地形 chunk 与 M43 对齐**；环状 toroidal 更新适合 Vulkan ring buffer（调研结论 3）。

- heightfield 数据为 M04 页格式资产，chunk ≡ D4.1 cell（尺寸对齐同一网格族，禁止第二套分格）。
- LOD 选择 / 视锥剔除 / 邻级缝合（stitch skirt 或指数网格 morph）全部进 compute，产出 indirect draw；CPU 侧零逐 chunk 提交。
- toroidal 更新：相机移动时环形窗口滚动复用 ring buffer，避免全量重传；与 M37 I/O 链直接对接（chunk 页迟到 → 父级 LOD 占位，同 M44 迟到页语义）。

### D4.8 贴花

调研依据：DBuffer 三通道（法线 + 材质属性，UE5 默认）**从帧图设计期占位**；screen-space cluster 化防过绘制；前向回退档 v1 就定义（调研结论 3）。

- DBuffer（法线 + 材质属性 + 可选第三通道）在 G-buffer pass 内合成，**帧图设计期即占位**——即使 v1 贴花数量为零，通道与 barrier 布局先行冻结，避免后期插 pass 改全局帧图。
- cluster 化：screen-space cluster（复用光照 cluster 结构）对贴花体求交，限制逐像素贴花评估数上界；过绘制计数器落 evidence。
- 前向回退档：无 DBuffer 的低端 profile 走 decal-forward pass，v1 即定义两档语义等价性判据。

### D4.9 HDR 显示管线与可插拔 view transform

调研依据：HDR 输出 = ACES RRT/ODT → scRGB 或 PQ/Rec.2020 **双交换链路径，运行时可切换一等资源**；view transform 必须插件化（ACES 1.3/2.0、AgX、中性矩阵并列——ACES filmic hue-skew 是公认缺陷，AgX 已进 Blender 4.0 默认/Godot/three.js，**锁死单一 tonemapper 是 2026 架构错误**；注意 AgX 对比度补偿陷阱）（调研结论 4）。

- 交换链族：SDR（Rec.709）/ scRGB / PQ-Rec.2020 三条 swapchain 路径为一等资源，运行时切换（重建交换链或双链过渡）；HDR 元数据（MaxCLL/MaxFALL/mastering primaries）由输出变换阶段填写。
- **view transform 插件面**：`ViewTransform` trait（输入 HDR 线性 + 显示参数 → 输出编码），ACES 1.3、ACES 2.0、AgX、中性矩阵四个内置实现并列，第三方可注册；golden 对拍按插件逐一建（§7）。
- AgX 陷阱登记：AgX 默认低对比 look 需要对比度补偿；补偿参数必须随 view transform 资产化，禁止硬编码进 tonemap 节点。
- 条件触发边界：M45 的 G8 字面触发条件是「HDR 显示设备资产/产品需求出现」。D4.9 拆为**管线/插件面（第一波可建，SDR 上即可验证 view transform 正确性）**与 **HDR 设备标定/认证（第三波，条件未触发则诚实登记 open-留痕，不假绿）**。

### D4.10 后处理栈

调研依据：UE 范式——histogram 自适应曝光 + 手动 EV → bloom（tonemap 前 HDR 域多尺度 mip）→ DOF（scatter-as-gather）→ tonemap → 色彩分级 LUT → 输出变换；全程 HDR 线性域；**曝光状态 = 帧间持久资源**（调研结论 4）。

- 顺序冻结（帧图语义）：exposure（histogram + EV 偏移）→ bloom（HDR 域多尺度 mip 链，down/up 双 pass）→ DOF（scatter-as-gather，散景 kernel 资产化）→ tonemap（经 D4.9 插件）→ 色彩分级（LUT 资产，tonemap 后）→ 输出变换（RRT/ODT 或中性）→ UI 合成（SDR 域）。
- 曝光状态帧间持久：histogram → 目标 EV 的 adapt（上/下不同速率）状态为 persistent resource，与 TAA/TSR（M24）的时域链显式排序。
- 全程 HDR 线性域；任何节点不得隐式 clamp 到 SDR（negative 测试点）。

### D4.11 OIT 三档

调研依据：三档策略（默认 TAA 半透明 → 有界近似 WBOIT 起步 / AVBOIT 目标 → linked-list 精确档仅服务毛发）；AVBOIT（Activision SIGGRAPH 2025，已随 CoD 出货，自适应体素收集、内存有界）为首选跟进项，体素结构可与 Froxel 复用；**nvpro vk_order_independent_transparency 七算法 sample 为性能基线，benchmark 先行再定默认档**；排序 fallback 永远保留（调研结论 5）。

- 三档语义：①默认档——半透明走 TAA 合成路径（现状延伸）；②有界近似档——WBOIT 起步实现，AVBOIT 为目标（内存有界、自适应体素收集，体素布局与 D4.3 Froxel 族对齐）；③精确档——linked-list per-pixel fragment list，仅毛发 strand 启用，场景级不开放。
- **benchmark 门先行**：以 nvpro 七算法 sample 为对照基线建 harness（同场景、同 overdraw 分布），测量 4070 Ti 上各算法帧时/内存曲线；**默认档选型由 benchmark 数据裁决，不由论文偏好裁决**。
- 排序 fallback：depth-sorted alpha 路径永远保留为最低端档与正确性对照。

---

## 5. 关键设计决策表

| # | 决策 | 依据（调研结论引用） | 否决的备选 | 可逆性 |
|---|---|---|---|---|
| D1 | 分区数据模型先行，渲染器只消费 cell 事件 | 结论 1：「分区=数据结构先行」 | 渲染器内嵌空间管理 | 高（事件接口稳定） |
| D2 | 流送预算为一等契约字段且逐帧落 evidence | 结论 1：「显式契约…须可测量可审计」 | 隐式节流/事后调参 | 中（schema 字段） |
| D3 | HLOD 纯离线烘焙、运行时零合并 | 结论 1：「产物即资产，禁止运行时合并」 | 运行时简化合批 | 高 |
| D4 | Data Layer 仅预留掩码位，v2 才实现激活语义 | 结论 1：「正交维度（v2 预留）」 | v1 全量 Data Layers | 中（schema 预留位） |
| D5 | 云/雾共用一个 Froxel 基础设施，两个前端 | 结论 2：「一个大气体渲染器两个前端」 | 云/雾各自独立体渲染器 | 中 |
| D6 | 云默认时序上采样路径，全分辨率仅高端档 | 结论 2：GDC 2022 默认路径；Meteoros <3ms 预算证据 | 全分辨率 ray-march 默认 | 高（档位切换） |
| D7 | weather map / 噪声 / 扩散 profile / 曝光曲线全部资产化走 M01 | 结论 2/3/4：「资产而非硬编码参数」 | 常量硬编码进 kernel | 高 |
| D8 | 水体大洋/浅水两条管线分离，仅共享着色输入面 | 结论 3 | 单一水体管线两模式 | 中 |
| D9 | 毛发三档几何退化，strand 档强制精确 OIT | 结论 3：EGSR 2025 13× 背书；「strand 必须精确 OIT」 | card-only 近似 | 低（跨模块依赖） |
| D10 | 皮肤 Burley 屏空单 pass 为主、LUT 为回退档 | 结论 3：行业已收敛 | pre-integrated LUT 唯一档 | 高 |
| D11 | 地形 chunk ≡ Partition cell，禁第二套分格 | 结论 3：「cell=地形 chunk 与 M43 对齐」 | 地形独立 clipmap 分格 | 低（schema 级） |
| D12 | 贴花 DBuffer 通道帧图设计期占位，即使 v1 用量为零 | 结论 3：「从帧图设计期占位」 | 贴花出现时再插 pass | 中（帧图冻结面） |
| D13 | view transform 插件化，ACES 1.3/2.0/AgX/中性并列 | 结论 4：「锁死单一 tonemapper 是 2026 架构错误」 | ACES 单一硬编码 | 高 |
| D14 | 后处理全程 HDR 线性域，tonemap 前 bloom | 结论 4：UE 范式顺序 | LDR bloom 捷径 | 高 |
| D15 | OIT 默认档由 nvpro 七算法 benchmark 数据裁决 | 结论 5：「benchmark 先行再定默认档」 | 按论文直接选 AVBOIT 默认 | 高 |
| D16 | AVBOIT 体素布局与 Froxel 族对齐（评估项，不承诺） | 结论 5：「体素结构可与 Froxel 复用」 | 独立体素内存体系 | 高 |
| D17 | 地形/贴花显式不依赖 SVT/RVT（M40/42 G8 no-go 维持） | G8 close-out 裁决字面 | 借地形需求复活 SVT | 高 |

---

## 6. 波次建议（族内分期）

原则：数据结构 → 可独立验证的显示面 → 资产依赖型专项 → 强耦合专项；每波有自己的退出证据，禁止用后续波证据充当前波门。

| 波次 | 内容 | 选此波的理由 |
|---|---|---|
| **W1 骨架波** | D4.1 分区数据模型 + 流送预算契约；D4.2 HLOD 烘焙管线首版；D4.11 OIT benchmark harness（仅测量，不定档）；D4.10 后处理骨架（exposure + bloom + 插件化 tonemap 空壳）；D4.9 view transform 插件面 + SDR 路径 | 分区是地形/HLOD/流送的共同前置；OIT benchmark 是选型门输入而非实现；后处理骨架与 view transform 在 SDR 上即可全量验证，不依赖 HDR 设备 |
| **W2 大气与地表波** | D4.3 Froxel + 雾前端；D4.7 地形（与 cell 对齐首发）；D4.8 贴花 DBuffer + cluster；D4.10 DOF/色彩分级完整化 | 地形消费 W1 分区事件；Froxel 是云与 AVBOIT 共用底座；贴花帧图占位越早冻结越便宜 |
| **W3 画质专项波** | D4.3 云前端（时序上采样）；D4.4 大洋水体；D4.6 皮肤；D4.9 HDR 设备标定（**条件触发**，否则登记 open-留痕） | 均为资产/设备依赖型；云依赖 W2 Froxel |
| **W4 精专波** | D4.11 有界近似档实现（WBOIT→AVBOIT 评估）→ 精确 linked-list 档；D4.5 毛发（strand/card/mesh）；D4.4 浅水水体 | 毛发强依赖 OIT 精确档；AVBOIT 体素复用依赖 W2 Froxel 布局稳定 |

---

## 7. 验收门草案（建议形态，G9 立项后转硬门）

通用四层（沿用 RuriX 防假绿口径）：断言 + device 真跑对拍 + golden + 负例 RED 臂；evidence schema 按子系统命名。RTX 4070 Ti 实测 baseline（G9 立项重测）为全部阈值的唯一来源，禁止手写。

| 子系统 | 断言 | device 对拍 | golden | 负例 RED 臂 | evidence schema（建议名） |
|---|---|---|---|---|---|
| D4.1 流送 | 每帧 load/spawn 计数 ≤ 预算契约字段；resident 集合 = target ∩ 预算 | 大世界场景 ≥30 min / ≥10000 帧 soak，**hitch p99 ≤ 实测阈值**，零崩溃零页错误 | 固定相机轨迹 cell load/unload 事件序列逐字 golden（确定性） | 预算注入违约（MaxStreamingCellsPerFrame=0 注入）：必须排队降级且计数器报警，出现静默超帧即 RED | `g9d4_partition_stream_soak_<ts>.json` |
| D4.2 HLOD | 产物哈希双构建相等（M79 形态）；运行时零合并代码路径断言 | 代理/全量互斥切换 device 渲染，screen-size 阈值边界帧对拍 | 代理 vs 原始屏幕空间误差上界 golden；切换距离表 golden | 运行时合并调用尝试 → 断言/编译期拒绝 | `g9d4_hlod_bake_golden_<ts>.json` |
| D4.3 大气体 | 预算字段（步数/分辨率档）逐帧落 evidence；云/雾共用 Froxel 分配器断言 | 低分辨率 march + temporal upsample 与全分辨率参考 device 对拍（误差阈值实测标定） | host 参考 ray-march 实现逐像素 golden（解析雾场景）；weather map 资产编解码往返 golden | 篡改 weather map 资产签名 → 拒录 RED；时序链断裂（首帧无历史）必须正确初始化不得复用脏帧 | `g9d4_atmo_froxel_evidence_<ts>.json` |
| D4.4 水体 | IFFT 谱参数资产化断言；双管线几何路径互斥断言 | 大洋 compute IFFT 位移/梯度/Jacobian 与 host FFT 参考逐值对拍（容差实测标定） | Jacobian 泡沫阈值响应 golden；CDLOD 分档切换 golden | 负风速/非法谱参数资产 → 拒录 RED；浅水域越界写检测 | `g9d4_water_spectrum_golden_<ts>.json` |
| D4.5 毛发 | strand 档必须走精确 OIT 路径断言（禁止落默认档） | Marschner R/TT/TRT 与参考实现逐瓣对拍；三档几何同视角画质差测量 | 瓣能量守恒 golden；strand→card 股替换映射烘焙确定性 golden | 单瓣系数置零的 RED 渲染（缺 TT 瓣必须可见差异，无差异即管线未接通） | `g9d4_hair_lobe_golden_<ts>.json` |
| D4.6 皮肤 | 扩散 profile 资产化断言；LUT 回退档画质差计入报告 | Burley 屏空 pass 与参考实现 device 对拍 | 扩散 profile 参数 → 扩散半径响应 golden | profile 全零衰减 → 输出必须退化为纯漫反射（否则 profile 未生效，RED） | `g9d4_skin_diffusion_golden_<ts>.json` |
| D4.7 地形 | chunk 尺寸 ≡ cell 网格族断言；toroidal ring 无泄漏断言 | LOD/剔除/缝合全 compute 路径 device 渲染与 host 参考对拍 | 邻级缝合处顶点位置连续性 golden（裂缝=0） | 相邻 chunk LOD 差 >1 注入 → 必须触发缝合路径，出现裂缝像素即 RED | `g9d4_terrain_stitch_golden_<ts>.json` |
| D4.8 贴花 | DBuffer 通道在帧图占位断言（即使零贴花）；逐像素贴花评估数 ≤ cluster 上界 | DBuffer 合成与参考逐像素对拍；过绘制计数器 device 采集 | 法线/材质属性三通道内容 golden；前向回退档与 DBuffer 档语义等价 golden | 超 cluster 上界贴花密度注入 → 必须受界降级，过绘制计数越界即 RED | `g9d4_decal_cluster_evidence_<ts>.json` |
| D4.9 HDR/view transform | 四内置 view transform 插件逐一注册断言；交换链三路径可切换断言 | 同一 HDR 输入帧经 ACES 1.3/2.0/AgX/中性分别输出 device 对拍 | **AgX/ACES golden 对**：每插件一组输出 golden（含 hue-skew 已知差异记录）；HDR 元数据字段 golden | 未注册插件名调用 → 拒录 RED；非 HDR 交换链携带 PQ 输出 → RED；HDR 设备缺失时标定门登记 SKIP=not-triggered（不充绿） | `g9d4_viewtransform_golden_<ts>.json` |
| D4.10 后处理栈 | 节点顺序冻结断言；全程 HDR 线性域断言（节点输出范围探针） | 曝光 histogram→EV adapt 状态帧间持久 device 对拍；与 TAA/TSR 排序断言 | bloom 多尺度 mip 能量 golden；DOF scatter-as-gather 焦外响应 golden；曝光 adapt 曲线 golden | 节点内隐式 SDR clamp 注入 → 探针越界即 RED；曝光状态跨帧丢失注入 → RED | `g9d4_poststack_evidence_<ts>.json` |
| D4.11 OIT | 三档接口完备断言；排序 fallback 永远可达断言 | **benchmark 门**：nvpro 七算法 harness 同场景测量，帧时/内存曲线落 evidence；默认档选型必须引 benchmark 数据 | 各算法正确性 golden（排序参考真值）；linked-list 精确档与排序真值 diff=0 | 无 benchmark 数据的默认档选型提交 → 聚合 RED；精确档内存无界增长注入 → RED | `g9d4_oit_benchmark_<ts>.json` |

---

## 8. 风险与止损

| ID | 风险 | 预警 | 止损 |
|---|---|---|---|
| R-D4-1 | 范围爆炸：分区 + 大气 + 五族 + 显示 + OIT 同模块 | W1 退出证据不齐就想并行进 W2/W3 | 严格波次串行（同 G8 纪律）；族内各渲染器可独立降档/推迟，D4.1/D4.9 插件面/D4.11 benchmark 不可推迟 |
| R-D4-2 | HDR 设备不可得（M45 触发条件未满足） | 用 SDR 截图冒充 HDR 验收 | 管线/插件面在 SDR 上验证；设备标定门登记 SKIP=not-triggered 或 open-留痕，**不假绿** |
| R-D4-3 | AVBOIT 研究风险（2025 新算法，Vulkan 复现无公开参照） | benchmark 显示 WBOIT 已满足全部 workload | 有界近似档钉 WBOIT；AVBOIT 转观察项；精确档（毛发）不受影响 |
| R-D4-4 | 毛发跨模块强依赖（OIT 精确档 + 资产烘焙 + closure 扩展） | 毛发抢跑导致 OIT 档选型被需求倒逼 | 毛发钉 W4；OIT 选型只由 benchmark 数据裁决（D15） |
| R-D4-5 | AgX 对比度补偿陷阱 / hue-skew 差异被当作 bug 反复返工 | golden 对拍频繁「意外」翻红 | 已知差异写进 golden 记录（§7 D4.9）；补偿参数资产化（D13） |
| R-D4-6 | 流送 hitch 阈值拍脑袋 | budget JSON 出现手写 estimated | 阈值一律 G9 baseline 实测标定；soak 门 p99 不可手写 |
| R-D4-7 | 地形/贴花静默依赖 SVT 复活（G8 no-go 项借尸还魂） | 设计文档出现虚拟纹理依赖 | D17 显式排除；如出现真实大纹理需求，走独立判档不搭 D4 便车 |
| R-D4-8 | MaterialClosure 32B 冻结面被毛发/皮肤静默扩张 | closure 字段 diff 出现在非 RFC PR | §9 RFC 修订先行；无 RFC 不得动冻结面（0-byte 纪律） |
| R-D4-9 | weather map / 扩散 profile 等资产制作缺工具链 | 专项渲染器绿灯但无真实资产可验 | 资产 baker 与渲染器同波交付；golden 先用程序化生成资产，真实资产需求另行登记 |

---

## 9. spec / RFC 需求（G9 立项后起草）

| 需求 | 内容要点 | 触及冻结面 |
|---|---|---|
| RFC-G9-D4-α 大世界分区 schema 与流送契约 | persistent world / cell / streaming source / 三项预算契约字段 / Data Layer 掩码位预留 / 四事件接口 / cell 页格式对 M04 ABI 的引用关系 | 消费 M04/M37 ABI（不改写） |
| RFC-G9-D4-β 显示管线与可插拔 view transform 语义 | 三交换链路径、运行时切换语义、ViewTransform trait ABI、HDR 元数据责任面、后处理节点顺序冻结、曝光持久资源与 TAA/TSR 时域排序 | 帧图/时域语义新增（G8 M24 契约只读引用） |
| RFC-G9-D4-γ OIT 策略与帧图语义 | 三档语义、benchmark 门形态、精确档仅限毛发的作用域限制、排序 fallback 永久保留条款、AVBOIT 体素与 Froxel 布局对齐评估窗 | 帧图新增 pass |
| RFC-G9-D4-δ closure 扩展（条件型） | Marschner R/TT/TRT 与 Burley diffusion 的 closure 表达：优先单 closure 参数化；确需扩面时按 G5 冻结面修订流程，**禁止静默扩 32B** | MaterialClosure 32B（G5 冻结面，修订路径） |
| spec 条款 | `.rx` 前端新增 kernel 族（froxel march、IFFT、波方程、SSS pass、DBuffer 合成、cluster 贴花、OIT 三档）的 shader 能力/capability 声明；M85 manifest 条目扩展 | spec-first + RED 先行（承 G8 §3 纪律） |
| 0-byte 纪律 | G8 closed 契约四件套、G8_P2_DECISIONS、G8_CANDIDATE_DECISIONS、G7/G6/G5 车道全部 0-byte；本文及其后续修订只追加 | — |

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08 | 首版 DRAFT：九节结构；承接 M43/M45~M49 六个 G8 锚；调研结论 1~6 逐条落位；四波次建议；11 子系统验收门草案（含流送 hitch soak、AgX/ACES golden 对、OIT benchmark 门）。 |
