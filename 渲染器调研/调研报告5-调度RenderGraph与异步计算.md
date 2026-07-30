# 调研报告 5：渲染调度 / Render Graph / 异步计算

> 面向 rurix（H:\rurix）的前沿论文与工业技术调研 · 2022–2026 · 出品日期：2026-07-28
>
> 本项目现状假设：已有 render graph 基础（`render/graph.rs`、`render/resources.rs`、`render/pass/`、`render/transient.rs`）与 RHI 同步/命令层（`rhi/sync.rs`、`rhi/command.rs`），以及 indirect/DHI 类 GPU-driven 提交（`indirect.rs`）。因此本方向的问题不是"要不要建图"，而是"图的成熟度到哪个档位"：屏障是否全自动推导、transient 是否池化别名、异步计算是否成车道、编译期是否有校验。本报告回答：技术清单、逐项（问题/算法/实现代价/工程复杂度）、rurix 模块映射、分阶段路线、以及需要新增的数据结构与验证方法。

---

## 结论摘要（TL;DR）

对已有 render graph 雏形的 rurix，最小可行 render graph（MVRG）的配方是：**Frostbite 式应用指定线性序 + 逐 pass 显式声明读写 + D3D12 Enhanced Barriers 三轴模型（sync/access/layout）做屏障推导 + transient 资源池别名 + 异步 compute 车道选择**。这条配方里每一项都有 2022–2026 的一手工业依据：执行序由应用指定而非启发式线性化，是 Frostbite FrameGraph 原始设计[^174^]；屏障推导以 EB 三轴为内部统一表示，可同时覆盖 Vulkan synchronization2 与 DX12 双后端，AnKi 的实证表明 PC/主机上 stage 可大幅简化[^157^][^162^]；transient 别名与异步 fence 调度直接照搬 UE RDG 的公开文档语义[^161^]。

三条同样重要的边界结论。**其一，流送/upload 屏障应放在帧图之外**——它们的时间尺度是跨帧的，入图只会让依赖分析爆炸，图内只以 acquire/release 屏障接口接入（与报告 6 的流送系统衔接）。**其二，异步计算不是免费午餐**：vkguide 的实战经验是 SSAO 与阴影异步重叠"未必有收益"[^158^]，而 Godot 4.3 的自动 DAG 在多粒子系统场景拿到数量级收益、普通后处理场景约 5–15%[^209^]——收益取决于帧内是否存在长且无图形依赖的计算段，rurix 应以"AO/GI 滤波/间接参数准备"为首批候选，并用时间戳验证而非默认开启。**其三，GPU 侧调度（D3D12 Work Graphs、mesh nodes、GPU Coroutines）全部归入 P3+ 评估项**：Work Graphs 1.0 直到 2024 年 3 月才正式发布、mesh nodes 仍是预览[^206^][^7^]，其真正价值在报告 1 的 cluster 管线动态扩展场景，不应污染 P0–P2 的图架构设计。

---

## 1. 2022–2026 论文 / 技术清单

### 1.1 集中对比表

| # | 论文 / 技术 | 年份·出处 | 解决什么问题 | 核心算法 / 机制 | 实现代价 / 工程复杂度 | rurix 推荐度 |
|---|---|---|---|---|---|---|
| 1 ★ | **Frostbite FrameGraph**（O'Donnell） | GDC 2017（基线）[^174^] | 整帧优化的调度抽象 | 应用指定 pass 序+声明读写，编译期推导依赖/屏障/生命周期 | 中；概念简洁 | **架构蓝本** |
| 2 ★ | **UE Render Dependency Graph（RDG）+ RDG Insights** | 2019–2026，Epic 官方文档[^161^][^188^] | 生产级帧图语义全集 | 异步 compute 自动 fence、transient 别名、split barrier 提前布局、并行录制、pass/资源剔除、setup 期验证、时间线可视化 | 高（全量）；可裁剪 | **语义字典，逐项裁用** |
| 3 ★ | **Godot 4.3 Rendering Acyclic Graph（RAG）** | 2024，Godot/W4 Games[^209^] | 手写屏障的维护灾难 | 命令记录时自动依赖检测→拓扑排序→层级分组；不可变资源免追踪；MSAA 自动 resolve | 高（自动派）；教训丰富 | **自动派对照组+技巧库** |
| 4 ★ | **D3D12 Enhanced Barriers** | 2022–2026，Microsoft 规范[^157^] | 传统状态机屏障的含糊 | sync/access/layout 三轴分离；texture/buffer/global 三类；split 与 fence barrier；单队列 buffer 并发读写；调试层验证 | 低-中（RHI 层重构） | **屏障推导的内部表示** |
| 5 ★ | **AnKi 简化管线屏障** | 2025，AnKi 引擎博客[^162^] | 25+ stage/30+ access 的组合爆炸 | VK↔D3D12EB 概念映射；PC 收敛为 graphics/compute 两大 stage | 低 | **直接照抄的简化表** |
| 6 | **Granite render graph（Themaister）** | 2017–2024，开源[^163^] | Vulkan 帧图的精细屏障经济学 | invalidation/flush 屏障注入；只读链 fake flush（access=0）；别名 handoff 布局重置 UNDEFINED | 中 | 屏障算法参照实现 |
| 7 | **vkguide 极简 framegraph** | 2023–2025，教程[^158^] | 最小教学实现 | Builder 模式 setup/execute 双 lambda；首用预屏障批处理；异步 SSAO 实测经验 | 低 | P0 快速原型参照 |
| 8 | **skaarj1989/FrameGraph** | 2022，开源库[^181^] | 渲染器无关的图实现 | 完整 declare/compile/execute+别名 | 低（可读代码） | 读代码建立直觉 |
| 9 | **Render Graphs 综述 + Production Engines 系列** | 2021 / 2026[^173^][^186^] | 知识结构化 | Frostbite/RDG/Granite/Wihlidal 谱系梳理；declare/compile/execute 教学 | — | 入门地图 |
| 10 | **UE 5.5 渲染并行化** | 2024，Epic（Tom Looman 摘要）[^115^] | CPU 侧提交瓶颈 | RHI 命令列表并行翻译（最高 2×/降 7ms）；RDG execute 任务异步化（关键路径省 0.4ms） | 中 | P2 并行录制证据 |
| 11 | **D3D12 Work Graphs 1.0 / mesh nodes 预览** | 2024–2026，Microsoft/AMD/NVIDIA[^206^][^7^][^4^] | GPU 侧动态工作扩展 | 节点着色器自调度 producer-consumer 图；mesh nodes 直达几何放大 | 高；API 面新 | **P3+ 评估** |
| 12 | **GPU Coroutines（Zheng 等）/ 程序化生成用 work graphs（Kuth 等）** | TOG 2024 / PACMCGIT 2024[^216^][^215^] | 渲染任务的灵活切分与调度 | GPU 协程帧内切分；work graphs 驱动程序化几何 | 研究向 | 跟踪项 |
| 13 | **GPU 调度学术谱系（Regragui / Chab 综述 / Zou RTGPU）** | 2022–2025[^217^][^218^][^219^] | 引擎任务图与 GPU 调度理论 | 游戏引擎并行任务图案例；GPU 调度算法综述；实时 GPU 细粒度调度 | 理论 | 背景阅读 |
| 14 | **Rust 生态参照（vulkanalia / vulkan-engine / Bevy 0.15）** | 2022–2025[^182^][^171^][^168^] | Rust+ash 的同步/图写法 | 教程级同步封装；ash 引擎样本；wgpu 之外的 ash 路线 | — | 实现参照 |

### 1.2 清单的读法

这份清单可以压成一句话：**2022–2026 的调度方向没有新的"算法突破"，有的是三件事的工业化成熟——屏障语义的统一（EB 三轴成为跨 API  lingua franca）、帧图语义的字典化（RDG 文档把剔除/别名/异步/验证全部写成可裁剪条目）、以及调度重心从 CPU 向 GPU 迁移的序章（Work Graphs）**。对 rurix 意味着：P0–P2 不需要任何学术创新，需要的是从 RDG/Godot/Granite 三个成熟实现里裁剪出与 Rust 模块边界匹配的最小子集；真正的研究跟踪项只有一个——GPU 侧调度在 cluster 管线的落点。

---

## 2. 逐项分析：问题、算法、实现代价与工程复杂度

### 2.1 帧图的三种形态——声明式、全自动、极简

**要解决的问题。** 帧图的根本动机是把"整帧知识"交给系统：谁先谁后、谁写谁读、哪段可以并行、哪块显存可以复用。Frostbite 2017 年给出的答案是声明式：应用以显式顺序注册 pass，每个 pass 声明资源读写，编译期做剔除、生命周期与屏障推导[^174^]；UE RDG 把这套语义扩展成字典——异步 compute 标记、transient 分配器、split barrier、并行录制、setup 期验证一应俱全[^161^]。Godot 4.3 走了另一条路：RenderingDevice 的命令在记录时被自动追踪依赖（邻接表），帧末拓扑排序并按"层级"分组执行，对上层完全透明——其动机是手写屏障已成维护灾难（`CONTINUE` action、barrier mask 误用、过度保守的"以防万一"屏障），合并后删除约 2500 行同步代码，连 AMD Polaris 上 MSAA+SSAO 的陈年伪影都作为副作用被修复[^209^]。

**实现代价与选型。** 声明式的代价是每个 pass 作者要诚实声明读写，收益是编译器可验证声明与实际 API 调用的一致性（RDG 的 setup 验证正是靠这个抓 bug）[^161^]；全自动的代价是追踪器实现的复杂度（Godot 靠"draw list/compute list 即节点"把帧图压到约 300 个节点，图构建+拓扑排序不足帧 CPU 的 1%[^209^]），以及调试上下文丢失——Godot 作者自己承认 debugging 是这条路线的弱项，必须配套强观测工具。rurix 已有显式 `render/pass/` 模块，**声明式是零迁移成本的选择**；但 Godot 贡献了两个可直接吸收的技巧：一是**不可变资源免追踪**（带初始数据创建且无修改标志的资源不建 tracker，使 tracker 从 2 万降到不足 1000）[^209^]，对应 rurix 的静态几何/纹理；二是**层级分组执行**（同层无依赖、可并行录制），这是 P2 并行录制的现成算法。vkguide 的 Builder 模式（setup/execute 双 lambda、首用预屏障批处理）则是 P0 一周内可完成的最小骨架[^158^]。

### 2.2 屏障推导——EB 三轴作为 RHI 内部统一表示

**要解决的问题。** 屏障是帧图编译器最核心的输出，也是 Vulkan/D3D12 双后端差异最大的地方。D3D12 Enhanced Barriers 把一次屏障分解为三个独立正交的轴：**sync**（哪些 GPU 工作必须等待/被阻塞）、**access**（哪些缓存需要 flush/invalidate）、**layout**（图像内存状态切换），并提供 texture/buffer/global 三类屏障、split barrier（转换可在一对 SPLIT 标记间的任意点完成）与新的 fence barrier（Signal/Wait 成对，功能类似 split 但驱动开销更低且支持 global barrier）[^157^]。这套三轴模型与 Vulkan synchronization2 几乎一一对应，AnKi 给出了映射表（`VkPipelineStageFlagBits↔D3D12_BARRIER_SYNC`、`VkAccessFlags↔D3D12_BARRIER_ACCESS`、`VkImageLayout↔D3D12_BARRIER_LAYOUT`），并实证指出：PC/主机硬件上真正不同的 stage 基本只有 graphics 与 compute 两个（传输队列正交另计），25+ stage 的完整枚举在实践上是浪费[^162^]。

**实现代价与 rurix 语义。** 推导算法本身（以 Frostbite/Granite 为参照）是：逐资源追踪"上一次使用的 sync/access/layout"，pass 声明新使用时，若 layout 变化或读写冲突则发射屏障；连续只读链可以用 fake flush（access=0）避免无效缓存失效，别名资源交接时前主以 `NO_ACCESS`+后主的 `LayoutBefore=UNDEFINED` 丢弃旧数据——这些细节 Granite 的实现全部公开[^163^][^157^]。rurix 的做法应是在 `rhi/sync.rs` 内建一个后端中立的 `Barrier { sync_before, sync_after, access_before, access_after, layout_before, layout_after }` 结构作为**内部规范形式**，Vulkan 后端映射到 `vkCmdPipelineBarrier2`、DXIL/D3D12 后端映射到 EB；stage/access 枚举采用 AnKi 式简化集而不是完整 API 枚举，把组合爆炸挡在引擎内部[^162^]。另有两条 EB 的实用规则值得写进 rurix 规范：**buffer 在同队列内可并发读写而无需中间屏障**（single-queue simultaneous access，indirect 参数缓冲与计数器直接受益）[^157^]；布局转换可用 split barrier 提前发射以隐藏延迟（RDG 文档明确将此列为帧图职责之一）[^161^]。

### 2.3 transient 资源与别名——显存复用的工业化做法

**要解决的问题。** GBuffer、AO、阴影中间结果、报告 2 的 radiance cache、报告 3 的 VSM 物理页池这类"帧内即生即灭"的资源是显存大头。帧图的答案是 transient：录制期只创建描述符，编译期计算每个资源的生命周期区间（首用 pass 到末用 pass），执行前从池里分配，生命周期不相交的资源别名到同一块物理内存。RDG 文档给出的语义是：create 只分配描述符、执行期才落到 RHI 资源，池在帧内不相交区间保持别名活跃[^161^]；社区文档的量化经验是别名可省下最高近半的临时资源占用[^192^]。别名交接的正确性靠屏障保证：旧主写出后以 `AccessAfter=NO_ACCESS` 结束，新主以 `LayoutBefore=UNDEFINED`+DISCARD 标志进入，GPU 可丢弃残留缓存写，这正是 EB 规范给出的 aliasing barrier 示例[^157^]。

**实现代价。** 一个够用的一级 transient 分配器约 1–2 周：描述符哈希分桶的空闲链表、按对齐类别分池、区间相交判定、以及峰值审计（记录每帧每池的最高水位）。真正需要设计的约束有两条。**其一，跨帧资源不得 transient**：历史帧资源（报告 7 的 TAA 历史、报告 3 的 VSM 页表反馈）必须以"外部资源"身份 import 进图，图只管理其状态转换不管理其内存[^161^]。**其二，别名与验证的相互作用**：一旦两个逻辑资源共享物理内存，编译期校验必须禁止"被别名资源的过期句柄被使用"，否则调试期会出现极难复现的花屏——RDG 的做法是句柄与生命周期绑定、越期访问直接报错[^161^]，rurix 应照抄这一纪律。

### 2.4 异步计算与跨队列同步——收益有边界，候选要挑剔

**要解决的问题。** 异步计算的目标是把与图形主时间线无依赖的计算段推到第二个硬件队列上与图形重叠。RDG 的调度模型是公开的标杆：pass 标 `AsyncCompute` 后，RDG 沿依赖图找到图形管线上最后一个生产者并插入 fence，异步段跑完后在首个图形消费者处 join 回图形队列；不支持异步的平台自动回落到图形管道[^161^]。UE 5.5 进一步把 RDG execute 任务本身异步化，关键路径省约 0.4ms[^115^]。收益证据呈两极：Godot 在多粒子系统场景拿到数量级提升（此前各粒子系统因误共享临时缓冲被错误串行化），但普通后处理场景只有约 5–15% 帧时间下降[^209^]；vkguide 作者的实战经验更直接——SSAO 与阴影贴的异步重叠在他的引擎里"未必有收益"[^158^]。

**rurix 的候选纪律。** 综合三方证据，异步车道只应接纳满足三个条件的 pass：**时长 ≥0.5ms 量级、无图形管线依赖（只读 GBuffer/只写自有缓冲）、消费者距离生产者足够远**。首批候选：AO 计算与滤波、报告 2 的 GI 滤波/探针更新、报告 7 的降噪器空间滤波趟、粒子模拟；明确不应上异步的：主光栅、阴影渲染、间接参数准备（`indirect.rs` 与光栅强耦合）。工程上还需两条护栏：fence 本身有成本，Godot 文章提醒命令队列提交有固定开销、切分收益要盖过成本[^209^]；异步段的描述符/常量准备必须在图内完成，不能依赖图形队列的中间状态。度量上，P2 验收标准是帧时间戳对比（开/关异步的重叠量），而不是"接上了就算赢"。

### 2.5 GPU 侧调度——Work Graphs 是序章，不是当下

**要解决的问题。** CPU 帧图调度的粒度是 pass；而当 producer-consumer 关系发生在 GPU 数据内部（剔除结果决定后续 dispatch 规模、cluster 展开的深度不可预知），CPU 只能按最坏情况过量派发或靠上一帧回读修正。D3D12 Work Graphs 让着色器节点自己产生后续工作：节点声明输入/输出记录，GPU 运行时负责同步与数据流，Epic 的 Graham Wihlidal 在发布引言中直言"GPU-driven 渲染里 CPU 本来只剩资源管理与 hazard 追踪，Work Graphs 把复杂资源与屏障管理从应用搬进了运行时"[^206^]。2024 年 7 月的 mesh nodes 预览进一步把节点类型扩展到网格放大（与报告 1 的 cluster 管线直接相关）[^7^]；学术侧，Kuth 等用 GPU work graphs 做实过程序化生成[^215^]，Zheng 等的 GPU Coroutines 提供渲染任务在帧内的灵活切分与调度[^216^]，Chab 等的 2025 综述系统分类了 GPU 任务调度算法谱系[^218^]。

**实现代价与定位。** Work Graphs 的落地成本对 rurix 是"新 API 面 + 新编程模型"双重投入：Agility SDK/DXC 的 SM 6.8+ 工具链、节点启动模式（broadcast/coalescing/thread）的语义重写、以及 Vulkan 侧尚无等价物的双后端策略。结论维持 P3+ 评估：**P0–P2 的图架构设计应为它预留接缝（pass 内部可以是任意提交单元），但不为它改变抽象**。最值得跟踪的落点是报告 1 的 cluster 剔除-光栅链（动态扩展最痛的点）与报告 6 的材质分类——这两项到 P3 评审时用 NVIDIA/AMD 的样例数据重新算账[^206^][^215^]。

---

## 3. rurix 现状映射

| rurix 模块 | 现状（按代码结构推断） | 本报告要求的演进 | 对应清单条目 |
|---|---|---|---|
| `render/graph.rs` | 有图基础，pass 注册与执行 | 补：逐 pass 读写声明的参数化结构、编译期剔除、层级分组 | #1/#2/#3 |
| `render/resources.rs` | 资源创建 | 拆：录制期描述符 vs 执行期物理资源；外部资源 import 通道 | #2/#7 |
| `render/transient.rs` | transient 雏形 | 升级：生命周期区间计算、池化别名、峰值审计 | #2/#6 |
| `render/pass/` | 各渲染 pass | 改造：每 pass 声明 reads/writes/AsyncCompute 标记，execute 闭包与声明分离 | #1/#2 |
| `rhi/sync.rs` | 同步封装 | 重构：EB 三轴内部规范形式 + VK sync2 / D3D12 EB 双映射 + AnKi 简化枚举 | #4/#5 |
| `rhi/command.rs` | 命令提交 | 补：多队列分段提交、fence 对注入、命令缓冲环形复用 | #2/#10 |
| `indirect.rs` | GPU-driven 提交 | 衔接：作为异步车道外的图形段保留；DHI 计数器利用 buffer 并发读写规则 | #4 |
| `gpu_profiler.rs` | 性能分析 | 扩展：pass 级 GPU 时间戳 + 帧图 dump（dot/JSON）喂观测工具 | #2 |
| （新增）`render/graph_compile.rs` | — | 新建：剔除/生命周期/屏障推导/车道划分四个编译趟 | 全部 |

映射的要点是"加层而不是推翻"：`render/pass/` 里的每个 pass 主体（shader 与绘制逻辑）不动，只在注册时补齐声明；屏障推导作为独立编译趟从 `graph.rs` 中分离到 `graph_compile.rs`，使得"逐 pass 手写屏障"的旧路径可以在迁移期共存——这与 Godot 保留"可整体禁用图"的调试阀门是同一思想[^209^]。

---

## 4. 分阶段落地路线

![rurix Render Graph 三段式架构](images/r5_rendergraph_arch.svg)

| 阶段 | 目标 | 关键工作 | 验收标准 |
|---|---|---|---|
| **P0 屏障自动化底座** | 消灭手写屏障 | 逐 pass 读写声明；EB 三轴内部表示与双后端映射；首用预屏障批处理；AnKi 简化 stage 集 | 帧内零手写屏障；验证层同步报错为零；屏障计数有统计 |
| **P1 transient 池 + 别名 + 编译期校验** | 显存复用与错误前置 | 生命周期区间；池化分配与别名 handoff（UNDEFINED 入局）；未声明访问/越期句柄/读写冲突的编译期报错；图 dump | 临时资源显存峰值下降可测；注入错误声明必被编译期捕获 |
| **P2 异步计算车道 + 并行录制** | 重叠与提交提速 | AsyncCompute 标记与 fence 对注入；AO/GI 滤波/降噪候选迁移；层级分组多线程录制；RHI 翻译下沉工作线程 | 候选 pass 重叠量以时间戳验证（无效则回退）；录制 CPU 时间下降 |
| **P3 帧图观测** | 可持续优化 | 时间线可视化（RDG Insights 类视图）；别名/屏障审计面板；异步重叠热力 | 任意帧可回答"为什么这段没重叠/这块内存和谁别名" |
| **P3+ GPU 侧调度评估** | 动态扩展场景 | Work Graphs/mesh nodes 原型（接报告 1 cluster 链）；GPU Coroutines 跟踪 | 以样例数据复算收益，不达标则继续 CPU 帧图路线 |

![调度方向分阶段路线](images/r5_roadmap.svg)

与整体计划的咬合：本方向的 P0+P1 是报告 1（几何）、报告 2（GI）、报告 3（VSM）、报告 7（时域）所有 P1+ 阶段的前置——它们都会产生新的中间资源与跨 pass 依赖，没有自动屏障与 transient 别名则每个方向都要重复发明同步代码；P2 的异步车道直接服务报告 2/7 的滤波与降噪候选；P3+ 的 Work Graphs 评估与报告 1 的 P4 评审合并进行。

---

## 5. 必须新增的数据结构、shader 阶段、资源布局与验证方法

**数据结构（CPU 侧为主，本方向几乎不动 GPU 数据格式）。** `PassNode { name, reads: &[Access], writes: &[Access], flags: AsyncCompute?, order: u32, execute: Box<dyn Fn(&mut CmdCtx)> }`；`ResourceNode { desc, producer: PassId, lifetime: (PassId, PassId), physical: Option<PoolSlot>, imported: bool }`；`AccessTracker { last_sync, last_access, last_layout, last_writer }`（逐资源，不可变资源不建）[^209^]；`BarrierBatch { textures: Vec<Barrier>, buffers: Vec<Barrier>, globals: Vec<Barrier> }`（EB 三类对齐，双后端共用）[^157^]；`FencePair { signal_after: PassId, wait_before: PassId, value: u64 }`（timeline semaphore）[^161^]；`TransientPool { buckets: HashMap<DescHash, FreeList>, high_water: usize }`。

**shader 阶段。** 本方向**不新增任何 shader 阶段**——这是它与报告 1–4 的显著区别，也是它应该先行完成的原因。仅有的 shader 侧连带要求来自异步车道：迁移到异步队列的 compute pass 的参数准备必须自包含（bindless 或常量缓冲一次性绑齐），不得读取图形队列中间状态的资源，这条约束会影响报告 2/7 滤波 pass 的参数组织方式。

**资源布局。** 三类新增内存结构：transient 堆按对齐/用途类别分池（GBuffer 类、compute UAV 类、AS scratch 类——后者与报告 4 的 BLAS 构建共用）；每队列命令缓冲环形分配器（graphics/async 各自独立，帧末 fence 回收）；流送 staging 完全走图外通道，仅在消费点以 acquire 屏障接入图内（呼应"流送屏障不入图"的边界）。

**验证方法。** 四个层次，全部可自动化。**编译期**：注入错误声明（漏声明写、声明后越期使用、读写冲突未声明）必须被图编译器捕获——这是 RDG setup 验证的翻版[^161^]。**正确性**：开启 Vulkan validation 的 synchronization 模式与 D3D12 debug layer（EB 规范自带调试层验证），目标帧零同步错误[^157^]。**性能回归**：屏障计数、异步重叠量、transient 峰值、录制 CPU 时间四指标进 CI，与"逐 pass 保守屏障"基线对比——Godot 的经验是图构建本身应 <1% 帧 CPU，超了说明实现有问题[^209^]。**可视化**：每帧导出 dot/JSON 图，附 pass 时间戳，支持"这一帧为什么没重叠"的事后查询。

---

## 6. 风险与缓解

**最大风险是过度工程**：帧图对"3–4 个固定 pass 永不变"的管线是纯浪费，Production Engines 系列作者明说图编译器的价值在管线要生长时才兑现[^186^]。rurix 的管线正在朝七方向同时生长（报告 1–4、6、7 每个都会加 pass），图的投资有明确回报，但 P0–P1 必须控制在"声明+屏障+别名+校验"四个特性内——**并行录制、异步车道、观测工具都是可独立延后的条目，不允许在 P0 混入**。

**异步计算收益不及预期**是第二大风险：vkguide 的"未必有收益"与 Godot 的 5–15% 都说明它不是默认正确的优化[^158^][^209^]。缓解：候选纪律（2.4 节的三条件）+ 时间戳验收 + 每个候选保留一键回退开关。第三条风险是**调试上下文丢失**：命令在录制与执行间多了一层间接，崩溃时的回溯信息变差，Godot 作者明确点出这是自动/间接路线的固有弱点[^209^]；缓解是把观测（P3）的图 dump 与时间戳从 P0 就以最简形式保留（哪怕只是文本日志），并在 pass 注册处强制携带名字与源码位置。

最后是**范围蔓延到 GPU 侧调度**：Work Graphs 的叙事很强（Epic 背书、与 Nanite 同源）[^206^]，但工具链成熟度与 Vulkan 缺位决定了它在 2026 年仍是评估项；rurix 的正确姿势是把图架构设计成"pass 内部提交单元可替换"，把 Work Graphs 的决策推迟到报告 1 的 P4 评审，届时用 mesh nodes 样例数据一次算清[^7^]。

---

[^4^]: https://gpuopen.com/learn/work_graphs_mesh_nodes/work_graphs_mesh_nodes-intro/
[^7^]: https://devblogs.microsoft.com/directx/d3d12-mesh-nodes-in-work-graphs/
[^115^]: https://tomlooman.com/unreal-engine-5-5-performance-highlights/
[^157^]: https://microsoft.github.io/DirectX-Specs/d3d/D3D12EnhancedBarriers.html
[^158^]: https://www.vkguide.dev/docs/ascendant/ascendant_light/
[^161^]: https://dev.epicgames.com/documentation/unreal-engine/render-dependency-graph-in-unreal-engine?lang=en-US
[^162^]: https://anki3d.org/simplified-pipeline-barriers/
[^163^]: https://github.com/Themaister/Granite
[^168^]: https://bevyengine.org/news/bevy-0-15/
[^171^]: https://github.com/michidk/vulkan-engine
[^173^]: https://logins.github.io/graphics/2021/05/31/RenderGraphs.html
[^174^]: https://www.gdcvault.com/play/1024612/FrameGraph-Extensible-Rendering-Architecture-in
[^181^]: https://github.com/skaarj1989/FrameGraph
[^182^]: https://kylemayes.github.io/vulkanalia/
[^186^]: https://stoleckipawel.dev/posts/frame-graph-production/
[^188^]: https://dev.epicgames.com/documentation/unreal-engine/rendering-dependency-graph?application_version=4.27
[^192^]: https://github.com/staticJPL/Render-Dependency-Graph-Documentation/blob/main/Render%20Dependency%20Graph%20(RDG).md
[^206^]: https://devblogs.microsoft.com/directx/d3d12-work-graphs/
[^209^]: https://godotengine.org/article/rendering-acyclic-graph/
[^215^]: https://dl.acm.org/doi/abs/10.1145/3675376
[^216^]: https://dl.acm.org/doi/abs/10.1145/3687766
[^217^]: https://link.springer.com/chapter/10.1007/978-3-031-12597-3_7
[^218^]: https://www.mdpi.com/1999-4893/18/7/385
[^219^]: https://ieeexplore.ieee.org/abstract/document/10012550/
