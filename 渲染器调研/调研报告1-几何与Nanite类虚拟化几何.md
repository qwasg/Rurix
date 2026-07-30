# 调研报告 1：几何 / Nanite 类虚拟化几何系统

> 面向 rurix（H:\rurix）的前沿论文与工业技术调研 · 2022–2026 · 出品日期：2026-07-28
>
> 本项目现状假设：已有 mesh/raster、Vulkan/DXIL、render graph/RHI 基础，但没有完整的场景几何流送与 Nanite 级可见性系统。本报告目标不是综述，而是给出可实现路线。

---

## 结论摘要（TL;DR）

对 rurix 而言，**投入产出比最高的 Nanite 类路线已经收敛为一条工业共识路径**：离线把网格构建成 **cluster/meshlet 的层级 DAG**（配合分组简化与裂缝保护），运行时以 **GPU 驱动的两阶段剔除**（实例→簇的视锥/背面锥/遮挡剔除 + 基于误差判定的 LOD cut）选出簇集合，光栅化写入 **64 位可见性缓冲（Visibility Buffer）**，再做材质解析与延迟着色。2022–2026 年间，这条路径的可复用积木大幅成熟：**meshoptimizer 的 clusterlod.h 单头库**（2024–2025，Nanite 式层级 LOD 构建，处理了 16.4 亿三角形的 Zorah 场景）[^45^][^201^]、**Bevy 0.14 的 Rust 开源实现**（与 rurix 同语言，帧管线与 Nanite 几乎同构，3092 只 14.4 万三角形兔子在 RTX 3080 上几何阶段仅 ~2.78ms）[^9^]、**NVIDIA vk_lod_clusters Vulkan 示例与 RTX Mega Geometry**（簇 LOD 与光追的统一）[^45^]、以及 D3D12 **Work Graphs Mesh Nodes**（2024，把剔除与绘制折叠进单张 GPU 工作图）[^4^][^7^]。

落地建议为五阶段：**P0 离线 meshlet 化 → P1 GPU 剔除+单层 LOD → P2 可见性缓冲+软件光栅 → P3 层级 DAG+两阶段 HZB → P4 cluster 流送+压缩**。前三个阶段不依赖任何新硬件特性（Vulkan 计算管线即可跑通），P3 起需要 `VK_KHR_shader_atomic_int64` 与 HZB 金字塔，P4 才触碰流送——严格满足"先做 meshlet + culling，再做 cluster streaming"的约束。全文第 5、6 节给出逐阶段改动点、数据结构、shader 阶段、资源布局与验证矩阵。

---

## 1. 调研范围与方法

本报告围绕用户指定的七个技术关键词展开：**meshlet/cluster、cluster culling、GPU-driven rendering、hierarchical culling、LOD virtualization、software raster vs mesh shader path、visibility buffer**。检索覆盖三类来源：一是 2022–2026 年的同行评议论文（Computer Graphics Forum、JCGT、Computers & Graphics、Eurographics/VMV/HPG 等，经学术检索引擎按年份过滤）；二是工业一手资料（Epic 官方文档与版本说明、Microsoft/GPUOpen 开发者博客、NVIDIA nvpro 示例、meshoptimizer 变更日志）；三是高质量开源实现的技术复盘（Bevy、Unity NADE 等）。 Nanite 的原始文献 Karis/Stubbe/Wihlidal SIGGRAPH 2021 课件[^14^]与 Karis HPG 2022 主题演讲[^31^]虽早于 2022 年，但它们是全部后续工作的定义性参照，本报告将其作为基线而非清单主体引用。

筛选标准只有一条：**能否映射到 rurix 现有的 RHI / render graph / mesh pass 并给出可验证的最小实现**。凡需要重写引擎、依赖封闭中间件、或仅有离线演示没有实时路径的工作，一律降级为"跟踪项"。评估实现代价时，优先采信有开源代码与实测数据的来源——例如 zeux 用 16 线程在约 3.5 分钟内完成 16.4 亿三角形场景的 cluster 层级构建（稀疏化修复后）[^45^]，JMS55 给出的逐 pass GPU 计时[^9^]，以及育碧/EPIC 公开的生产环境数字[^46^][^36^]。

---

## 2. 2022–2026 最值得落地的论文 / 技术清单

下表是全部候选的集中对比。标注 ★ 者为对 rurix"必读级"条目；其余按相关度排序。基线文献（2013–2021）单列于表后，不计入 2022–2026 清单但属于必读背景。

| # | 论文 / 技术 | 年份·出处 | 解决什么问题 | 核心算法一句话 | 实现代价 | rurix 相关度 |
|---|---|---|---|---|---|---|
| 1 ★ | **clusterlod.h / meshoptimizer 层级 cluster LOD** | 2024–2025，zeux 开源库[^45^][^201^] | 从任意高模自动生成 Nanite 式 DAG | 分簇→邻簇分组→保边界简化→再分簇递归 | 纯 CPU 离线库，可 FFI 或移植 Rust | **极高**：直接解决 P0 离线构建 |
| 2 ★ | **Virtual Geometry in Bevy 0.14（Rust 实现）** | 2024，JMS55 博客+代码[^9^] | 在 Rust/wgpu 上复刻 Nanite 帧管线 | 两遍剔除+VisBuffer+深度金字塔，帧 12 个 pass | 完整开源，架构可直接对照移植 | **极高**：同语言、同 API 世代 |
| 3 ★ | **UE 5.5→5.7 Nanite 工业化演进**（骨骼、Foliage/Voxels/Assemblies/Skinning） | 2024–2025，Epic[^36^][^17^][^37^] | 把 Nanite 从刚体推广到植被/动画 | 体素化远景+部件微实例化+骨骼驱动剔除界限 | 引擎级，仅作路线参照 | **高**：定义了功能演进顺序 |
| 4 ★ | **D3D12 Work Graphs + Mesh Nodes** | 2024.3 正式 / 2024.7 预览，Microsoft+AMD[^4^][^7^] | 剔除-绘制间的 CPU 往返与管线切换 | 计算节点在 GPU 上直接启动 mesh shader PSO | 需 SM6.8+Agility SDK，Vulkan 无对等物 | **高**（DXIL 路径）/ 中（Vulkan 路径） |
| 5 | **End-to-End Compressed Meshlet Rendering** | 2024，CGF（Mlakar 等）[^8^] | 簇几何不解压直接渲染，省显存带宽 | GPU 解码器+amplification shader 视锥/锥剔除，剔除数据仅 3B/簇 | 需重写顶点获取路径，中等 | **高**：P4 压缩阶段的论文蓝本 |
| 6 | **Performance Comparison of Meshlet Generation Strategies** | 2023，JCGT（Jensen 等）[^203^] | 簇形状如何影响剔除与光栅效率 | 系统对比聚类策略，提炼两条生成原则 | 纯离线，结论可直接采用 | **高**：P0 的簇质量依据 |
| 7 | **Real-time Meshlet Decompression / Towards Practical Meshlet Compression** | 2025 C&G / 2024 VMV[^205^][^26^] | 簇级实时解压放进渲染管线 | 拓扑/顶点分离编码，warp 级并行解码 | 中等，依赖 subgroup 操作 | 中高：P4 备选方案 |
| 8 | **Meshlets and How to Shade Them（纹理空间着色）** | 2022，CGF（Neff 等）[^204^] | 簇粒度着色 atlas 消除 quad 过绘制 | meshlet shading atlas（MSA）+ mesh shader 内剔除 | 中高，改动着色路径 | 中：长期优化项 |
| 9 | **NVIDIA vk_lod_clusters + RTX Mega Geometry** | 2025，nvpro Vulkan 示例[^45^] | 簇 LOD 与硬件光追 BVH 统一 | 光追最优簇化+簇级 BLAS 流送 | 仅 Vulkan RT 用户关心 | 中：与报告 4 的交叉点 |
| 10 | **CuRast：CUDA 软件光栅（十亿级三角形）** | 2026，arXiv[^30^] | 通用 GPU 软件光栅吞吐极限 | 大规模 binning+tile 光栅的 CUDA 实现 | 研究向，CUDA 限定 | 中：P2 软件光栅的参考数据 |
| 11 | **UE 5.7 Nanite 工程优化**（MinLOD 剔除、HZB prime） | 2025，Epic 版本说明[^36^][^37^] | 摄像机切换时 HZB 失效、候选簇内存 | 低分辨率低 LOD 预渲染建 HZB；跳过子簇 | 小，属于剔除细节 | **高**：P3 阶段直接可抄 |
| 12 | **Parallel Dense-Geometry-Format Topology Decompression（EG 2025）** | 2025，Eurographics Short[^26^] | 稠密几何拓扑的 GPU 并行解压 | 拓扑流 warp 并行解码 | 中等 | 中：与 #5/#7 同族 |

基线文献（必读背景，不计入上表）：Haar & Aaltonen《GPU-Driven Rendering Pipelines》（SIGGRAPH 2015，实例→簇两级 GPU 剔除与索引压缩的工业原型，PS4 上遮挡深度生成约 600µs）[^46^]；Burns & Hunt《The Visibility Buffer》（JCGT 2013，每像素只存三角形索引+实例 ID、最少 4 字节的延迟着色）[^47^]；Karis/Stubbe/Wihlidal《A Deep Dive into Nanite Virtualized Geometry》（SIGGRAPH 2021，全部术语与管线定义的来源）[^14^]；Karis《The Journey to Nanite》（HPG 2022，记录被否决方案——MAPS 化简、几何影像、三角形光追、纹理空间 GBuffer 缓存等——及其否决理由，对避免 rurix 走弯路价值极高）[^31^]。

---

## 3. 逐项技术分析：问题、算法与实现代价

### 3.1 离线构建：层级 cluster DAG 是全部后续工作的地基

**要解决的问题。** Nanite 式系统的本质是把"逐网格 LOD"换成"逐簇 LOD"：网格被切分为约 128 三角形的小簇，相邻簇分组合并并简化（保持组边界不裂缝），简化结果再切簇，递归至根，形成一棵簇的有向无环图（DAG）；运行时沿 DAG 选取一个"切"（cut），使屏幕上任意区域都以恰好足够细的簇渲染，从而彻底消灭手工 LOD、交叉淡入与 popping[^9^][^14^]。离线构建要同时保证三件事：简化误差可量化（每簇存一对自身/父级误差包围球）、簇间无裂缝（组边界顶点锁定）、以及簇形状有利于剔除（紧致、法向集中）。

**核心算法与 2022–2026 进展。** 工业界已在 meshoptimizer 中把这条流水线产品化：`meshopt_buildMeshletsFlex/Spatial` 分簇（0.25 版起分"光栅最优"与"光追最优"两种簇化器）、`meshopt_partitionClusters` 分组、`meshopt_simplify*` 保边界简化，再由 **clusterlod.h** 单头库串成递归构建；zeux 用它在 16 线程、约 55GB 内存下 3 分半钟构建完 16.4 亿三角形、36.1GB 的 Zorah 场景（修复稀疏访问 memset 热点之前为 7–9 分钟）[^45^][^201^]。学术侧，Jensen 等（JCGT 2023）系统对比了簇化策略对渲染性能的影响，给出"顶点局部性优先、簇界对齐剔除粒度"的两条生成原则，实测于 NVIDIA 硬件[^203^]；Neff 等（CGF 2022）从着色侧证明簇粒度本身就是 reduce overdraw 的资源（MSA）[^204^]。压缩方向见 3.5 节。

**实现代价与对 rurix 的意义。** 这是七个方向中**代价最低、风险最小**的一块：完全离线、纯 CPU、有成熟开源库可 FFI（meshoptimizer 为 C 接口）或逐文件移植为 Rust crate。建议 rurix 直接以 `clodBuild` 风格 API 为模板定义自己的构建接口（输入索引+位置、回调输出簇组），先不碰属性（法线/UV 在 P2 阶段再进入顶点获取路径）——zeux 的最小示例仅十余行[^45^]。验收标准也清晰：任意输入网格输出簇 DAG + 每簇误差包围球 + 统计信息（簇数、层级数、平均三角形数）。

### 3.2 运行时可见性：两级剔除、LOD cut 与两阶段 HZB

**要解决的问题。** GPU-driven rendering 的目标是把"画什么"的决定完全留在 GPU：实例级视锥/遮挡剔除 → 簇级视锥/背面锥/遮挡剔除 → 索引/绘制参数压缩成 `DrawIndirect` 参数，全程无 CPU 回读[^46^]。Nanite 在此基础上增加了 LOD cut 判定——每个簇并行地检查"自身误差不可感知（<1 像素）且父级误差可感知"，满足条件的簇恰构成 DAG 上的一个切，无需簇间通信即可得到全局一致的 LOD 选择[^9^]。遮挡剔除采用**两阶段 HZB**：第一遍用上一帧的深度金字塔（配上一帧变换）剔除并光栅化大概率可见的簇，用结果重建当前帧 HZB，第二遍只对第一遍被剔除者重测，通常只占主遍的一小部分[^16^]。

**核心算法与 2022–2026 进展。** 这一子领域 2022 年后的增量主要来自 Epic 的工程迭代而非新论文：UE 5.7 引入 `r.Nanite.Culling.MinLOD`（剔除时跳过子簇，降低候选簇内存与剔除耗时，默认开启）与 **HZB prime**——摄像机剪辑（camera cut）导致上一帧 HZB 失效时，先以低分辨率+LOD 偏置（甚至用光追远场几何）预渲一版带深度偏置的 HZB，显著降低该类帧的剔除代价[^37^]。剔除数据结构方面，Mlakar 等（CGF 2024）证明每簇只需**额外 3 字节**即可支撑视锥+背面锥剔除（8bit 半径缩放、8bit 锥角、8bit 锥顶点偏移）[^8^]，这与 Nanite 的簇记录大小量级一致，可直接照抄到 rurix 的簇记录布局。调度侧，D3D12 Work Graphs Mesh Nodes（2024）把"剔除计算→mesh shader 绘制"折叠进单张 GPU 工作图，消掉中间 barrier 与 CPU 干预，AMD RX 7000 已有日零驱动[^4^][^7^]；但它是 D3D12 专属且 2024 年仍为预览，Vulkan 无线索，建议 rurix 将其列为 DXIL 后端的可选加速路径而非主路径。

**实现代价。** 剔除本身是 2–3 个计算 pass（实例、簇、压缩），工作量与场景簇数线性相关；Bevy 实现在 2240×1260 下对 44.7 万簇级场景的第一遍剔除约 0.49ms、第二遍 0.11ms、HZB 重建 0.03ms[^9^]。对 rurix 的真正成本在**数据准备**：所有实例与簇记录须常驻 GPU 结构化缓冲，scene 侧需要一个"GPU scene 展平"步骤——这恰好是 rurix 已有 mesh pass 的进化方向而非新系统。HZB 需要一条约 10 级的降采样链（可用单 pass SPD 或逐层 compute），属于标准件。

### 3.3 光栅化路径：software raster vs 硬件光栅 vs mesh shader

**要解决的问题。** 当三角形缩小到亚像素级，固定功能光栅器的 2×2 quad 调度、三角形 setup 与 binning 开销成为主矛盾：覆盖 1 像素的三角形也要付 4 条像素通道的成本，约 75% 浪费[^33^]。Nanite 的答案是按屏幕尺寸分箱：小三角形走**计算着色器软件光栅**（每线程一三角形，scanline 遍历，深度与负载打包成 64 位整数用 `atomicMax` 写入可见性缓冲），大三角形走硬件间接绘制；Epic 按 profile 调定的分箱阈值通常引述为边长约 32 像素级[^32^]，社区实测与教学资料均复现了 3–4 倍于硬件路径的亚像素吞吐优势[^33^][^41^]。

**核心算法与 2022–2026 进展。** 软件光栅的现代化身是"单 pass 无 binning 的原子写"模式——compute_rasterizer、ComputeRaster、webgpu-compute-rasterizer 等开源项目演示了同一模式的不同变体，其中 compute_rasterizer 在 RTX 3090 上实时渲染 20 亿点[^32^]；学术侧的 CuRast（2026）则给出了 CUDA 下十亿级三角形软件光栅的系统实现与参照数据[^30^]。**mesh shader 路径**（VK_EXT_mesh_shader / D3D12 mesh shader）以 amplification+mesh shader 替代计算剔除+间接绘制，是另一条可行路线，NVIDIA/AMD 均有成套最佳实践；但值得注意两个事实：其一，Bevy 的 Nanite 复刻在 wgpu 上**完全用计算管线**实现并达到实用性能，证明 mesh shader 不是 Nanite 类系统的必要条件[^9^]；其二，Epic 自己在 UE 5.7 的方向恰是"用骨骼矩阵算更紧的剔除界限，让植被回归固定功能光栅器"，因为可编程分箱随材质数增多会成为性能问题[^17^]。换言之，**mesh shader 是优化项而非地基**。

**实现代价与路径建议。** 软件光栅需要：64 位原子整数（Vulkan 即 `VK_KHR_shader_atomic_int64`，D3D12 即 SM6.6 int64 atomics——这也是 Nanite 放弃 GTX 10 系与 RDNA1 之前硬件的原因）[^33^]、一个每簇变换→组共享顶点→逐三角形光栅化的 compute shader、以及与硬件路径共享的 64 位 VisBuffer 写出格式。工作量集中在边界覆盖规则与细长三角形的 scanline 效率，参考实现公开可得[^32^]。对 rurix 的建议：**P2 先做软件光栅+现有硬件间接绘制的双路径，mesh shader（VK_EXT_mesh_shader）作为 P3 之后的可选第三路径**；DXIL 后端若已有，则 Work Graphs Mesh Nodes 留作后续性能实验[^4^]。

### 3.4 着色耦合：可见性缓冲、材质分类与纹理空间着色

**要解决的问题。** 虚拟化几何与材质系统必须解耦：光栅化阶段每像素只写"哪个簇的哪个三角形最近"，材质求值整体推迟——这样过绘制不再浪费着色，几何 pass 输出仅为每像素 8 字节，并且任何材质模型都能叠加在几何之上[^14^][^47^]。Nanite 的 64 位像素负载为 **30 位深度 + 27 位簇索引 + 7 位三角形索引**，以无符号整数原子比较同时完成深度测试与可见性记录[^33^]；随后材质分类阶段按屏幕 tile 把像素分桶到材质，逐材质 compute 解析出 GBuffer[^16^]。

**核心算法与 2022–2026 进展。** Epic 在 UE 5.0–5.1 做了两件直接影响 rurix 设计的事：一是把 Nanite 硬件光栅器重构为材质 shader 形式，为 WPO/像素深度偏移/遮罩材质打开**可编程光栅框架**的大门（5.1 主题）[^20^][^42^]；二是把材质分类从读 64 位 VisBuffer 改为经 MaterialResolve 直接取 16 位材质槽 ID，带宽节省 3/4，RTX 2070S 上该 pass 提速 40% 并默认开启（`r.Nanite.ClassifyWithResolve`）[^42^]——这说明**材质解析应设计为独立的窄缓冲 pass，而非从 VisBuffer 反查**。学术侧，Neff 等（CGF 2022）的 meshlet shading atlas 证明簇粒度纹理空间着色可进一步消掉 quad 过绘制，属于画质/性能长期优化项[^204^]。对极小实例（整网格只剩一簇），Nanite 用 12×12 视角方向预烘焙的可见性缓冲 imposter 直接注入，避免重复实例占内存[^14^]。

**实现代价。** VisBuffer 本体只是 render graph 里一张与屏幕等大的 R64Uint transient 纹理；材质分类需要一张 tile→材质列表的间接分派结构（逐 tile 原子计数+前缀和，标准 GPU 分桶模式）。rurix 已有材质/shader 编译底座，P2 阶段的最小版本可以只做**单一 debug 材质 + 材质 ID 直写**，把分桶留到 P3；UE 的数据表明这不是性能关键路径的先决条件[^42^]。

### 3.5 工业实现对照：Epic 演进顺序、Rust 实现与压缩/流送

**Epic 的官方演进顺序就是 rurix 最好的路线参照**：5.0 刚体 Nanite（不支持半透明/遮罩/变形/骨骼）[^20^]；5.0–5.1 压缩与可编程光栅框架[^42^]；5.3 曲面细分试验性开启但被社区证实"远未到生产状态"[^5^]；5.5 引入 Nanite 骨骼网格体（社区实测视角相关 LOD 在关节处可能过度化简，需用 `Nanite.MaxPixelsPerEdge` 与关闭 ViewDependentCulling 调参）[^27^]；5.7（2025-11-12）发布实验性 **Nanite Foliage**，由三件套构成——**Nanite Voxels**（远景树冠等聚合体体素化，无交叉淡化）、**Nanite Assemblies**（部件微实例化，把部件簇编码进总装层级、剔除时动态求变换；最大单树资产磁盘占用 3.5GB→29MB，单视角流送内存 36MB→2.7MB）、**Nanite Skinning**（十万骨骼 GPU 更新约 0.1ms，以骨骼界限替代 WPO 的保守包围盒）[^36^][^17^]。这条"刚体→可编程光栅→骨骼→聚合体"的次序，本质是**按剔除界限的可预测性排序**——rurix 应原样遵循。

**开源对照实现。** Bevy 0.14 的虚拟几何是**对 rurix 价值最高的单一参照物**：Rust 语言、wgpu 抽象（与 Vulkan 世代一致）、帧结构 12 个 pass 与 Nanite 一一对应、并公开了 RTX 3080 上的逐 pass 计时（总几何阶段 ~2.78ms，其中 VisBuffer 光栅 1.85ms 为主项）[^9^]。Unity 侧的 NADE 项目把 Nanite 拆成可复现的 8 个阶段（含软件光栅阈值、64 位原子写、imposter），并列出四个可参照的软件光栅开源仓库[^32^]。压缩与流送方面，Nanite 的磁盘格式约 1M 输入三角形→11MB（Kraken 5 级）[^10^]，显存侧按 **128KB 页**管理、首页（含 LOD 顶层）常驻、按剔除反馈流送[^16^]；CGF 2024 的端到端压缩 meshlet 渲染[^8^]与 C&G 2025 的实时 meshlet 解压[^205^]给出了解压进 GPU 的论文蓝本；zeux 则提醒一个现实数字——**簇 DAG 构建是内存密集型**（Zorah 需 55GB 级别内存、稀疏数据结构是前提）[^45^]。

**小结。** 把 3.1–3.5 拼起来，rurix 的目标系统形态已经唯一确定：离线 clusterlod 式构建 → 运行时两级剔除+LOD cut → 双路径光栅写 64 位 VisBuffer → 窄材质解析+延迟着色 →（后期）分页流送与压缩。下一节把它逐模块映射到 rurix 的现有底座。

---
## 4. 映射到 rurix 现有 RHI / render graph / mesh pass

### 4.1 现状差距评估

rurix 现有的是"渲染底座"（mesh/raster 通路、Vulkan/DXIL 双后端、render graph/RHI 与资源状态管理），缺的是**几何数据的生产方式与可见性决策的位置**：几何仍以逐网格 LOD+CPU 提交 draw 为主，剔除与 LOD 在 CPU，可见性与材质耦合在 GBuffer。Nanite 类改造的本质不是加几个 pass，而是把三个职责搬家——**LOD 决策从资产管线搬进 GPU（逐簇）、剔除从 CPU 搬进 GPU（两级）、可见性记录从 GBuffer 搬进 64 位整数缓冲**。好消息是三件事都发生在 rurix 已有抽象之内：计算 pass 与间接绘制是 render graph 的一等公民，transient 资源生命周期由 graph 管理，RHI 已隔离后端差异。

需要预支的底层能力只有两类：其一，**Vulkan 特性暴露**——`VK_KHR_shader_atomic_int64`（VisBuffer 的 64 位原子写，硬性需求）、subgroup 操作（压缩/解码与分桶加速，软性需求）、buffer device address 或大型 storage buffer 绑定（簇记录池，依现有 RHI 风格二选一）、`VK_EXT_mesh_shader`（仅 P3 后可选路径）；其二，**GPU scene 展平**——把实例表、簇记录池、层级数据做成 GPU 常驻结构化缓冲，由 scene 变更增量更新，这是全部剔除 pass 的输入契约[^16^][^34^]。

### 4.2 模块映射表

| Nanite 类系统模块 | rurix 落点 | 新增 / 复用 | 关键接口契约 |
|---|---|---|---|
| 离线 cluster DAG 构建 | 新 crate `geom-build`（FFI meshoptimizer 或移植 clusterlod） | **新增** | 输入：索引+位置；输出：簇记录数组+DAG 层级+误差包围球+页打包[^45^][^201^] |
| 实例/簇两级剔除 | 新 `gpu_cull` pass（compute ×2–3），挂 graph.rs | **新增** | 读：实例表+簇记录+HZB；写：可见簇列表+间接参数缓冲[^46^] |
| LOD cut 判定 | 并入簇剔除 pass（每簇一次自身/父级误差判定） | **新增** | 簇记录内含自/父误差包围球；屏幕误差阈值（默认 1px）[^9^] |
| HZB 深度金字塔 | graph 内 transient mip 链 + 降采样 pass | **新增**（半标准件） | 输入上一帧 VisBuffer 深度位或场景深度；约 10 级[^16^] |
| 软件光栅（小三角形） | 新 `sw_raster` pass（compute，每线程一三角形） | **新增** | 簇顶点变换→group shared→scanline→`atomicMax` 写 VisBuffer[^32^][^33^] |
| 硬件光栅（大三角形） | 复用现有 mesh pass + `DrawIndirect`（或 mesh shader 可选路径） | 复用改造 | 与 SW 路径共享同一 VisBuffer 写出格式[^34^] |
| 可见性缓冲 | render graph transient `R64Uint` 全屏纹理 | **新增** | 负载格式 depth30+cluster27+tri7[^33^] |
| 材质分类/解析 | 新 `mat_classify`+`mat_resolve` compute pass，接现有延迟着色 | **新增** | tile 分桶+16 位材质槽 ID 窄缓冲（参照 UE5 的 40% 提速路径）[^42^] |
| 流送与页池 | 新 `streaming` 子系统 + RHI 异步拷贝队列 | **新增**（P4 才做） | 128KB 页、首页常驻、剔除反馈驱动请求队列[^16^] |
| 场景图→GPU scene | 现有 scene/mesh 系统增量展平 | 改造 | 实例表与变换的 GPU 常驻镜像[^34^] |

### 4.3 不适合照搬的部分

同样重要的是明确**不做什么**。其一，不要先做 mesh shader 主路径：Bevy 证明纯计算管线可达实用性能，而 mesh shader 在 Vulkan 侧的生态一致性（多厂商扩展行为、subgroup 大小差异）会增加调试面[^9^]；其二，不要先做曲面细分/位移——Epic 三年仍未把它推过生产门槛[^5^]；其三，不要为"Nanite 全功能矩阵"设计超前抽象：骨骼（Assemblies/Skinning 思路[^17^]）、植被体素化、半透明支持都以 P4 之后的独立迭代处理；其四，RTX Mega Geometry 式的簇-光追统一[^45^]与报告 4（RT 方向）耦合，等 rurix 的 RT 闭环稳定后再评估。

---

## 5. 分阶段落地路线图

严格遵循"**先做 meshlet + culling，再做 cluster streaming**"的约束：P0–P2 得到的是一个"全量常驻内存的 Nanite 核心"，P3 得到"完整可见性系统"，P4 才把数据搬到页上。每个阶段都有独立可演示、可回归的验收标准。

![Nanite 类管线架构与 rurix 模块映射](images/r1_nanite_pipeline.svg)

![分阶段路线图](images/r1_roadmap.svg)

| 阶段 | 目标（可演示的里程碑） | 主要改动点 | 依赖 | 验收标准 | 预估 |
|---|---|---|---|---|---|
| **P0** 离线 meshlet 化 | 任意 glTF → 簇包（单层，无 DAG） | 新 crate `geom-build`；簇化+包围球/锥+序列化格式 v0 | meshoptimizer FFI[^45^][^201^]；簇生成原则[^203^] | 100% 输入网格成功转换；簇三角形数 128±20%；包围数据正确性单测 | 3–4 周 |
| **P1** GPU 剔除+单层 LOD | 全 GPU 驱动渲染：CPU 只发一次 draw | `gpu_cull` 两 pass（实例/簇：视锥+背面锥）+索引压缩+`DrawIndirect` | rurix graph/RHI 间接绘制[^46^] | 与 CPU 蛮力剔除结果逐簇一致；剔除 pass <0.5ms@50 万簇 | 3–4 周 |
| **P2** 可见性缓冲+软件光栅 | 亚像素三角形正确着色、无 quad 浪费 | VisBuffer transient+`sw_raster` compute+HW/SW 分箱+debug 材质解析 | `VK_KHR_shader_atomic_int64`[^33^]；阈值调参[^32^] | 与硬件光栅参考渲染 PSNR/逐像素 diff 通过；SW/HW 三角形计数可视化（类 `r.Nanite.ShowStats`） | 4–5 周 |
| **P3** 层级 DAG+两阶段 HZB | 逐区域 LOD 无缝过渡；遮挡剔除生效 | DAG 层级载入+LOD cut 判定+HZB mip 链+第二遍补漏剔除 | P0 构建器升级分组简化[^45^]；两阶段算法[^16^]；UE5.7 MinLOD/PrimeHZB 细节[^37^] | 误差≤1px 的屏幕误差度量；LOD 过渡无可见 popping（TAA 下）；遮挡场景帧率提升可测 | 4–5 周 |
| **P4** cluster 流送+压缩 | 超显存场景（>10 亿三角形）可运行 | 128KB 页池+请求队列+异步 IO（DirectStorage 式）+GPU 解压 | 分页格式[^16^]；压缩蓝本[^8^][^205^] | 页命中率/颠簸监控；超内存场景稳定帧率；流送无可见缺页（低 LOD 兜底常驻） | 5–6 周 |

**阶段间的设计不变量**要提前冻结，避免返工：簇记录布局（P0 定，含剔除用的 3 字节锥/球压缩字段[^8^]）、VisBuffer 位格式（P2 定）、GPU scene 实例表契约（P1 定）。流送（P4）对 P0 的打包格式有反向约束——簇须按"空间局部性+LOD 层级"聚页——因此 P0 的序列化格式 v0 就要预留页表字段，但 P4 之前全部页标记为常驻即可[^16^]。

**人力与并行性。** P0 与 P1 可由两人并行（离线/运行时解耦）；P2 的软件光栅是全项目最"算法密集"的部分，建议安排有 compute 优化经验者并预留调参时间；P3 的难点在 DAG 构建质量（裂缝、误差度量），zeux 的博客记录了真实场景中会遇到的数据规模问题（稀疏 memset、线程不均衡），应提前阅读[^45^]；P4 的难点是 IO 与解压的延迟隐藏，与平台存储栈耦合最深。

---

## 6. 必须新增的清单：数据结构、shader 阶段、资源布局、验证方法

### 6.1 数据结构（CPU 侧与磁盘格式）

| 结构 | 内容 | 规模估算 | 备注 |
|---|---|---|---|
| `ClusterRecord` | 顶点/索引在池中的偏移、三角形数、剔除球（中心+8bit 半径缩放）、法向锥（轴+8bit 角+8bit 顶点偏移）、自身/父级 LOD 误差球、材质槽 ID、页号 | 32–48 B/簇[^8^] | P0 冻结布局；剔除字段对齐 Mlakar 的 3B 方案 |
| `ClusterGroup / DagNode` | 子簇索引区间、组边界标志、层级深度 | 16–24 B/节点 | P3 启用 |
| `InstanceRecord` | 变换、包围球、指向网格 DAG 根的句柄、LOD 偏置 | 64–96 B/实例 | GPU scene 展平的单位[^34^] |
| `PageTable / PageEntry` | 128KB 页：常驻标志、引用计数、LOD 层级、磁盘偏移 | 16 B/页 | P0 预留、P4 启用；首页（DAG 顶层）永驻[^16^] |
| `VisBuffer 像素` | R64Uint：depth30+cluster27+tri7 | 8 B/像素 | P2 冻结[^33^] |

### 6.2 Shader 阶段（全部 compute，除注明外）

| Pass | 输入 → 输出 | 线程映射 | 新增于 |
|---|---|---|---|
| `instance_cull` | 实例表+上一帧 HZB → 可见实例列表 | 1 线程/实例 | P1 |
| `cluster_cull`（含 LOD cut） | 簇记录+可见实例+HZB → 可见簇列表+光栅分箱计数 | 1 线程/簇（subgroup 压缩） | P1/P3 |
| `compact_draw_args` | 分箱计数 → `DispatchIndirect`/`DrawIndirect` 参数 | 单线程组前缀和 | P1 |
| `hzb_build` | 深度 → 10 级 mip 链 | 逐层或 SPD 单 pass | P3 |
| `sw_raster` | 可见小簇 → VisBuffer（atomicMax） | 组内先逐顶点变换入 shared，再 1 线程/三角形 scanline | P2[^32^] |
| `hw_raster`（图形管线） | 可见大簇 → VisBuffer（PS 写 64 位） | 标准 mesh/vertex 管线+间接绘制 | P2（P3 后可换 mesh shader[^4^]） |
| `mat_classify` | VisBuffer → tile×材质分桶列表 | 1 线程/像素块 | P2（简化版）/P3（完整版）[^42^] |
| `mat_resolve` | 分桶列表+材质表 → GBuffer | 1 线程/像素，按桶间接分派 | P2 |
| `page_stream`（含解压） | 页请求队列 → GPU 页池 | warp 级并行解码 | P4[^8^][^205^] |

### 6.3 资源布局（render graph 视角）

全部运行时资源都是 graph 内 **transient**：VisBuffer（R64Uint 全屏）、HZB mip 链（R32Float×10 级）、可见簇/实例列表与间接参数（storage buffer，容量=簇数上限）、tile 分桶缓冲（tiles×材质上限）。跨帧持久资源只有三类：**GPU scene 缓冲**（实例表、簇记录池、DAG 层级——scene 系统持有，graph 导入）、**HZB 历史**（或按 UE5.7 思路每帧重建+失效时 prime[^37^]）、**页池**（P4，固定显存预算的 ring/空闲链表）。注意把"剔除读上一帧 HZB"建模为 graph 的跨帧读依赖，rurix 的资源状态系统需要为此支持历史帧资源的延迟释放语义。

### 6.4 验证方法（每阶段可回归）

| 验证维度 | 方法 | 通过标准 | 阶段 |
|---|---|---|---|
| 构建正确性 | 簇化单测：三角形守恒、边界锁定、包围球包含性 | 100% 用例通过 | P0 |
| 剔除正确性 | 与 CPU 蛮力（逐簇视锥/背面锥）逐帧对拍 | 逐簇一致；遮挡剔除允许保守误差但不得漏可见簇 | P1/P3 |
| 光栅正确性 | SW 光栅 vs 硬件光栅同场景逐像素 diff + PSNR | 深度一致、覆盖差异仅在抗锯齿容差内 | P2 |
| LOD 质量 | 屏幕空间误差度量可视化（热力图）+ popping 检测（逐帧 SSIM） | 误差≤1px；相机路径下无可见跳变[^9^] | P3 |
| 性能 | 逐 pass GPU 计时（对标 Bevy 公布的 12 pass 分解[^9^]）、SW/HW 三角形计数、剔除率 | 几何阶段占帧预算 <30%；遮挡密集场景提速可测 | P1–P3 |
| 流送 | 页命中率、请求队列深度、缺页率监控 | 超内存场景无持续颠簸；缺页有低 LOD 兜底 | P4 |
| 压力场景 | Stanford Bunny ×3000（Bevy 基准[^9^]）、Zorah 级公开 glTF 场景[^45^] | 达到上述全部标准且内存有界 | P3–P4 |

---

## 7. 风险与缓解

**DAG 构建质量风险（最高危）。** 层级简化中的裂缝与误差度量失真会直接表现为运行时 popping 与孔洞，且调试困难。缓解：P0 即引入簇界包含性与误差单调性单测；P3 先用 meshoptimizer 的成熟分组/简化组合，不自研简化器；保留 zeux 记录的稀疏数据结构与线程均衡教训[^45^]。**软件光栅的边角风险**：细长/退化三角形、浮点 snapping、与硬件深度精度一致性（建议 VisBuffer 深度位直接用与硬件深度缓冲相同的量化）是已知坑位，NADE 与 Bevy 的公开代码可作为对拍参照[^32^][^9^]。**硬件碎片化风险**：`VK_KHR_shader_atomic_int64` 排除了部分老硬件与早期移动 GPU[^33^]，rurix 需在 RHI 能力查询层暴露该特性并保留"传统逐网格 LOD+GBuffer"回退路径——该回退路径同时是验证 Nanite 路径正确性的对拍基准，不应视为浪费。

**范围蔓延风险**是最后也是最容易被低估的：Nanite 的全部附加能力（骨骼、植被体素、半透明、曲面细分、光追统一）都有明确的 Epic 落地时点与前置条件[^36^][^5^][^17^]，rurix 应严格按本报告的 P0–P4 顺序推进，把每阶段验收作为进入下一阶段的门禁。待 P4 稳定后，再按报告 2–7 的对应路线接入 Lumen 类 GI（报告 2）、虚拟阴影（报告 3）与光追（报告 4）——Nanite 的 VisBuffer 与簇层级恰恰是那些系统的输入而不是障碍。

---

[^4^]: https://gpuopen.com/learn/work_graphs_mesh_nodes/work_graphs_mesh_nodes-intro/
[^5^]: https://forums.unrealengine.com/t/nanite-tessellation-in-ue-5-3/1198899
[^7^]: https://devblogs.microsoft.com/directx/d3d12-mesh-nodes-in-work-graphs/
[^8^]: https://onlinelibrary.wiley.com/doi/full/10.1111/cgf.15002
[^9^]: https://jms55.github.io/posts/2024-06-09-virtual-geometry-bevy-0-14/
[^10^]: https://gamedev.stackexchange.com/questions/198454/how-does-unreal-engine-5s-nanite-work
[^14^]: https://advances.realtimerendering.com/s2021/Karis_Nanite_SIGGRAPH_Advances_2021_final.pdf
[^16^]: https://www.thecandidstartup.org/2023/04/03/nanite-graphics-pipeline.html
[^17^]: https://dev.epicgames.com/documentation/unreal-engine/nanite-foliage
[^20^]: https://cdn2.unrealengine.com/nanite-for-educators-and-students-2-b01ced77f058.pdf
[^26^]: https://coburggraphicslab.github.io/publications/
[^27^]: https://forums.unrealengine.com/t/nanite-on-skeletal-mesh-shows-extremely-low-poly-from-certain-angles-in-ue-5-5/2573728
[^30^]: https://arxiv.org/html/2604.21749v2
[^31^]: https://www.highperformancegraphics.org/slides22/Journey_to_Nanite.pdf
[^32^]: https://github.com/Unfinished-B/NADE-Unity-Virtual-Geometry-Engine-Demo/blob/main/How_To_Build_Nanite.md
[^33^]: https://unbiased-gamer.com/the-mental-model-for-unreal-engines-nanite-virtualized-geometry-and-cluster-culling/
[^34^]: https://developer.arm.com/community/arm-community-blogs/b/mobile-graphics-and-gaming-blog/posts/mali-and-unreal-engine-s-nanite-enabling-the-future-of-mobile-graphics
[^36^]: https://www.unrealengine.com/news/unreal-engine-5-7-is-now-available
[^37^]: https://tomlooman.com/unreal-engine-5-7-performance-highlights/
[^41^]: https://cs418.cs.illinois.edu/website/text/nanite.html
[^42^]: https://dev.epicgames.com/documentation/zh-cn/unreal-engine/unreal-engine-5.0-release-notes?application_version=5.0
[^45^]: https://zeux.io/2025/09/30/billions-of-triangles-in-minutes/
[^46^]: http://advances.realtimerendering.com/s2015/aaltonenhaar_siggraph2015_combined_final_footer_220dpi.pdf
[^47^]: https://jcgt.org/published/0002/02/04/
[^201^]: https://github.com/zeux/meshoptimizer/blob/master/demo/clusterlod.h
[^203^]: https://jcgt.org/published/0012/02/01/paper-lowres.pdf
[^204^]: https://onlinelibrary.wiley.com/doi/abs/10.1111/cgf.14474
[^205^]: https://doi.org/10.1016/j.cag.2025.104292
