# G9-R7 — 大世界分区、专项渲染器族与显示管线（深度调研）

> **所属**：G9 文档集（`milestones/g9/`）——本文是 [G9_PLAN.md](../G9_PLAN.md) / [G9_CAPABILITY_MATRIX.md](../G9_CAPABILITY_MATRIX.md) 与 [design/G9_D4_WORLD_AND_SPECIALTY_RENDERERS.md](../design/G9_D4_WORLD_AND_SPECIALTY_RENDERERS.md) 的调研输入材料（编号顺延 G8 的 R1~R3）。
>
> **调研基准日**：2026-08-08；**全部来源访问日期**：2026-08-08。**调研方式**：联网检索（20+ 次检索，优先一手来源：Epic 官方文档、advances.realtimerendering.com、GitHub、GDC Vault、ACM DOI、Ubisoft La Forge、Blender/Godot/three.js 官方资料、ACEScentral/oscars.org），全部结论附来源 URL。
>
> **纪律**：零编号占用——本文不新设任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用既有编号（M43/M45~M49、M01/M04/M37/M38/M44/M85、RD-037、G5/G6 冻结面等）；内容以 design/G9_D4 草案为事实基线，本文只做正式化与来源落位，不回写草案；G8 已 closed 的契约与判据 0-byte。
>
> **定位**：本文是五路联网调研中「大世界分区 × 专项渲染器族 × 显示管线」一路的正式化落盘；D4 草案正文引用的「调研结论 1~6」在此逐条独立成节，每条写明调研发现与对 G9/D4 的判定含义。

## 目录

1. 结论摘要
2. World Partition 与 HLOD（调研结论 1）
3. 大气体：云·雾·Froxel（调研结论 2）
4. 专项渲染器族：水体·毛发·皮肤·地形·贴花（调研结论 3）
5. HDR 显示管线与后处理栈（调研结论 4）
6. OIT 策略谱系（调研结论 5）
7. 共同设计模式（调研结论 6）
8. 对 G9-D4 的判定清单
9. 参考来源
10. 修订记录

---

## 一、结论摘要

| # | 调研结论（以 D4 草案为准的压缩表述） | D4 落点 | 主要来源 |
|---|---|---|---|
| 1 | UE World Partition 范式：单一持久世界 + 2D 网格 cell 运行时流送 + 显式流送预算契约 + Data Layers 正交维度 + always-loaded vs spatially-loaded schema 层区分；分区 = 数据结构先行，渲染器只是 cell 加载事件消费者；HLOD 离线烘焙代理、产物即资产、禁止运行时合并 | D4.1 / D4.2 | [S01]~[S05] |
| 2 | Schneider 大气体范式（Perlin-Worley 塑形 + Worley 侵蚀 + 2D weather map + 高度梯度 + ray-march）至今是行业基线；Meteoros 证明纯 Vulkan compute <3ms 可复现；时序上采样为默认路径；云与雾共用 Froxel 基础设施，一个大气体渲染器两个前端；weather map 资产化 | D4.3 | [S06]~[S12] |
| 3 | 五族专项渲染器各有收敛基线：水体 Tessendorf IFFT 大洋与浅水波方程两条管线分离 + tiling-and-blending 防重复；毛发 Marschner R/TT/TRT 三瓣 + 几何三层退化 + strand 精确 OIT；皮肤 Burley normalized diffusion 屏空单 pass + LUT 回退；地形 GPU-driven heightfield + toroidal 更新；贴花 DBuffer 三通道帧图设计期占位 + cluster 化 | D4.4~D4.8 | [S13]~[S23] |
| 4 | HDR 输出 = ACES RRT/ODT → scRGB 或 PQ/Rec.2020 双交换链路径；view transform 必须插件化（ACES 1.3/2.0、AgX、中性矩阵并列）；UE 后处理范式：histogram 自适应曝光 + 手动 EV → HDR 域 bloom → DOF（scatter-as-gather）→ tonemap → LUT → 输出变换；全程 HDR 线性域；曝光状态 = 帧间持久资源 | D4.9 / D4.10 | [S24]~[S32] |
| 5 | OIT 三档策略：默认 TAA 半透明 → 有界近似（WBOIT 起步 / AVBOIT 目标）→ linked-list 精确档仅服务毛发；AVBOIT 为首选跟进项；nvpro 七算法 sample 为性能基线，benchmark 先行再定默认档；排序 fallback 永远保留 | D4.11 | [S33]~[S37] |
| 6 | 共同设计模式：离线烘焙 + 运行时预算化流送 + compute-first GPU 管线 + 分级回退档（综合工程共识） | D4 全模块 | [S03][S05][S11][S20] 等 |

六条结论在 D4 草案中已逐条落位为设计决策 D1~D17 与四波次分期（W1 骨架 / W2 大气与地表 / W3 画质专项 / W4 精专）；本文的职责是为每条结论给出可公开复核的来源链，并显式记录判定的边界条件。

**来源质量与数值复核状态**：37 条来源全部经本次联网检索真实定位（优先一手：Epic 官方文档、advances.realtimerendering.com 课程 PDF、GDC Vault、ACM/Eurographics DOI、GitHub 官方仓库、Ubisoft La Forge、Blender/Godot/three.js 官方资料、oscars.org/ACEScentral）。其中三个数值型声明已直接复核原文：Meteoros「<3ms/Frame @ Full HD / GTX 1070 笔记本」（抓取 GitHub README 复核 [S08]）；EGSR 2025 毛发 LOD「up to 13× speedup」（论文页/项目页复核 [S17]，注意是上限值）；three.js `AgXToneMapping`（抓取官方文档页复核 [S29]）。WBOIT 的 JCGT 页面 [S33] 经引用网络定位（URL 见多条独立文献的参考条目），本次未直接打开正文，不影响其作为标准出处的效力。无「来源 URL 未能独立复核」条目。

---

## 二、World Partition 与 HLOD（调研结论 1）

### 2.1 调研发现

**单一持久世界 + 运行时 2D 网格 cell 流送。** Epic 官方文档将 World Partition 定义为「自动数据管理与基于距离的世界流送系统」：整个世界存为单一持久世界（single persistent world），编辑器与运行时按 2D 网格把世界划分为 cell，cell 依据 streaming source（玩家控制器即默认 streaming source，也可用 Streaming Source 组件自定义探针）动态加载与卸载 [S01]。Actor 在 schema 层显式区分 **spatially loaded**（参与空间分格流送）与 **always loaded**（全局常驻）两类，这一区分是资产属性而非运行时状态 [S01]。cell 边长与加载半径（loading range）均为可配置资产参数 [S01]。

**显式流送预算契约（防 hitch）。** Epic 官方的 Level Streaming Hitching Guide 把大世界 hitch 的防线落在「每帧节流 + 预算可测量」上：逐帧限制加载/生成工作量、异步加载优先、监控逐 cell 成本，而不是事后调参 [S05]。D4 草案将其上升为一等契约字段 `MaxStreamingCellsPerFrame` / `MaxActorsToSpawnPerFrame` / `MemoryBudgetMB`（字段名为 D4 自定契约，UE 公开等价物是 streaming source 距离环与逐帧节流实践 [S01][S05]）。

**Data Layers 是正交于空间分格的激活维度。** Data Layers 是 World Partition 内独立于 2D 网格的 Actor 组织系统：编辑器与运行时均可按层动态加载/卸载，服务于编辑分工与 gameplay 驱动（任务进度、状态切换） [S02]。官方文档同时给出性能告诫：同一资产挂在过多运行时 Data Layer 上会劣化 World Partition 流送性能 [S02]——这直接支撑 D4「v1 只预留掩码位、v2 才实现激活语义」的保守分期。

**分区 = 数据结构先行。** World Partition 官方文档的叙述模型是「数据管理 + 距离流送」：cell 是数据组织与 I/O 的单位，渲染侧（含地形、HLOD、光源集）只是 cell 加载/卸载事件的消费者 [S01]。这是 D4.1 把分区数据模型放在第一波的文献依据。

**HLOD：离线烘焙代理，产物即资产。** UE 的 HLOD 文档给出两条关键事实：① World Partition HLOD 以 cell 为单位构建——World Partition 用网格把世界分为可动态加载/卸载的 cell，HLOD 系统在此基础上为远处观察生成简化代理 [S03]；② HLOD 构建是离线流程（Generate Clusters → 逐层生成代理网格与合并材质），构建产物是持久资产，运行时只做按屏幕尺寸的切换 [S03][S04]。「Builder 按 Component 分发」对应 UE 的 HLOD Layer/Builder 可插拔机制（不同层可选 instancing / merging / simplification 等构建器） [S03]。**禁止运行时合并**是「产物即资产」的直接推论：代理在 cook/构建期定型，运行时零合并、零简化 [S03][S04]。

### 2.2 对 G9/D4 的判定含义

- **D4.1 分区数据模型先行（决策 D1/D2）成立**：schema 层 `always_loaded` vs `spatially_loaded` 区分、cell 为正方形 2D 网格（边长为资产属性）、streaming source 距离环求 target 集合——均可逐字对照 UE 公开范式 [S01]；预算三字段作为一等契约并逐帧落 evidence，是 hitching guide 节流实践的可审计化 [S05]。
- **Data Layers v1 仅预留掩码位（决策 D4）成立**：官方文档自身警示滥用成本 [S02]，v2 再实现激活语义可避免 schema 二次迁移。
- **D4.2 HLOD 纯离线烘焙、运行时零合并（决策 D3）成立**：烘焙确定性（双构建 hash 相等）与代理误差 golden 可挂接在「产物即资产」通道（M01/M04）上，无运行时路径 [S03][S04]。
- **渲染器只做事件消费者**：`CellLoadBegin / CellResident / CellUnloadBegin / CellEvicted` 四事件为唯一消费面，地形 chunk、HLOD、贴花 cluster、流送光源集一律挂事件、不反向查询分区状态——与 UE「数据管理先行」模型一致 [S01]。

---

## 三、大气体：云·雾·Froxel（调研结论 2）

### 3.1 调研发现

**Schneider 范式是行业基线。** Guerrilla 在 SIGGRAPH 2015 Advances 课程发表的 Horizon Zero Dawn 体积云方案确立了实时体积云的标准配方：3D Perlin-Worley 噪声做低频基础塑形、高频 Worley 噪声做边缘侵蚀、2D weather map（覆盖度/降水/云类型）驱动宏观分布、高度梯度定义云层剖面，最后 ray-march 出图，设计目标即「art direct-able、多云型、与天气系统集成」 [S06]。SIGGRAPH 2017 的 Nubis 演讲把它工程化为 Decima 引擎的创作管线（实时创作者工作流 + 效率改进） [S07]；SIGGRAPH 2022 的 Nubis, Evolved 继续沿此线演进 [S10]。

**纯 Vulkan compute 可复现，预算 <3ms。** UPenn CIS 565 期末项目 Meteoros 用纯 Vulkan（compute ray-march + 光栅合成 + 后处理）复现了 Schneider/Nubis 管线，README 明确记录「Runs at < 3ms/Frame at a Full HD Resolution on a notebook GTX 1070」（本次调研已抓取 README 复核该数值），并实现 reprojection 优化（每帧仅 ray-march 1/16 像素，其余重投影） [S08]。这为 D4.3「compute-first + 低分辨率 march 默认」提供了独立实现级证据。

**时序上采样是默认路径。** GDC 2022《The Real-time Volumetric Superstorms of Horizon Forbidden West》讲述了 Nubis 的后续扩展，其中明确包含「用 temporal upscaling 渲染高速移动云的解决方案」 [S09]；SIGGRAPH 2022 Nubis, Evolved 同样沿此路线 [S10]。低分辨率 ray-march + temporal reprojection 上采样因此应作为默认档，全分辨率 march 只作高端档。

**云与雾共用 Froxel 基础设施。** Frostbite 的 Sébastien Hillaire 在 SIGGRAPH 2015 提出统一的视锥对齐体素（froxel）体积渲染：把参与介质的密度与光照累积进 frustum 对齐 3D 纹理，一个基础设施服务雾、光柱与体积云 [S11]。UE 的 Volumetric Fog 文档是同一范式的引擎落地：Volumetric Fog 在视锥网格上逐点计算参与介质密度与光照，作为 Exponential Height Fog 的组成部分 [S12]。「一个大气体渲染器、雾（高度雾前端）与云（Schneider 前端）两个前端」由此不是发明而是行业收敛形态。

**weather map 资产化。** Schneider 范式中 weather map 本就是数据纹理而非 shader 常量 [S06]；Meteoros 复现同样使用外部 weather map 纹理 [S08]。D4 将其走 M01 资产管线（离线烘焙 + 签名 + DDC）是对范式的忠实执行。

### 3.2 对 G9/D4 的判定含义

- **D4.3 统一 Froxel 基础设施一次性建造（决策 D5）成立**：雾前端解析项直写 Froxel 密度场（第一波），云前端噪声 baker + weather map + 低分辨率 march（第二波），共用体素内存布局——后续 AVBOIT 体素评估（D4.11/D16）可复用同族布局 [S11][S12]。
- **时序上采样为默认路径（决策 D6）成立**：GDC 2022 生产级背书 [S09] + Meteoros 预算证据 [S08]；全分辨率仅高端档。
- **weather map / 噪声纹理全部资产化（决策 D7 的大气部分）成立** [S06][S08]。
- **预算契约**：ray-march 最大步数、froxel 分辨率档、上采样开关为预算字段，逐帧落 evidence；验收上「低分辨率 march + temporal upsample vs 全分辨率参考」对拍阈值须实测标定，禁止手写。

---

## 四、专项渲染器族：水体·毛发·皮肤·地形·贴花（调研结论 3）

### 4.1 水体（D4.4）

**Tessendorf IFFT 谱大洋。** Tessendorf 的 SIGGRAPH 课程笔记《Simulating Ocean Water / Simulating Ocean Surface》（1999–2004 持续更新，作者主页存档）确立了谱方法大洋的行业基线：由风浪谱出发做 IFFT 得到位移场，配套梯度（法线）与 Jacobian（泡沫判定，负值折叠处产沫） [S14]。几何侧 CDLOD（Strugar, J. Graphics Tools 2009/2010）给出距离连续 LOD 的高度图网格分档与平滑 morph，是大洋网格分档的经典参照 [S15]。**两条管线分离**：局部交互水域（池塘/河流/波纹）走波方程高度场模拟（GPU Gems 第 1 章给出 GPU 上局部水面模拟的经典公开参考 [S22]），与谱大洋不共享几何路径，仅共享水面着色输入面。**tiling-and-blending 防重复感**：Ubisoft La Forge 2024 年发表的《Making Waves in Ocean Surface Rendering using Tiling and Blending》正是针对 FFT/谱贴图周期重复问题，提出多尺度 tiling-and-blending 方案（该文同时把 Tessendorf 课程笔记列为参考 [1]） [S13]。

**判定含义**：决策 D8（大洋/浅水两管线分离，仅共享着色 closure 输入面）与「位移/梯度/Jacobian 三贴图 + CDLOD 分档 + 多尺度谱 tiling-and-blending」的资产化谱参数均有公开一手来源支撑 [S13][S14][S15][S22]；浮力接口面预留但 D4 不实现（M77 边界维持）。

### 4.2 毛发（D4.5）

**Marschner R/TT/TRT 三瓣是 2003 至今唯一物理基线。** Marschner 等 SIGGRAPH 2003《Light Scattering from Human Hair Fibers》提出圆柱纤维的纵向/方位角分离散射模型，分解为 R（表面反射）、TT（透射-透射）、TRT（内反射）三条光路 [S16]——实时毛发着色二十余年的全部工程变体都在此框架内。**几何三层退化有最新定量背书**：EGSR 2025《Real-time Level-of-detail Strand-based Rendering》给出实时股级 LOD 框架，在不同 LOD 间无缝过渡、以股替换发簇，报告最高 13× 加速（数值已从论文页/项目页复核） [S17]——直接支撑「近 strand / 中 card / 远 mesh」三档退化与离线股聚类烘焙。**strand 档必须精确 OIT**：发丝级排序不可行，精确档（per-pixel linked list，见 §6 [S37]）仅服务 strand；card/mesh 档走默认半透明路径 [S16][S37]。

**判定含义**：决策 D9 成立；毛发因此排在 OIT 精确档落地之后（W4），strand→card 股替换映射由离线烘焙产出并做确定性 golden [S17]；三瓣着色参数（每缕基调色、高光偏移、medulla）资产化。

### 4.3 皮肤（D4.6）

**Burley normalized diffusion 屏空单 pass 已收敛。** Activision 的 Golubev 在 SIGGRAPH 2018 Advances《Efficient Screen-Space Subsurface Scattering Using Burley's Normalized Diffusion in Real-Time》把 Disney 离线 diffusion profile 落到屏空 separable 实现：Burley 归一化 profile 等价于能量守恒的曲面模糊，可用屏空 bilateral 滤波近似 [S18]。**低端回退**：Penner 的 Pre-Integrated Skin Shading（SIGGRAPH 2011 Advances，曲率 × NdotL 预积分 LUT）是移动端/低端 profile 的标准替代 [S19]。扩散 profile（RGB 三通道 falloff）应做成 per-material 资产而非硬编码。

**判定含义**：决策 D10 成立（屏空单 pass 为主、LUT 为回退档）；两档画质差纳入 golden 对照；profile 全零衰减必须退化为纯漫反射（RED 臂语义来源即 profile 的能量守恒性质 [S18]）。

### 4.4 地形（D4.7）

**GPU-driven heightfield。** GDC 2018《Terrain Rendering in 'Far Cry 5'》官方摘要明确：该 session 覆盖「用于高度场地形 LODing、culling、stitching 与渲染的 GPU compute 管线」 [S20]——LOD 选择/视锥剔除/邻级缝合全进 compute、CPU 零逐 chunk 提交，是 Far Cry 5 的生产实践。思想谱系上，geometry clipmaps（Losasso & Hoppe 2004 提出，GPU Gems 2 第 2 章 GPU 化）给出嵌套规则网格随视点增量平移、环状（toroidal）窗口复用的 LOD 结构 [S21]；toroidal 更新天然适配 Vulkan ring buffer。**chunk ≡ Partition cell**：地形分格与 D4.1 cell 对齐同一网格族，禁止第二套分格（决策 D11）——这是「分区 = 数据结构先行」结论 1 在地形上的直接推论 [S01][S20]。

**判定含义**：决策 D11 成立；toroidal 滚动与 M37 I/O 链对接（chunk 页迟到 → 父级 LOD 占位）；邻级缝合处顶点连续性 golden（裂缝 = 0）与「LOD 差 >1 注入必须触发缝合」RED 臂均源于缝合进 compute 的设计 [S20][S21]。

### 4.5 贴花（D4.8）

**DBuffer 三通道是 UE5 默认路径。** UE 官方贴花材质文档：DBuffer 混合模式的贴花在 BasePass 之前累积进 DBuffer（存 base color、normal、roughness 等通道），BasePass 再采样 DBuffer 合成 [S23]；材质侧的 Decal Response（DBuffer）属性定义逐通道响应（Color / Normal / Roughness 及组合） [S23]。**帧图设计期占位**：DBuffer 通道与 barrier 布局必须在帧图冻结面先行占位，即使 v1 贴花数量为零——后期插 pass 改全局帧图的代价远高于预留；**screen-space cluster 化防过绘制**（复用光照 cluster 结构对贴花体求交、限制逐像素评估上界）为 D4 的设计决定，以草案为准，公开文献中最接近的支撑是 UE 的 DBuffer 通道化实践 [S23]。

**判定含义**：决策 D12 成立；前向回退档（无 DBuffer 的低端 profile 走 decal-forward）v1 即定义两档语义等价性判据；过绘制计数器落 evidence。

---

## 五、HDR 显示管线与后处理栈（调研结论 4）

### 5.1 调研发现

**HDR 输出 = ACES RRT/ODT → scRGB 或 PQ/Rec.2020 双交换链路径。** UE 官方 HDR Display Output 文档：HDR 输出经 ACES viewing transform（RRT/ODT 族），输出设备与色域由控制台配置，覆盖 scRGB（线性 FP16）与 PQ/Rec.2020 等路径，HDR 元数据（显示亮度、mastering 参数）由输出阶段填写 [S24]。ACES 体系本身在 2025 年完成代际更替：美国电影艺术与科学学院 2025 年发布 ACES 2.0（随 Academy Software Foundation 进入新阶段），官方公告称其带来色彩渲染一致性等一系列增强 [S25]——「ACES 1.3 与 2.0 并列」是真实存在的双版本格局。

**view transform 必须插件化，锁死单一 tonemapper 是 2026 架构错误。** 三个事实支撑：① ACES 1.x filmic 的 hue-skew 是公认缺陷——ACES 官方 CineGear 2025 材料把 hue 相关问题列入 ACES 2.0 的针对性修复清单 [S32]，SIGGRAPH 2025 的 GT7 色调映射课程文也把「hue skew minimized」列为 ACES 2.0 相对 1.3 的改动点 [S32]；② AgX 已进入主流工具链默认：Blender 4.0 起 AgX 取代 Filmic 成为新文件默认 view transform [S26]，AgX 原始实现为 Troy Sobotka 的开源 OCIO 配置 [S27]，Godot 4.4 起内置 AgX tonemapper [S28]，three.js 亦提供 `AgXToneMapping` 常量（官方文档已复核） [S29]；③ **AgX 对比度补偿陷阱**：Blender 的 AgX 默认 look 为低对比（None），对比度须由 look 档（Medium/High/Very High Contrast 等）补偿 [S26]——D4 登记「补偿参数必须随 view transform 资产化，禁止硬编码进 tonemap 节点」正是对这一陷阱的显式处理（该陷阱表述以 D4 草案为准，Blender look 档事实见 [S26]）。

**UE 后处理范式顺序。** UE 官方文档给出的语义链：自动曝光基于场景亮度 histogram（eye adaptation，高/低百分位提取 + EV100 手动补偿） [S30]；后处理栈覆盖 bloom、DOF、色彩分级等节点 [S31]；bloom 在 tonemap 之前的 HDR 域做多尺度 mip 累积、DOF 走 scatter-as-gather、色彩分级 LUT 在 tonemap 之后、最终经输出变换（RRT/ODT 或中性）编码——全程 HDR 线性域 [S24][S30][S31]。**曝光状态 = 帧间持久资源**：eye adaptation 的上/下自适应速率即跨帧状态 [S30]，必须与 TAA/TSR（M24）时域链显式排序。

### 5.2 对 G9/D4 的判定含义

- **D4.9 三交换链路径为一等资源、运行时切换（决策 D13）成立**：SDR/scRGB/PQ-Rec.2020 语义对照 UE 公开配置面 [S24]；`ViewTransform` trait 内置 ACES 1.3 / ACES 2.0 / AgX / 中性矩阵四实现并列，第三方可注册——多版本 ACES 共存与 AgX 生态位使「单一硬编码」失去辩护空间 [S25][S26][S27][S28][S29][S32]。
- **AgX/ACES golden 对**：hue-skew 与对比度补偿差异是已知差异而非 bug，写入 golden 记录（风险 R-D4-5 的文献依据 [S26][S32]）。
- **D4.10 节点顺序冻结（决策 D14）成立**：exposure（histogram + EV）→ bloom（HDR 域）→ DOF → tonemap（插件）→ LUT → 输出变换 → UI 合成（SDR 域）；任何节点隐式 clamp 到 SDR 即 RED [S24][S30][S31]。
- **M45 条件触发诚实边界**：管线/插件面在 SDR 上即可全量验证；HDR 设备标定门在条件未触发时登记 SKIP=not-triggered，不假绿。

---

## 六、OIT 策略谱系（调研结论 5）

### 6.1 调研发现

**三档策略。** ① 默认档：半透明走 TAA 合成路径（现状延伸，成本最低、近似可接受）。② 有界近似档：WBOIT（McGuire & Bavoil, JCGT 2013）给出单 pass、内存有界、无需排序的加权混合近似 [S33]；**AVBOIT**（Activision，SIGGRAPH 2025 Advances）是首选跟进目标——自适应体素收集（Adaptive Voxel-Based OIT），为 Call of Duty 的透明渲染管线开发并已随产品出货，官方 slides 与 Activision Research 出版物页均可复核 [S34][S35]。③ 精确档：per-pixel linked list（Yang 等，Computer Graphics Forum 2010，GPU 上并发链表构建） [S37]，仅服务毛发 strand，场景级不开放。

**benchmark 先行再定默认档。** NVIDIA nvpro 的 `vk_order_independent_transparency` sample 在同一 Vulkan harness 内实现了七种 OIT 技术（含 weighted blended、linked list 等，README 明示「Demonstrates seven different techniques」），是现成的一站式对照基线 [S36]。默认档选型必须由同场景、同 overdraw 分布下的帧时/内存曲线裁决，不由论文偏好裁决；**排序 fallback（depth-sorted alpha）永远保留**为最低端档与正确性对照真值。

### 6.2 对 G9/D4 的判定含义

- **D4.11 benchmark 门先行（决策 D15）成立**：W1 只建 harness（以 nvpro 七算法为对照 [S36]）不测不定档；默认档由 RTX 4070 Ti 实测数据裁决。
- **AVBOIT 为首选跟进项（决策 D16）**：已出货的 2025 新算法 [S34][S35]，Vulkan 复现无公开参照属登记风险（R-D4-3）；其自适应体素结构与 D4.3 Froxel 族对齐列为评估项、不承诺复用。
- **精确档作用域限制**：linked-list [S37] 仅毛发 strand 启用；内存无界增长注入即 RED。
- **排序 fallback 永久保留**写入条款：正确性 golden 以排序真值 diff=0 校验精确档。

---

## 七、共同设计模式（调研结论 6）

### 7.1 调研发现

跨 §2~§6 的来源可归纳出四条反复出现的工程模式，此为综合工程共识（可引用最接近的公开资料支撑）：

- **离线烘焙，产物即资产**：HLOD 代理离线构建、运行时零合并 [S03][S04]；Perlin-Worley/Worley 噪声纹理离线烘焙 [S06][S08]；weather map、扩散 profile、谱参数、股替换映射全部资产化 [S06][S13][S18]。
- **运行时预算化流送**：cell 流送逐帧节流 + 预算可测量可审计 [S01][S05]；ray-march 步数/froxel 分辨率/OIT 内存界同为预算字段（§3、§6）。
- **compute-first GPU 管线**：froxel 统一体积基础设施 [S11][S12]、地形 LOD/剔除/缝合全进 compute [S20]、IFFT/波方程 compute 化 [S14][S22]、OIT benchmark harness [S36]。
- **分级回退档**：云的全分辨率高端档 vs 时序上采样默认档 [S09]；皮肤 Burley 屏空 pass vs pre-integrated LUT [S18][S19]；贴花 DBuffer vs decal-forward [S23]；OIT 三档 + 排序 fallback [S33][S36][S37]；毛发 strand/card/mesh 三档 [S17]。

四条模式在 D4 各子系统的落位对照：

| 子系统 | 离线算死（烘焙/资产化） | 运行时预算契约 | 回退档 |
|---|---|---|---|
| D4.1 分区流送 | cell 归属、Data Layer 掩码位 schema | 三项预算字段逐帧落 evidence [S05] | 超预算排队降级 [S05] |
| D4.2 HLOD | 代理网格逐 Component 烘焙 [S03][S04] | 运行时零合并，仅 screen-size 切换 | —（切换档位即回退） |
| D4.3 大气体 | 噪声纹理、weather map [S06][S08] | march 步数 / froxel 分辨率档 | 时序上采样 ↔ 全分辨率 [S09] |
| D4.4 水体 | 谱参数、三贴图参数化 [S13][S14] | IFFT/波方程 compute 预算 | 周期谱表寻址档 |
| D4.5 毛发 | 股聚类 + card 图集烘焙 [S17] | strand 档精确 OIT 内存界 [S37] | strand → card → mesh [S17] |
| D4.6 皮肤 | 扩散 profile 资产 [S18] | 屏空 pass 分辨率/半径 | pre-integrated LUT [S19] |
| D4.7 地形 | heightfield 页资产（M04） | toroidal ring 驻留窗口 [S21] | 父级 LOD 占位 [S20] |
| D4.8 贴花 | DBuffer 通道帧图占位 [S23] | 逐像素评估数 cluster 上界 | decal-forward 档 [S23] |
| D4.11 OIT | benchmark 数据资产化 | 各档内存/帧时界 [S36] | WBOIT ← AVBOIT；排序 fallback [S33] |

### 7.2 对 G9/D4 的判定含义

- D4 内每个专项渲染器设计文档第一节必须是「**哪些离线算死、哪些是运行时预算契约**」——本报告 §2~§6 的来源链证明这不是风格偏好而是各子系统收敛后的共同形态。
- 全部预算阈值（hitch p99、ray-march 步数、OIT 内存界）禁止手写，须由 G9 立项重测的 4070 Ti baseline 实测标定（R-D4-6 的执行口径）。
- 分级回退档必须 v1 即定义档间语义等价性判据，禁止「先高端档、回退以后再说」。

---

## 八、对 G9-D4 的判定清单

| 结论 | 判定 | D4 落点（草案决策/波次） | 关键来源 | 边界条件 |
|---|---|---|---|---|
| 1 | 采纳：分区数据模型先行、预算一等契约、HLOD 离线烘焙零合并、Data Layers v1 仅预留位 | D4.1/D4.2；D1~D4；W1 | [S01]~[S05] | 预算三字段名为 D4 自定契约，UE 公开等价物为 streaming source 距离环与逐帧节流 [S01][S05] |
| 2 | 采纳：Schneider 范式为基线；时序上采样默认；云/雾共用 Froxel 两前端；weather map 资产化 | D4.3；D5~D7；W2 雾/W3 云 | [S06]~[S12] | Meteoros <3ms 为 GTX 1070 笔记本/Full HD 数据 [S08]，4070 Ti 阈值须实测重标 |
| 3 | 采纳：五族各按收敛基线落位；水体双管线分离；毛发三档+strand 精确 OIT；皮肤 Burley 主档+LUT 回退；地形 chunk≡cell；贴花 DBuffer 帧图占位 | D4.4~D4.8；D8~D12；W2~W4 | [S13]~[S23] | EGSR 13× 为论文报告上限值（up to） [S17]；贴花 cluster 化为 D4 设计决定，公开支撑取 DBuffer 实践 [S23] |
| 4 | 采纳：view transform 插件化四内置并列；后处理顺序冻结、全程 HDR 线性域；曝光状态帧间持久 | D4.9/D4.10；D13/D14；W1 骨架 | [S24]~[S32] | AgX 对比度补偿陷阱表述以 D4 草案为准，Blender look 档事实见 [S26]；HDR 设备标定条件未触发则登记 open-留痕 |
| 5 | 采纳：OIT 三档；AVBOIT 首选跟进；nvpro 七算法为 benchmark 基线；排序 fallback 永保留 | D4.11；D15/D16；W1 benchmark/W4 精确档 | [S33]~[S37] | AVBOIT Vulkan 复现无公开参照（R-D4-3）；精确档仅毛发作用域 |
| 6 | 采纳：离线烘焙 + 预算化流送 + compute-first + 分级回退档为全模块统一模式 | D4 全模块；各设计文档第一节强制 | [S03][S05][S11][S13][S17][S18][S20][S36] | 阈值一律 G9 baseline 实测标定，禁止手写 estimated |

**未决/观察项**（随草案风险表登记，不构成新结论）：AVBOIT 跟进节奏由 benchmark 数据裁决（R-D4-3）；AgX/hue-skew 已知差异进 golden 记录（R-D4-5）；资产工具链与渲染器同波交付（R-D4-9）。

---

## 九、参考来源

- **[S01]** World Partition in Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/world-partition-in-unreal-engine （访问日期 2026-08-08）
- **[S02]** World Partition — Data Layers in Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition---data-layers-in-unreal-engine （访问日期 2026-08-08）
- **[S03]** World Partition — Hierarchical Level of Detail in Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/world-partition---hierarchical-level-of-detail-in-unreal-engine （访问日期 2026-08-08）
- **[S04]** Building Hierarchical Level of Detail Meshes in Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/building-hierarchical-level-of-detail-meshes-in-unreal-engine （访问日期 2026-08-08）
- **[S05]** Level Streaming Hitching Guide（Epic Developer Community 官方教程）：https://dev.epicgames.com/community/learning/tutorials/qpll/unreal-engine-level-streaming-hitching-guide （访问日期 2026-08-08）
- **[S06]** The Real-time Volumetric Cloudscapes of Horizon: Zero Dawn（Schneider & Vos，SIGGRAPH 2015 Advances 课程 PDF）：https://advances.realtimerendering.com/s2015/The%20Real-time%20Volumetric%20Cloudscapes%20of%20Horizon%20-%20Zero%20Dawn%20-%20ARTR.pdf （访问日期 2026-08-08）
- **[S07]** Nubis: Authoring Real-Time Volumetric Cloudscapes with the Decima Engine（SIGGRAPH 2017 Advances 课程页）：https://advances.realtimerendering.com/s2017/index.html （访问日期 2026-08-08）
- **[S08]** Meteoros — Real-time Cloudscape Rendering in Vulkan（UPenn CIS 565，GitHub；README 载 <3ms/Frame @ Full HD / GTX 1070 笔记本）：https://github.com/AmanSachan1/Meteoros （访问日期 2026-08-08）
- **[S09]** The Real-time Volumetric Superstorms of Horizon Forbidden West（Schneider，GDC 2022 演讲 slides）：https://media.gdcvault.com/GDC+2022/Speaker+Slides/TheReal-timeVolumetricSuperstormsOfHorizonForbiddenWest_Schneider_Andrew.pdf （访问日期 2026-08-08）
- **[S10]** Nubis, Evolved: Real-Time Volumetric Clouds for Skies, Environments, and VFX（SIGGRAPH 2022 Advances 课程 PDF）：https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-NubisEvolved-NoVideos.pdf （访问日期 2026-08-08）
- **[S11]** Physically Based and Unified Volumetric Rendering in Frostbite（Hillaire，SIGGRAPH 2015 Advances，DOI）：https://doi.org/10.1145/2776880.2787701 （访问日期 2026-08-08）
- **[S12]** Volumetric Fog in Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/volumetric-fog-in-unreal-engine （访问日期 2026-08-08）
- **[S13]** Making Waves in Ocean Surface Rendering using Tiling and Blending（Ubisoft La Forge，2024-07-25）：https://www.ubisoft.com/en-us/studio/laforge/news/5WHMK3tLGMGsqhxmWls1Jw/making-waves-in-ocean-surface-rendering-using-tiling-and-blending （访问日期 2026-08-08）
- **[S14]** Simulating Ocean Water / Simulating Ocean Surface（Tessendorf，SIGGRAPH 课程笔记 1999–2004，作者 Clemson 主页存档页）：https://jtessen.people.clemson.edu/reports/index.html （访问日期 2026-08-08）
- **[S15]** Continuous Distance-Dependent Level of Detail for Rendering Heightmaps（CDLOD，Strugar，J. Graphics, GPU, & Game Tools 14, 2009；作者存档 PDF）：https://aggrobird.com/files/cdlod_latest.pdf （访问日期 2026-08-08）
- **[S16]** Light Scattering from Human Hair Fibers（Marschner et al.，SIGGRAPH 2003，ACM TOG 22(3), 780–791）：https://dl.acm.org/doi/10.1145/1201775.882345 （访问日期 2026-08-08）
- **[S17]** Real-time Level-of-detail Strand-based Rendering（Huang et al.，EGSR 2025；up to 13× speedup，作者存档 PDF）：https://sites.cs.ucsb.edu/~lingqi/publications/paper_egsr25_hairlod.pdf （访问日期 2026-08-08）
- **[S18]** Efficient Screen-Space Subsurface Scattering Using Burley's Normalized Diffusion in Real-Time（Golubev，Activision，SIGGRAPH 2018 Advances 课程 PDF）：https://advances.realtimerendering.com/s2018/Efficient%20screen%20space%20subsurface%20scattering%20Siggraph%202018.pdf （访问日期 2026-08-08）
- **[S19]** Pre-Integrated Skin Shading（Penner，SIGGRAPH 2011 Advances 课程页条目）：https://advances.realtimerendering.com/s2011/ （访问日期 2026-08-08）
- **[S20]** Terrain Rendering in 'Far Cry 5'（GDC 2018，GDC Vault 官方页）：https://gdcvault.com/play/1025480/Terrain-Rendering-in-Far-Cry （访问日期 2026-08-08）
- **[S21]** GPU Gems 2 第 2 章 Terrain Rendering Using GPU-Based Geometry Clipmaps（Asirvatham & Hoppe；含 Losasso & Hoppe 2004 geometry clipmaps 溯源，NVIDIA 官方页）：https://developer.nvidia.com/gpugems/gpugems2/part-i-geometric-complexity/chapter-2-terrain-rendering-using-gpu-based-geometry （访问日期 2026-08-08）
- **[S22]** GPU Gems 第 1 章 Effective Water Simulation from Physical Models（Finch，NVIDIA 官方页）：https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models （访问日期 2026-08-08）
- **[S23]** Decal Materials in Unreal Engine（DBuffer 通道与 Decal Response，Epic 官方文档）：https://dev.epicgames.com/documentation/en-us/unreal-engine/decal-materials-in-unreal-engine （访问日期 2026-08-08）
- **[S24]** High Dynamic Range Display Output in Unreal Engine（ACES viewing transform、HDR 输出设备/色域配置，Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/high-dynamic-range-display-output-in-unreal-engine （访问日期 2026-08-08）
- **[S25]** Academy of Motion Picture Arts and Sciences Launches the Next Chapter of ACES with the Academy Software Foundation（含 ACES 2.0 发布事实，oscars.org 官方新闻稿）：http://press.oscars.org/news/academy-motion-picture-arts-and-sciences-launches-next-chapter-aces-academy-software （访问日期 2026-08-08）
- **[S26]** Blender 4.0 Release Notes — Color Management（AgX 取代 Filmic 成为默认 view transform；look 对比度档，Blender 官方）：https://developer.blender.org/docs/release_notes/4.0/color_management/ （访问日期 2026-08-08）
- **[S27]** AgX 原始实现（Troy Sobotka，GitHub OCIO 配置）：https://github.com/sobotka/AgX （访问日期 2026-08-08）
- **[S28]** Environment — Godot Engine 官方文档（TONE_MAPPER_AGX 条目）：https://docs.godotengine.org/en/stable/classes/class_environment.html （访问日期 2026-08-08）
- **[S29]** three.js 官方文档 — Renderer 常量（AgXToneMapping 条目，已抓取复核）：https://threejs.org/docs/#api/en/constants/Renderer （访问日期 2026-08-08）
- **[S30]** Auto Exposure in Unreal Engine（histogram eye adaptation / EV100，Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/auto-exposure-in-unreal-engine （访问日期 2026-08-08）
- **[S31]** Post Process Effects in Unreal Engine（bloom/DOF/色彩分级节点族，Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/post-process-effects-in-unreal-engine （访问日期 2026-08-08）
- **[S32]** ACES Presents: Color Management That Can Up Your Game!（CineGear 2025 官方 deck；ACES 2.0 修复点含 hue 相关问题，ACEScentral）：https://acescentral.com/wp-content/uploads/2025/06/CineGear-2025-Deck-FIN-1.pdf （访问日期 2026-08-08）
- **[S33]** Weighted Blended Order-Independent Transparency（McGuire & Bavoil，JCGT 2(2), 2013）：http://jcgt.org/published/0002/02/09/ （访问日期 2026-08-08）
- **[S34]** Adaptive Voxel-Based Order-Independent Transparency（AVBOIT，Drobot 等，SIGGRAPH 2025 Advances 课程 PDF）：https://advances.realtimerendering.com/s2025/content/AVBOIT_SIG2025_MDROBOT-final.pdf （访问日期 2026-08-08）
- **[S35]** Adaptive Voxel-Based Order-Independent Transparency（Activision Research 出版物页）：https://research.activision.com/publications/2026/adaptive-voxel-based-order-independent-transparency （访问日期 2026-08-08）
- **[S36]** nvpro-samples/vk_order_independent_transparency（七种 OIT 技术 Vulkan sample，GitHub）：https://github.com/nvpro-samples/vk_order_independent_transparency （访问日期 2026-08-08）
- **[S37]** Real-Time Concurrent Linked List Construction on the GPU（Yang et al.，Computer Graphics Forum 29(4), 2010，DOI）：https://doi.org/10.1111/j.1467-8659.2010.01725.x （访问日期 2026-08-08）

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-08 | 初版：五路调研之「大世界×专项渲染器×显示管线」路正式化落盘（G9.0 文档集输入材料）。调研结论 1~6 逐条独立成节并附对 G9/D4 判定含义；37 条参考来源全部经联网检索定位，访问日期 2026-08-08。 |
