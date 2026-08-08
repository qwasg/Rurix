# G9-R5 — 全局光照与光照缓存（深度调研）

> **所属**：G9 文档集（`milestones/g9/`）——本文是 G9 五路联网调研之「全局光照与光照缓存（GI）」路的正式化落盘，编号 R5（顺延 G8 的 R1~R3 与 G9 既有 R4）。下游消费者：`G9_PLAN.md`、`G9_CAPABILITY_MATRIX.md`、`design/G9_D2_GI_LIGHTING.md`（D2 设计草案）。
>
> **与 D2 草案的关系**：D2 草案正文引用的「调研 1」~「调研 5」即本文五个正文章节。调研结论以 `design/G9_D2_GI_LIGHTING.md` 为事实基线，本文为其补全来源链与判定含义，**不回写、不修订草案**；两处如有出入，以草案为准。
>
> **调研基准日与访问日期**：2026-08-08。**调研方式**：联网检索（WebSearch / FetchURL），全部结论附来源 URL，优先一手来源（advances.realtimerendering.com、Epic 官方文档、jcgt.org、arXiv、NVIDIA 开发者博客/GitHub、Arm 官方文档、Khronos registry/blog）；个别数值性结论沿用调研轮综合口径，凡未能以公开 URL 独立复核者均显式标注。
>
> **纪律**：零编号占用——本文不新设任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用既有编号；G8 已 closed，其契约与判据 0-byte 改动。

## 目录

1. 结论摘要
2. 调研 1：Lumen 全链路架构
3. 调研 2：多灯直接光——MegaLights×ReSTIR
4. 调研 3：irradiance field 谱系与偏置教训
5. 调研 4：Path Tracer 参照器架构
6. 调研 5：RT 执行路径与 AS 预算
7. 对 G9-D2 的判定清单
8. 参考来源

---

# 一、结论摘要

本轮对 D2 草案「调研 1」~「调研 5」全部五条线索做了独立联网复核。总判定：**五路调研的技术方向全部成立，§7 判定清单 16 行（D2-Q1~Q14 全部 14 条决策 + SPG/Radiance Cache 参数口径 + RD-040 前置）全部有公开一手资料或可标注口径支撑，无任何一路需要推翻**；唯一未能以公开 URL 复核的是 AS 更新成本的 >100 k 实例阈值（§6，已显式标注）。

- **主架构无替代品**：截至调研基准日，Lumen 仍是唯一公开完整工程化细节（Surface Cache + 四级追踪降级 + Screen Probe Gather + Radiance Cache）的生产级实时 GI 架构；G9-D2 以 Lumen 全链路为主参照的决策（D2-Q1）成立，本轮检索未发现同等级公开替代方案。[S1][S2]
- **多灯直接光的成本现实**：MegaLights 已从 UE 5.5 Experimental 走到 UE 5.8 Production-Ready；其作者（Narkowicz & Costa，SIGGRAPH 2025）明确给出「1 ray/pixel 预算下完整 ReSTIR 复用需 2–3× 验证射线、常数成本过高」的对比结论。D2 的「低档 MegaLights 式默认、ReSTIR 仅高档可选」（D2-Q3）有直接一手依据。[S5][S7][S8]
- **偏置教训必须进验收**：DDGI 谱系（JCGT 2019/2021）确立「防漏光优先」的 probe 设计原则；GI-1.0（arXiv 2023）展示了以跳验证换取可扩展性会引入系统性偏置的工程教训；RTXGI 2.0（GDC 2024）表明行业缓存结构走向「空间哈希 + 按需分级」。三者共同支撑 D2-Q4/Q5/Q6 与 L1/L2 档定义。[S14][S15][S16][S18]
- **参照器先行**：pbrt-v4 的 wavefront 架构是文献基线，UE Path Tracer 的「共享输入、不共享算法」是工程模式；M17 参照器先于一切 GI 档位建造（D2-Q7）、megakernel 起步 + wavefront 阶段化接口（D2-Q8）均有来源支撑。[S19][S21]
- **执行路径纪律**：Arm 最佳实践明确 RayQuery 的甜区与「ray query 嵌入 RT pipeline shader 强烈不建议」；UE 官方性能指南确认 TLAS 每帧重建成本显著；VK SER（VK_EXT_ray_tracing_invocation_reorder）2025-11 刚落地、工具链未稳。D2-Q9/Q10/Q13 全部成立。[S22][S23][S24]

**与 G8 决策行的衔接**：本路调研同时是 G8 收口决策行的重判依据——M12（Surface Cache）defer、M16（irradiance field 档位）defer 由调研 1/3 提供建造期参照；M14（HWRT hit lighting / Far Field）no-go 的「M50 后评估」条件已随 M50 转绿而到期，调研 1 提供重判档所需的架构事实；M15（MegaLights / ReSTIR）no-go 的 RD-040 触发条件由调研 2 的产品化曲线与成本结论支撑立项举证；M17（Path Tracer 参照器）no-go 的 backfill 字面条件「GI/材质画质门需要跨路径 golden 时」由调研 4 的参照器架构结论直接兑现。本文只读引用上述决策行，不改动 G8 任何契约文本。

**复核状态**：本报告 26 条参考来源的 URL 均经联网检索定位（含此前预判难定位的 Narkowicz & Costa、PSMS-ReSTIR、RGB ReSTIR、GI-1.0 四条）；唯「HWRT AS 更新成本 >100 k 实例时显著」的具体阈值沿用调研轮综合口径，公开来源只佐证趋势、未复核该数值（见 §6 与 [S23] 条目注记）。

**检索过程备注**：本轮共执行 20 组联网检索 + 3 次页面抓取（1 次成功、1 次因站点 JS 渲染失败、1 次仅取得壳层），多源交叉验证。三条定位经验值得记录：(1) 草案引用的「Narkowicz & Costa, SIGGRAPH 2025」经检索确认即 MegaLights 课程讲义本身——2–3× 验证射线结论出自该讲义，不是另一篇独立论文，引用已归并到 [S5]；(2) PSMS-ReSTIR 与 RGB ReSTIR 均定位到公开 PDF（Utah Graphics Lab / SciTePress），路线无断链；(3) GI-1.0 的 arXiv 摘要页（arXiv:2310.19855，2023-10-30 提交）经抓取确认标题、作者与「two-level radiance caching、免预处理、低运行时成本」的设计取向，「系统性变暗偏置」为 D2 调研轮对该设计取向的解读性教训，本文按草案口径保留并在 §4.2 显式标明解读属性。

**与 D2 out 面的一致性**：本轮调研同样反向支撑 D2 §2.2 的排除项——NRC（RTXGI 2.0 神经腿）触及既有 SG 禁止面且训练基建超模块范围，维持观察项 [S16]；SER 仅接口预留（见 §6）；Far Field HLOD 代理生成链归几何/资产模块，D2 只消费接口（见 §2）；DXIL RT 腿维持 blocked，全部 RT 走 Vulkan 主线。调研未发现需要推翻上述排除处置的新事实。

**五路落点速查**（详细论证见对应章节）：

| 调研路 | 核心一手来源 | D2 决策锚 |
|---|---|---|
| 调研 1 Lumen 全链路 | [S1] 讲义 + [S2][S3][S4] 官方文档 | D2-Q1/Q2/Q11/Q12，§4.1–4.3 |
| 调研 2 MegaLights×ReSTIR | [S5] 讲义 + [S7][S8] 版本线 + [S10]–[S13] 演进线 | D2-Q3/Q4，§4.4 |
| 调研 3 irradiance field 与偏置 | [S14][S15] JCGT + [S16][S17] RTXGI 2.0 + [S18] GI-1.0 | D2-Q4/Q5/Q6，§4.5 |
| 调研 4 参照器架构 | [S19][S20] pbrt-v4 + [S21] UE Path Tracer | D2-Q7/Q8，§4.6 |
| 调研 5 执行路径与预算 | [S22] Arm + [S23] UE RT 性能 + [S24]–[S26] SER | D2-Q9/Q10/Q13 |

---

# 二、调研 1：Lumen 全链路架构

## 2.1 来源与定位

核心一手来源是 SIGGRAPH 2022 Advances 课程的 Lumen 讲义（Wright, Narkowicz, Kelly, Epic Games）[S1]，配套 Epic 官方文档三件套：Lumen GI & Reflections 主文档 [S2]、Lumen Technical Details [S3]、Lumen Performance Guide [S4]。讲义是目前唯一系统披露 Lumen 内部机制的公开材料，官方文档提供运行语义与性能口径，两者交叉一致。时效注记：Epic 官方文档滚动更新（本文访问时为 5.8 代口径），讲义为 2022 年快照；机制描述以讲义为准、运行语义以现行文档为准，本路未发现两者冲突。

## 2.2 事实要点

- **Surface Cache**：Lumen 对近场 mesh 做自动参数化（Card 参数化；官方文档确认由系统自动生成近场参数化），把命中点辐射度（完整材质求值 + 直接光 + 已缓存间接光，单帧延迟反馈）缓存进 Card 图集；未覆盖区域回退到次级追踪/ambient，**只丢能量、不产生漏光裂缝**——这是 Lumen 鲁棒性的核心语义。D2 草案「默认上限 12 Card/mesh」为 Lumen 口径沿用的工程默认值，超出部分按表面积/视角覆盖率裁剪。[S1][S3]
- **四级追踪降级**：L1 Screen Trace（屏幕空间高度场步进，覆盖近场屏幕内、成本最低）→ L2 SWRT（Mesh SDF 近场逐对象 + Global SDF 远场合并，覆盖中距）→ L3 HWRT（硬件光追全场景，命中着色可用 Hit Lighting 完整材质求值）→ L4 Far Field（远场 HLOD 代理辐射度，~1 km 量级、覆盖视距外）。逐档可关、成本与覆盖范围递增，D2 §4.2 的 ~50 m / ~200 m 覆盖划分即沿用该口径。[S1][S2]
- **Screen Probe Gather（SPG）**：屏幕空间按 16 px/probe 基线放置探针，按深度/法线不连续性自适应细分（边界处加密探针以保接触阴影与间接光细节），再做 3×3 probe 空间滤波（≈48×48 屏幕有效滤波）重建全分辨率间接光；探针着色复用 Surface Cache 与追踪降级链，时域累积压噪。[S1]
- **Radiance Cache**：屏幕空间级（复用探针历史）+ 世界空间 clipmap 级（绕相机分级、覆盖屏幕外区域，供反射等路径采样）双级缓存；第一反弹采用 BRDF×入射光 product importance sampling，较均匀采样显著压低方差——G-D2-4 负例臂「关 product IS → 方差回归」的可检测性正源于此对比关系。[S1]
- **Hit Lighting 与材质简化**：HWRT 命中点做完整材质求值成本不可控，UE 提供 RayTracingQualitySwitch 材质节点，让材质为光追路径提供简化版本（官方文档确认该节点同时作用于 Lumen Surface Cache 与 HWRT 路径）。[S3][S4]
- **参考档同位**：Lumen 的性能/可扩展性文档给出从低开销档位到接近参考画质的逐级配置口径；D2 档位阶梯的 L3 per-pixel 参考档即与 Lumen 的参考模式同位，用于 golden 对比与截图级验收，而非实时默认档。[S4]
- **反射路径共享底座**：Lumen Reflections 与 GI 共用同一追踪降级链与 Surface Cache，standalone reflections 需启用 HWRT 并自动打开 hit lighting [S2]。反射本身不在 D2 范围，但「共享追踪底座」意味着 §4.2 的接口与计数面设计须预反射路径的共存，不得在接口上假设 GI 独占。

## 2.3 对 G9/D2 的判定含义

- D2-Q1（主架构对齐 Lumen 全链路而非自研替代）成立：Lumen 是唯一公开完整工程细节的实时 GI 架构，且「UE5 级等价」是项目既定验收基线。
- D2-Q2（缺失覆盖只丢能量不漏光、进负例 RED 臂）直接来自 Surface Cache 语义；漏光比丢能量视觉上更不可接受。
- D2 的 §4.2 四级降级表与 §4.3 SPG/Radiance Cache 参数口径（16 px/probe、3×3 滤波、product IS）逐条可回溯到 [S1]；L3 档 hit lighting 必须配 RayTracingQualitySwitch 式开关（D2-Q11），消费 G8 M50 多 hit group 增量面。
- Far Field 的 HLOD 代理生成属资产/几何模块，D2 只定义消费接口（D2-Q12），与 Lumen 的分工方式一致。
- Lumen 官方性能指南的分档配置口径（[S4]）佐证「档位阶梯 + 每档预算行」是生产侧已验证的管理方式，与 D2 §4.5 的档位定义格式同构。
- Card 图集页格式复用 M04 版本化 ABI、禁止私定磁盘格式（§4.1 接口纪律）：调研 1 不涉及磁盘格式，该项为仓内资产管线纪律，与 Lumen 运行时语义正交，不产生调研层面的冲突。

---

# 三、调研 2：多灯直接光——MegaLights×ReSTIR

## 3.1 来源与定位

核心一手来源是 SIGGRAPH 2025 Advances 课程讲义《MegaLights: Stochastic Direct Lighting in Unreal Engine 5》，作者 Krzysztof Narkowicz 与 Thiago Costa（Epic Games）[S5]，课程主页可佐证其 venue 与摘要 [S6]；讲义 PDF 与课程主页摘要的核心论点（灯数数量级提升、解析/随机拆分估计、验证射线成本）交叉一致。产品化时间线由 Epic 官方材料佐证：UE 5.5 Release Notes 载 MegaLights 为 Experimental [S7]，UE 5.8 Release Notes 载其进入 Production-Ready [S8]，另有 MegaLights 官方功能文档 [S9]。ReSTIR 谱系的学术基线为 ReGIR（Boksansky et al., Ray Tracing Gems 2 第 23 章, 2021）[S10] 与 GRIS（Lin et al., SIGGRAPH 2022）[S11]；时空相关性伪影的最新修复工作为 PSMS-ReSTIR（2025）[S12] 与 RGB ReSTIR（2026）[S13]。

## 3.2 事实要点

- **MegaLights 机制**：每像素固定随机选灯 + 解析无阴影直接光（U）与随机阴影（Sn）拆分估计（S/U 比值法——U 解析可算、S 用少量随机射线估计，比值法让两部分信息都不浪费），再以 hidden light budget 把灯数增长时的射线成本钉在有界范围；使艺术家可放置数量级更多的动态投影灯。阴影走统一的随机光追阴影路径，与其他阴影系统解耦。[S5][S9]
- **ReSTIR 的成本结论（关键）**：讲义作者明确对比——在主机级 ~1 ray/pixel 预算下，完整 ReSTIR reservoir 复用除复用本身开销外，每个复用样本还需验证可见性，合理质量至少 1 sample/pixel，意味着实际需追踪 **2–3 条射线/pixel**，常数成本超出预算。这正是 D2 草案引用「Narkowicz & Costa, SIGGRAPH 2025」的出处。[S5]
- **产品化曲线**：UE 5.5（2024-11）Experimental → UE 5.8（2026-06）Production-Ready；5.8 版本重点降低噪声、改进整体性能，目标当代主机（PS5 / Xbox Series X）60 FPS，并补充调试与验证工具。[S7][S8]
- **ReSTIR 演进线**：ReGIR 把 reservoir 组织为世界空间网格，面向 many-light 离线/实时复用 [S10]；GRIS 给出跨域复用（shift mapping）的理论基础与无偏条件 [S11]；PSMS-ReSTIR（Sample Space Partitioning）用样本空间划分抑制 ReSTIR PT 时空传播离群路径造成的相关性伪影 [S12]；RGB ReSTIR 用逐通道 reservoir 分别估计目标函数，比 ReSTIR PT 更高效地去相关、缓解时域收敛后的色噪 [S13]。

## 3.3 对 G9/D2 的判定含义

- D2-Q3 成立：低档默认 MegaLights 式固定随机选灯、ReSTIR 仅高档可选，是有一手来源背书的成本决策，不是保守偏好。
- 「验证射线纪律」（任何复用路径不得跳过验证射线）与调研 3 的 GI-1.0 教训合流为 D2-Q4，进 G-D2-5 统计性亮度偏置门。
- ReGIR / GRIS / PSMS-ReSTIR / RGB ReSTIR 列为高档 ReSTIR 档的候选演进方向（§4.4），本轮检索确认四条文献均可公开获取，技术路线无断链。
- 海量灯阴影统一接口随多灯子系统联动交付（M22 在 G8 决策表中「随 M15/RD-040」）：MegaLights 的统一随机阴影路径表明「灯数与阴影成本解耦」在工程上可行，D2 的接口随动设计有公开先例。[S5][S9]
- RD-040 backfill 触发举证（立项时附多灯 workload 证据）仍是治理前置，不因 MegaLights 已生产化而豁免。
- 举证材料可直接引用公开产品化事实作旁证：MegaLights 从 Experimental 到 Production-Ready 历时约 19 个月（5.5 → 5.8），说明多灯路径从可用到生产级需要长周期打磨——D2 把低档做成默认、高档保持可选的波次安排（D2.5）与该工程节奏一致。[S7][S8]

---

# 四、调研 3：irradiance field 谱系与偏置教训

## 4.1 来源与定位

- DDGI 原始论文：Majercik et al., JCGT 8(2), 2019 [S14]。
- DDGI 生产化续作（含 probe 重采样）：Majercik, Marrs, Spjut, McGuire, JCGT 10(2), 2021 [S15]。
- 行业缓存结构走向：NVIDIA RTXGI 2.0（GDC 2024 发布），引入 NRC / SHaRC / DDGI 三腿，官方博客 [S16] 与 SDK 仓库 [S17]。
- 偏置教训案例：GI-1.0（Harada et al., arXiv:2310.19855, 2023-10）[S18]。

## 4.2 事实要点

谱系时间线：DDGI（2019，规则体积 probe + 可见性项）→ DDGI Resampling（2021，生产化重采样）→ RTXGI 2.0 / SHaRC（2024，空间哈希缓存 + 按需分级）→ GI-1.0（2023，免验证两级缓存的偏置反例，时间上与 SHaRC 并行）。行业主线清晰可见：缓存空间索引从规则走向哈希，可见性/验证从「可省」走向「必须」。

- **DDGI 基线设计**：八面体编码 irradiance 8×8 + **高分辨率 visibility 16×16**，配合每帧轮换更新摊销；论文设计取向明确——防漏光优先于提高 irradiance 分辨率，因为漏光是高频可见伪影，irradiance 分辨率不足只是软误差。[S14]
- **DDGI Resampling（2021）**：在 probe 间做重采样（resampling），把纯漫反射 irradiance field 扩展到可补直射光与非漫反射项，是 DDGI 走向生产可扩展的关键续作；D2 草案将其列为 L1 档之后的演进项而非首版范围。[S15]
- **RTXGI 2.0 / SHaRC（GDC 2024）**：Spatially Hashed Radiance Cache 以空间哈希组织世界空间 radiance 缓存，按需分配、固定内存；SDK 同时保留 DDGI 与新增 NRC（神经）。行业缓存结构的重心正从「规则体积」走向「哈希缓存 + 按需分级」。[S16][S17]
- **GI-1.0 的教训（arXiv 2023）**：GI-1.0 以两级 radiance caching 换取免预处理、易集成与低成本，其缓存复用路径不做逐样本可见性验证；D2 调研轮将其解读为「跳验证射线引入系统性变暗偏置」的典型案例——偏置随场景复杂度放大、逐帧难察、事后不可归因。[S18]
- **GI-1.0 摘要的定位判断（佐证格局）**：其摘要明言 probe 类技术以少量射线近似 irradiance 但细节缺失、光照变化响应慢，reservoir 类重采样细节更丰富但性能更差、噪声更大——两级缓存正是这一折中的产物。该判断佐证 D2「irradiance field 阶梯为主线、ReSTIR 仅可选增强」的整体格局。[S18]

## 4.3 对 G9/D2 的判定含义

- D2-Q6 成立：L1 DDGI 档的 visibility 16×16 优先于 irradiance 8×8，直接沿袭 JCGT 2019 的防漏光优先原则。
- D2-Q5 成立：L0–L3 档位共享 probe 着色与八面体编码内核、只换空间索引，使档间 golden 对拍可归因到索引结构而非实现差异；DDGI→SHaRC 的谱系演进（规则体积→空间哈希）正是 L1→L2 档的公开先例。
- D2-Q4 成立：GI-1.0 教训固化为「任何复用路径禁止跳验证射线」硬纪律，偏置门进验收（G-D2-5 负例 RED 臂）。
- NRC（神经 radiance cache）维持 D2 §2.2 out 口径：GPU tensor/神经网络属既有 SG 禁止面，RTXGI 2.0 三腿中只取 SHaRC 式的非神经路线作 L2 档参照。
- 档位两端的衔接均有调研锚：L0 档即调研 1 的 SPG 完整形态（§2），L3 per-pixel 参考档与 Lumen 参考模式同位（§2 [S4]）；中间 L1/L2 两档由本路谱系支撑，四档之间无来源断档。

---

# 五、调研 4：Path Tracer 参照器架构

## 5.1 来源与定位

- 文献基线：pbrt-v4（Pharr, Jakob, Humphreys）第四版新增 GPU wavefront 渲染路径，官方在线章《Wavefront Rendering on GPUs》[S19]（第四版全文在 pbr-book.org 免费公开），源码仓库 [S20]。
- 工程模式：UE Path Tracer 官方文档——渐进式、硬件加速的参照渲染模式，与实时管线共享场景/材质输入 [S21]。该文档滚动更新（本文访问时为 5.8 代口径），其「物理正确参照模式」的定位多年稳定，未见语义漂移。

## 5.2 事实要点

- **pbrt-v4 wavefront**：把路径追踪拆成阶段化队列处理——沿 pbrt-v4 的分法，ray gen（相机射线生成）→ intersect（求交）→ shade（按材质队列着色/采样新方向）之间以显式队列交接，路径终止只收缩队列而不空置线程；这消除了 megakernel 中路径提前终止与材质分歧造成的 warp 资源浪费，是 GPU 路径追踪架构的文献基线。[S19]
- **UE Path Tracer 模式**：与实时管线共享场景与材质输入、不共享光照算法——因此 golden diff 可归因到算法层而非输入层；这是「参照器」而非「另一套渲染器」的关键工程属性，也是 Epic 官方对 Path Tracer 的定位（物理正确的参照渲染模式，用于校验实时特性）。[S21]
- **megakernel 起步的合理性**：wavefront 的收益随分歧程度增长；参照器规模受控（单向 PT + NEE/MIS/RR）时 megakernel 工程上更简单，但接口按 wavefront 阶段化切分（D2 草案口径：ray gen / intersect / shade / reservoir 各阶段独立可替换）可为后续 SER 与 hit-lighting 递归演进留位。[S19]

## 5.3 对 G9/D2 的判定含义

- D2-Q7 成立：无跨路径 golden 的画质门不可验收；M17 参照器是 D2 第一前置（门序：G-D2-1 未绿 → G-D2-2~6 不得验收）。
- D2-Q8 成立：megakernel 起步 + wavefront 阶段化接口切分，双锚对照（正确性锚 = pbrt-v4，工程模式锚 = UE Path Tracer）。
- 确定性协议（固定 seed、逐像素 sample count 导出、方差/收敛曲线、1/2/full bounce 匹配深度）是 golden 可归因的前提，承 G8 `ref_tracer` PCG32 对拍模式；其 evidence 字段与对拍 harness 约定即草案 §9 RFC-G9-δ 的输入。
- pbrt-v4 对拍的落地形态：同场景同 spp 的收敛曲线对比（容差带）+ 1/2/full bounce 三匹配深度各一 golden（G-D2-1 门），使每个 GI 档位的验收容差都有可声明的前提而非经验值。

---

# 六、调研 5：RT 执行路径与 AS 预算

## 6.1 来源与定位

- Arm GPU Best Practices Developer Guide，Ray tracing / Efficient ray tracing 章 [S22]。
- UE 官方 Ray Tracing Performance Guide（TLAS 每帧重建成本口径）[S23]。
- Khronos 官方博客：VK_EXT_ray_tracing_invocation_reorder（SER）发布，2025-11-18 [S24]；Vulkan 官方扩展提案页 [S25]；Khronos 官方示例（Vulkan-Samples）README [S26]。

## 6.2 事实要点

- **RayQuery 甜区**：Arm 最佳实践建议批量、均匀的射线负载走 RayQuery（compute/fragment shader 内联追踪：fragment 路径可不中断 render pass、享受 AFBC/AFRC 等优化，compute 路径获得跨线程通信的灵活性；vertex 路径不推荐），并明确指出「在 RT pipeline shader 内再使用 ray query 是强烈不建议的」——即两种执行路径不得混用同一射线流，否则破坏两边的性能特征。[S22]
- **AS 更新成本**：UE 官方性能指南明确 TLAS 每帧重建，其成本同时落在 Rendering Thread、RHI Thread 与 GPU 上，需纳入预算管理；形变/蒙皮几何还涉及 BLAS 重建或 refit，成本高于纯 TLAS 更新 [S23]。「>100 k 实例时 AS 更新成本显著」的具体阈值为调研轮综合口径（含既有 measured 数据），公开来源佐证趋势而未直接给出该数值——**此阈值未能以公开 URL 独立复核**，沿用 D2 草案陈述。
- **SER 现状**：VK_EXT_ray_tracing_invocation_reorder 于 2025-11-18 由 Khronos 正式发布，提供跨厂商 shader execution reordering（HitObject + reorderThreadEXT，可选 hint 位）[S24][S25]；Khronos 官方示例报告其在光追负载下可带来 20–50% 量级的性能改进——收益可观，但示例同时指出当前 Vulkan SDK 的 glslc 尚不支持 `GL_EXT_shader_invocation_reorder`、示例默认须以 Slang 构建，驱动与工具链面未稳 [S26]。收益与成熟度的这一组合，正是「接口预留、实现延后」的判据。

## 6.3 对 G9/D2 的判定含义

- D2-Q9 成立：GI 各档批量均匀射线全走 RayQuery+compute；RT pipeline 仅服务 M17 与未来的 hit-lighting 递归；严禁混用同一射线流，队列化中间层作为唯一交汇点。
- D2-Q10 成立：每 GI 档位定义强制含 AS 更新预算行，档位切换判据消费 AsManager 既有 `AsStats` 计数面；>100 k 实例阈值的出处如上标注，立项后应以本仓 measured 数据复测钉死。
- D2-Q13 成立：SER 只预留（队列化中间层/接口位）不实现——扩展 2025-11 刚落地、工具链未稳，接口预留成本极低而实现风险高。
- M14 重判档的 measured 需求方即 D2 自身画质门：hit lighting（L3 高档）与 Far Field（L4）的证据计数面（命中率/射线量/耗时/AS 预算）由本节纪律直接定义，满足 G8 决策行「无画质 measured 需求方」留档的解除条件。
- 本节纪律即草案 §9 RFC-G9-γ 的语义化输入：RayQuery/RT pipeline 混用禁令、队列化中间层（SER 预留位）、hit group 递归扩展都需在立项后的 spec-first 流程中落成条款，本报告只提供外部依据。

---

# 七、对 G9-D2 的判定清单

| 判定 | 草案锚 | 结论 | 依据 |
|---|---|---|---|
| 主架构对齐 Lumen 全链路 | D2-Q1 | **采纳**，调研 1 全链支撑 | [S1][S2][S3] |
| Surface Cache 缺失覆盖只丢能量不漏光，进 RED 臂 | D2-Q2 / §4.1 | **采纳**，Lumen 鲁棒性核心语义 | [S1][S3] |
| 四级追踪降级 + hit lighting 配材质简化开关 | D2-Q11 / §4.2 | **采纳**，RayTracingQualitySwitch 官方语义佐证 | [S1][S3][S4] |
| SPG 16 px/probe + 自适应细分 + 3×3 滤波；product IS | §4.3 | **采纳**，Lumen 口径可回溯 | [S1] |
| 多灯低档 MegaLights 式默认，ReSTIR 仅高档可选 | D2-Q3 / §4.4 | **采纳**，2–3× 验证射线成本结论有一手出处 | [S5] |
| 复用路径禁跳验证射线，偏置门进验收 | D2-Q4 / G-D2-5 | **采纳**，GI-1.0 教训 + MegaLights 验证成本双侧佐证 | [S5][S18] |
| irradiance field 阶梯共享内核、只换空间索引 | D2-Q5 / §4.5 | **采纳**，DDGI→SHaRC 谱系为公开先例 | [S14][S16][S17] |
| DDGI 档 visibility 16×16 优先于 irradiance 8×8 | D2-Q6 | **采纳**，防漏光优先原则 | [S14] |
| M17 参照器第一前置，golden 门序硬约束 | D2-Q7 / §4.6 | **采纳**，pbrt-v4 + UE Path Tracer 双锚 | [S19][S21] |
| M17 megakernel 起步 + wavefront 阶段化接口 | D2-Q8 | **采纳**，为 SER/递归留演进位 | [S19] |
| RayQuery+compute 与 RT pipeline 严禁混用 | D2-Q9 | **采纳**，Arm 最佳实践明确 | [S22] |
| 每档位含 AS 更新预算行，消费 AsStats | D2-Q10 | **采纳**；>100 k 阈值沿用草案口径，公开来源仅佐证趋势（未独立复核） | [S23] |
| SER 只预留不实现 | D2-Q13 | **采纳**，2025-11 落地、工具链未稳 | [S24][S25][S26] |
| Far Field 只定义消费接口，HLOD 生成归几何模块 | D2-Q12 / §4.2 L4 | **采纳**，与 Lumen 资产/运行时分工一致 | [S1][S2] |
| probe 历史/时域累积全经 temporal 公共底座 | D2-Q14 / §4.3 | **维持**，仓内 G8 纪律锚（`rt/mod.rs` 头注），非外部调研事项 | 仓内锚（只读引用） |
| RD-040 触发举证为立项前置 | §4.4 / D2.0 | **维持**，治理纪律不因外部产品化豁免 | [S7][S8] |

**判读口径**：上表「采纳」仅表示调研证据支持草案对应决策，不构成验收承诺；G9 契约四件套未定稿前，本表供 G9_PLAN 波次排序与 G9_CAPABILITY_MATRIX 能力行定级引用，凡涉门/判据的落地以立项后的契约文本为准。Q14 行的「仓内锚」指 `src/rurix-render/src/rt/mod.rs` 头注所载 G8 时域纪律，为只读引用而非新调研结论。

**遗留项**：

- 「AS 更新成本 >100 k 实例显著」的数值阈值需 G9 立项后以本仓 measured 数据复测钉死（当前仅 [S23] 趋势佐证）。
- GI-1.0 的「系统性变暗偏置」表述为 D2 调研轮对其设计取向的解读性教训（本文 §4.2 已标明解读属性）；若立项评审要求一手精读级证据，建议在 D2.0 治理包内补一轮 GI-1.0 全文复核 [S18]。
- jcgt.org 两页（[S14][S15]）因站点渲染方式无法直接抓取正文，URL 经多组独立文献引用交叉确认；如需逐字核对论文条款，建议抓取其 PDF 版本。
- ReGIR / GRIS / PSMS-ReSTIR / RGB ReSTIR 为高档候选演进方向，本轮仅做来源定位与路线确认，未做实现级评估。
- NRC / 神经路线维持观察项（D2 §2.2 out），不随 RTXGI 2.0 全貌引入。
- Lumen 讲义（[S1]，2022）之后的引擎内部演进（如 5.4–5.8 各版本 Lumen 改进）不在本路调研范围；G8-R1 已覆盖版本演进全景，本文不重复。
- MegaLights 早期版本（5.5 Experimental 期）的灯型覆盖与 HWRT 依赖等限制未逐条核对；立项举证时以现行官方文档 [S9] 与 5.8 Release Notes [S8] 为准。

---

# 八、参考来源

- **[S1]** Lumen: Real-time Global Illumination in Unreal Engine 5（Wright, Narkowicz, Kelly，SIGGRAPH 2022 Advances 课程讲义）：https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Lumen-Wright%20et%20al.pdf （访问日期 2026-08-08）
- **[S2]** Lumen Global Illumination and Reflections in Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/lumen-global-illumination-and-reflections-in-unreal-engine?lang=en-US （访问日期 2026-08-08）
- **[S3]** Lumen Technical Details in Unreal Engine（Epic 官方文档，含 RayTracingQualitySwitch 与 Surface Cache 语义）：https://dev.epicgames.com/documentation/unreal-engine/lumen-technical-details-in-unreal-engine?lang=en-US （访问日期 2026-08-08）
- **[S4]** Lumen Performance Guide for Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/lumen-performance-guide-for-unreal-engine?lang=en-US （访问日期 2026-08-08）
- **[S5]** MegaLights: Stochastic Direct Lighting in Unreal Engine 5（Narkowicz & Costa，SIGGRAPH 2025 Advances 课程讲义）：https://advances.realtimerendering.com/s2025/content/MegaLights_Stochastic_Direct_Lighting_2025.pdf （访问日期 2026-08-08）
- **[S6]** Advances in Real-Time Rendering in Games, SIGGRAPH 2025 课程主页：https://advances.realtimerendering.com/s2025/index.html （访问日期 2026-08-08）
- **[S7]** Unreal Engine 5.5 Release Notes（MegaLights Experimental）：https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5-5-release-notes?lang=en-US （访问日期 2026-08-08）
- **[S8]** Unreal Engine 5.8 Release Notes（MegaLights Production-Ready）：https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5-8-release-notes?lang=en-US （访问日期 2026-08-08）
- **[S9]** MegaLights in Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/megalights-in-unreal-engine?lang=en-US （访问日期 2026-08-08）
- **[S10]** Rendering Many Lights with Grid-Based Reservoirs（Boksansky, Jukarainen, Wyman，Ray Tracing Gems 2 第 23 章 / NVIDIA Research，2021）：https://research.nvidia.com/labs/rtr/publication/boksansky2021rendering/ （访问日期 2026-08-08）
- **[S11]** Generalized Resampled Importance Sampling: Foundations of ReSTIR（Lin et al., SIGGRAPH 2022 / NVIDIA Research）：https://research.nvidia.com/labs/rtr/publication/lin2022generalized/ （访问日期 2026-08-08）
- **[S12]** Sample Space Partitioning and Spatiotemporal Reservoir Resampling（PSMS-ReSTIR，Utah Graphics Lab，2025）：https://graphics.cs.utah.edu/research/projects/psms-restir/psms-restir.pdf （访问日期 2026-08-08）
- **[S13]** RGB ReSTIR: Decorrelating Spatiotemporal Importance Resampling with Per-Channel Reservoirs（Mäkitalo et al., GRAPP/GRIVAPP 2026）：https://www.scitepress.org/Papers/2026/144151/144151.pdf （访问日期 2026-08-08）
- **[S14]** Dynamic Diffuse Global Illumination with Ray-Traced Irradiance Fields（Majercik et al., JCGT 8(2), 2019）：http://jcgt.org/published/0008/02/01/ （访问日期 2026-08-08）
- **[S15]** Scaling Probe-Based Real-Time Dynamic Global Illumination for Production（Majercik, Marrs, Spjut, McGuire, JCGT 10(2), 2021）：http://jcgt.org/published/0010/02/01/ （访问日期 2026-08-08）
- **[S16]** Generative AI for Digital Humans and New AI-powered NVIDIA RTX Lighting（RTXGI 2.0 / SHaRC，NVIDIA 开发者博客，GDC 2024）：https://developer.nvidia.com/blog/generative-ai-for-digital-humans-and-new-ai-powered-nvidia-rtx-lighting/ （访问日期 2026-08-08）
- **[S17]** NVIDIA RTXGI SDK（GitHub）：https://github.com/NVIDIA-RTX/RTXGI （访问日期 2026-08-08）
- **[S18]** GI-1.0: A Fast Scalable Two-Level Radiance Caching Scheme for Real-Time Global Illumination（Harada et al., arXiv:2310.19855, 2023）：https://arxiv.org/abs/2310.19855 （访问日期 2026-08-08）
- **[S19]** Wavefront Rendering on GPUs（Physically Based Rendering 第四版在线章，pbrt-v4）：https://www.pbr-book.org/4ed/Wavefront_Rendering_on_GPUs （访问日期 2026-08-08）
- **[S20]** pbrt-v4 源码仓库（mmp/pbrt-v4）：https://github.com/mmp/pbrt-v4 （访问日期 2026-08-08）
- **[S21]** Path Tracer in Unreal Engine（Epic 官方文档）：https://dev.epicgames.com/documentation/unreal-engine/path-tracer-in-unreal-engine?lang=en-US （访问日期 2026-08-08）
- **[S22]** Efficient ray tracing（Arm GPU Best Practices Developer Guide, Ray tracing 章）：https://developer.arm.com/documentation/101897/0304/Ray-tracing/Efficient-ray-tracing （访问日期 2026-08-08）
- **[S23]** Ray Tracing Performance Guide in Unreal Engine（Epic 官方文档；佐证 TLAS 每帧重建成本趋势，「>100 k 实例」具体阈值沿用调研轮综合口径，来源 URL 未能独立复核该数值）：https://dev.epicgames.com/documentation/unreal-engine/ray-tracing-performance-guide-in-unreal-engine?lang=en-US （访问日期 2026-08-08）
- **[S24]** Boosting Ray Tracing Performance with Shader Execution Reordering: Introducing VK_EXT_ray_tracing_invocation_reorder（Khronos 官方博客，2025-11-18）：https://www.khronos.org/blog/boosting-ray-tracing-performance-with-shader-execution-reordering-introducing-vk-ext-ray-tracing-invocation-reorder （访问日期 2026-08-08）
- **[S25]** VK_EXT_ray_tracing_invocation_reorder 扩展提案（Vulkan 官方文档项目）：https://docs.vulkan.org/features/latest/features/proposals/VK_EXT_ray_tracing_invocation_reorder.html （访问日期 2026-08-08）
- **[S26]** Vulkan-Samples：ray_tracing_invocation_reorder 示例 README（Khronos 官方示例，含工具链支持现状注记）：https://github.com/KhronosGroup/Vulkan-Samples/blob/main/samples/extensions/ray_tracing_invocation_reorder/README.adoc （访问日期 2026-08-08）

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-08 | 初版：五路调研之「GI 与光照缓存」路正式化落盘（G9.0 文档集输入材料） |
