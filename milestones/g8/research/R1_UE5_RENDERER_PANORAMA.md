# G8-R1 — UE5 渲染器能力全景与前沿论文（深度调研）

> **所属**：G8 计划定稿档（`milestones/g8/`）——本文是 [G8_CAPABILITY_MATRIX.md](../G8_CAPABILITY_MATRIX.md) 与 [G8_PLAN.md](../G8_PLAN.md) 的调研输入之一。
>
> **与既有调研的关系**：`渲染器调研/` 七报告 + 设备化报告（2026-07，G5 期）为冻结输入，本文不重复其基础内容，只覆盖其后的增量——UE 5.4→5.8 演进全景与 2023–2026 前沿论文。
>
> **调研基准日**：2026-08-02。**调研方式**：联网深度调研（26 次检索，多源交叉验证，全部结论附来源 URL）。
>
> **纪律**：零编号占用——本文不新设任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用既有编号；不改写 G5/G6/G7 已收口结论。

## 目录

1. 结论与判定标准 
2. UE 5.4—5.8 渲染演进时间线 
3. 虚拟化几何、光照、阴影与时域重建 
4. 材质、虚拟纹理、大世界与流送 
5. 完整场景渲染和后处理能力 
6. 2023—2026 前沿论文与生产化状态 
7. Rurix G8 能力缺口总矩阵 
8. 参考来源

---

# 一、结论与判定标准

本报告进行了 26 次联网检索，并重点阅读 Epic 官方文档、UE 发布说明、SIGGRAPH Advances、GDC Vault、ACM/HPG 论文和 NVIDIA/AMD 官方资料。

截至 2026 年中，最新正式版本是 **UE 5.8，发布于 2026-06-17**；Epic 称其为 UE5“最后一个计划中的主要版本”，但保留发布 5.9 的可能性。[S1][S2]

因此，“UE5 级别渲染器”不能仅定义为拥有 Nanite、Lumen、VSM 和 TSR，而应定义为：

> 在现代显式图形 API 上，具备完整 GPU-driven 场景表达、可虚拟化的几何/纹理/阴影资源、动态直接与间接光照、生产级材质和透明系统、完整场景类型、后处理、资产构建、诊断和可扩展硬件能力分层。

建议 G8 采用以下验收层级：

- **核心等价**：实现 UE5 核心架构能力，而非逐像素复刻。
- **功能闭环**：运行时、编译器、资产构建和调试工具必须全部闭环。
- **可降级**：HWRT、mesh shader、bindless 不可用时有明确 fallback。
- **可生产化**：具备缓存失效、流送预算、PSO/shader permutation 控制、故障诊断和离线验证器。
- **Vulkan 主线**：所有架构首先映射到 Vulkan；DXIL/PTX 作为并行能力而非设计基准。

---

# 二、UE 5.4—5.8 渲染演进时间线

| 版本 | 关键渲染变化 | 对“UE5 级”定义的影响 |
|---|---|---|
| 5.4（2024） | Nanite 动态 tessellation/displacement；Spline Nanite 默认启用；TSR History Resurrection、Has Pixel Animation 和诊断视图 | 微多边形系统需要可编程变形；时域系统需要材质语义协作，而不只是 motion vector |
| 5.5（2024） | MegaLights Experimental；Nanite Skeletal Mesh Experimental；Substrate Beta；Path Tracer Production-Ready；Lumen HWRT 面向 60 Hz；Vulkan RT 与 DX12 功能趋于对齐 | GPU-driven 范围扩展到骨骼几何和大量随机采样灯光；Vulkan RT 成为一等路径 |
| 5.6（2025） | Lumen HWRT CPU/GPU 优化；Fast Geometry Streaming Experimental；VSM、Nanite 小实例、渲染器并行化和 GPU Profiler 改进；TSR Thin Geometry Detection | “功能存在”升级为稳定 60 Hz、低卡顿、可分析、可扩展 |
| 5.7（2025） | Substrate Production-Ready；MegaLights Beta；Nanite Foliage、Voxel、Assemblies、Skinning；程序化植被编辑器 | 完整虚拟几何必须覆盖动态植被和微实例，不再只服务静态刚体 |
| 5.8（2026） | MegaLights Production-Ready；Lumen Lite；Nanite 植被成熟；shader 去重与 PSO precache 改进；实验 Toon Shader、Fog Screen Space Scattering | UE5 终态基线已包含低端 GI 档位、随机直接光照和全面 shader/PSO 工程化 |

5.4：[S3]；5.5：[S4][S5]；5.6：[S6]；5.7：[S7]；5.8：[S1][S2]。

## 对 Rurix 的前置能力要求

- **A 语言/编译器**
 - 材质可见的历史有效性、像素动画、位移边界等语义。
 - mesh/task、compute raster、RayQuery 和完整 RT pipeline 的统一 IR。
 - specialization/static switch 与 permutation 成本分析。
- **B 运行时/RHI**
 - Vulkan bindless、buffer device address、indirect-count、timeline semaphore。
 - AS 异步构建、compaction、refit、streaming 与 fallback。
 - PSO 缓存和异步编译。
- **C 引擎库**
 - 质量档位和 feature-level 系统。
 - GPU profiler、可视化诊断、帧预算和 residency 管理。
- **D 工具/资产**
 - cook/派生数据缓存、PSO 收集、shader 去重和回归场景。

---

# 三、虚拟化几何、光照、阴影与时域重建

## 3.1 Nanite 完整能力面

### 能力矩阵

| 能力 | UE 5.8 状态 | 技术含义 | 硬件/API 前置 |
|---|---|---|---|
| Cluster LOD DAG | 成熟 | cluster 分组简化、无裂缝边界、GPU 选择、遮挡剔除、按页流送 | DX12 SM6 或 Vulkan 对等能力；64 位原子对 VSM 尤其重要 |
| HW/SW 双光栅 | 成熟 | 大三角形走硬件光栅，小/亚像素三角形走 compute 软件光栅，输出 visibility buffer | subgroup、原子、indirect dispatch；mesh shader 并非唯一实现路径 |
| Tessellation/displacement | 5.4 后支持动态可编程位移 | 根据屏幕像素密度动态 diced triangles；WPO 先作用于基网格，displacement 后作用于细分顶点 | 高吞吐 compute、细粒度内存分配、材质位移采样 |
| WPO | 支持但受限 | cluster 各自剔除，必须声明或夹紧最大位移，否则 bounds 与剔除不可靠 | 材质元数据、速度输出、缓存失效传播 |
| Skeletal Mesh | 5.5 Experimental，5.7/5.8 扩展 | GPU skinning、动画 LOD；官方资料仍列有 morph target 等限制 | skin cache、动态 bounds、RT AS 更新 |
| Foliage | 5.7/5.8 专项体系 | Preserve Area、Nanite Voxel、Assemblies、Skinning；建议用真实几何代替 masked cards | 微实例、骨骼/风场、体素化和专用压缩 |
| 压缩/磁盘格式 | 核心能力 | 磁盘与 GPU 内存格式分离；属性量化/bit-pack；分页流送、GPU 友好解码 | GPU 解压、异步 I/O、residency feedback |
| Fallback Mesh | 自动生成 | HWRT、Path Tracer 或不支持 Nanite 的路径使用；精度由 triangle percentage/relative error 控制 | 独立 fallback 构建和 RT BLAS |
| 材质 | Opaque/Masked；不支持普通 Translucent Nanite | programmable raster、material binning、visibility→GBuffer | bindless 或大资源表、GPU material dispatch |

官方 Nanite 文档：[S8][S9]；SIGGRAPH 2021 Deep Dive：[S10]；GDC 2024 GPU-Driven Materials：[S11]。

Nanite 的生产关键并不是“meshlet”，而是以下完整闭环：

1. 离线 DAG 构建和无裂缝简化；
2. 磁盘页、内存页、root-resident 数据布局；
3. GPU 实例/节点/cluster 两阶段剔除；
4. occlusion feedback 后的 persistent culling；
5. HW/SW raster 分流；
6. programmable raster/material binning；
7. 页请求、异步 I/O、解压与迟到页面处理；
8. WPO、骨骼、植被和 RT fallback 的一致性。

### Rurix 判断

Rurix 已有 meshlet、分组简化 DAG、两级剔除、VisBuffer 和 HW/SW 对拍，说明基础算法方向正确。G8 最大缺口更可能在：

- 可编程几何变形；
- 压缩及正式磁盘格式；
- 骨骼和植被；
- RT representation/fallback；
- residency、页迟到和生产级调试。

### 对 Rurix 的前置能力要求

- **A**
 - `displacement`/tessellation kernel 阶段或等价 compute-dicing 语义。
 - WPO 最大位移、动态性、velocity、shadow invalidation 属性。
 - 量化解码、bitfield、subgroup 和可控无界资源访问。
 - skinning/deformer kernel ABI。
- **B**
 - GPU 页表、反馈缓冲、异步 copy/decompress、稀疏 residency。
 - indirect dispatch chain、64 位原子、跨队列 ownership。
 - BLAS fallback、refit/rebuild 和 streamed geometry RT 路径。
- **C**
 - 动态 tessellation、WPO bounds、骨骼 Nanite、foliage assembly。
 - programmable raster/material binning。
 - fallback mesh 与主几何误差联动。
- **D**
 - cluster DAG builder、压缩器、磁盘格式版本化。
 - fallback/RT proxy 生成、植被 assembly 和位移烘焙工具。
 - DAG、页、误差、裂缝和 residency 可视化。

---

## 3.2 Lumen 完整能力面

Lumen 是分层追踪系统，而不是单一 GI 算法：

- **Screen Trace**：优先复用屏幕可见数据。
- **SWRT**：Detail Tracing 使用单 mesh distance field；Global Tracing 使用 Global Distance Field。
- **Surface Cache**：离线生成 cards，运行时缓存材质/光照参数。
- **HWRT**：三角形级交点，支持 skinned mesh；反射可选 Surface Cache lighting 或 Hit Lighting。
- **Screen Probe Gather**：主视图 diffuse GI。
- **World-Space Radiance Cache**：远距离/粗尺度光照和透明体积；更新有 probe budget。
- **Reflections**：屏幕、SW/HW trace 与 surface cache/hit lighting 混合。
- **Translucency GI**：透明表面主要使用较低质量 radiance cache；HWRT 高质量模式通常只改善最前层镜面反射。
- **Far Field**：近场 RT 场景半径外用便宜的远场表示继续 GI/reflection。
- **5.8 Lumen Lite**：irradiance field + probe occlusion；官方称相对 Lumen High Quality 最多约快 2 倍，目标包含 Switch 2 60 fps。[S2]

官方文档明确指出，HWRT 场景超过约 **100,000 instances** 时更新成本显著，不能把 RTX 4070 Ti 的静态小场景结果外推到大世界。[S12]

官方文档：[S12][S13]；SIGGRAPH 2022：[S14]。

### 对 Rurix 的前置能力要求

- **A**
 - distance-field tracing intrinsic 或高效软件遍历代码生成。
 - Surface Cache card capture 的材质可重定向编译。
 - ray hit 上完整材质求值与简化缓存求值两种 ABI。
- **B**
 - mesh/global distance-field 资源和增量更新。
 - TLAS instance 分层、近/远场 AS、异步 AS 构建。
 - 多级 radiance cache 的 GPU allocator/hash/grid。
- **C**
 - screen trace → SWRT/HWRT → far field 的 tiered tracing。
 - Surface Cache、card placement、coverage 检测。
 - diffuse GI、reflection、translucency GI 的共享 radiance cache。
 - 低成本 irradiance-field 档位。
- **D**
 - distance field/card 离线生成。
 - Surface Cache 覆盖、probe age、ray source、泄漏和更新预算可视化。

---

## 3.3 MegaLights

MegaLights 在 5.5 为 Experimental，5.7 为 Beta，5.8 Production-Ready。[S4][S7][S15]

其本质是 **随机采样直接光照**：

- 每像素不再遍历所有灯；
- 通过灯光采样、引导、RT/VSM 阴影查询、时空复用和去噪，使灯数对每像素成本的影响更弱；
- 支持 textured area light、soft shadow、light function、媒体纹理和体积阴影；
- 首选 RT shadow，以保持近似固定成本；VSM shadow 可直接使用 Nanite 几何，但有明显逐灯成本；
- 它只解决直接光照，不替代 Lumen GI/reflection。

SIGGRAPH 2025 官方课程给出了 sampling、ray guiding、几何不匹配、translucency、volumetrics、sample shading 和 denoising 的完整范围。[S15]

### 对 Rurix 的前置能力要求

- **A**：reservoir/随机采样库、wave cooperative sampling、RayQuery 与材质 sample shading。
- **B**：灯光 GPU 数据库、alias table/light tree、RT/VSM 阴影统一接口。
- **C**：时空灯光 reservoir、可见性复用、去噪、透明和体积接入。
- **D**：灯光采样概率、reservoir age、方差、shadow source 和噪声热图。

---

## 3.4 Virtual Shadow Maps

VSM 的官方规格包括：

- 每张虚拟阴影图概念分辨率 **16K×16K**；
- page 为 **128×128** texels；
- directional light 使用多级 clipmap；
- local light 使用各自虚拟页；
- 通过主视图深度分析 mark 所需页；
- 页跨帧缓存；
- light、transform、WPO、skeletal deformation、LOD streaming 等触发失效；
- Nanite 遮挡下的失效可跳过，非 Nanite 通常不能；
- directional Nanite views 和 local-light Nanite views 分批处理；
- 非 Nanite 几何受支持，但 raster/LOD 和失效成本更高。

API 要求：DX12 SM6.6 atomics 或 Vulkan `VK_KHR_shader_atomic_int64`；官方列出 PC、PS5、Xbox Series、Apple Silicon M2+ 等支持范围。[S16]

### 对 Rurix 的前置能力要求

- **A**：shadow view 批量化、64 位原子、WPO/skin invalidation 元数据。
- **B**：物理页池、页表、LRU/age、clipmap scroll、批量 multi-view。
- **C**：directional clipmap、local-light pages、cache invalidation、非虚拟几何 caster。
- **D**：页驻留、失效原因、cache hit、clipmap level 和 redraw cost 可视化。

---

## 3.5 TSR

生产级 TSR 需要：

- jitter、history reprojection、motion vector、depth/velocity disocclusion；
- shading rejection；
- reactive/translucency 处理；
- flicker temporal analysis；
- spatial anti-aliaser；
- history resolution 与锐度控制；
- dynamic resolution；
- history resurrection；
- 材质声明 Pixel Animation；
- WPO/透明速度处理；
- thin geometry 专项检测；
- 可视化和自动化画质回归。

5.4 加入 History Resurrection、Has Pixel Animation 和 TSR Visualize；5.6 增强 foliage/hair 薄几何稳定性。官方支持 D3D11/12、Vulkan、Metal、PS5、Xbox；高端路径通常配合 SM6。[S3][S17]

透明对象默认往往没有可靠 velocity，因此 TSR 无法正确重投影所有叠层，这是“实现一个 TAA”无法覆盖的生产问题。

### 对 Rurix 的前置能力要求

- **A**：`pixel_animation`、WPO velocity、translucency velocity、history-validity 材质属性。
- **B**：统一 motion-vector contract、动态分辨率和历史资源生命周期。
- **C**：history resurrection、shading rejection、thin geometry、透明响应遮罩。
- **D**：TSR 分阶段视图、序列画质测试、ghosting/flicker 指标。

---

# 四、材质、虚拟纹理、大世界与流送

## 4.1 Substrate、材质图与 shader permutation

Substrate 从 5.2 Experimental、5.5 Beta 到 5.7 Production-Ready。[S4][S7][S18]

其核心不是“更多 shading model”，而是：

- 以物理参数化 **Slab/closure** 表达介质；
- closure 可混合、分层；
- 材质图经简化、量化、打包后进入自适应每像素存储；
- 可重定向到 raster、Lumen 和 path tracer；
- 根据平台预算降级 closure；
- 支持 per-pixel topology，但复杂度直接影响存储、带宽和 shader 数量。

UE 静态参数的每种组合都会生成新材质编译结果；Epic 提供 Material Analyzer 去除冗余 static override。5.8 又强化 shader dedup 和 PSO precache，Epic 称 Fortnite shader 数减少 68%，但该数字属于特定项目结果，不应视为通用收益。[S2][S19]

### 对 Rurix 的前置能力要求

- **A**
 - closure 类型系统、组合规则、能量守恒和单位检查。
 - closure lowering 到 GBuffer/VisBuffer、RT hit、path tracer。
 - specialization 常量、静态分支和 permutation key。
- **B**
 - bindless descriptor、material table、PSO cache。
- **C**
 - adaptive material payload、material binning、平台 closure simplifier。
 - skin、hair、cloth、clear coat、anisotropy、SSS、thin transmission。
- **D**
 - 材质图编译器、实例系统、permutation analyzer、shader DDC。

---

## 4.2 SVT、RVT、World Partition、HLOD 和流送

UE 的 Virtual Texturing 分两条路径：

- **SVT**：纹理离线 cook 后从磁盘按页流送；适合大型美术纹理、UDIM、lightmap。
- **RVT**：GPU 运行时生成 texel；适合地形分层、程序材质、贴花和地形/物体混合。
- 大世界可把 RVT 低 mip 烘焙为 SVT，高 mip 继续运行时生成。[S20]

World Partition 将单一持久世界划分为网格 cell，按 streaming source 加载；HLOD builder 为远景生成代理，并形成独立加载层。[S21]

“UE5 级流送”还包括：

- texture mip 与 VT page 两种路径；
- Nanite/mesh page；
- geometry/physics 状态异步创建销毁；
- I/O 优先级、预算、预取、退化和迟到资源；
- GPU feedback 与 CPU/world streaming source 协作。

### 对 Rurix 的前置能力要求

- **A**：虚拟采样 intrinsic、缺页/feedback-safe 采样语义。
- **B**：统一 residency manager、异步 I/O/copy、页池、预算和优先级。
- **C**：SVT、RVT、texture mip、mesh page、scene-cell 和 HLOD 联合调度。
- **D**：VT cooker、HLOD builder、world-cell cooker、streaming trace 和带宽回放。

---

# 五、完整场景渲染和后处理能力

## 5.1 能力全景

| 子系统 | UE5 生产能力 | 主要前置 |
|---|---|---|
| Bloom | 多尺度 Gaussian、Convolution kernel、dirt mask | HDR scene color、FFT/卷积或多级 blur |
| DOF | 实时 cinematic DOF；Path Tracer 可作高质量参照 | circle-of-confusion、散景分类、透明合成顺序 |
| Motion Blur | 基于 velocity，支持 shutter/target FPS | 物体与相机 motion vector、tile/max velocity |
| Auto Exposure | Basic/Histogram/Manual、EV compensation | luminance reduction、时间适应 |
| Local Exposure | Bilateral/Fusion，保护高光阴影细节 | 边缘感知滤波、HDR |
| Tonemap | filmic curve、色彩分级、HDR 输出 | 颜色管理、display transform |
| 透明 | 排序透明、Thin Translucent、Surface Forward Shading、体积透明 | 单独 translucency pass、lighting volume、velocity |
| OIT | 官方资料未能确认 UE 5.8 存在通用生产级 OIT；仍主要依赖排序和专项路径 | 若 Rurix 实现，应选 weighted blended、PPLL 或 moment OIT |
| Volumetric Fog | froxel lighting、局部雾、RT light injection | 3D froxel、temporal reprojection |
| Volumetric Cloud | ray-marched cloud、阴影和大气耦合 | 空间跳跃、天气图、时域降噪 |
| Water | Single Layer Water、反射/折射、几何水体 | 专用 shading、SSR/Lumen/RT、caustic 可选 |
| Hair/Groom | strand/card/mesh；RT hair 仍昂贵且部分实验 | strand culling、深度/coverage、专用 BSDF、AS |
| Skin | preintegrated/SSS；RT 下部分仍借助 raster | profile SSS、屏幕空间扩散 |
| Landscape | 分块 LOD、Nanite landscape、RVT、grass、VSM | heightfield、地形 cook、虚拟纹理 |
| Decal | deferred decal、mesh decal；Nanite 不支持 translucent mesh decal | DBuffer/GBuffer 修改、排序 |
| Niagara GPU VFX | sprite/ribbon/mesh、GPU simulation、fluid；GPU RT collision 为实验 | GPU allocator、indirect draw、async compute |
| Path Tracer | 5.5 Production-Ready；ground truth、电影输出 | 完整 RT pipeline、累积、MIS、材质一致性 |

后处理官方入口：[S22][S23]；透明：[S24]；HWRT 场景支持矩阵：[S25]；Niagara RT collision：[S26]。

## 5.2 硬件/API要求

- 普通后处理、雾、云、水、透明和粒子：现代 Vulkan/DX12 compute、subgroup、FP16、storage image 即可。
- Hair strand、复杂体积和高质量 TSR：强依赖带宽、共享内存和异步计算，但不必强制 HWRT。
- HWRT Lumen、MegaLights RT、Path Tracer、Niagara RT collision：需要 Vulkan RT 或 DXR、AS build、RayQuery/RT shaders。
- UE Vulkan RT 在 5.5 默认启用并宣称趋近 DX12 parity；完整路径还依赖 bindless。[S27]
- Nanite 主路径要求 SM6 级桌面能力；VSM 明确要求 64 位原子扩展。[S16]

## 对 Rurix 的前置能力要求

- **A**
 - fragment/interlock 可选能力、透明材质阶段、volume kernel。
 - callable/closest-hit/any-hit/miss 等完整 RT shader ABI。
 - FP16、wave、ray differential 或纹理 LOD 辅助语义。
- **B**
 - HDR swapchain、颜色空间、异步 compute、3D texture。
 - 完整 RT pipeline/SBT；目前只有 inline RayQuery 不足以构建 path tracer。
- **C**
 - 后处理 graph；透明和 OIT 策略；froxel fog/cloud。
 - water、hair、skin、landscape、decal、GPU particle 渲染器。
 - progressive path tracer 作为材质和光照参照器。
- **D**
 - LUT/颜色管理、groom importer、terrain cooker、Niagara 类 VFX 图。
 - path-traced golden image 与 raster/HWRT 差异测试。

---

# 六、2023—2026 前沿论文与生产化状态

## 6.1 虚拟化几何与 meshlet

1. **Real-Time Ray Tracing of Micro-Poly Geometry with Hierarchical Level of Detail** 
 HPG 2023。将类似 Nanite 的 cluster LOD DAG、量化压缩和每帧 LOD 选择扩展到硬件 RT BLAS 构建；cluster 最大约 256 triangles。[S28]

2. **End-to-End Compressed Meshlet Rendering** 
 Computer Graphics Forum / Eurographics 2024。磁盘和 GPU 内存保持同一压缩格式，在 mesh shader 内即时解压，避免 CPU 解压和额外 GPU 常驻展开数据。[S29]

3. **Towards Practical Meshlet Compression** 
 VMV 2024，Best Paper。使用 generalized triangle strips 和 crack-free attribute quantization；报告相对传统 vertex pipeline 最高 16:1 index compression，15.5M triangles 在 RX 7900 XTX 上解压并渲染约 0.59 ms。[S30][S31]

4. **Nanite GPU-Driven Materials** 
 GDC 2024。虽不是学术论文，但对 programmable raster、raster binning、GPU material shading 和 visibility→GBuffer 的生产实现最有参考价值。[S11]

**判断**：Rurix G8 不宜只复刻 Nanite 2021 数据格式；应评估“压缩态直渲染”和“虚拟几何直接参与 RT AS”的新方向。

---

## 6.2 ReSTIR 家族

- **ReSTIR DI** 已通过 NVIDIA RTXDI 形成可集成 SDK，是当前生产成熟度最高的分支。
- **ReSTIR GI** 在 RTXDI 2.0 加入，用于二级表面的间接漫反射重采样。
- **ReSTIR PT** 在 RTXDI 3.0 加入，将路径级重采样接入 path tracer。
- **A Gentle Introduction to ReSTIR Path Reuse in Real-Time**，SIGGRAPH Course 2023。
- **Conditional Resampled Importance Sampling and ReSTIR**，SIGGRAPH Asia 2023。
- **Area ReSTIR: Resampling for Real-Time Defocus and Antialiasing**，SIGGRAPH 2024。
- **ReSTIR PT Enhanced: Algorithmic Advances for Faster and More Robust ReSTIR Path Tracing**，ACM TOG，2026；论文报告 2—3× 算法加速，并明确目标是更接近 production-ready。[S32]

生产状态应区分：

- ReSTIR DI：可进入 G8 实验/产品路线；
- ReSTIR GI：可作为现有 probe GI 的高质量档；
- ReSTIR PT：适合参照器和长期路线，不能替代 G8 主线 GI；
- reservoir 仍需要 visibility、temporal validity、disocclusion、bias/variance 控制和去噪，SDK 并非即插即用。[S33]

---

## 6.3 辐射缓存与探针 GI

1. **GI-1.0: A Fast and Scalable Two-level Radiance Caching Scheme for Real-time Global Illumination**，arXiv 2023。
2. **Real-Time Rendering of Glossy Reflections Using Ray Tracing and Two-Level Radiance Caching**，SIGGRAPH Asia Technical Communications 2023。
3. **Radiance Caching with On-Surface Caches for Real-Time Global Illumination**，HPG 2024。 
 在 texture/surface space 缓存 primary/secondary hit directional radiance，支持多 bounce、glossy reflection 和多 viewer 共享；论文报告相对比较对象约 5%—10% 质量/速度改善。[S34]
4. UE 5.8 **Lumen Lite** 的 irradiance field + probe occlusion，表明生产引擎仍重视低成本 probe GI，而非全面转向逐像素路径追踪。[S2]

**判断**：Rurix 已有屏幕探针 GI，G8 应补世界空间缓存、表面缓存和多级缓存一致性，而不是立即以神经 GI 替代。

---

## 6.4 神经渲染生产化

### DLSS 4

DLSS 4 于 2025 随 RTX 50 系列推出：

- Multi Frame Generation 最多为每个传统渲染帧生成 3 个附加帧，仅 RTX 50 系列支持该核心模式；
- Super Resolution、Ray Reconstruction 和 DLAA 使用 transformer 模型，并可覆盖更广 RTX 硬件；
- NVIDIA 官方 2025 年资料称已有超过 125 个游戏/应用支持 DLSS 4 MFG，但这是厂商统计。[S35][S36]

对 Rurix 的意义不是实现 DLSS，而是提供标准化输入：

- jitter；
- depth、motion vector；
- exposure；
- reactive/transparency mask；
- disocclusion；
- optical-flow/frame-generation 插件接口；
- UI 与生成帧分离。

### Neural Texture Compression

**Random-Access Neural Compression of Material Textures**，ACM TOG / SIGGRAPH 2023：

- 以每材质小型网络联合压缩多通道 PBR 纹理及 mip；
- 支持随机访问实时解压；
- 论文称可获得两个额外细节层级，即约 16× texels。[S37]

RTX NTC SDK 截至检索为 **v0.9.2 Beta**，最多支持 16 通道；提供 load-time transcode 和 sample-time shader inference。NVIDIA 宣称相对传统 block compression 最多约 8× VRAM 改善，但仍属厂商结果且 SDK 未到 1.0。[S38]

### Neural Radiance Cache

NRC 在 Portal with RTX 等 NVIDIA 路径追踪示例中已可运行：少量间接 ray 作为在线训练样本，网络预测更多 bounce radiance。[S36] 
它已进入展示性生产，但仍存在厂商硬件、训练稳定性、缓存失效和调试成本，不应作为 G8 主 GI 唯一路径。

### RTX Neural Shaders

RTX Neural Shaders 通过 cooperative vectors 在 shader 内运行小型 MLP：

- Vulkan：`VK_NV_cooperative_vector`，官方样例最低 RTX 20 系；
- DirectX：Shader Model 6.9 与 Preview Agility SDK；截至资料时间仍有 preview 警告；
- SDK 使用 Slang 封装 inference/training。[S39][S40]

因此 Rurix 可设计 cooperative-matrix/vector 抽象，但 G8 不应把 NVIDIA 专有扩展设为基础要求。

## 对 Rurix 的前置能力要求

- **A**
 - reservoir 泛型、可微/训练可选语义、cooperative vector/matrix 抽象。
 - FP8/FP16/INT8 数据类型、矩阵布局和量化支持。
- **B**
 - reservoir history、neural weights、在线训练 buffer 生命周期。
 - Vulkan vendor-extension capability negotiation。
- **C**
 - ReSTIR DI 试验路径、两级 radiance cache。
 - upscaler/frame-generation 插件接口。
 - NTC load-time transcode 优先，sample-time inference 后置。
- **D**
 - reservoir/debugger、神经网络资产版本化、训练与质量评估工具。

---

# 七、UE5 渲染器能力 → Rurix 前置要求总表

| UE5 能力 | A 编译器 | B RHI/运行时 | C rurix-render | D 工具/资产 | G8 优先级 |
|---|---|---|---|---|---|
| Nanite 压缩与正式页格式 | bit decode、量化 | 页表、I/O、GPU 解压 | residency/反馈 | DAG/压缩/cooker | P0 |
| WPO/动态 displacement | 位移语义、bounds、velocity | 动态 buffer/失效 | programmable geometry | 位移验证器 | P0 |
| Skeletal/Foliage Nanite | deformer ABI | skin cache、动态 AS | assemblies、wind/skin | 植被 builder | P1 |
| RT fallback geometry | RT shader ABI | BLAS/TLAS 管理 | fallback policy | proxy generator | P0 |
| Lumen SWRT | DF traversal | distance-field 资源 | tiered tracing | DF builder | P0 |
| Surface/Radiance Cache | cache-friendly IR | GPU allocator/hash | cards/probes/cache | coverage 工具 | P0 |
| Lumen HWRT/Far Field | hit material lowering | 多层 AS | near/far tracing | RT 场景诊断 | P1 |
| MegaLights/ReSTIR DI | sampling/reservoir | light DB、RayQuery | 时空复用/去噪 | 方差视图 | P1 |
| 完整 VSM | 原子/multi-view | page pool/cache | clipmap/local pages | 失效视图 | P0 |
| 生产级 TSR | 时域材质语义 | motion contract | rejection/resurrection | 序列回归 | P0 |
| Substrate 类材质 | closure 类型和 lowering | bindless/PSO | adaptive material | 图编译/analyzer | P0 |
| Shader permutation/PSO | specialization 分析 | async PSO cache | feature levels | DDC/precache | P0 |
| SVT/RVT | 虚拟采样语义 | residency manager | VT feedback/render | VT cooker | P1 |
| World Partition/HLOD | — | scene-cell streaming | world scheduler | HLOD/cell builder | P1 |
| 完整透明/OIT | 透明阶段/interlock | fragment storage | sorting/OIT/lighting | overdraw 调试 | P1 |
| 雾/云/水 | volume kernel | 3D texture/async | froxel/cloud/water | profile/editor | P1 |
| Hair/Skin | 专用 BSDF | strand buffer/AS | groom/SSS | groom importer | P2 |
| Landscape/Decal | 专项材质 lowering | tiled resources | terrain/decal renderer | terrain cooker | P1 |
| GPU VFX | particle kernel ABI | GPU allocator/indirect | emitter/render modules | VFX 图编辑 | P2 |
| Path Tracer | 完整 RT stages | SBT/RT pipeline | MIS/累积/去噪 | golden image | P1 |
| DLSS/FSR/XeSS 插件面 | 标准输入 ABI | vendor interop | upscaler adapter | capture/验证 | P1 |
| Neural shading/NTC | coop-vector、低精度 | vendor capability | 实验后端 | model trainer | P3 |

### 附：调研侧实施顺序建议（输入性质）

*本节仅为调研输入，不构成计划裁决；波次裁决以 [G8_PLAN.md](../G8_PLAN.md) 为准。*

1. **P0 架构地基** 
 bindless、统一资源 residency、完整 RT pipeline、材质 closure IR、PSO/permutation、VSM 完整缓存、TSR production contract。

2. **P0 几何和 GI 闭环** 
 Nanite 正式页格式与压缩、WPO/displacement、fallback mesh、SWRT distance field、Surface/Radiance Cache。

3. **P1 场景完整性** 
 SVT/RVT、World Partition/HLOD、透明、地形、体积、水体、MegaLights/ReSTIR DI。

4. **P1 参照与验证** 
 Path Tracer、材质跨路径一致性、golden-image 测试。

5. **P2/P3 扩展** 
 Groom、复杂 GPU VFX、Neural Texture Compression、NRC 和 cooperative-vector 实验。

---

# 八、参考来源

- **[S1]** UE 5.8 发布公告：https://forums.unrealengine.com/t/unreal-engine-5-8-released/2729274 
- **[S2]** State of Unreal 2026：https://www.unrealengine.com/news/state-of-unreal-2026-top-news-from-the-show 
- **[S3]** UE 5.4 Release Notes：https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5.4-release-notes?application_version=5.4 
- **[S4]** UE 5.5 官方发布：https://www.unrealengine.com/en-US/blog/unreal-engine-5-5-is-now-available 
- **[S5]** UE 5.5 Release Notes：https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5-5-release-notes 
- **[S6]** UE 5.6 发布公告：https://forums.unrealengine.com/t/unreal-engine-5-6-released/2538952 
- **[S7]** UE 5.7 发布公告：https://forums.unrealengine.com/t/unreal-engine-5-7-released/2673913/1 
- **[S8]** Nanite Virtualized Geometry：https://dev.epicgames.com/documentation/unreal-engine/nanite-virtualized-geometry-in-unreal-engine 
- **[S9]** Working with Nanite：https://dev.epicgames.com/documentation/unreal-engine/working-with-naniteenabled-content 
- **[S10]** Nanite Deep Dive，SIGGRAPH 2021：https://advances.realtimerendering.com/s2021/Karis_Nanite_SIGGRAPH_Advances_2021_final.pdf 
- **[S11]** Nanite GPU-Driven Materials，GDC 2024：https://media.gdcvault.com/gdc2024/Slides/GDC+slide+presentations/Nanite+GPU+Driven+Materials.pdf 
- **[S12]** Lumen GI and Reflections：https://dev.epicgames.com/documentation/unreal-engine/lumen-global-illumination-and-reflections-in-unreal-engine 
- **[S13]** Lumen Performance Guide：https://dev.epicgames.com/documentation/unreal-engine/lumen-performance-guide-for-unreal-engine 
- **[S14]** Lumen，SIGGRAPH 2022：https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Lumen-Wright%20et%20al.pdf 
- **[S15]** MegaLights，SIGGRAPH Advances 2025：https://advances.realtimerendering.com/s2025/ 
- **[S16]** Virtual Shadow Maps：https://dev.epicgames.com/documentation/en-us/unreal-engine/virtual-shadow-maps-in-unreal-engine 
- **[S17]** Temporal Super Resolution：https://dev.epicgames.com/documentation/unreal-engine/temporal-super-resolution-in-unreal-engine 
- **[S18]** Substrate，SIGGRAPH Advances 2023：https://advances.realtimerendering.com/s2023/index.html 
- **[S19]** Material Analyzer：https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-material-analyzer-tool 
- **[S20]** Virtual Texturing：https://dev.epicgames.com/documentation/en-us/unreal-engine/virtual-texturing-in-unreal-engine 
- **[S21]** World Partition：https://dev.epicgames.com/documentation/unreal-engine/world-partition-in-unreal-engine 
- **[S22]** Post Process Effects：https://dev.epicgames.com/documentation/unreal-engine/post-process-effects-in-unreal-engine 
- **[S23]** Auto/Local Exposure：https://dev.epicgames.com/documentation/en-us/unreal-engine/auto-exposure-in-unreal-engine 
- **[S24]** Lit Translucency：https://dev.epicgames.com/documentation/en-us/unreal-engine/lit-translucency-in-unreal-engine 
- **[S25]** Hardware Ray Tracing：https://dev.epicgames.com/documentation/en-us/unreal-engine/hardware-ray-tracing-in-unreal-engine 
- **[S26]** Niagara GPU RT Collisions：https://dev.epicgames.com/documentation/en-us/unreal-engine/gpu-raytracing-collisions-in-niagara-for-unreal-engine 
- **[S27]** UE 5.5 Vulkan RT/Bindless：https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5-5-release-notes 
- **[S28]** Real-Time Ray Tracing of Micro-Poly Geometry with HLOD，HPG 2023：https://momentsingraphics.de/Media/HPG2023/benthin2023-real_time_ray_tracing_of_micro_poly_geometry_with_hlod-paper.pdf 
- **[S29]** End-to-End Compressed Meshlet Rendering：https://doi.org/10.1111/cgf.15002 
- **[S30]** Towards Practical Meshlet Compression：https://arxiv.org/abs/2404.06359 
- **[S31]** AMD Meshlet Compression：https://gpuopen.com/learn/mesh_shaders/mesh_shaders-meshlet_compression/ 
- **[S32]** ReSTIR PT Enhanced：https://doi.org/10.1145/3804494 
- **[S33]** NVIDIA RTXDI：https://github.com/NVIDIA-RTX/RTXDI 
- **[S34]** On-Surface Radiance Caching，HPG 2024：https://doi.org/10.1145/3675382 
- **[S35]** NVIDIA DLSS 4：https://www.nvidia.com/en-us/geforce/news/dlss-4-multi-frame-generation-out-now/ 
- **[S36]** DLSS 4/NRC 生产应用：https://www.nvidia.com/en-us/geforce/news/125-dlss-4-multi-frame-gen-games-more-announced-computex-2025/ 
- **[S37]** Random-Access Neural Compression of Material Textures：https://doi.org/10.1145/3592407 
- **[S38]** RTX NTC SDK：https://github.com/NVIDIA-RTX/RTXNTC 
- **[S39]** RTX Neural Shaders SDK：https://github.com/NVIDIA-RTX/RTXNS/ 
- **[S40]** D3D12 Cooperative Vector：https://devblogs.microsoft.com/directx/cooperative-vector/

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-02 | 初版：UE5 渲染器深度调研成果落盘（G8 计划定稿档输入材料） |
