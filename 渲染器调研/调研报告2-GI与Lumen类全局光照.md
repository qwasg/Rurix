# 调研报告 2：GI / Lumen 类全局光照与反射系统

> 面向 rurix（H:\rurix）的前沿论文调研 · 2022–2026 · 出品日期：2026-07-28
>
> 本项目现状假设：已有实时/离线路径追踪基础与图形底座（render graph/RHI、Vulkan/DXIL），但缺少成熟的混合 GI 架构；RT 相关能力尚不稳定（详见报告 4）。本报告回答：候选论文列表、适配本项目的推荐路线、最小可落地版本、集成点、需补齐的底层能力。

---

## 结论摘要（TL;DR）

2022–2026 年的实时 GI 已经分化为两条清晰的工业路线，rurix 不必二选一，而应**按阶段复用两者的交集**：其一是 **Lumen 式"缓存为中心"架构**——以屏幕空间辐射缓存（Screen Probes，约 1/16 分辨率、每探针少量光线）为近场主体、世界空间辐射缓存为远场与多次反弹兜底、Surface Cache/Mesh Card 服务软件追踪与多次反弹[^77^]；其二是 **GI-1.0 / SHaRC 式"两级辐射缓存"架构**——屏幕缓存贴在主可见面、世界缓存用空间哈希无需任何预处理，¼ spp@1080p 在 RX 6900 XT 上总耗时 1.9–3.1ms[^83^][^81^]。两者的公共内核——**屏幕探针 + 探针空间滤波 + 世界空间缓存 + 重要性采样 + 时域累积**——就是 rurix 的最小可落地版本（MVP）。

推荐路线：**P0 用 Vulkan ray query（计算着色器内联追踪，绕开完整 RT 管线与 SBT 管理）做均匀屏幕探针 + 时域累积，先拿单反弹间接光闭环；P1 补探针空间 SH 滤波与平面感知插值；P2 加空间哈希世界辐射缓存拿多反弹与离屏能量；P3 做自适应探针放置与反射管线复用；P4 才引入 SDF 软追踪降级路径、Surface Cache 与 ReSTIR GI/NRC 预研**。这条路线刻意把 Epic 走了三年的 Surface Cache/Mesh SDF 重资产路径后置——Lumen 本体约 140 个 pass、5.6 万行 C++ 与 2 万行 shader[^70^]，而 GI-1.0 证明"屏幕+世界两级缓存"用 ¼ spp 即可达到可比质量[^83^]，SimLumen 则证明 Lumen 核心流程可以被大幅简化复刻[^73^]。

---

## 1. 候选论文 / 技术列表（2022–2026 为主，含必读基线）

### 1.1 集中对比表

| # | 论文 / 技术 | 年份·出处 | 解决什么问题 | 核心算法一句话 | 实现代价 | rurix 推荐度 |
|---|---|---|---|---|---|---|
| 1 ★ | **GI-1.0：两级辐射缓存**（Boissé 等，AMD/GPUOpen） | 2023 arXiv / 2024 I3D[^83^] | 无预处理、动态场景的高质量 GI | 屏幕缓存贴主可见面 + 世界缓存空间哈希 + 辐射 LOD | DXR1.1，¼spp，~2-3ms 级 | **首选蓝本** |
| 2 ★ | **Lumen**（Wright/Narkowicz/Kelly） | SIGGRAPH 2022[^77^] | 全动态 GI+反射，烘焙级室内质量 | SDF 混合追踪 + Surface Cache + 屏幕/世界双 Radiance Cache | 极大（140 pass 级[^70^]） | **架构导师，分块借鉴** |
| 3 ★ | **Radiance Caching for Real-time GI**（Wright） | SIGGRAPH 2021（基线）[^50^] | Lumen 不透明 Final Gather 的方法论 | 屏幕探针自适应降采样+探针空间滤波+重要性采样 | 中 | **MVP 直接依据** |
| 4 ★ | **RTXGI 2.0：SHaRC + NRC**（NVIDIA） | GDC 2024[^81^] | 路径追踪语境的辐射缓存复用 | 空间哈希辐射缓存（无厂商绑定）/ 神经辐射缓存（Tensor Core） | SHaRC 中、NRC 高 | **P2 世界缓存参照** |
| 5 | **DDGI Resampling**（Majercik 等） | CGF 2022[^210^] | 探针体 GI 的噪声与滞后 | 对探针辐射做储层重采样（ReSTIR 思想入探针） | 中 | 高（P2 增强项） |
| 6 | **ReSTIR GI / GRIS（ReSTIR PT）** | CGF 2021 / TOG 2022[^86^][^85^] | 低采样路径追踪的时空复用 | 路径空间储层重采样+广义 RIS 与移位映射 | 高（需完整 RT 闭环） | 长线（P4） |
| 7 | **Rearchitecting Spatiotemporal Resampling for Production**（Wyman & Panteleev） | HPG 2021（基线）[^94^] | ReSTIR 从论文到生产 | 储层布局、偏置控制、与管线集成 | 中 | 长线配套 |
| 8 | **SimLumen（MiniEngine 简化 Lumen）** | 2024 开源[^73^][^69^] | Lumen 核心流程的可复刻最小集 | mesh SDF+卡片+体素注入+SH 八面体滤波，混合 GI | 中，代码公开 | **实现教辅** |
| 9 | **Efficient Light Probes**（Guo 等） | SIGGRAPH 2022[^55^] | 光场探针支持光泽 GI | 光场探针编码可见性，光泽重投影+硬件 RT | 中高 | 中（反射方向参考） |
| 10 | **Radiance Caching with On-Surface Caches**（Tatzgern 等） | I3D 2024[^211^] | 免 SDF 的纹理空间缓存 | 直接在网格表面 UV 空间建缓存+硬件 RT 追踪 | 中 | 中（P3 表面缓存备选） |
| 11 | **UE 5.6/5.7 Lumen 工程演进** | 2025，Epic[^66^][^37^] | 60Hz HWRT、自适应探针、弃 SWRT 细节追踪 | 自适应探针放置、天空像素快速跳出、半分辨率积分 | 小（逐项） | **高（直接可抄的优化）** |
| 12 | **DDGI（基线）与生产化**（Majercik 2019/2021） | JCGT 2019/2021[^213^][^214^] | 动态漫反射 GI 探针体 | 八面体探针图集+逐帧更新+滞后滤波 | 低-中 | 高（MVP 备选形态） |
| 13 | **Spatial MIS for Real-time Irradiance Probes**（Chen 等） | IEEE TVCG 2026[^212^] | 探针间共享追踪结果 | 空间多重重要性采样跨探针复用光线 | 中 | 跟踪项 |
| 14 | **Neural Radiance Cache / Neural Irradiance Volume** | TOG 2021 / CGF 2026[^57^][^215^] | 用小型 MLP 在线学习辐射场 | 路径追踪自训练哈希网格缓存 | 高 | 研究轨 |
| 15 | **无预处理实时 GI 方法综述**（Chen 等） | 计算机辅助设计与图形学学报 2025[^62^] | 领域地图 | SDF/体素/探针/光追采样/神经五族对比 | — | 扫盲与查缺 |

### 1.2 读法说明

**Lumen 是必须读懂但不应照抄的系统。** 它的工程答案由一连串"问题→否决"组成：卡片高度场追踪被否（覆盖不可靠导致漏光）、改投 Mesh SDF 球体追踪（遮挡可靠但只有位置与法线、没有材质）、于是光照交给 Surface Cache（缺失只丢能量而不漏光）、Global SDF 无法索引卡片所以再建体素光照 clipmap（4 级、每体素 6 方向辐射度）、屏幕探针拿近场、世界缓存兜底远场[^77^]。每一步都是可独立摘取的模块，但整体耦合度极高。**GI-1.0 与 SHaRC 则证明世界缓存可以完全无预处理**——空间哈希在线建格、按距离自适应量化形成辐射 LOD，任何几何输入与动态内容都直接适配[^83^][^81^]。对没有烘焙管线、场景系统尚年轻的 rurix 而言，后者是天平倾斜的决定性论据。

**ReSTIR 家族是质量天花板而非地基。** ReSTIR DI（Bitterli 2020）解决动态多光源直接光、ReSTIR GI（Ouyang 2021）把储层复用推广到间接路径、GRIS（Lin 2022）给出无偏理论基础[^85^][^86^]；但三者都假设"已有稳定的逐像素追踪闭环"，而这恰是 rurix 当前短板（报告 4 详述）。因此本报告把 ReSTIR GI 放在 P4 预研轨：届时以 Falcor/RTXDI 的开源集成与 Wyman 2023 课程笔记[^94^]为参照，把储层复用叠加在 P0–P2 的缓存架构之上——事实上 DDGI Resampling（CGF 2022）已经演示了"探针缓存×储层重采样"的混合形态[^210^]。

---

## 2. 适配 rurix 的推荐路线

### 2.1 路线选择的约束推理

rurix 的三个现状直接决定路线形状。第一，**RT 能力不稳定但存在**——这意味着追踪层必须做"能力分级"：主路径用 Vulkan ray query（`VK_KHR_ray_query`）在计算着色器里内联追踪，不需要 raygen pipeline、不需要 SBT 管理、不需要独立的 RT pass 调度，把 RT 依赖压到最小可验证单元；SDF 软追踪作为非 RT 硬件与未来主机降级路径后置[^77^][^81^]。第二，**没有成熟场景/材质系统**（报告 6 的主题）——Surface Cache/Mesh Card 那套"每网格 6–8 方向卡片+图集+逐帧重捕获"的重资产方案对场景系统依赖极深，应推迟到场景系统成型之后[^73^]。第三，**已有 render graph/RHI 与路径追踪基础**——探针缓存的全部 pass（追踪、滤波、插值、累积）都是计算着色器与 transient 纹理的组合，是 graph 的天然公民。

由此得到的路线只有一条主干：**先做"以 ray query 为追踪、以屏幕探针+世界哈希缓存为复用"的 GI 内核，再把 Epic/NVIDIA 各自的重资产模块作为可选扩展挂进来**。它同时满足"先可验证的最小闭环，再长期演进"的要求：P0 的屏幕探针闭环只用约 6 个 pass，GI-1.0 的实测数据表明这一形态在 ¼ spp 下即可达到 2–3ms 级耗时与可用质量[^83^]。

### 2.2 推荐架构总览

下图给出目标架构。追踪层按"Screen Trace（HZB）→ Ray Query → SDF（可选）→ 天空"逐级兜底；缓存层以屏幕探针为主、世界哈希缓存为兜底、Surface Cache 为后期多次反弹模块；滤波合成层复用 Lumen 验证过的三件小事——探针空间滤波（3×3 探针核等效 48×48 屏幕核）、BRDF+光照 PDF 重要性采样（光照 PDF 用上一帧重投影的屏幕缓存，失效处回落世界缓存）、平面感知插值+时域累积+Contact AO[^77^]。

![rurix 推荐混合 GI 架构](images/r2_gi_architecture.svg)

反射管线不从零建设：它与不透明 Final Gather 共享同一套辐射缓存（Lumen 即如此，半透明多层也复用世界缓存探针）[^77^]，P3 阶段以"半分辨率反射追踪+命中点读缓存+独立降噪"接入，UE 5.6 的实测优化（32 位输出格式、异步计算重叠修复）可直接照抄[^66^]。

---

## 3. 最小可落地版本（MVP）与分阶段路线

### 3.1 MVP 定义（P0 的完成态）

**MVP = 均匀网格屏幕探针 + ray query 单反弹追踪 + 探针 SH 投影 + 平面感知插值 + 时域累积。** 明确排除：自适应探针放置、世界缓存、SDF 追踪、Surface Cache、反射复用——全部是后续阶段。验收场景为 Cornell-box 级室内与一个中等户外场景；验收指标为间接光方向正确（与路径追踪参考的 FLIP/SSIM 对比）、快速相机运动下无大面积闪烁、GI 总耗时 <2ms@1080p（GI-1.0 在 ¼spp 下为 1.9–3.1ms@RX 6900XT，rurix MVP 无世界缓存应低于此）[^83^]。

MVP 的每一个组件都有公开参照：屏幕探针布置与时域累积照 Lumen Final Gather（Wright 2021）[^50^][^77^]；探针到 SH/带边八面体投影照 SimLumen 的简化实现[^73^]；ray query 追踪照 RTXGI/SHaRC 的 Vulkan 示例集成[^81^]；重要性采样先做 BRDF PDF 单因子，光照 PDF 待 P1 引入上一帧重投影[^77^]。

### 3.2 分阶段路线

![rurix 混合 GI 分阶段路线](images/r2_gi_roadmap.svg)

| 阶段 | 目标（可演示里程碑） | 核心工作 | 关键依据 | 验收标准 | 预估 |
|---|---|---|---|---|---|
| **P0** GI 最小闭环 | 单反弹间接漫反射可见、时域稳定 | 均匀屏幕探针（~1/16 分辨率）+ray query 追踪+SH 投影+插值+时域累积 | GI-1.0 屏幕缓存[^83^]；Lumen Final Gather[^77^] | 参考图 FLIP 对比通过；GI <2ms@1080p | 4 周 |
| **P1** 探针空间滤波 | 噪声达标、无插值泄漏 | 探针空间 3×3 滤波、BRDF+光照 PDF 双因子重要性采样、平面权重、Contact AO | Lumen 滤波与 IS 设计[^77^]；UE5.6 自适应/快速跳出优化[^66^] | 暗部无可见泄漏；运动场景闪烁受控 | 4 周 |
| **P2** 世界辐射缓存 | 二次反弹、离屏与远处能量回归 | 空间哈希缓存（双哈希+线性探测）、辐射 LOD（距离自适应量化）、屏幕缓存失效处回落 | GI-1.0 世界缓存[^83^]；SHaRC 集成文档[^81^] | 开关门/离屏光源场景能量正确；缓存命中率监控 | 5 周 |
| **P3** 自适应探针+反射接入 | 复杂几何区质量、半分辨率反射 | 自适应探针放置（细分网格）、反射管线复用缓存+独立降噪 | UE5.6 自适应探针[^66^]；Lumen 反射复用[^77^] | 几何密集区探针分布可视化合理；反射 ½ 分辨率质量达标 | 4 周 |
| **P4** 降级与长线 | 非 RT 硬件可用；质量上限预研 | Mesh SDF 离线烘焙+Global SDF clipmap 软追踪；Surface Cache/Mesh Card；ReSTIR GI/DDGI Resampling/NRC 评估 | Lumen SDF 管线[^77^]；SimLumen[^73^]；ReSTIR 课程[^94^]；DDGI Resampling[^210^] | SDF 路径与 RT 路径画面对拍一致；预研报告 | 6 周+ |

**阶段间不变量**应在 P0 冻结：探针记录格式（位置/法线锚点+SH 系数+置信度）、缓存纹理的图集布局与索引跳表（世界缓存即"3D 跳表纹理+图集纹理"结构[^80^]）、以及"追踪层输出=命中点辐射度"的统一接口——SDF、ray query、未来 ReSTIR 都实现同一接口，保证追踪层可替换[^77^][^83^]。

---

## 4. 与 render graph / RT pass / Vulkan 路径的集成点

### 4.1 render graph 集成

GI 子系统在 graph 中表现为一条**支路**：在主深度/GBuffer 之后分叉，于合成光照前汇合。P0–P2 的全部 pass（探针布置→追踪→SH 投影→滤波→插值→累积）都是计算 pass，输入输出均为 transient 纹理与缓冲，依赖关系由 graph 自动推导；跨帧资源只有两类——探针缓存历史（上一帧 SH+深度锚点，用于时域累积与光照 PDF 重投影）与世界哈希缓存（常驻显存、惰性更新），二者按"跨帧读依赖"建模并要求 graph 支持历史资源的延迟释放[^77^][^83^]。UE 5.6 把 GBuffer tile 分类抽成独立 pass 供 Lumen 与 MegaLights 复用、并修复了异步 Lumen 反射的异步计算重叠问题，这一经验直接适用于 rurix：GI 支路应挂到异步计算队列，与主光栅重叠执行（报告 5 详述调度）[^66^]。

### 4.2 RT / Vulkan 集成

**集成点是 ray query 而不是 RT pipeline**：在计算着色器内通过 `VK_KHR_ray_query` 对场景的顶层加速结构（TLAS）发光线，这是 Vulkan 侧最轻的 RT 使用方式——无需 `VK_KHR_ray_tracing_pipeline`、无需 SBT 表管理、无需 shader binding 调度，命中后的材质/光照求值就在本着色器内完成（或按 GI-1.0 做法在命中处直接读辐射缓存）[^81^][^83^]。TLAS 的构建/复用由 rurix 现有路径追踪基础提供（若尚不稳定，报告 4 的 P0 先行）；SDF 降级路径则完全不触碰 RT 扩展，只要 `VK_KHR_shader_atomic_int64` 级别的通用能力即可实现体素可见性缓冲（Lumen 的体素注入正是 24 位 mesh 索引+8 位命中距离的原子写）[^77^]。

### 4.3 与路径追踪/离线的关系

rurix 的离线路径追踪器在 GI 项目中承担**参考真值**角色：所有阶段的画面验收都以它为基准做 FLIP/SSIM 对拍（GI-1.0 论文即以参考路径追踪作对照）[^83^]。反过来，GI 缓存也能回馈离线器：世界哈希缓存本质是一个在线学习的辐射场查询结构，可作为路径追踪的多次反弹近似（SHaRC 的定位正是"路径追踪管线中替换整条间接路径为单次缓存查询"）[^81^]。长线看，NRC/神经辐射体积[^57^][^215^]与 ReSTIR GI[^86^]都是在这条"在线缓存↔离线真值"轴上的延伸。

---

## 5. 需要补齐的底层能力

| 能力 | 现状缺口 | 补齐内容 | 阶段 |
|---|---|---|---|
| **Vulkan ray query 封装** | RT 不稳定/未内联化 | RHI 暴露 `VK_KHR_ray_query`：TLAS 句柄、计算内 `rayQueryEXT` 编译支持（DXIL/SPIR-V 双路径）、能力查询 | P0 |
| **TLAS 构建与增量更新** | 依赖现有 RT 底座 | 每帧 BLAS 集合更新、实例变换缓冲、构建成本预算（与报告 4 共用） | P0 |
| **探针缓存资源体系** | 无 | 屏幕探针图集（~1/16 屏）+SH 系数纹理+深度锚点纹理+历史帧双缓冲；graph transient 契约 | P0 |
| **探针空间滤波器** | 无 | 3×3 探针核、SH/带边八面体投影（硬件双线性采样友好）[^73^] | P1 |
| **重要性采样框架** | 无 | BRDF PDF（SH 形式投影）+光照 PDF（上一帧重投影，失效回落世界缓存）+结构化重要性采样[^77^] | P1 |
| **空间哈希缓存** | 无 | 双哈希+线性探测哈希表、距离自适应量化（辐射 LOD）、散射写入与冲突解决[^83^] | P2 |
| **异步计算重叠** | 部分 | GI 支路上异步队列，与主光栅重叠；graph 跨队列同步（报告 5） | P2–P3 |
| **自适应探针放置** | 无 | 屏幕细分网格、按几何复杂度分裂、天空像素快速跳出[^66^] | P3 |
| **SDF 资产管线（降级轨）** | 无 | Mesh SDF 离线烘焙（Embree 或 GPU 化）、Global SDF clipmap 合并、320MB 式砖池流送[^77^][^73^] | P4 |
| **验证与画像** | 无 | 参考对拍（FLIP/SSIM）、逐 pass GPU 计时、缓存命中率/泄漏热力图、闪烁检测 | 全程 |

**验证方法的三个要点**值得单独强调。其一，**能量守恒检查**：关闭滤波与累积，仅开单反弹追踪，对比参考路径追踪的间接能量曲线，确认缓存系统不凭空造能/丢能——Lumen 用 Surface Cache"缺失只丢能量不漏光"的取舍正是为此[^77^]。其二，**滞后与闪烁分离度量**：时域累积引入滞后、探针复用引入闪烁，二者需分别用运动序列的逐帧 SSIM 与变化区域响应时间衡量，UE 5.3 修复的"高几何密度场景辐射缓存滞后"（探针请求过多导致缓存无法更新）就是这类回归的典型案例[^67^]。其三，**分级硬件矩阵**：ray query 路径（RT 卡）、SDF 路径（非 RT 卡，P4）、以及纯环境光兜底三档必须全部纳入 CI 对拍，避免 GI 成为只在开发机上正确的系统[^63^]。

---

## 6. 风险与备注

**最大的工程风险是把 Lumen 当产品目标。** Lumen 是 140 pass、数万人小时的系统[^70^]，且 Epic 自己仍在持续重写（5.6 弃 SWRT 细节追踪、全面转向 HWRT 60Hz；120Hz 模式仍在研发）[^66^][^37^][^59^]。rurix 的正确姿势是把 Lumen 当"模块辞典"：屏幕探针、世界缓存、探针空间滤波、重要性采样、SDF 分级追踪、Surface Cache，各自独立可摘。本报告 P0–P2 摘的是其中投入产出比最高的四件，并把 GI-1.0/SHaRC 的空间哈希世界缓存作为 Lumen 世界缓存+体素光照的平替[^83^][^81^]。

**其次的技术风险有三。** 一是屏幕探针类方法对**薄几何与强视差**的固有泄漏，缓解手段是平面感知插值+Contact AO+自适应放置，但无法根除——这也是 Epic 持续投入 HWRT 精确路径的原因[^66^]；二是世界哈希缓存在**高频移动光源**下的滞后，需要按 DDGI Resampling 的思路引入重采样或缩短缓存生命周期[^210^]；三是 ray query 对 rurix RT 底座的依赖——若 TLAS 构建本身不稳定，P0 会被阻塞，因此报告 4 的"最小 RT 闭环"应与本报告 P0 并行甚至先行一周启动。最后，**MegaLights**（UE 5.5 引入、5.7 转 Beta 的动态多光源投影系统）[^36^]与 GI 的接口在"灯光求值共享 tile 分类与缓存"层面，rurix 在 P3 之后再评估接入，避免灯光系统与 GI 系统同时施工。

---

[^36^]: https://www.unrealengine.com/news/unreal-engine-5-7-is-now-available
[^37^]: https://tomlooman.com/unreal-engine-5-7-performance-highlights/
[^50^]: https://app.cinevva.com/blog/2026-05-03-aaa-rendering-techniques
[^55^]: https://dl.acm.org/doi/pdf/10.1145/3550454.3555452
[^57^]: https://arxiv.org/html/2412.04634v1
[^59^]: https://www.cgchannel.com/2025/01/see-the-new-features-due-in-unreal-engine-5-6-and-beyond/
[^62^]: https://www.jcad.cn/en/article/doi/10.3724/SP.J.1089.2024-00683
[^63^]: https://altheragames.com/en/blog/ue5-lumen-guide
[^66^]: https://tomlooman.com/unreal-engine-5-6-performance-highlights/
[^67^]: https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5.3-release-notes?application_version=5.3&lang=zh-CN
[^69^]: https://shawntsh1229.github.io/2024/05/18/Simplified-Lumen-GI-In-MiniEngine/
[^70^]: https://zhidao.baidu.com/question/1970698449515838060.html
[^73^]: https://github.com/ShawnTSH1229/SimLumen.git
[^77^]: https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Lumen-Wright%20et%20al.pdf
[^80^]: https://howuerenderacube.super.site/update-radiance-cache
[^81^]: https://github.com/NVIDIAGameWorks/RTXGI
[^83^]: https://ar5iv.labs.arxiv.org/html/2310.19855
[^85^]: https://dl.acm.org/doi/10.1145/3532720.3535632
[^86^]: https://dl.acm.org/doi/10.1145/2504435.2504444
[^94^]: https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf
[^210^]: https://onlinelibrary.wiley.com/doi/abs/10.1111/cgf.14427
[^211^]: https://dl.acm.org/doi/abs/10.1145/3675382
[^212^]: https://ieeexplore.ieee.org/abstract/document/11419929/
[^213^]: http://jcgt.org/published/0008/02/01/
[^214^]: http://jcgt.org/published/0010/02/01/
[^215^]: https://onlinelibrary.wiley.com/doi/abs/10.1111/cgf.70400
