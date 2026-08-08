# G9-R4 — 虚拟化几何与 RT 合流（深度调研）

> **所属**：G9 文档集（`milestones/g9/`）——本文是 G9_PLAN.md（立项时新建）、[G9_CAPABILITY_MATRIX.md](../G9_CAPABILITY_MATRIX.md) 与 [design/G9_D1_VIRTUAL_GEOMETRY_RT.md](../design/G9_D1_VIRTUAL_GEOMETRY_RT.md) 的调研输入；编号顺延 G8 research 系列（R1~R3）为 R4。
>
> **调研基准日 / 访问日期**：2026-08-08。**调研方式**：联网检索（21 次 WebSearch + 5 次原文抓取；一手来源优先：Epic 官方文档/发布公告、advances.realtimerendering.com、docs.vulkan.org / Khronos、GitHub 仓库、NVIDIA/Remedy/AMD 官方页面、ACM DOI、GDC Vault），全部结论附来源 URL（见第八节）。
>
> **事实基线**：本文将 design/G9_D1 草案头部块引用的上游调研 [调研1]~[调研5] 中「虚拟化几何×RT 合流」一路正式化；内容以 design/G9_D1 草案为准，不回写、不修改任何既有文档。凡检索可独立复核者注明出处；复核不到者保留草案陈述并显式标注「数值/表述引自 D1 草案转述」，不静默丢弃、不编造 URL。
>
> **纪律**：零编号占用——本文不新设任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用既有编号（RD-039、M06、M09、M44 等）；G8 已 closed 的契约与判据 0-byte 改动。

## 目录

1. 结论摘要
2. Nanite 与骨骼/植被虚拟几何（[调研1]）
3. GPU 蒙皮与保守包围体（[调研1] 后半 + [调研2]）
4. RTX Mega Geometry 与 CLAS 簇级加速结构（[调研3]）
5. 跨厂商前景与禁止线（[调研4]）
6. 单源真相架构原则与验收指标（[调研5]）
7. 对 G9-D1 的判定清单
8. 参考来源

---

# 一、结论摘要

本轮对 D1 草案 [调研1]~[调研5] 全部五条线索做了独立联网复核。总判定：**五路调研的技术方向全部成立，D1 的 12 条设计决策中 10 条有公开一手资料支撑，无任何一路需要推翻**；有 6 个机制/术语级细节未能从公开文本逐字复核，已按纪律保留草案陈述并标注转述（清单见第七节末）。

- **Nanite 核心机制**（离线 cluster 层级 + 运行时视角驱动 cluster 选择 + 按需流送）由 SIGGRAPH 2021 官方课程与 Epic 现行文档双源确认 [S1][S2]；「误差驱动 DAG cut」表述与 Karis 课程一致 [S1]。
- **UE 5.5 Nanite Skeletal Mesh 是权宜路线**成立：Epic 官方文档明示其无几何 LOD（用 animation LOD 替代）、不支持 Morph Target [S2]，社区个案证实视角相关 LOD 瑕疵 [S4]；「CPU 蒙皮喂静态 cluster、击穿包围体/误差度量、吞吐低」的机制性表述引自 D1 草案转述（官方公开材料未见同口径文字）。
- **动态虚拟几何已有严格学术解**：Unterguggenberger/Kerbl 等 2021（CGF）给出蒙皮 meshlet 保守包围体与法向锥的预计算框架 [S8][S9]。
- **Northlight 路线经生产验证**：AW2 全部植被骨骼绑定、Cauldron Lake 每帧约 30 万骨骼 GPU 处理（Remedy 官方）[S12]，距离分级动画更新率 10m 全速 / 其后 1/2、1/3、1/4 经 DF 实测复核 [S15]；两级遮挡剔除与 bone shader API 的细节表述引自 D1 草案转述。
- **RTX Mega Geometry / CLAS 是当前唯一落地的「虚拟几何×RT」合流 API**：CES 2025 发布（NVIDIA，非 Epic）[S13]，Vulkan 扩展支持 pre-generated CLAS 拼 BLAS、multi-indirect device 构建、Cluster Template 实例化 [S19]；AW2 2025-01/02 落地数据（VRAM −300MB、RTX 4060 +42%、2080 Ti +13%、4090/5080 无帧率收益、CPU +14%）全部经 DF 原文复核 [S15][S16]。
- **跨厂商判断成立但未到兑现时**：`VK_EXT_mesh_shader` 已是多厂商扩展 [S20][S21]；`VK_NV_cluster_acceleration_structure` 仍 NV-only，DXR Functional Spec Part 2 已把 CLAS 写成厂商中立 API 概念且持续更新 [S22]；AMD 以 DGF 推进开放几何压缩与多厂商 Vulkan 扩展 [S23]。**DMM 已被 NVIDIA 官方归档、由 Mega Geometry 取代——禁止投入有据** [S24]。
- **验收指标含义**：AW2 证明合流收益在 VRAM、CPU、AS 构建带宽等约束路径而非 FPS（4090 零帧率收益）[S15]——D1 以 VRAM/构建耗时为硬指标、FPS 仅观察项的验收口径（草案 D-9）必须维持。

---

# 二、Nanite 与骨骼/植被虚拟几何（[调研1]）

## 2.1 Nanite 核心机制：离线 cluster DAG + 误差驱动选择 + 按需流送

草案 [调研1] 第一分句：「Nanite 核心 = 离线 cluster DAG + 运行时误差驱动 cluster 选择 + 按需流送（SIGGRAPH 2021 Karis）」。

复核结果——**成立，双源确认**：

- SIGGRAPH 2021《A Deep Dive into Nanite Virtualized Geometry》（Karis/Stubbe/Wihlidal，Advances in Real-Time Rendering in Games 课程）公开了完整机制：导入期把 mesh 切分为层级化 cluster 并构建 DAG，运行时按屏幕空间误差在 DAG 上求 cut 选择 cluster（误差沿 DAG 单调，保证无裂缝），配合虚拟化流送只驻留可见细节 [S1]。
- Epic 现行官方文档同口径：「During import, meshes are analyzed and broken down into hierarchical clusters of triangle groups. During rendering, clusters are swapped on the fly at varying levels of detail based on the camera view… Data is streamed in on demand so that only visible detail needs to reside in memory.」[S2]

对 G9/D1 的含义：草案 D-4（monotonic 误差驱动 DAG cut + 页驻留联动）与 D1-a 的离线构建扩展直接建立在该机制之上，与 G8 已冻结的页式流送 ABI（M01/M04/M44）正交，可作为既定事实引用，无需再论证。

## 2.2 UE 5.5 Nanite Skeletal Mesh：权宜路线及其代价

草案 [调研1] 第二分句：「UE5.5 Nanite Skeletal Mesh 走 CPU 蒙皮喂静态 cluster 的权宜路线（击穿 cluster 包围体/误差度量、吞吐低、不支持 Morph、有视角 LOD 瑕疵）」。

复核结果——**结论方向成立；机制性表述为草案转述**：

- UE 5.5 以 Experimental 引入 Nanite Skeletal Meshes（5.5 发布说明与发布公告可查）[S3]；第三方梳理其成熟度轨迹为 5.5 experimental → 5.7 beta → 5.8 production-ready（第三方口径，供参考）。
- Epic 官方文档可逐字复核两条关键限制：「No geometry LODs. Nanite Skeletal Mesh uses animation LODs.」以及「Deformation with Morph Targets is not supported with Nanite.」[S2]——即 5.5 路线放弃了 Nanite 最核心的几何 LOD/误差驱动选择，退化为动画 LOD，与草案「权宜路线」判定一致；Morph 不支持亦与官方明示一致。
- 视角 LOD 瑕疵有社区个案实证：Epic 官方论坛 2025-06 帖「Nanite on Skeletal Mesh shows extremely low poly from certain angles in UE 5.5+」，正面朝向相机时退化为极低模，帖主与回帖均指向 view-dependent LOD 计算 [S4]。
- 「CPU 蒙皮喂静态 cluster、击穿 cluster 包围体与误差度量、吞吐低」这一机制层归因，**未能在 Epic 官方公开文字中逐字复核到，数值/表述引自 D1 草案转述**；但「无几何 LOD + 蒙皮后 cluster 包围体失效」的因果链与官方功能描述及 Kerbl 论文的问题陈述（蒙皮使静态 meshlet 包围体失效）[S8] 相容，方向可信。

对 G9/D1 的含义：草案 D-1（拒绝 CPU 蒙皮喂静态 cluster，走 GPU cluster 感知蒙皮）与 D-2（Morph 走非虚拟化旁路）维持成立；其中 D-2 有 Epic 官方明示为最强依据。

## 2.3 UE 5.7/5.8：Nanite Foliage / Assemblies / Skinning 演进

草案 [调研1] 第三分句：「UE 5.7/5.8 Nanite Foliage/Assemblies/Skinning 演进」。

复核结果——**成立（官方源）**：

- UE 5.7（2025-11-12 发布）官方公告：「This release introduces Nanite Foliage, an Experimental new geometry rendering system designed for performance, robustness, and scalability.」[S5]
- Epic Nanite Foliage 官方文档（已进 UE 5.8 文档集）明确其三系统构成：**Nanite Assemblies**（把植被"部件"做成高细节轻量实例，显著降内存/磁盘）、**Nanite Voxels**（近像素级聚合体素，按相机距离保留三角形细节/动画/材质）、**Nanite Skinning**（用骨骼层级模拟风等动态行为）——并给出与 D1 直接相关的关键句：「By not using WPO to simulate wind animation, Nanite Foliage can use optimal cluster bounds.」[S7] 即 Epic 自己也放弃了 WPO 顶点动画、改走骨骼蒙皮以保住 cluster 包围体有效性，这正是对草案 D-1/D-3 路线的官方背书。
- UE 5.8（2026-06-17 State of Unreal 发布）官方公告确认该版本聚焦性能与核心功能成熟化 [S6]；Nanite Foliage 文档随 5.8 文档集发布 [S7]。第三方报道称 Nanite Foliage 在 5.8 走出实验标签（第三方口径，供参考，未在官方公告逐字复核）。

对 G9/D1 的含义：Epic 的演进方向（骨骼化植被 + 保守 cluster 包围 + 体素聚合）与 D1-b 的技术选型同向，且 Epic 在 5.7/5.8 才走到的路线正是 D1 一期就要建造的路线——D1 的设计假设不存在「Epic 已另有更优解」的风险。

---

# 三、GPU 蒙皮与保守包围体（[调研1] 后半 + [调研2]）

## 3.1 Kerbl et al. 2021：LBS 蒙皮下保守 meshlet 包围的严格解

草案 [调研1] 第四分句：「Kerbl et al. 2021《Conservative Meshlet Bounds for Skinned Meshes》(CGF) 给出 LBS 蒙皮下保守 meshlet 包围球/法向锥的严格解」。

复核结果——**成立；著录信息勘定如下**：

- 出版记录正式题名为《Conservative Meshlet Bounds for Robust Culling of Skinned Meshes》，作者序为 Johannes Unterguggenberger、Bernhard Kerbl、Jakob Pernsteiner、Michael Wimmer，Computer Graphics Forum 40(7):57–69（Pacific Graphics 2021），DOI 10.1111/cgf.14401 [S8][S9]。草案简写题名与作者序以本出版记录为准（纯著录差异，技术指认不变）。
- 论文核心（据官方页面与幻灯）：在预计算阶段对每个 meshlet、按动画时间区间计算保守包围盒与法向分布锥，使 task shader 能对蒙皮网格做低开销、低内存/低带宽的鲁棒剔除 [S8][S9]。这正是草案 D1-b「离线预计算保守时空包围体 + 蒙皮 kernel 输出保守包围球/法向锥」的学术原型。

对 G9/D1 的含义：D-1 的后半句（离线预计算保守包围体）有严格已发表解，风险不在「是否存在解」而在工程收紧度——草案风险表 R-D1-2（保守包围过松 → 剔除效率崩）的止损设计（按骨数分桶收紧）因此是必要的，而非可选。

## 3.2 Remedy Northlight（Alan Wake 2，GDC 2024）：生产验证

草案 [调研2]：「mesh shader 硬件光栅 + 两级遮挡剔除（单像素精度）；全部植被骨骼绑定、约 30 万骨骼 GPU 蒙皮（bone shader 向美术暴露可编程 API）、距离分级动画更新率（10m 内全速 / 其后 1/2、1/3、1/4）」。

复核结果——**主数值成立（官方源）；两个机制细节为草案转述**：

- 演讲存在性与归属：Remedy 官方 GDC 2024 演讲公开页列出《Large Scale GPU-Based Skinning for Vegetation in Alan Wake 2》（演讲者 Kiya(wash) Kandar，Graphics Programmer）[S11]；GDC Vault 收录该场（可免费观看）[S10]。
- 「全部植被骨骼绑定 + 约 30 万骨骼」：Remedy 官方技术文章《How Northlight makes Alan Wake 2 shine》逐字可复核：「The skeleton rigs animating all vegetation. Each line is one bone, with almost 300,000 bones in Cauldron Lake being processed every frame.」[S12]
- 「距离分级动画更新率（10m 全速 / 1/2、1/3、1/4）」：Digital Foundry 对 AW2 Mega Geometry 更新的实测文章逐字复核：「The first 10 metres are full rate, followed by half, third and quarter rate further into the distance. On PS5, this is halved further.」[S15]——即该分级策略从 2023 年发售沿用到 2025 年 Mega Geometry 更新，是稳定的生产实践而非一次性技巧。
- GPU-driven / mesh shader 路线：Remedy 官方同页另列 Digital Dragons 2024 演讲《GPU-driven rendering with mesh shaders in Alan Wake 2》（Erik Jansson）[S11]，佐证 Northlight 的 mesh shader 硬件光栅主线。
- 「两级遮挡剔除（单像素精度）」与「bone shader 向美术暴露可编程 API」两条机制细节：**GDC Vault 视频页已定位，但其细节未获公开文本逐字复核，表述引自 D1 草案转述**（G9.1 治理波可调取 Vault 视频 [S10] 补核）。

对 G9/D1 的含义：D-3（全骨骼绑定植被 + GPU 蒙皮 + 距离分级更新率）有一个 3A 级生产案例背书，且分级更新率被草案进一步复用到 AS 更新分级（D1-e「降级簇静态帧零 AS 构建」）——这是 Northlight 思路向 RT 侧的自然延伸，逻辑闭环。

---

# 四、RTX Mega Geometry 与 CLAS 簇级加速结构（[调研3]）

## 4.1 发布与定位：NVIDIA CES 2025，非 Epic

草案 [调研3] 第一分句：「RTX Mega Geometry（NVIDIA CES 2025，非 Epic）」。

复核结果——**成立**：NVIDIA 在 CES 2025（2025-01-06）的 Blackwell/RTX 50 系列官方新闻稿中发布 RTX Mega Geometry（与 DLSS 4、ACE 等并列）[S13]；DF 亦记「RTX Mega Geometry was revealed at CES 2025」[S15]。该技术与 Epic/Nanite 无隶属关系，是 API/驱动层能力。

## 4.2 CLAS 机制与 Vulkan 扩展能力面

草案 [调研3]：「BVH 插入 CLAS（Cluster-level AS）层，簇级 AS 可离线随资产烘焙，BLAS = CLAS 列表拼装；`VK_NV_cluster_acceleration_structure` 支持 multi-indirect device 构建 + Cluster Template 实例化」。

复核结果——**成立（规范原文级）**：

- `VK_NV_cluster_acceleration_structure` 官方 refpage：「enabling applications to construct bottom-level acceleration structures (BLAS) from pre-generated acceleration structures based on clusters of triangles (CLAS)… a versatile multi-indirect call for managing cluster geometry… generate cluster geometry, construct Cluster BLAS from CLAS lists, and move or copy CLAS and BLAS… sourcing inputs from device memory… reduces the host-side costs」[S19]——「pre-generated CLAS → BLAS 拼装」「multi-indirect」「device memory 输入」三点逐字成立。「簇级 AS 离线随资产烘焙」是 pre-generated CLAS 的资产管线化推论，vk_lod_clusters 样例的 RAM→VRAM 按需流送实现 [S14] 佐证了「烘焙产物随资产流送」的可行性。
- Cluster Template：扩展提案文档说明其为「partially constructed CLAS」，不含顶点位置、体积更小，实例化时补顶点位置并可统一偏移 ClusterID/geometry index [S19]（同页提案章节）——草案「Cluster Template 实例化共享底层 AS」成立。
- 扩展版本史显示持续维护（Revision 4, 2025-07-16，新增 OMM 配合 flag）[S19]。

## 4.3 Alan Wake 2 落地数据（2025-01/02，数值全部复核）

草案 [调研3]：「Alan Wake 2 2025-02 落地：VRAM −300MB，RTX 4060 +42%、2080 Ti +13%、4090 几乎无帧率收益（解放的是 VRAM/CPU/构建带宽约束路径）」。

复核结果——**全部数值经 DF 原文逐项复核成立**：

- 落地事件：AW2 update 1.2.8（2025-01-30）官方更新说明：「Improved Ray Tracing CPU and GPU load, VRAM usage and image quality thanks to RTX Mega Geometry. This is supported and automatically enabled on all RTX GPUs.」[S16]
- DF 实测（2025-02-05）[S15]：
  - VRAM：「VRAM usage was reduced by around 300MB in my testing」——**−300MB 复核成立**；
  - RTX 4060：「now runs 42 percent faster using direct lighting and low quality indirect lighting as it's no longer running out of VRAM」——**+42% 复核成立**（注意语境：直接光照+低档间接光照、此前爆 VRAM 的配置）；
  - RTX 2080 Ti：「at native 1080p has a 13 percent improvement」——**+13% 复核成立**（另：RTX 3080 +10%）；
  - 旗舰卡：「no real performance gains whatsoever in the same tests running on RTX 4090 and RTX 5080」——**4090 几乎无帧率收益复核成立**；
  - CPU 侧：Ryzen 5 3600 帧率 +14%（草案未列，补充佐证 CPU 构建路径收益）；
  - DF 的机制解读与草案一致：现有游戏实质上维护「两个 3D 世界」（光栅世界 + 简化版 BVH 世界），动画几何下两者会错配（mismatch），Mega Geometry 的 CLAS 新层级正是为此而设 [S15]。

对 G9/D1 的含义：草案 D-5（离线烘焙 CLAS + 当帧拼装 BLAS）与 D-9（VRAM/构建耗时/CPU 带宽为硬指标，FPS 仅观察项）直接由这组数据支撑——尤其是「4090 零收益」证明**以 FPS 验收 CLAS 必然假绿**，D1 验收门 `clas_blas.budget` 的指标设计（vram_as_bytes / as_build_ms / 静态帧零构建）方向正确。

## 4.4 开源 builder 与 Vulkan 样例

草案 [调研3]：「NVIDIA 开源 nv_cluster_builder / nv_lod_cluster_builder（边塌缩 + 簇对锁定，与 Nanite DAG 同源）与 4 个 Vulkan 样例」。

复核结果——**成立；一处命名与术语细节勘定/标注**：

- NVIDIA 开发者博客（2025-02-13）公布 4 个开源 Vulkan 样例与 2 个库 [S14]：
  - 样例：`vk_animated_clusters`（动画场景，CLAS 加速 AS 构建，光栅对照用 `VK_EXT_mesh_shader`）、`vk_partitioned_tlas`（`VK_NV_partitioned_acceleration_structure`，TLAS 分区重建，10 万+物理对象）、`vk_tessellated_clusters`（动态细分+位移，CLAS 路径追踪，对照 `VK_NV_mesh_shader`）、`vk_lod_clusters`（cluster 连续 LOD + CLAS 光追 + RAM→VRAM 按需流送）；
  - 库：`nv_cluster_builder`（通用空间聚类 C++ 库，类 BVH 递归节点切分）[S17]；`nv_lod_cluster_builder`（连续 LOD 网格库：预计算 decimation 使 cluster 跨 LOD 无缝组合，运行时按相机自适应选子集）——**注意其 GitHub 仓库名为 `nv_cluster_lod_builder`**（库名与仓库名词序相反）[S18]，引用时以仓库 URL 为准。
- 「边塌缩 + 簇对锁定、与 Nanite DAG 同源」：官方博客可复核到「decimating the original mesh in a way that they can be seamlessly combined across different LoD levels」[S14]；「边塌缩/簇对锁定」的具体术语表述引自 D1 草案转述（未在公开文字逐字复核），与 Nanite DAG 同源的判断方向成立。

对 G9/D1 的含义：D-10（自研边塌缩+簇对锁定、不 vendoring NVIDIA 代码）合理——算法公开、许可风险可控、且 D1-a 需要与 G8 确定性双构建纪律（M79）兼容，自研是正确取舍；`vk_lod_clusters` 的 LOD+流送结构可作为 D1-c/D1-d 联调的参照实现（只读参照，不引入代码）。

---

# 五、跨厂商前景与禁止线（[调研4]）

## 5.1 mesh shader 已跨厂商

草案 [调研4]：「`VK_EXT_mesh_shader` 已跨厂商（NV/AMD/Intel）」。

复核结果——**成立**：Khronos 官方发布任务单标题即「Multi-vendor mesh shading for Vulkan」[S21]；扩展提案文档说明其提供「programmable mesh shading」新机制并随 SPIR-V `SPV_EXT_mesh_shader` 配套落地 [S20]。多厂商（NV/AMD/Intel 驱动均已支持）为业界既成事实，与任务单的多厂商定位一致。

## 5.2 CLAS：NV-only 现状与标准化路径

草案 [调研4]：「`VK_NV_cluster_acceleration_structure` 目前 NV-only，但 DXR Functional Spec Part 2 已把 CLAS 写成厂商中立设计（2025-09 仍在更新），Khronos EXT 标准化可期」。

复核结果——**现状与方向成立；时间点是草案转述**：

- NV-only：`VK_NV_cluster_acceleration_structure` 为 NV 厂商扩展，规范未见其他厂商实现公告 [S19]（截至访问日期）。
- 厂商中立设计：Microsoft《DirectX Raytracing (DXR) Functional Spec, Part 2》已包含 Clustered Geometry 章节：「An acceleration structure level referred to as Cluster Level Acceleration Structure (or CLAS)」[S22]——CLAS 被写进 DXR 规范即意味着 API 概念脱离单一厂商；检索快照显示该页 2026-07 仍有版本更新，「持续更新中」成立；草案「2025-09 仍在更新」的具体时间点为转述，未逐字复核。
- 「Khronos EXT 标准化可期」是**判断**而非事实：支撑面包括 mesh_shader 从 NV 到 EXT 的先例 [S20][S21]、DXR 侧厂商中立化 [S22]、以及 AMD 正以 DGF 推进多厂商 Vulkan 几何扩展（与 Samsung 合作）[S23]。G9 治理不应把 EXT 落地当作既定日程。

## 5.3 AMD：DGF（Dense Geometry Format）路线

草案 [调研4]：「AMD RDNA4 走 DGF 路线」。

复核结果——**方向成立；代际指认存在口径差异，已标注**：

- GPUOpen 官方（2026-05）：DGF 是「An Open Geometry Compression Standard」，直指现行光追 AS API「黑盒」设计的内存/构建效率缺陷，并宣布 **AMD 正与 Samsung 合作推进 DGF 的多厂商 Vulkan 扩展** [S23]。
- 口径差异标注：GPUOpen 及同期报道的表述为 DGF 原生硬件支持面向**未来 RDNA 代际**，RDNA4 及更早 GPU 经软件解码获得存储侧收益；草案「RDNA4 走 DGF 路线」的代际指认与公开口径存在差异——保留草案陈述作为基线，DGF 代际细节以 GPUOpen 滚动更新为准（G9.1 复核项）。

对 G9/D1 的含义：AMD 侧没有 CLAS 等价物落地，D-6 的「非 CLAS 传统 BLAS 回退腿为正确性基线」在可见期内不可省略；DGF 的多厂商扩展动向进 G9 中期观察清单（对应风险 R-D1-1 的预警信号）。

## 5.4 DMM 禁止线

草案 [调研4]：「DMM（`VK_NV_displacement_micromap`）已被 NVIDIA 官方归档并被 Mega Geometry 取代——禁止投入」。

复核结果——**成立（归档原文级）**：nvpro-samples 的 DMM 样例仓库 `vk_displacement_micromaps` 页面顶部逐字写明：「DEPRECATED — The NVIDIA RTX Mega Geometry technology supercedes displaced Micro-Meshes. This repository is archived as result.」并指向替代样例 `vk_tessellated_clusters` [S24]（`vk_raytrace_displacement` 仓库同状态）。NVIDIA 官方归档 + 明示取代关系，禁止投入有据。

对 G9/D1 的含义：D-7（DMM 永久禁止）与风险 R-D1-7（micromap 提案借「位移需求」复活即否决）维持；位移需求走 WPO/tessellation 既有面，或参照 `vk_tessellated_clusters` 的 CLAS+动态细分路线（只读参照）。

## 5.5 本节对 G9/D1 的判定含义汇总

`VK_EXT_mesh_shader` 跨厂商使 D1-b 的 device kernel「预留 mesh shading 接缝」（草案 D-12）零成本成立；CLAS 的 NV-only 现状决定 D-6 双腿制（NV CLAS 主腿 + 传统 BLAS 回退腿）是当期唯一稳妥分层；DXR Part 2 的厂商中立 CLAS 设计是 API 抽象的预留参照 [S22]；DMM 禁止线不可谈判 [S24]。

---

# 六、单源真相架构原则与验收指标（[调研5]）

## 6.1 共识陈述与来源说明

草案 [调研5]：「光栅剔除选出的可见 cluster 集合 = 当帧 BLAS 拼装 + 动画更新率分级的单源真相，避免几何/RT 两个世界错配；验收指标须含 VRAM 与 AS 构建耗时而非只看 FPS」。

**来源性质声明**：本条为 Epic/NVIDIA/Remedy 工程实践的综合共识，无单一文献承载，按任务纪律引用最接近的公开资料：

- 「两个世界错配」问题陈述：DF 对 Mega Geometry 的机制解读逐字描述了现状——每个游戏实质维护「the world as you see it in-game」与「a secondary BVH structure」两个 3D 世界，「while the game world may be animated, it does not mean the BVH will be, potentially leading to mismatches」[S15]。这是「双世界错配是返工根源」最直接的行业公开表述。
- 「可见集合直接喂 AS」的工程样板：`vk_animated_clusters` / `vk_lod_clusters` 样例展示了动画/LOD 场景下 CLAS→BLAS 的当帧构建路径 [S14]；AW2 1.2.8 的落地（光栅世界即高细节动画几何、BVH 同细节合流）是该原则的首个 3A 实证 [S15][S16]。
- 「动画更新率分级同时约束几何与 AS」：Northlight 的距离分级（10m/1/2/1/3/1/4）[S12][S15] 在 Mega Geometry 更新后继续生效，说明分级策略天然横跨光栅与 RT 两侧——D1-e「降级簇静态帧零 AS 构建」是其直接推论。
- 生态仍在沿此方向演进：NVIDIA GDC 2026 公告已含「RTX Mega Geometry Foliage System」等后续条目 [S25]，佐证「植被/动画几何 + 簇级 AS」合流是持续主线而非一次性发布。

## 6.2 验收指标维度

AW2 数据（第四节）给出验收口径的实证边界：**旗舰 GPU 帧率对 CLAS 不敏感，收益集中在 VRAM（−300MB）、CPU 帧率（+14%）、中低端卡不再爆 VRAM（4060 +42%）** [S15]。因此 D1 验收若只看 FPS 会把真实收益测成零——草案 D-9 与验收门 `clas_blas.budget`（`vram_as_bytes` / `as_build_ms` / 静态帧零构建）的指标面必须硬化进 G9 ACCEPTANCE_MAP。

## 6.3 对 G9/D1 的判定含义

- D-8（`VisibleClusterSet` 一份三喂：光栅 VisBuffer / RT BLAS / VSM 页标记，禁止独立再算可见性）成立，且是 D1 防假绿的核心——验收门 `clas_blas.merge_parity` 的负例 RED 臂（可见集与 BLAS 输入错开一簇即 RED）正是把本原则变成可执行断言。
- 单源真相同时是后续模块的地基：M12/M16/M43/M55 等 defer 项都以统一场景表示为前置（草案 §1），本原则若失守，GI/阴影/大世界全部返工。

---

# 七、对 G9-D1 的判定清单

| D1 决策点 | 调研依据 | 复核结论 | 关键来源 |
|---|---|---|---|
| D-1 GPU cluster 感知蒙皮 + 离线保守包围体，拒绝 CPU 蒙皮喂静态 cluster | [调研1] | **成立**（官方限制 + 学术严格解双支撑；机制归因为转述） | [S2][S4][S8] |
| D-2 Morph target 非虚拟化旁路 | [调研1] | **成立**（Epic 明示不支持） | [S2] |
| D-3 植被全骨骼绑定 + GPU 蒙皮 + 距离分级更新率 | [调研2] | **成立**（30 万骨骼/分级更新率官方与实测复核；两级遮挡/bone shader 细节为转述） | [S10][S11][S12][S15] |
| D-4 monotonic 误差 DAG cut + 页驻留联动 | [调研1] | **成立** | [S1][S2] |
| D-5 离线烘焙 CLAS + 当帧拼装 BLAS + Template 实例化 | [调研3] | **成立**（规范原文级） | [S14][S19] |
| D-6 NV CLAS 主腿 + 非 CLAS 回退腿，抽象按 DXR Part 2 预留 | [调研4] | **成立**（NV-only 现状 + 厂商中立设计已入 DXR 规范） | [S19][S22] |
| D-7 DMM 永久禁止 | [调研4] | **成立**（NVIDIA 官方归档原文） | [S24] |
| D-8 `VisibleClusterSet` 单源真相 | [调研5] | **成立**（综合工程共识，最近来源支撑） | [S14][S15][S16] |
| D-9 VRAM/AS 构建耗时/CPU 带宽为硬指标，FPS 仅观察 | [调研3][调研5] | **成立**（AW2 全数值复核） | [S15][S16] |
| D-10 自研边塌缩 + 簇对锁定，不 vendoring | [调研3] | **成立**（算法公开；术语细节为转述） | [S14][S17][S18] |
| D-11 页格式 v2 新 major、v1 0-byte | G8 治理延续 | 不依赖外部调研（G8 R-G8-4 纪律），维持 | — |
| D-12 mesh shader 不主线化、预留接缝 | [调研4] + G8 治理 | **成立**（EXT 跨厂商使预留接缝零成本；主线化归 G9 决策表） | [S20][S21] |

**未能独立复核条目清单**（均保留草案陈述为事实基线，G9.1 治理波复核）：

| # | 条目 | 草案出处 | 状态 | 处置 |
|---|---|---|---|---|
| U1 | Northlight 两级遮挡剔除（单像素精度）机制细节 | [调研2] | 演讲存在性已核实 [S10][S11]，细节未获公开文本逐字复核 | 表述引自 D1 草案转述；可调取 GDC Vault 视频补核 |
| U2 | bone shader 向美术暴露可编程 API 的措辞 | [调研2] | 同上 | 同上 |
| U3 | UE5.5「CPU 蒙皮喂静态 cluster、击穿包围体/误差度量、吞吐低」机制归因 | [调研1] | 官方仅可复核「无几何 LOD / animation LOD / 不支持 Morph」[S2] 与视角 LOD 个案 [S4] | 机制表述引自 D1 草案转述 |
| U4 | nv_lod_cluster_builder「边塌缩 + 簇对锁定」术语 | [调研3] | 官方博客可复核「decimation + 跨 LOD 无缝组合」[S14] | 术语引自 D1 草案转述 |
| U5 | AMD「RDNA4 走 DGF 路线」的代际指认 | [调研4] | GPUOpen 口径为「原生硬件支持面向未来 RDNA 代际，RDNA4 及更早软件解码受益」[S23] | 保留草案陈述并注明口径差异，以 GPUOpen 滚动更新为准 |
| U6 | DXR Part 2「2025-09 仍在更新」的具体时间点 | [调研4] | 「持续更新中」成立（检索快照 2026-07 有版本更新）[S22] | 时间点引自 D1 草案转述 |

---

# 八、参考来源

- **[S1]** A Deep Dive into Nanite Virtualized Geometry（Karis/Stubbe/Wihlidal，SIGGRAPH 2021 Advances 课程）：https://advances.realtimerendering.com/s2021/Karis_Nanite_SIGGRAPH_Advances_2021_final.pdf （访问日期 2026-08-08）
- **[S2]** Nanite Virtualized Geometry in Unreal Engine（Epic 官方文档，UE 5.8 版页面；经 FetchURL 原文核对）：https://dev.epicgames.com/documentation/en-us/unreal-engine/nanite-virtualized-geometry-in-unreal-engine （访问日期 2026-08-08）
- **[S3]** Unreal Engine 5.5 Release Notes（Epic 官方）：https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5-5-release-notes （访问日期 2026-08-08）
- **[S4]** Nanite on Skeletal Mesh shows extremely low poly from certain angles in UE 5.5+（Epic 官方论坛个案，2025-06）：https://forums.unrealengine.com/t/nanite-on-skeletal-mesh-shows-extremely-low-poly-from-certain-angles-in-ue-5-5/2573728 （访问日期 2026-08-08）
- **[S5]** Unreal Engine 5.7 is now available（Epic 官方公告，2025-11-12，Nanite Foliage Experimental）：https://www.unrealengine.com/news/unreal-engine-5-7-is-now-available （访问日期 2026-08-08）
- **[S6]** Unreal Engine 5.8 is now available（Epic 官方公告，2026-06-17 State of Unreal）：https://www.unrealengine.com/news/unreal-engine-5-8-is-now-available （访问日期 2026-08-08）
- **[S7]** Nanite Foliage（Epic 官方文档，UE 5.8 文档集；Assemblies/Voxels/Skinning 三系统）：https://dev.epicgames.com/documentation/unreal-engine/nanite-foliage （访问日期 2026-08-08）
- **[S8]** Conservative Meshlet Bounds for Robust Culling of Skinned Meshes（Unterguggenberger/Kerbl/Pernsteiner/Wimmer，Computer Graphics Forum 40(7):57–69，DOI）：https://doi.org/10.1111/cgf.14401 （访问日期 2026-08-08）
- **[S9]** TU Wien 论文主页（含 BibTeX、幻灯 PDF）：https://www.cg.tuwien.ac.at/research/publications/2021/unterguggenberger-2021-msh/ （访问日期 2026-08-08）
- **[S10]** GDC Vault：Large Scale GPU-Based Skinning for Vegetation in 'Alan Wake 2'（Kiyavash Kandar，GDC 2024；视频内细节数值未获公开文本逐字复核）：https://gdcvault.com/play/1034310/Large-Scale-GPU-Based-Skinning （访问日期 2026-08-08）
- **[S11]** Explore Remedy's GDC 2024 Talks on Creating Alan Wake 2（Remedy 官方，经 FetchURL 原文核对）：https://www.remedygames.com/article/explore-remedys-gdc2024-talks-on-creating-alan-wake-2 （访问日期 2026-08-08）
- **[S12]** How Northlight makes Alan Wake 2 shine（Remedy 官方，2023-11；约 30 万骨骼逐帧处理）：https://www.remedygames.com/article/how-northlight-makes-alan-wake-2-shine （访问日期 2026-08-08）
- **[S13]** NVIDIA Blackwell GeForce RTX 50 Series Opens New World of AI Computer Graphics（NVIDIA 官方新闻稿，CES 2025，含 RTX Mega Geometry）：https://investor.nvidia.com/news/press-release-details/2025/NVIDIA-Blackwell-GeForce-RTX-50-Series-Opens-New-World-of-AI-Computer-Graphics/default.aspx （访问日期 2026-08-08）
- **[S14]** NVIDIA RTX Mega Geometry Now Available with New Vulkan Samples（NVIDIA 开发者博客，2025-02-13；经 FetchURL 原文核对）：https://developer.nvidia.com/blog/nvidia-rtx-mega-geometry-now-available-with-new-vulkan-samples/ （访问日期 2026-08-08）
- **[S15]** RTX Mega Geometry in Alan Wake 2 — improved, faster, more efficient ray tracing（Digital Foundry，2025-02-05；经 FetchURL 原文核对，VRAM −300MB / 4060 +42% / 2080 Ti +13% / 4090 无收益 / CPU +14% / 动画分级逐字复核）：https://www.digitalfoundry.net/articles/digitalfoundry-2025-rtx-mega-geometry-in-alan-wake-2-improved-faster-more-efficient-ray-tracing （访问日期 2026-08-08）
- **[S16]** Alan Wake 2 update 1.2.8 notes（Remedy 官方，2025-01-30）：https://www.alanwake.com/story/alan-wake-2-update-notes/ （访问日期 2026-08-08）
- **[S17]** nvpro-samples/nv_cluster_builder（GitHub）：https://github.com/nvpro-samples/nv_cluster_builder （访问日期 2026-08-08）
- **[S18]** nvpro-samples/nv_cluster_lod_builder（GitHub；库名 nv_lod_cluster_builder，仓库名词序相反，以本 URL 为准）：https://github.com/nvpro-samples/nv_cluster_lod_builder （访问日期 2026-08-08）
- **[S19]** VK_NV_cluster_acceleration_structure（Vulkan Documentation Project 官方 refpage，含扩展提案章节）：https://docs.vulkan.org/refpages/latest/refpages/source/VK_NV_cluster_acceleration_structure.html （访问日期 2026-08-08）
- **[S20]** VK_EXT_mesh_shader proposal（KhronosGroup/Vulkan-Docs）：https://github.com/KhronosGroup/Vulkan-Docs/blob/main/proposals/VK_EXT_mesh_shader.adoc （访问日期 2026-08-08）
- **[S21]** Task list for VK_EXT_mesh_shader release（KhronosGroup/Vulkan-Docs issue #1927，标题「Multi-vendor mesh shading for Vulkan」）：https://github.com/KhronosGroup/Vulkan-Docs/issues/1927 （访问日期 2026-08-08）
- **[S22]** DirectX Raytracing (DXR) Functional Spec, Part 2（microsoft.github.io/DirectX-Specs，含 Clustered Geometry / CLAS 章节，持续更新中）：https://microsoft.github.io/DirectX-Specs/d3d/Raytracing2.html （访问日期 2026-08-08）
- **[S23]** AMD DGF: An Open Geometry Compression Standard（AMD GPUOpen，2026-05；含与 Samsung 合作多厂商 Vulkan 扩展动向）：https://gpuopen.com/learn/amd-dgf-an-open-geometry-compression-standard/ （访问日期 2026-08-08）
- **[S24]** nvpro-samples/vk_displacement_micromaps（GitHub，DEPRECATED/archived：「The NVIDIA RTX Mega Geometry technology supercedes displaced Micro-Meshes. This repository is archived as result.」）：https://github.com/nvpro-samples/vk_displacement_micromaps （访问日期 2026-08-08）
- **[S25]** GeForce @ GDC 2026 公告（NVIDIA 官方，含 RTX Mega Geometry Foliage System 等持续演进条目）：https://www.nvidia.com/en-us/geforce/news/gdc-2026-nvidia-geforce-rtx-announcements/ （访问日期 2026-08-08）

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-08 | 初版：五路调研之「虚拟几何×RT」路正式化落盘（G9.0 文档集输入材料） |
