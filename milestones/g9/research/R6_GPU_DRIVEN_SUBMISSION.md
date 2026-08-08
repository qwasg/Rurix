# G9-R6 — GPU-driven 提交与着色器系统（深度调研）

> **所属**：G9 文档集（`milestones/g9/`）——本文是 [G9_D3_GPU_DRIVEN_SUBMISSION.md](../design/G9_D3_GPU_DRIVEN_SUBMISSION.md) 的调研输入之一，
> 下游消费者为 G9 立项档（G9_PLAN.md / G9_CAPABILITY_MATRIX.md）与 design/G9_D3 草案本身。编号顺延 G8 调研档 R1~R3。
>
> **事实基线**：本文六条调研结论以 design/G9_D3 草案 §①「调研结论 1~6」为事实基线，本文只做来源复核与判定含义展开，
> 不与草案矛盾、不回写草案。草案正文引用为「依据 N」，本文章节与之一一对应。
>
> **调研基准日**：2026-08-08。**访问日期**：全部来源 URL 访问日期 2026-08-08。
> **调研方式**：联网深度调研（14 次检索，一手来源优先，全部结论附来源 URL；未能独立复核的条目显式标注）。
>
> **纪律**：零编号占用——本文不新设任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用既有编号；内容以 design/G9_D3 草案为准不回写；
> G8 已 closed，本文对其契约与判据保持 0-byte 改动。

## 目录

1. 结论摘要
2. DGC 跨厂商化与 Indirect Execution Set（依据 1）
3. descriptor 大表与 VK_EXT_descriptor_heap 动向（依据 2）
4. SER 双 API 标准化（依据 3）
5. shader library 与变体治理（依据 4）
6. mesh shader 跨厂商收敛（依据 5）
7. GPU-driven 提交范式（依据 6）
8. 对 G9-D3 的判定清单
9. 参考来源
10. 修订记录

---

# 一、结论摘要

本路调研覆盖 GPU-driven 提交面与着色器系统治理两条线，六条结论全部复核到一手或权威二手来源
（个别数值/子声明标注「引自 D3 草案转述」或「未能独立复核」，见各节与参考来源条目）：

| # | 结论 | 对 G9/D3 的判定 |
|---|---|---|
| 1 | `VK_EXT_device_generated_commands`（EXT 跨厂商版）= Indirect Commands Layout + DGC buffer + Indirect Execution Set；Work Graphs 仍 D3D12 独占，Vulkan 无对应物 | DGC 为 D3 提交面主轴；Execution Set 是 Vulkan 独有差异化能力，D3D12 侧诚实降级；Work Graphs 维持 no-go |
| 2 | `VK_EXT_descriptor_buffer` 是大规模 bindless 现行标准；`VK_EXT_descriptor_heap` 为 2026 新动向 | descriptor buffer 单一大表进 D3 主体；descriptor_heap 关注不实现，仅预留 feature 位 |
| 3 | SER 双 API 标准化完成（VK_EXT 2025-11 + DXR 1.2/SM 6.9），收益目前集中 NVIDIA | M52 由 no-go 调整为「语言层原语 + capability 可选」，渲染器集成延后 |
| 4 | shader library 工业答案 = 编译期 IR 链接（Slang 式模块系统）为主轴 + API 期链接为后端映射；UE 变体治理 = 枚举 + 去重 + 审计 | D3 以自有 IR 函数级链接为主轴；变体预算/审计工具为硬门 |
| 5 | `VK_EXT_mesh_shader` 跨厂商收敛完成（活跃驱动 GPU 95.95%） | M61 由 no-go 调整为「可选 geometry pipeline」；VS 光栅保留为唯一 fallback |
| 6 | GPU-driven submission 范式 = compute pre-pass 零 CPU 回读产出命令 + 三级剔除入场券 + 新 barrier 依赖边类型 | D3 落 DgcBuffer 无 host 读接口 + `IndirectCommandRead` AccessKind 扩展 |

---

# 二、DGC 跨厂商化与 Indirect Execution Set（依据 1）

## 2.1 事实

`VK_EXT_device_generated_commands`（下称 DGC）于 2024 年下半年随 Vulkan 1.3.296 发布，由 NVIDIA 厂商版
`VK_NV_device_generated_commands` 升级为跨厂商 EXT 扩展，参与者包括 Valve、Intel、AMD、NVIDIA、Collabora、
Igalia 等 [S01][S06]。其核心模型由三部分构成 [S01][S02]：

- **Indirect Commands Layout**：声明式模板，描述一个命令 sequence 的 token 序列（shader、push constant、
  vertex/index buffer 绑定、draw/dispatch 等）。
- **DGC buffer**：命令数据 buffer，可由 GPU compute 直接填充（必要时经
  `vkCmdPreprocessGeneratedCommandsEXT` 翻译为实现私有格式），消费端为
  `vkCmdExecuteGeneratedCommandsEXT`。
- **Indirect Execution Set（IES）**：同状态仅换 shader 的管线数组，DGC buffer 中存放指向集合成员的索引，
  实现 GPU 侧 shader 索引切换。提案文档明确指出 D3D12 的 indirect execution 表达能力不及此——
  DX12 无 Execution Set 对应能力 [S02][S04]。

扩展的限制同样明确：每个 sequence 恰一个 dispatch/draw 终止 token 且必须位于最后；sequence 内不可开
render pass、不可插 barrier、不可绑 descriptor set [S02][S04]。

DGC 是 Vulkanised 2025 的重点议题：Ricardo Garcia（Igalia）在大会上做了「Device-Generated Commands in
Vulkan」专题演讲（T10），系统讲解 DGC 概念与 IES 机制 [S03][S04][S05]。Valve 侧开发者（Mike Blumenkrantz）
称其为「自光追以来 Vulkan API 最大的新增」[S06]。

在跨 API 语义层面，DGC 已成为 D3D12 `ExecuteIndirect` 语义的跨 API 最小公倍数与转译目标：提案文档将
「为其他 API 提供 emulation target」列为设计目标之一 [S02]；vkd3d-proton 自 2.7 起即以
`VK_NV_device_generated_commands` 实现高级 ExecuteIndirect（RADV 与 NVIDIA 均支持），后续版本持续用
`VK_NV_device_generated_commands_compute` 优化 ExecuteIndirect 路径 [S10]——DGC 是 Proton/DXVK 生态的
底层共识。

**Work Graphs 对照**：Work Graphs 仍 D3D12 独占 [S07][S08][S29]，Vulkan 无对应物。按 Sawicki 站点 2025
年末 GPU 硬件统计（D3d12infoDB × Steam Hardware Survey，2025-11 数据），Work Graphs 在「活跃驱动 GPU」
（排除 No-data 部分）中支持率为 **74.1%**，全部 GPU 口径仅 55.75%；且「几乎没有 shipped 游戏使用」（唯一
已知用例为 UE 对 Nanite 的优化），作者估计距离首款硬性要求 Work Graphs 的游戏「至少还有约 3 年」 [S09]。

## 2.2 对 G9/D3 的判定含义

- DGC 抽象层取 `VK_EXT_device_generated_commands` 语义集为基准、token 集取跨 API 最小公倍数（D3-Q1）：
  有 Proton/DXVK 转译层背书，语义集不会选错边。
- Execution Set 是 Vulkan 独有差异化能力；D3D12 侧必须显式降级为 CPU 侧选 PSO 再录 ExecuteIndirect，
  登记「GPU 侧 shader 索引切换不可表达」，不伪造（D3-Q2）。
- token 限制（恰一 dispatch token 且最后、禁 render pass/barrier/descriptor set）应内化为 layout 声明的
  装配期核验，fail-closed，而非运行时检查。
- Work Graphs 维持 M56 no-go：Vulkan 无对应物即不满足 G8 RD-041 双条件字面；74.1% 支持率与「首款硬需
  约 3 年」的估计均不支持立项实现（D3-Q6）。render graph schema 仅预留「GPU 端 fan-out」表达能力字段。

---

# 三、descriptor 大表与 VK_EXT_descriptor_heap 动向（依据 2）

## 3.1 事实

`VK_EXT_descriptor_buffer` 自 2022 年底发布以来即定位「bindless 正式化」：descriptor 置于 buffer 内存，
绘制/dispatch 与具体绑定调用解耦 [S12]。NVIDIA 官方 API 性能指南明确推荐：「优先 bindless 设计，使用指向
大 descriptor 表的无界数组 descriptor」[S11]。该扩展是大规模 bindless 的现行标准实现路径。

2026 新动向是 `VK_EXT_descriptor_heap`：该扩展对 Vulkan descriptor 体系做整体重做——descriptor 以裸内存
形式管理、免 image view 对象、统一常量数据通道、真正的 heap 化（两条 heap：resource + sampler），被认为
是支持驱动上的 bindless 推荐新路径 [S13][S14]。Sascha Willems 于 2026-06-13 发布了基于
`VK_EXT_descriptor_heap` 的官方示例集 [S14][S15]；截至调研基准日，该扩展处于早期采用阶段（首批驱动与
示例落地），Khronos 发布任务列表显示其规范化流程已启动 [S13]。

## 3.2 对 G9/D3 的判定含义

- D3 主体采用 `VK_EXT_descriptor_buffer` 单一大表架构，shader 侧以全局 descriptor 索引寻址；反射/manifest
  记录面升级为「资源→全局 descriptor 索引」映射（与 set/binding 对并存不删，保 M31/M85 digest 链，
  D3-Q3）。
- `VK_EXT_descriptor_heap` **关注不实现**（D3-Q4）：提案与生态仍在早期，现在实现等于绑定未冻结面；
  仅在 capability profile 预留 feature 位（如 `bindless.descriptor_heap` 占位 ID），跟踪其规范化进展。

---

# 四、SER 双 API 标准化（依据 3）

## 4.1 事实

Shader Execution Reordering（SER）已完成双 API 标准化：

- **Vulkan 侧**：`VK_EXT_ray_tracing_invocation_reorder` 于 2025-11 随 Vulkan 1.4.333 发布，由
  `VK_NV_ray_tracing_invocation_reorder`（2022 年首发）升级为多厂商 EXT [S16][S17][S19]。语言面为
  hit object + `reorderThreadEXT(hint, bits)`（hit object 可用于 raygen/closest-hit/miss 阶段，
  `reorderThreadEXT` 仅 raygen 阶段）；无硬件重排能力的设备上 `reorderThreadEXT` 退化为 no-op，
  语法可用、实现可选 [S16]。
- **D3D12 侧**：SER 随 DXR 1.2 / Shader Model 6.9 进入正式（retail）状态，由 Agility SDK 1.619 发布
  （2026-02），同样「语法强制、实现可选」——在不支持重排的硬件上 reorder 步骤为 no-op [S18]。

性能收益证据目前集中于 NVIDIA：Khronos 官方案例中，Vulkan glTF path tracer（vk_gltf_renderer）仅改数行
代码启用 SER，`vkCmdTraceRaysKHR` 耗时 38.34 ms → 20.02 ms，即 **+47%**（47.78%）提升；Nsight 测得
warp coherence 从 **23.0% 提升到 54.2%** [S16][S19]。该收益场景甚至不是 SER 理想工况（单一
closest-hit übershader、未用 coherence hint）[S16]。

## 4.2 对 G9/D3 的判定含义

- G8 M52（SER no-go 留档，RD-040 触发条件未满足）应调整为「**语言层原语 + capability 可选**」（D3-Q5）：
  双 API 标准化已完成，RD-040 的「分歧 RT workload」条件可由 capability 机制（`#[requires("rt.ser")]` +
  profile fallback）精确表达，不再需要渲染器硬承诺。
- 收益集中 NVIDIA、跨厂商证据不足，故承诺面止步于「语言可表达 + capability 可选 + 双后端物化」；
  材质 hint 消费、coherence 测量、默认开启均属后续专项。材质 flags 预留 2~4 bit coherence hint 位段，
  值域冻结进 spec、消费端延后（D3-Q12）。

---

# 五、shader library 与变体治理（依据 4）

## 5.1 事实

工业界对 shader library 的答案分两层，分工不互相替代：

- **编译期 IR 链接（解决组合爆炸——数量问题）**：Slang 式模块系统是代表。Slang 2024-11 起由 Khronos
  Group 托管为开源项目（NVIDIA 贡献，多方治理），提供 modules/interfaces/generics 等模块化能力，面向
  大规模 shader 代码库的维护与编译时间削减 [S20][S21]。Slang 官方仓库与站点见 [S20][S21]；草案另述
  「有官方 Rust crate」——经检索，shader-slang 官方 GitHub org 仓库列表中未见 Rust 绑定仓库（官方语言绑定
  以 Python 为主），crates.io 上存在社区维护的 `shader-slang` 绑定（FloatyMonkey/slang-rs）[S22]；
  该子声明**来源 URL 未能独立复核**，保留草案陈述并在此标注。
- **API 期链接（解决运行时 hitching——延迟问题）**：Vulkan 提供 `VK_EXT_graphics_pipeline_library`
  （把 graphics pipeline 拆成四段独立预编译、最终链接）[S23][S24]、`VK_EXT_shader_object`（免管线对象
  直接绑 shader）[S25]；RT 侧有 pipeline collection；DGC 侧有 Indirect Execution Set（见 §二）。
  Khronos 官方对 GPL 的定位即「消除 draw-time hitching」[S24]。

UE 的 permutation 治理是「离线枚举 + 去重 + 审计」的成熟案例：Epic 官方确认 Fortnite 的完整 PSO 组合空间
在百万量级，运行时仅用其中极小子集（一场 Battle Royale 预缓存约 3 万 PSO、实使用约 1 万）；UE 5.2 起以
PSO precaching + manifest 式精确枚举治理 hitching [S26]。State of Unreal 2026 官方通报：UE 5.8 通过优化
shader 编译与改进去重（deduplication），将 Fortnite 的 shader 数量削减 **68%** [S27]。草案引用的案例数值
「1300 万→400 万变体」与官方 −68% 口径相容（13M × 32% ≈ 4.2M），但**绝对数值未在官方文本中出现，
数值引自 D3 草案转述**；官方可复核口径为「Fortnite shader 数量 −68%」[S27]。

## 5.2 对 G9/D3 的判定含义

- D3 主轴 = 编译期 IR 链接（D3-Q10）：RuriX 自有编译器可在自家 IR 层做函数级组合链接、物化 SPIR-V/DXIL，
  不需外挂 Slang 运行时依赖；Slang 生态仅作语义对标与互操作评估对象。
- 分工纪律：离线组合解决数量，运行时链接解决延迟——D3 承诺离线侧（manifest 精确枚举 + DDC 去重 +
  execution set 全离线物化），API 期链接面（GPL/shader object）只登记为后端映射备选，不进承诺。
- 变体治理必须带工程级硬闸（D3-Q11）：manifest 精确枚举 + DDC 去重 + 变体审计工具（按 axis/module/命中
  率分解报告 + 工程级总预算超线硬失败 + 死变体检测），复现 UE「枚举—去重—审计」治理三角而非仅
  per-entry 预算。

---

# 六、mesh shader 跨厂商收敛（依据 5）

## 6.1 事实

`VK_EXT_mesh_shader` 提供 task/mesh 两个新 stage，图形管线可在没有 vertex/tessellation/geometry shader 的
情况下以 mesh shader 完成图元生成 [S28]。跨厂商收敛按公开统计已实质完成：Sawicki 站点 2025 年末统计，
mesh shader 在「活跃驱动 GPU」（排除 No-data）中支持率 **95.95%**（全部 GPU 口径 72.18%），即在仍有
驱动维护的硬件上接近普及；该文并指出对合适的游戏可以直接要求 mesh shader 支持（仅损失 4.05% 潜在
用户）[S09]。

架构顺序上，mesh shader 的入口数据是 cluster（meshlet）流：mesh shader 路径必须排在 meshlet 格式与
GPU-driven 剔除之后，否则没有合法输入。fallback 纪律上，保留传统 VS 光栅为唯一 fallback，不做双套全
功能并行——Sawicki 文的分析亦支持「为老硬件再写一套优化路径不划算」的判断 [S09]。

## 6.2 对 G9/D3 的判定含义

- G8 M61（mesh shader 第三光栅 no-go，RD-039 双条件未成立）应调整为「**可选 geometry pipeline**」
  （D3-Q8）：RD-039「跨厂商收敛」条件按 95.95% 公开证据实质成立；「measured」条件以本机 4070 Ti +
  CI 设备 measured 补齐。
- 顺序硬约束写进条款：cluster 流 → mesh shader 入口 → DGC 提交；meshlet 格式（G8.3 M01/M04 已绿）与
  三级剔除（渲染器主体）先行，D3 只落通道条款。
- VS 光栅为唯一 fallback，capability 选择律（`mesh.task` ID）裁定路径切换；task shader（M62）维持不
  开放——cluster fan-out 已由 DGC 承担，Amplification 语义无消费方（D3-Q9）。

---

# 七、GPU-driven 提交范式（依据 6）

## 7.1 事实

GPU-driven submission 范式：compute pre-pass 在 GPU 上产出命令数据，直接被 indirect 执行消费，全程零 CPU
回读。DGC 正好解决了该范式的历史痛点——compute pre-pass 结果回传 CPU 再录制命令的 stall：DGC buffer
GPU 可写、`vkCmdExecuteGeneratedCommandsEXT` 就地消费 [S01][S02]；Work Graphs 与 NVIDIA 的 GPU-driven
渲染布道亦以「GPU 自决后续工作、消除 CPU 往返」为同一目标 [S08][S29]。

工程入场券是三级剔除（instance→cluster→triangle）：没有 GPU 侧剔除产出的命令流，DGC 没有消费对象。
对 render graph 自动 barrier 系统的新增要求是依赖边类型扩展：同一 buffer 从「compute storage-write」
转为「indirect-command-read」消费，需要新的 AccessKind 边（Vulkan 侧 `SHADER_WRITE`→
`INDIRECT_COMMAND_READ`；D3D12 侧 `UNORDERED_ACCESS`→`INDIRECT_ARGUMENT`），否则 GPU 生成 →
GPU 消费之间存在同步空洞。

## 7.2 对 G9/D3 的判定含义

- 零 CPU 回读做成结构性保证：DgcBuffer 类型不提供 host 读接口（镜像 G8 AsyncBuffer 在途态先例），调试
  dump 走显式 readback pass；验收以「回读计数器 = 0」断言。
- AccessKind 封闭枚举加性扩展 `StorageWrite→IndirectCommandRead` 边类型 + 双后端映射新行，同居单一
  事实源；DGC 数据流全程在 RXS-0239 单 queue 全序内表达，不引入 pass 内重排，async compute（M59）
  维持 no-go（D3-Q7）。
- 三级剔除算法本体归渲染器主体模块；D3 只交付其提交通道与依赖边类型——这决定了 D3 波次必须先于/
  伴随渲染器主体的剔除落地（W1/W2 为提交入场券）。

---

# 八、对 G9-D3 的判定清单

| 调研结论 | 复核状态 | 支撑的 D3 裁决（草案 §⑤） |
|---|---|---|
| 依据 1：DGC 跨厂商化（EXT 化、IES、token 限制、ExecuteIndirect 最小公倍数） | ✅ 一手复核 [S01][S02][S04][S06][S10] | D3-Q1（DGC 语义集为主轴）、D3-Q2（D3D12 诚实降级） |
| 依据 1 附：Work Graphs D3D12 独占、74.1%、首款硬需约 3 年 | ✅ 统计出处复核 [S07][S08][S09] | D3-Q6（M56 维持 no-go，仅预留 schema 字段） |
| 依据 2：descriptor buffer 现行标准 + descriptor_heap 动向 | ✅ 一手复核 [S11][S12][S13][S14][S15] | D3-Q3（全局索引记录面）、D3-Q4（关注不实现） |
| 依据 3：SER 双 API 标准化；+47%、coherence 23%→54% | ✅ 一手复核，数值命中 [S16][S17][S18][S19] | D3-Q5（M52 调整为语言原语 + capability 可选）、D3-Q12（hint 位段预留） |
| 依据 4：IR 链接主轴 + API 期链接映射 + 变体治理 | ✅ 主体复核 [S20][S23][S24][S25][S26][S27]；「官方 Rust crate」与「1300 万→400 万」绝对值未独立复核（官方口径：Fortnite shader −68%） | D3-Q10（编译期 IR 链接主轴）、D3-Q11（枚举+去重+审计+总预算门） |
| 依据 5：mesh shader 收敛 95.95%、cluster 流顺序、VS 唯一 fallback | ✅ 统计出处复核 [S09][S28] | D3-Q8（M61 调整为可选路径）、D3-Q9（M62 维持不开放） |
| 依据 6：零 CPU 回读范式、三级剔除入场券、新 barrier 边类型 | ✅ 机制复核 [S01][S02][S08][S29] | D3-Q7（M59 维持 no-go）、D3.3 AccessKind 扩展 |

**未能独立复核条目汇总**（保留草案陈述，不静默丢弃）：

1. Slang「有官方 Rust crate」：官方 org 未见 Rust 绑定仓库，crates.io 现有为社区维护绑定 [S22]——标注
   「来源 URL 未能独立复核」。
2. UE「1300 万→400 万变体」绝对数值：官方可复核口径为 Fortnite shader 数量 −68% [S27]，绝对数值标注
   「数值引自 D3 草案转述」。

---

# 九、参考来源

> 全部 URL 访问日期 2026-08-08。标注说明：✅复核 = 页面内容经检索/取回直接命中声明；
> 个别条目附「数值引自 D3 草案转述」或「来源 URL 未能独立复核」标注（见正文 §五、§八）。

- **[S01]** VK_EXT_device_generated_commands(3) — Vulkan Documentation Project：https://docs.vulkan.org/refpages/latest/refpages/source/VK_EXT_device_generated_commands.html （访问日期 2026-08-08）
- **[S02]** VK_EXT_device_generated_commands 提案文档（Khronos/Vulkan-Docs）：https://github.com/KhronosGroup/Vulkan-Docs/blob/main/proposals/VK_EXT_device_generated_commands.adoc （访问日期 2026-08-08）
- **[S03]** Vulkanised 2025 会议页（Khronos）：https://vulkan.org/events/vulkanised-2025 （访问日期 2026-08-08）
- **[S04]** Device-Generated Commands in Vulkan — Ricardo Garcia（Igalia），Vulkanised 2025 T10 演讲幻灯：https://vulkan.org/user/pages/09.events/vulkanised-2025/T10-Ricardo-Garcia-Igalia.pdf （访问日期 2026-08-08）
- **[S05]** Device-Generated Commands at Vulkanised 2025 — Ricardo Garcia 博客（Geek Blight）：https://rg3.name/202503111630.html （访问日期 2026-08-08）
- **[S06]** Vulkan 1.3.296 Released With VK_EXT_device_generated_commands — Phoronix（2024-09）：https://www.phoronix.com/news/Vulkan-1.3.296-Released （访问日期 2026-08-08）
- **[S07]** Work Graphs — Microsoft DirectX-Specs 规范：https://microsoft.github.io/DirectX-Specs/d3d/WorkGraphs.html （访问日期 2026-08-08）
- **[S08]** D3D12 Work Graphs 发布 — Microsoft DirectX Developer Blog：https://devblogs.microsoft.com/directx/d3d12-work-graphs/ （访问日期 2026-08-08）
- **[S09]** State of GPU Hardware (End of Year 2025) — Adam Sawicki 站点客座文章（Dmytro “Boolka” Bulatov；74.1% Work Graphs、95.95% mesh shader、首款硬需约 3 年出处）：https://asawicki.info/articles/state_of_gpu_hardware_2025.php （访问日期 2026-08-08）
- **[S10]** vkd3d-proton CHANGELOG（ExecuteIndirect 基于 VK_NV_device_generated_commands / _compute 实现）：https://github.com/HansKristian-Work/vkd3d-proton/blob/master/CHANGELOG.md （访问日期 2026-08-08）
- **[S11]** Advanced API Performance: Descriptors — NVIDIA Developer Blog（官方推荐 bindless 大表）：https://developer.nvidia.com/blog/advanced-api-performance-descriptors/ （访问日期 2026-08-08）
- **[S12]** VK_EXT_descriptor_buffer — Khronos Blog：https://www.khronos.org/blog/vk-ext-descriptor-buffer （访问日期 2026-08-08）
- **[S13]** VK_EXT_descriptor_heap 提案文档（Khronos/Vulkan-Docs）：https://github.com/KhronosGroup/Vulkan-Docs/blob/main/proposals/VK_EXT_descriptor_heap.adoc （访问日期 2026-08-08）
- **[S14]** New Vulkan samples for using descriptor heaps — Sascha Willems Blog（2026-06-13）：https://www.saschawillems.de/blog/2026/06/13/new-vulkan-samples-for-using-descriptor-heaps/ （访问日期 2026-08-08）
- **[S15]** descriptorheap 示例源码 — SaschaWillems/Vulkan（GitHub）：https://github.com/SaschaWillems/Vulkan/blob/master/examples/descriptorheap/descriptorheap.cpp （访问日期 2026-08-08）
- **[S16]** Boosting Ray Tracing Performance with Shader Execution Reordering: Introducing VK_EXT_ray_tracing_invocation_reorder — Khronos Blog（2025-11-18；glTF PT +47%、coherence 23.0%→54.2% 出处）：https://www.khronos.org/blog/boosting-ray-tracing-performance-with-shader-execution-reordering-introducing-vk-ext-ray-tracing-invocation-reorder （访问日期 2026-08-08）
- **[S17]** VK_EXT_ray_tracing_invocation_reorder(3) — Vulkan Documentation Project：https://docs.vulkan.org/refpages/latest/refpages/source/VK_EXT_ray_tracing_invocation_reorder.html （访问日期 2026-08-08）
- **[S18]** Shader Model 6.9 (retail) 与 DXR 1.2 发布 — Microsoft DirectX Developer Blog：https://devblogs.microsoft.com/directx/shader-model-6-9-retail-and-more/ （访问日期 2026-08-08）
- **[S19]** Vulkan SER Showing Up To ~47% Performance Improvement For Ray-Tracing — Phoronix（2025-11-18）：https://www.phoronix.com/news/Vulkan-SER-Performance （访问日期 2026-08-08）
- **[S20]** Khronos Group Launches Slang Initiative, Hosting Open Source Compiler Contributed by NVIDIA — Khronos 新闻稿（2024-11-21）：https://www.khronos.org/news/press/khronos-group-launches-slang-initiative-hosting-open-source-compiler-contributed-by-nvidia （访问日期 2026-08-08）
- **[S21]** The Slang Shading Language 官方站点：https://shader-slang.org/ （访问日期 2026-08-08）
- **[S22]** shader-slang — crates.io（社区维护 Rust 绑定；草案「官方 Rust crate」子声明来源 URL 未能独立复核）：https://crates.io/crates/shader-slang （访问日期 2026-08-08）
- **[S23]** VK_EXT_graphics_pipeline_library 提案文档（Khronos/Vulkan-Docs）：https://github.com/KhronosGroup/Vulkan-Docs/blob/main/proposals/VK_EXT_graphics_pipeline_library.adoc （访问日期 2026-08-08）
- **[S24]** Reducing Draw Time Hitching with VK_EXT_graphics_pipeline_library — Khronos Blog：https://www.khronos.org/blog/reducing-draw-time-hitching-with-vk-ext-graphics-pipeline-library （访问日期 2026-08-08）
- **[S25]** VK_EXT_shader_object 提案文档 — Vulkan Documentation Project：https://docs.vulkan.org/features/latest/features/proposals/VK_EXT_shader_object.html （访问日期 2026-08-08）
- **[S26]** Game engines and shader stuttering: Unreal Engine's solution to the problem — Unreal Engine Tech Blog（PSO precaching；Fortnite 组合空间百万量级、单场约 3 万预缓存 / 1 万使用）：https://www.unrealengine.com/tech-blog/game-engines-and-shader-stuttering-unreal-engines-solution-to-the-problem （访问日期 2026-08-08）
- **[S27]** State of Unreal 2026: Top news from the show — Unreal Engine 官方（UE 5.8 shader 编译优化与去重使 Fortnite shader 数量 −68%；「1300 万→400 万」绝对数值引自 D3 草案转述）：https://www.unrealengine.com/news/state-of-unreal-2026-top-news-from-the-show （访问日期 2026-08-08）
- **[S28]** VK_EXT_mesh_shader 提案文档 — Vulkan Documentation Project：https://docs.vulkan.org/features/latest/features/proposals/VK_EXT_mesh_shader.html （访问日期 2026-08-08）
- **[S29]** Advancing GPU-Driven Rendering with Work Graphs in Direct3D 12 — NVIDIA Developer Blog：https://developer.nvidia.com/blog/advancing-gpu-driven-rendering-with-work-graphs-in-direct3d-12/ （访问日期 2026-08-08）

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-08 | 初版：五路调研之「GPU-driven 提交与着色器系统」路正式化落盘（G9.0 文档集输入材料） |
