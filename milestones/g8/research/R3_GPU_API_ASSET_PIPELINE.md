# G8-R3 — GPU API/平台演进与内容资产管线前置（深度调研）

> 所属：G8 计划定稿档（`milestones/g8/`）——本文是 [G8_CAPABILITY_MATRIX.md](../G8_CAPABILITY_MATRIX.md) 与 [G8_PLAN.md](../G8_PLAN.md) 的调研输入之一。
> 与既有调研的关系：与 `deep-research/` r1~r12（语言/编译器/运行时/治理，2026-06）互补；着色器编译体系一章对 rurixc 语言与编译器设计有直接对标价值（Slang/SM 6.9/permutation/PSO）。
> 与既有面的关系：只读引用 RXS-0242~0248（RT pipeline 条款预留）、RD-034（DXIL RT 腿 blocked）、`rurix-geom-build`（既有离线几何构建器）等既有事实，不改写任何已收口结论。
> 调研基准日：2026-08-02；目标平台：Windows-first，RTX 4070 Ti；渲染主线 Vulkan，兼顾 D3D12/DXIL 与 PTX。调研方式：联网深度调研（30 余次检索与规范交叉核验，全部结论附来源 URL）。
> 纪律：零编号占用——本文不新设任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用既有编号。

## 目录

1. 结论摘要
2. 2024—2026 GPU API 与平台能力
3. 着色器语言与编译体系
4. 内容与资产管线
5. G8 能力缺口汇总总表
6. 附：调研侧实施顺序建议（输入性质）
7. 参考来源

---

# 一、结论摘要

Rurix 要达到“可支撑 UE5 级渲染器和物理引擎”的前置水平，G8 不应只追逐 Work Graphs、神经着色等前沿 API，而应优先补齐以下基础闭环：

1. **完整 RT pipeline 语言语义与 RHI**
 - 当前 RayQuery 只能覆盖内联光追。
 - UE5 级路径追踪、材质命中着色、SER、OMM 都要求 raygen/miss/closest-hit/any-hit/intersection/callable、SBT、payload/callable data、递归深度及栈管理。
 - RXS-0242～0248 和 DXIL RT 腿是 G8 的高优先级阻塞项。

2. **能力查询驱动的后端模型**
 - 2024—2026 的 GPU 特性高度碎片化：D3D12 正式、预览和实验性 Shader Model 并存；Vulkan 同时存在 KHR、EXT、NV、AMDX。
 - 语言和标准库不能直接把“RTX 4070 Ti”当作能力；必须使用 capability/profile 约束，并由后端查询真实 feature/property/tier。

3. **shader permutation、PSO 预编译和缓存**
 - 现代渲染器的主要工程难题已不只是生成 DXIL/SPIR-V，而是管理数十万潜在 shader/PSO 组合、避免运行时卡顿、建立确定性缓存键。
 - UE 的 PSO Precaching 会自动异步编译所有可能状态，但可能生成实际使用量约 4～5 倍的 PSO，仍需 permutation pruning 和 bundled cache 配合。[S20][S21]

4. **内容寻址的派生资产系统**
 - `rurix-geom-build` 已解决几何层级 DAG 的一部分，但还缺少统一 import → normalize → derive → cook → package → stream 管线。
 - 纹理压缩、VT 页烘焙、OMM、BLAS 输入、shader library、PSO manifest 都应进入同一个确定性派生数据缓存体系。

5. **前沿能力采用策略**
 - D3D12 Work Graphs：可在 RTX 4070 Ti 真机验证，适合实验和可选快速路径。
 - Vulkan `VK_AMDX_shader_enqueue`：仍为 AMD provisional 扩展，RTX 4070 Ti 无法验证，不应成为 G8 必选基线。
 - DXR 1.2 SER/OMM：RTX 4070 Ti 是合适的验证平台。
 - Cooperative Vector：Vulkan/NVIDIA 路径可实验；DirectX 原始 cooperative-vector 设计已经被微软标为将弃用并转向统一线性代数设计，暂不应固化为稳定语言 ABI。[S4]
 - DirectStorage：应建设抽象层和 GDeflate/Zstd 资产布局，但不能假设 GPU 解压总能改善帧时间；Ada 上需通过真实 workload 测量。

---

# 二、2024—2026 GPU API 与平台能力

## 2.1 D3D12 Work Graphs 与 Vulkan shader enqueue

### 能力

Work Graphs 允许 GPU shader 节点直接请求其他节点执行，payload 在节点间流动，系统负责队列和 backing memory；其目标是减少 CPU dispatch、间接参数缓冲和固定 pass 拓扑。[S1]

Mesh nodes 将 mesh shader 图形流水线作为叶节点，使 work graph 能直接驱动光栅化。D3D12 mesh nodes 在 2024 年进入预览，shader 使用实验性的 `lib_6_9` 目标。[S2]

### 两侧状态

- **D3D12**
 - 基础 Work Graphs 已进入正式 D3D12/Agility SDK 路径。
 - Windows 驱动接口从 Windows 11 24H2、WDDM 3.2 起定义。[S3]
 - NVIDIA Ampere/Ada 在 551.76+ 驱动支持基础 Work Graphs，因此 RTX 4070 Ti 可验证。[S5]
 - Mesh nodes 的最终跨厂商生产稳定度低于基础 compute nodes，仍应运行时查询 tier，而非仅根据显卡型号判断。
- **Vulkan**
 - `VK_AMDX_shader_enqueue` 可从 compute node enqueue compute workgroup，并在 revision 2 加入 mesh nodes。
 - 它明确是 **provisional、not ratified、不可视为生产稳定接口**；目前主要面向 AMD 驱动。[S6][S7]
 - `VK_EXT_device_generated_commands` 是跨厂商 DGC：GPU 生成 tokenized draw/dispatch、切换 shader/pipeline，并可在 async compute 上 preprocess；它不是 Work Graphs 的语义等价物。[S8]

### 对 UE5 级渲染器的意义

适合 GPU-driven cluster culling、材质分类、粒子/程序化生成、递归细分、meshlet 提交和不规则物理任务。但 Work Graphs 当前缺少自然的全图 join/barrier 语义；微软规范也说明初始设计没有原生“等待多个 producer 完成再运行 consumer”的概念。[S1]

因此它是补充 render graph 的局部调度工具，不应取代帧级 render graph。

### 生产采用度

- 基础 D3D12 路径已可生产试用，但尚不是跨 API、跨主流 GPU 的共同最低基线。
- Vulkan AMDX 路径仍是实验性。
- 本次未找到 Epic 官方文档确认 UE5 已把 Work Graphs 设为通用生产依赖；网络文章存在相互矛盾说法，因此“UE5 已全面生产采用”标记为**未能核实**。

---

## 2.2 DXR 1.2：SER 与 Opacity Micromap

### Shader Execution Reordering

SER 在光追 shader 中插入 invocation repack/reorder 点，根据 hit object 或 hint 重组线程，提高 ray coherence，缓解路径追踪中的严重分歧。

- **D3D12**：DXR 1.2/SM 6.9 已转为 retail；微软给出的硬件表显示 RTX 4000+ 真正执行重排，AMD RX 9000 可暴露 API 但不重排，Intel Arc B 系列可重排。[S9]
- **Vulkan**：先有 `VK_NV_ray_tracing_invocation_reorder`，后提升为跨厂商命名的 `VK_EXT_ray_tracing_invocation_reorder`，依赖 ray-tracing pipeline 和 hit-object SPIR-V 指令。[S10]
- **RTX 4070 Ti**：Ada 架构原生 SER，可真机验证。NVIDIA 架构文档给出高度分歧 RT shader 最多约 2×、Cyberpunk RT Overdrive 整体最高约 44% 的厂商测试结果；这些数字不能外推为普遍收益。[S11]

### Opacity Micromap

OMM 将三角形细分成 microtriangle，并以 1 bit（2-state）或 2 bit（4-state）编码 opaque/transparent/unknown，避免树叶、栅栏等 alpha-tested 几何频繁进入 any-hit shader。

- **D3D12**：DXR 1.2 retail；RTX 4000+ 硬件加速，旧 RTX 可软件模拟。[S9]
- **Vulkan**：`VK_EXT_opacity_micromap` 提供 micromap build、存储、BLAS attachment 和 pipeline flag。[S12]
- **RTX 4070 Ti**：可真机验证硬件路径。

### Position Fetch

`VK_KHR_ray_tracing_position_fetch`/早期 NV 路径允许 hit shader 从 acceleration structure 获取命中三角形位置，减少必须另行绑定顶点缓冲的场景。它对压缩几何和统一几何/RT 数据表示有价值，但不能替代完整材质属性获取。

### 意义

SER、OMM、position fetch 都建立在完整 RT pipeline 之上。对 Rurix 而言，先补 RT pipeline、SBT、payload ABI 和 AS 生命周期，再做这些优化，否则会形成只有扩展枚举、没有可编程语义的“空 RHI”。

---

## 2.3 Cooperative Vector 与神经着色

### 能力

Cooperative vector 让 shader 用向量—矩阵运算调用 Tensor Core/矩阵硬件，避免开发者手动安排 subgroup 分片、矩阵 packing 和同步。主要用途包括：

- 神经材质与 BRDF 近似；
- Neural Texture Compression；
- radiance cache/field；
- 小型 MLP 推理和 shader 内训练；
- 超分辨率和降噪辅助网络。

NVIDIA RTX Neural Shaders SDK 支持 shader 内训练和推理，官方样例最低为 Turing+，因此 RTX 4070 Ti 可运行。[S13]

### 两侧状态

- **DirectX**
 - Cooperative Vector 曾作为 SM 6.9 实验能力发布。
 - 微软随后宣布原设计将弃用，转向统一支持 vector-matrix 与 matrix-matrix 的新线性代数设计；替代设计未随 SM 6.9 retail 一起发布。[S4]
 - 因此不宜把当前 DirectX preview ABI 固化进 Rurix 稳定标准。
- **Vulkan**
 - `VK_NV_cooperative_vector` 是神经着色相关的 NVIDIA 扩展；另有跨厂商 `VK_KHR_cooperative_matrix`，但 cooperative matrix 是 subgroup 分布式矩阵，不等同于 per-invocation cooperative vector。
 - 必须查询实现支持的数据类型、布局、转置和训练能力。
- **Slang**
 - Slang 已提供 cooperative-vector 和 autodiff 相关工作流，是 RTX Neural Shaders 的主要语言入口。[S14]

### 语言设计建议

Rurix 不应直接暴露某厂商内建名称，而应先定义：

- `coop_vector<T, N>` 或更一般的 `tensor_fragment` 类型；
- 显式矩阵 layout、component interpretation、accumulate type；
- `capability(cooperative_vector)`；
- 可降级到普通 ALU 的标准库实现；
- 后端 intrinsic 层，隔离未来 DirectX ABI 变化。

自动微分对 G8 不是运行时渲染硬门槛，但若目标包含可训练神经材质，应预留 differentiability effect、tangent/adjoint IR 和自定义导数接口。

---

## 2.4 Vulkan 1.4 与关键扩展

Vulkan 1.4 于 2024-12-03 发布；核心目标是把 Roadmap 2022/2024 中已经成熟的能力和最低限制纳入一致基线。它保证 8K、最多 8 个 render targets，纳入 maintenance 5/6、push descriptor、dynamic-rendering local read、scalar block layout 等。[S15]

关键能力：

- **`VK_EXT_device_generated_commands`**
 - GPU 生成 draw/dispatch、vertex/index binding、shader/pipeline 选择。
 - 适合 GPU-driven renderer，是比 AMDX shader enqueue 更应优先实现的 Vulkan 能力。
- **`VK_EXT_descriptor_buffer`**
 - descriptor 作为内存 blob，通过 buffer address/offset 绑定，可由 host 或 device 更新。
 - 与 Rurix 已有 bindless 和布局推导直接相关，应作为 Vulkan 高性能 descriptor 后端。[S16]
- **`VK_EXT_shader_object`**
 - 独立创建、绑定 shader stage，大部分 graphics state 改为 dynamic。
 - 可减少完整 PSO 组合，但不能消除驱动编译和 shader interface 兼容问题。[S17]
- **maintenance 5/6**
 - 大量 API 可用性、格式、descriptor 和 dynamic-rendering 修补；应纳入 Vulkan 1.4 profile，而非逐项向语言暴露。
- **Host Image Copy**
 - CPU 直接在 host 上复制到 optimal image，无需 staging buffer/command buffer。
 - Vulkan 1.4 虽已 core，功能仍是 optional；实现必须提供 host image copy 或额外 dedicated transfer queue。[S18]
 - 对 UMA 很重要；在 RTX 4070 Ti 独显上主要是便利路径，不应假设优于 copy queue。
- **Ray-tracing position fetch**
 - 降低 RT shader 获取命中三角形位置的资源绑定成本。
- **Pipeline binary**
 - `VK_KHR_pipeline_binary` 允许显式取得和复用 pipeline binary；有效性依赖 device、enabled feature/layer 等 global key。[S22]

### RTX 4070 Ti

当前 NVIDIA 驱动可提供 Vulkan 1.4 以及 descriptor buffer、shader object、DGC 等多数能力，但最终必须通过 feature/property query 建立测试记录。`VK_AMDX_shader_enqueue` 明确不可用。Host Image Copy、具体 DGC token 和 shader-object feature 需逐驱动查询，不能写死。

---

## 2.5 DirectStorage、GPU 解压和 NVMe

### DirectStorage 状态

- DirectStorage 1.1 引入 GDeflate 和 GPU decompression；所有 D3D12 + SM 6.0 GPU均有 fallback，厂商可提供优化 metacommand。[S23]
- 运行时可能选择硬件优化 GPU、DirectCompute fallback 或 CPU fallback；1.2 增加路径查询。[S24]
- 1.3（2025）加入批量 `EnqueueRequests` 和 D3D12 fence 协调，便于保证 texture load 与 tile mapping 顺序。[S25]
- 1.4 在 2026 年仍为 public preview，引入 Zstd、Game Asset Conditioning Library 和 CreatorID 队列归组。[S26]

GDeflate 参考实现为 Apache 2.0，可进入 Rurix 的离线打包工具。[S23]

### Vulkan/跨平台对应

Vulkan 没有跨厂商标准化的“文件系统到 GPU”API：

- NVIDIA 提供 `VK_NV_memory_decompression` 和 `VK_NV_copy_memory_indirect`，Windows/Linux 可用，但属于 NV vendor path。[S27]
- GPUDirect Storage 提供 NVMe/网络存储到 GPU memory 的 direct DMA，但仅支持 Linux x86-64，不适合 Rurix 的 Windows 基线。[S28]
- 跨平台基线仍应是异步文件 I/O → pinned/staging memory → transfer queue → compute/CPU 解压。

### 生产评价

2025 年公开报道显示 DirectStorage PC 游戏采用仍有限，代表包括 Forspoken、Ratchet & Clank、Forza Motorsport、Horizon Forbidden West、Spider-Man 2。[S29] 部分 Ada 游戏测试中 GPU 解压会与渲染争抢 GPU，降低 1% low；因此应把 GPU decompression 当作调度型 workload，而不是无条件快路径。[S30]

### 对架构的影响

资产应按可独立请求、解压和上传的块组织：

- 64 KiB 左右的压缩/稀疏页粒度；
- 每块校验和、目标资源、目标 mip/tile 和依赖；
- I/O、解压、copy、tile map 分离 timeline；
- GPU 繁忙时可切换 CPU 解压；
- render graph 能表示 decompression queue 与 graphics 的竞争和 barrier；
- 需要 residency budget 与优先级，而不仅是“异步读文件”。

RTX 4070 Ti 可验证 DirectStorage GDeflate 和 NVIDIA Vulkan 解压路径。

---

## 2.6 Sparse residency、Sampler Feedback 与虚拟纹理

- D3D12 Tiled Resources 提供 reserved resource、tile pool 和 GPU timeline tile mapping。Sampler Feedback 把 shader 实际需要的 mip 写入 feedback map；微软建议至少 Tier 2 才适合可用的 streaming 实现。[S31]
- Tiled Resources Tier 4 允许 `64KB_UNDEFINED_SWIZZLE` texture array 搭配完整 mip chain。[S32]
- Vulkan sparse image 需要 `sparseBinding`、`sparseResidencyImage2D`；若 shader 要检测 resident texel，还需 `shaderResourceResidency`。[S33]
- Vulkan 没有与 D3D12 Sampler Feedback 完全一致且普及的跨厂商核心能力。生产实现通常需要 shader feedback image/atomic bitset 作为通用路径。

RTX 4070 Ti 可验证 D3D12 sampler feedback、tiled resources 和 Vulkan sparse residency。

UE5 级 VT 还要求软件层具备 feedback 去重、页优先级、LRU、mip fallback、物理 atlas、页表双缓冲、边界 texel、压缩页和 I/O 预算；只有 sparse API 不构成完整虚拟纹理系统。

---

## 2.7 VRS、多队列与现代 present

### VRS

D3D12 VRS Tier 1 提供 per-draw rate；Tier 2 增加 per-primitive 和 screen-space shading-rate image。[S34] Vulkan 对应 `VK_KHR_fragment_shading_rate`。RTX 4070 Ti 可验证 Tier 2/fragment shading rate。

需要语言支持 shading-rate builtins，RHI 支持 combiner、attachment 和 tile size 查询；引擎还要基于运动、材质频率、VR gaze 或 upscaler mask 生成 rate image。

### 多队列和异步计算

Vulkan timeline semaphore 是单调增长的 64 位计数器，支持 host/device signal/wait 和 wait-before-signal，可统一管理 graphics/compute/copy timeline。[S35]

但“存在独立 compute queue”不等于物理并行：真正收益取决于硬件引擎、资源带宽、occupancy 和同步粒度。Render graph 应基于 profiling 决定 queue placement，并支持回退到单队列。

RTX 4070 Ti 可完整验证 Vulkan timeline semaphore、D3D12 fence、async compute/copy。

### Swapchain、HDR 与低延迟

- `VK_KHR_present_wait`/present ID 可限制未显示帧数量并控制 pacing。[S36]
- 2025 年的 `VK_KHR_swapchain_maintenance1` 增加 per-present mode、present fence、延迟分配、释放未 present image 和更平滑的 resize 处理。[S37]
- HDR 需要 surface format/colorspace negotiation、`VK_EXT_hdr_metadata`、DXGI HDR metadata，并要求引擎具备 scene-linear、paper white、tone mapping 和 UI compositing。
- NVIDIA Reflex 将 CPU/GPU 工作对齐到 just-in-time，并提供 click-to-photon markers；RTX 4070 Ti 可用。[S38]
- AMD Anti-Lag 2 从驱动功能转为引擎集成，Vulkan 支持要求 AMD 24.9.1+；RTX 4070 Ti 无法真机验证 AMD 路径。[S39]

## 本章：对 Rurix 的前置能力要求清单

### A. 语言/编译器级

- 完整 RT pipeline 阶段、payload、attribute、callable data、SBT record 类型；**4070 Ti：可验证**。
- Work Graph node、record、node array、launch mode、递归和 backing-memory 语义；**D3D12 可，Vulkan AMDX 不可**。
- mesh-node 及 graphics state association；**部分可，具体 tier 需查询**。
- OMM、SER hit-object/reorder intrinsic；**可验证**。
- cooperative vector/tensor 类型和 capability；**Vulkan NV 可实验，D3D 稳定 ABI 暂不可**。
- sparse residency result、VRS、position-fetch builtins；**可验证**。
- target capability/profile、可选 feature 约束和 fallback specialization；**可验证**。

### B. 运行时/RHI 级

- DXR/Vulkan RT pipeline、SBT、pipeline library、stack sizing。
- DGC/ExecuteIndirect、Work Graph backing memory。
- Descriptor Buffer、Shader Object、Pipeline Binary。
- sparse/tiled resource、tile mapping、feedback readback。
- DirectStorage 与 Vulkan/CPU I/O 抽象、GPU/CPU decompression 调度。
- graphics/compute/copy timeline、queue ownership 和跨队列 barrier。
- HDR、present ID/wait、swapchain maintenance、Reflex/Anti-Lag 接口。
- 除 AMDX 和 AMD Anti-Lag 外，RTX 4070 Ti 均可进行至少一种后端验证。

### C. 引擎库级

- GPU-driven culling/material bucketing/meshlet dispatch。
- VT residency manager 和 sampler-feedback fallback。
- async-compute 调度器及 profiling 决策。
- RT scene、BLAS/TLAS cache、OMM residency。
- latency marker、frame pacing、HDR color pipeline。
- GPU decompression budgeter，避免与关键渲染 pass 抢占。

### D. 工具/资产管线级

- OMM 离线烘焙。
- GDeflate/Zstd packer、chunk manifest、校验和。
- VT tile、border、mip-tail 和页目录生成。
- RT 几何布局和可选 position-fetch 数据路径。
- feature profile 驱动的多平台 cook。

---

# 三、着色器语言与编译体系

## 3.1 Slang 对 Rurix 的直接启示

Slang 于 2024-11 转入 Khronos 托管和多公司治理，项目使用 Apache 2.0。[S40] 其核心能力包括：

- modules、访问控制和独立编译；
- generics、interfaces、associated constraints；
- capability system，在类型检查阶段阻止目标不支持的能力；
- DXIL、SPIR-V、CUDA、Metal、WGSL 等多后端；
- reflection API、LSP、RenderDoc/SPIR-V 调试；
- forward/reverse autodiff；
- 模块离线编译为 IR，运行时链接为最终 shader。[S41]

成熟度需要谨慎看待：Slang 官方将 modules 标为 stable，但 first-order autodiff、direct-SPIR-V mesh shader、multi-entrypoint SPIR-V 等仍标为 experimental。[S42]

### 对 Rurix 的启示

Rurix 的“双层安全宿主 + kernel 子语言”比 Slang 更适合统一 GPU 系统编程，但目前至少应对齐：

1. 模块签名和独立编译；
2. 泛型/interface specialization；
3. target capability；
4. entry-point 组合与链接；
5. 结构化 reflection；
6. source-level debug map；
7. 可序列化的中间 IR；
8. 后端无关 resource layout；
9. 显式 specialization/permutation domain；
10. 标准库版本作为构建输入。

---

## 3.2 HLSL Shader Model 6.8/6.9

### SM 6.8

- `SV_StartVertexLocation`；
- `SV_StartInstanceLocation`；
- `[WaveSize(min,max,preferred)]`，允许后端在范围内选择 wave size。[S43]

Rurix 应支持 wave-size range，而不是只有固定 subgroup size；固定值会过度绑定厂商。

### SM 6.9

- 5～1024 元素 long vector；
- DXIL 1.9 native vector；
- native 16-bit ops、wave ops、int64 ops 变为符合 SM 6.9 实现的必需能力；
- SER/OMM 的 HLSL 暴露。[S9][S44]

Cooperative Vector 原计划属于 SM 6.9，但其原始设计没有进入最终统一稳定方案，需与 long vector 区分。[S4]

---

## 3.3 WGSL 的边界

WGSL 的优势是严格验证、确定行为和 WebGPU 可移植性，但不是 UE5 级原生渲染语言的合适上界：

- 没有 Slang 式泛型/interface/module specialization；
- pointer 使用受限；
- core 不含 ray tracing、mesh/task shader、work graph；
- 明确限制 private/function/workgroup memory、类型嵌套和 entry-point 资源；
- 目标是 WebGPU 安全子集，而不是暴露全部 D3D12/Vulkan 能力。[S45]

WGSL 可以作为未来的受限输出目标，但不应反向限制 Rurix IR 和类型系统。

---

## 3.4 Shader permutation 与 PSO

UE 的关键实践：

- `ShouldCompilePermutation` 在编译前排除无效组合。
- Material Shader Map 以静态参数集标识和缓存材质 shader 集合。[S46]
- Material Analyzer 找出冗余 static switch/component mask，减少 permutation 和存储。[S47]
- PSO Precaching 在 component `PostLoad` 后收集可能使用的 graphics PSO，并异步编译；global compute permutations 可在启动时预编译。[S20]
- Bundled PSO Cache 来源于运行采集，更精确；PSO Precaching 覆盖面更大但会过度编译。[S21]
- Vulkan `VkPipelineCache` 可跨 pipeline 和跨进程运行复用驱动编译结果，但内容由实现管理、并非跨设备通用资产。[S48]
- `VK_KHR_pipeline_binary` 提供更显式的 pipeline binary 复用，但仍需 global key 校验。[S22]

### Rurix 应采用的分层缓存

1. **源码/模块缓存**：规范化源码及依赖哈希。
2. **泛型 specialization 缓存**：类型参数、常量参数、capability profile。
3. **后端 IR 缓存**：DXIL/SPIR-V/PTX，含编译器和 validator 版本。
4. **Shader library 缓存**：entry point、resource layout、RT export。
5. **PSO descriptor 缓存**：shader hashes + render state + formats + specialization。
6. **驱动 binary/cache**：GPU/driver/API/feature-key 绑定，不进入通用 cook。
7. **运行遥测**：实际使用 PSO、miss、compile stall、预缓存覆盖率。

## 一门 UE5 级着色语言/编译器必须具备的编译面

- 全 shader stage 与 RT pipeline。
- 模块、包、访问控制、独立编译和链接。
- 泛型、interface/trait、associated type/value constraints。
- capability/profile 与跨后端 fallback。
- address space、pointer、atomic、memory model 和 subgroup 语义。
- resource/interface reflection；稳定、可验证的 ABI/layout。
- specialization constant、permutation domain 和 pruning。
- 多 entry point、shader library、pipeline library。
- work graph node、mesh node、cooperative vector。
- source map、诊断、LSP、调试信息、反汇编和 IR validator。
- deterministic mode、标准化输入和内容哈希。
- 编译统计：寄存器、shared memory、occupancy hint、instruction class。
- 缓存可序列化、版本化和远程共享。
- 可选 autodiff 与自定义导数。

## 本章：对 Rurix 的前置能力要求清单

### A. 语言/编译器级

- 模块/独立编译/链接；**4070 Ti 无关，可主机测试**。
- trait/interface 泛型 specialization。
- capability profile 和 target-conditional API。
- permutation domain、静态裁剪和 permutation budget。
- RT library、SBT ABI、Work Graph 和 cooperative-vector IR。
- wave-size range、long vector/native vector。
- 稳定 reflection schema 和 shader interface hash。
- 可重复编译模式及 IR/source-map 调试。

### B. 运行时/RHI 级

- 异步 shader/PSO 编译服务。
- pipeline cache/binary 的设备及驱动隔离。
- shader hot reload 与旧 pipeline 安全退役。
- compile-required 检测和 fallback PSO。
- RT/graphics/compute pipeline library。
- **RTX 4070 Ti 可验证全部主路径；AMDX 除外。**

### C. 引擎库级

- Material permutation manager。
- ShaderMap/ShaderLibrary。
- PSO collector、precacher、bundled cache 和遥测。
- 加载屏障：关键 PSO 未完成时阻止资源可见，或使用 fallback material。
- 远程 shader compile/DDC client。

### D. 工具级

- shader dependency scanner。
- permutation analyzer/budget 报告。
- DXIL/SPIR-V/PTX 验证与反汇编。
- PSO manifest 合并、去重和覆盖率分析。
- 编译产物 reproducibility diff 工具。

---

# 四、内容与资产管线

## 4.1 UE5 资产管线对标

UE 的典型链路：

1. **Import**
 - 读取 DCC/交换格式；
 - 保存源数据、import settings、依赖和编辑器资产描述。
2. **Derived Data Cache**
 - 生成 shader、压缩纹理、mesh 派生表示；
 - DDC 内容可删除、可从 source asset 再生，不应作为源资产进入版本控制。
 - UE 5.4+ 默认使用 Zen Store 作为本地 DDC。[S49]
3. **Cook**
 - 将内部资产转换为目标平台格式；
 - 可生成 streaming install manifest，并执行 asset validation。[S50]
4. **Package/Stage**
 - 生成平台容器、chunk、asset registry。
5. **Runtime Stream**
 - 按包、mip、VT tile、几何 cluster、world partition cell 异步加载。
 - Zen cooked snapshot 可导出/导入 cook 结果，并支持 Zen Streaming。[S51]

Cloud DDC 使用 content-addressable compact binary object 和 blob replication；Epic 建议 DDC 更偏向预复制而非跨区域 miss 时同步拉取，因为后者延迟不稳定。[S52]

### Rurix 对标模型

建议定义：

- `SourceAsset`：不可变源 blob + logical URI。
- `ImportRecipe`：导入器版本、坐标系、单位、颜色空间。
- `DerivedArtifact`：内容哈希寻址，完全可再生。
- `CookProfile`：平台、GPU profile、格式、质量等级。
- `PackageChunk`：对齐、压缩、依赖和 streaming priority。
- `RuntimeResource`：GPU 上传布局、residency 页和 fallback。
- `BuildManifest`：所有输入/工具/配置哈希及 SBOM/许可证。

---

## 4.2 网格导入与 meshlet 构建

`meshoptimizer` 是 MIT 许可的成熟 C/C++ 库，支持：

- vertex/index cache 优化；
- meshlet 构建及 cone/bounds；
- attribute-aware simplification；
- mesh compression；
- cluster partition；
- `clusterlod.h` 连续 LOD、DAG BVH，目标类似 Nanite。[S53][S54]

2026 年 v1.2 还加入 meshlet codec、DAG BVH 和面向未来 DXR compressed position 的共享指数支持。[S55]

### 对 rurix-geom-build 的建议

现有“meshlet 化 → 分组简化层级 DAG”方向正确，不建议替换为黑盒。可将 meshoptimizer 作为：

- 导入后拓扑清理和参考实现；
- meshlet partition/simplification 后端；
- codec 和 benchmark 基准；
- 输出质量交叉验证。

仍需自研：

- 稳定的 DAG 文件格式和版本；
- cluster page packing/residency；
- 材质边界、skin/morph、tangent 和 UV seam 策略；
- Nanite 类误差传播、屏幕误差和 cut selection；
- BLAS/OMM 派生数据；
- 确定性并行调度和稳定排序。

---

## 4.3 纹理压缩

### 推荐格式

- Windows/Desktop：BC1～BC7；HDR 重点 BC6H。
- 跨平台/mobile：ASTC。
- 传输/通用资产：KTX2 + Basis Universal。
- 普通贴图应使用线性空间、特殊 mip filter 和通道权重，不能套用颜色纹理默认参数。

### 可 vendor

- **Basis Universal**：Apache 2.0；KTX2/`.basis`，支持 ETC1S、UASTC、HDR 路径，并快速转码到 BC/ASTC/ETC。[S56]
- **KTX-Software**：KTX2 创建、编码、转码、验证工具；Apache 2.0 主线，但需审核仓库内第三方许可证。[S57]
- **Arm astc-encoder**：Apache 2.0，建议 pin 5.x stable tag。[S58]
- **AMD Compressonator**：支持 BC1～7、ASTC、ETC 和 GPU/OpenCL/DX compute；仓库组件许可证分散，vendor 前必须逐目录审计，不能仅凭 GPUOpen 标签认定整体为单一 MIT/Apache。[S59]
- **NVTT 3**：CUDA 加速 BC1～7/ASTC，RTX 4070 Ti 可运行，但属于 NVIDIA SDK，不符合“仅 MIT/Apache/BSD 类源码 vendor”要求，可作为可选外部工具，不应成为可复现构建基线。[S60]

### 需要自研

- 纹理语义和颜色空间识别；
- mip generation；
- normal map renormalization；
- alpha coverage preservation；
- 平台格式选择；
- encoder quality/profile 固定；
- deterministic wrapper、校验和和质量指标；
- GPU 编码可选加速但 CPU 编码作为确定性基准。

---

## 4.4 虚拟纹理烘焙

UE 区分：

- **Streaming VT**：texel 离线 cook，从磁盘按 tile 流入。
- **Runtime VT**：GPU 运行时生成；低分辨率 mip 可离线烘焙为 streaming VT。[S61][S62]

Rurix VT builder 应生成：

- 固定 tile 和 border；
- 各 mip 页；
- mip tail；
- 每 layer 格式；
- page directory；
- 页级压缩 blob；
- fallback color/normal；
- hash 与 dependency；
- feedback 到页 ID 的稳定映射。

需要自研的核心不是图片切块，而是不同 layer 同步 residency、页表 ABI、feedback 降噪、prefetch、压缩页寻址、页优先级和物理缓存回收。

RTX 4070 Ti 可验证 D3D12 sampler feedback 和 Vulkan shader-feedback fallback。

---

## 4.5 glTF、OpenUSD 与 MaterialX

### glTF

glTF 2.0 仍是运行时资产交付的首选交换格式；KTX2/Basis、Draco/meshopt、GPU instancing 已成熟。2025—2026 的 Gaussian splat、voxel、physics、PBR Next 等仍处于不同草案阶段，不能当作稳定导入契约。[S63][S64]

可 vendor：

- `cgltf`、`tinygltf`、`fastgltf` 均为 MIT 类选择；其中 fastgltf 为 C++17 且依赖 simdjson，Rust 项目需 FFI 或参考其解析策略。[S65]
- 对 Rust-first 的 Rurix，长期更适合自研严格 glTF schema/validator 或选用经过审计的 Rust crate，但需锁定扩展集合。

### OpenUSD

OpenUSD 提供 scalable scene composition、references、variants、payload、time samples 和 asset resolver，适合 DCC/大型场景互换，不适合作为运行时 GPU 资源格式直接消费。[S66]

2025—2026 重点是 Ar 2.0：

- URI scheme 多 resolver；
- asset read/write；
- resolver interface 重构；
- 初始发布由 build flag 开启，随后计划成为默认。[S67]

许可是 TOST 1.0，条款基本沿袭 Apache 2.0但商标条款不同；可视为 Apache-like，仍需法律清单明确标注，不能写成“Apache-2.0”。[S68]

建议：G8 先做受限 USD ingest/export adapter，不把完整 USD composition 内嵌进运行时。

### MaterialX

MaterialX 当前 1.39.x，Apache 2.0，提供标准材质 node graph 和 ShaderGen；可生成 GLSL、OSL、MDL、MSL，2025 工作包括 Slang codegen 和 OpenUSD/OpenPBR 集成。[S69][S70]

建议：

- vendor MaterialX schema、stdlib 和验证器；
- 自研 `MaterialX → Rurix typed material IR`；
- 不直接依赖 MaterialX 生成的 GLSL 作为最终生产路径；
- 对不支持 node 提供清晰诊断和 bake fallback。

---

## 4.6 确定性构建与缓存

每个 artifact key 至少包含：

- 源文件内容 hash；
- 所有间接依赖 hash；
- importer/compiler/encoder 精确版本；
- 标准库和 schema 版本；
- cook profile、GPU capability profile；
- 坐标系、单位、颜色空间；
- 浮点模式和质量参数；
- 目标格式及 layout ABI；
- build recipe hash。

确定性要求：

- canonical serialization；
- map/set 稳定排序；
- 并行任务结果按稳定 ID 合并；
- 禁止 timestamp、绝对路径、随机种子进入输出；
- 固定 NaN、负零和浮点量化策略；
- 每次构建生成 manifest 和输入证明；
- CI 对同一输入执行双构建并比较 hash；
- GPU 加速工具若不能跨驱动稳定复现，则输出只能作为本地 cache，不作为签名 cook artifact。

Rurix 已有 lockfile、vendor、checksum 且禁止任意构建脚本，这是优势。应把资产 transform 变为声明式、版本化的受限工具调用，而不是放开 package build script。

## 本章：对 Rurix 的前置能力要求清单

### A. 语言/编译器级

- 稳定 shader/material reflection 和 layout hash。
- 可导出的 typed material IR。
- feature profile 控制材质和纹理 cook。
- shader library/PSO manifest 格式。
- **硬件无关；均可 CI 验证。**

### B. 运行时/RHI 级

- 统一异步资源请求、upload、decompression 和 residency。
- sparse/tiled resource 与 VT page table。
- meshlet page、BLAS/OMM cache。
- asset handle generation、热更新和安全退役。
- RTX 4070 Ti 可验证实际 streaming 和 residency。

### C. 引擎库级

- Asset Registry、dependency graph。
- Content-addressed DDC。
- cook/package/chunk 系统。
- VT manager、mesh cluster streamer。
- MaterialX 材质实例和 shader permutation 生成。
- glTF/USD scene conversion。

### D. 工具/资产管线级

- glTF importer：优先 vendor MIT loader 或自研 Rust parser。
- USD adapter：OpenUSD/TOST 1.0，受限集成。
- MaterialX：Apache 2.0，可 vendor schema/stdlib。
- meshoptimizer：MIT，可 vendor。
- Basis Universal/KTX/astcenc：Apache 2.0，可 vendor。
- NVTT 3：仅可选外部工具。
- VT baker、OMM baker、texture semantic pipeline、determinism verifier：需要自研。

---

# 五、G8 能力缺口汇总总表

| 优先级 | 档位 | 能力 | 当前判断 | RTX 4070 Ti |
|---|---|---|---|---|
| P0 | A/B | 完整 RT pipeline、SBT、payload ABI | 当前明确缺口，阻塞 SER/OMM/路径追踪 | 可 |
| P0 | A | capability/profile 类型检查 | 三后端和扩展碎片化的基础 | 可 |
| P0 | A/C | permutation domain、裁剪、预算 | UE5 级 shader 规模必需 | 可 |
| P0 | B/C | PSO precache、pipeline binary/cache | 消除 shader hitch 必需 | 可 |
| P0 | C/D | 内容寻址 DDC + 声明式 cook | 资产闭环核心 | 可 |
| P0 | B/C | async I/O、upload、residency timeline | 大世界流送核心 | 可 |
| P1 | B/C/D | sparse VT + feedback + baker | UE5 级纹理内存核心 | 可 |
| P1 | C/D | meshlet page/cluster streamer | 已有 builder，缺运行时闭环 | 可 |
| P1 | D | BCn/ASTC/KTX2/Basis pipeline | 可大量 vendor | 可 |
| P1 | A/B/D | OMM 编程与烘焙 | foliage RT 性能关键 | 可 |
| P1 | A/B | SER/hit-object | 路径追踪优化 | 可 |
| P1 | B | descriptor buffer/shader object/DGC | Vulkan GPU-driven 优化 | 可，需查询 |
| P1 | B/C | 多队列调度与 timeline | 渲染/物理/解压并发 | 可 |
| P1 | B/C | HDR、present pacing、Reflex | 生产显示与低延迟 | 可 |
| P2 | A/B | D3D12 Work Graph compute nodes | 可选高性能路径 | 可 |
| P2 | A/B | Work Graph mesh nodes | 仍需 feature query/兼容路径 | 部分 |
| P2 | A/B | Cooperative Vector | API 仍演进，不宜稳定化 | 部分 |
| P2 | A | autodiff | 神经材质有价值，非渲染基线 | 可 |
| P3 | A/B | Vulkan AMDX shader enqueue | provisional、AMD-only | 不可 |
| P3 | B | GPUDirect Storage | Linux-only，不合 Windows 基线 | 不可 |

---

# 六、附：调研侧实施顺序建议（输入性质）

*本章仅为调研输入，其分期编号（G8.1~G8.4）不构成计划裁决；正式波次划分以 [G8_PLAN.md](../G8_PLAN.md) 为准。*

### G8.1：可生产的 shader/RT 基座

1. 完整 RT pipeline 语义和 Vulkan SBT。
2. DXIL RT 后端解阻。
3. capability/profile 系统。
4. 模块独立编译、reflection、shader interface hash。
5. permutation domain 和 analyzer。
6. PSO precache/cache/binary。

### G8.2：资产构建闭环

1. SourceAsset/Recipe/Artifact/CookProfile schema。
2. 内容寻址 DDC。
3. glTF importer。
4. meshoptimizer 对接和现有 DAG 格式固化。
5. BCn/ASTC/KTX2/Basis。
6. package/chunk/manifest 和确定性验证。

### G8.3：大世界流送

1. async I/O + transfer queue。
2. DirectStorage GDeflate 和 CPU fallback。
3. sparse/tiled residency。
4. VT baker、feedback 和 page cache。
5. meshlet/BLAS/OMM page streaming。

### G8.4：现代 GPU 快速路径

1. SER、OMM、position fetch。
2. Vulkan DGC、descriptor buffer、shader object。
3. D3D12 Work Graphs。
4. cooperative vector 实验 profile。
5. Reflex/HDR/frame pacing。

关键决策是：**Work Graphs 和神经着色应建立在完整编译、缓存、资产和 RT 基础之上，而不应反过来成为 G8 的主干。**

---

# 七、参考来源

- [S1] D3D12 Work Graphs 规范：https://microsoft.github.io/DirectX-Specs/d3d/WorkGraphs.html
- [S2] D3D12 Mesh Nodes：https://devblogs.microsoft.com/directx/d3d12-mesh-nodes-in-work-graphs/
- [S3] Windows Work Graphs DDI：https://learn.microsoft.com/en-us/windows-hardware/drivers/display/work-graphs
- [S4] SM 6.9 与 Cooperative Vector 后续：https://devblogs.microsoft.com/directx/shader-model-6-9-and-the-future-of-cooperative-vector/
- [S5] NVIDIA D3D12 Work Graphs：https://developer.nvidia.com/blog/advancing-gpu-driven-rendering-with-work-graphs-in-direct3d-12/
- [S6] `VK_AMDX_shader_enqueue`：https://github.khronos.org/Vulkan-Site/refpages/latest/refpages/source/VK_AMDX_shader_enqueue.html
- [S7] AMD Vulkan Work Graph Mesh Nodes：https://gpuopen.com/learn/gpu-workgraphs-mesh-nodes-vulkan/
- [S8] `VK_EXT_device_generated_commands`：https://github.khronos.org/Vulkan-Site/refpages/latest/refpages/source/VK_EXT_device_generated_commands.html
- [S9] SM 6.9/DXR 1.2 retail：https://devblogs.microsoft.com/directx/shader-model-6-9-retail-and-more/
- [S10] Vulkan RT invocation reorder：https://registry.khronos.org/vulkan/specs/latest/man/html/VK_EXT_ray_tracing_invocation_reorder.html
- [S11] NVIDIA Ada 架构：https://images.nvidia.com/aem-dam/Solutions/geforce/ada/nvidia-ada-gpu-architecture.pdf
- [S12] Vulkan Opacity Micromap：https://registry.khronos.org/vulkan/specs/latest/man/html/VK_EXT_opacity_micromap.html
- [S13] RTX Neural Shaders：https://developer.nvidia.com/blog/get-started-with-neural-rendering-using-nvidia-rtx-kit/
- [S14] Neural Shading/Slang：https://developer.nvidia.com/blog/how-to-get-started-with-neural-shading-for-your-game-or-application/
- [S15] Vulkan 1.4 发布：https://www.khronos.org/news/press/khronos-streamlines-development-and-deployment-of-gpu-accelerated-applications-with-vulkan-1.4
- [S16] Descriptor Buffer：https://docs.vulkan.org/features/latest/features/proposals/VK_EXT_descriptor_buffer.html
- [S17] Shader Object：https://docs.vulkan.org/features/latest/features/proposals/VK_EXT_shader_object.html
- [S18] Host Image Copy：https://github.khronos.org/Vulkan-Site/refpages/latest/refpages/source/VK_EXT_host_image_copy.html
- （S19 空缺为源材料跳号，非遗漏）
- [S20] UE PSO Precaching：https://dev.epicgames.com/documentation/en-us/unreal-engine/pso-precaching-for-unreal-engine
- [S21] UE shader stutter/PSO：https://dev.epicgames.com/community/learning/tutorials/xjzE/unreal-engine-epic-for-indies-game-engines-shader-stuttering-ue-s-solution
- [S22] Vulkan Pipeline Binary：https://docs.vulkan.org/features/latest/features/proposals/VK_KHR_pipeline_binary.html
- [S23] DirectStorage 1.1/GDeflate：https://devblogs.microsoft.com/directx/directstorage-1-1-now-available/
- [S24] DirectStorage 1.2：https://devblogs.microsoft.com/directx/directstorage-1-2-available-now/
- [S25] DirectStorage 1.3：https://devblogs.microsoft.com/directx/directstorage-1-3-is-now-available/
- [S26] DirectStorage 1.4 preview：https://devblogs.microsoft.com/directx/directstorage-1-4-release-adds-support-for-zstandard/
- [S27] RTX IO/Vulkan decompression：https://developer.nvidia.com/rtx-io
- [S28] GPUDirect Storage：https://developer.nvidia.com/gpudirect-storage
- [S29] DirectStorage 采用情况：https://www.pcworld.com/article/2609584/what-happened-to-directstorage-why-dont-more-pc-games-use-it.html
- [S30] DirectStorage GPU 解压实测：https://www.tomshardware.com/pc-components/gpus/testing-directstorage-with-gpu-decompression-do-blackwell-gpus-have-the-upper-hand
- [S31] D3D12 Sampler Feedback：https://microsoft.github.io/DirectX-Specs/d3d/SamplerFeedback.html
- [S32] Tiled Resources Tier 4：https://microsoft.github.io/DirectX-Specs/d3d/D3D12TiledResourceTier4.html
- [S33] Vulkan Sparse Image：https://docs.vulkan.org/samples/latest/samples/extensions/sparse_image/README.html
- [S34] D3D12 VRS：https://microsoft.github.io/DirectX-Specs/d3d/VariableRateShading.html
- [S35] Vulkan Timeline Semaphore：https://docs.vulkan.org/refpages/latest/refpages/source/VK_KHR_timeline_semaphore.html
- [S36] Vulkan Present Wait：https://docs.vulkan.org/refpages/latest/refpages/source/VK_KHR_present_wait.html
- [S37] Swapchain Maintenance 1：https://docs.vulkan.org/refpages/latest/refpages/source/VK_KHR_swapchain_maintenance1.html
- [S38] NVIDIA Reflex：https://developer.nvidia.com/performance-rendering-tools/reflex
- [S39] AMD Anti-Lag 2：https://gpuopen.com/anti-lag-2/
- [S40] Khronos Slang Initiative：https://www.khronos.org/news/press/khronos-group-launches-slang-initiative-hosting-open-source-compiler-contributed-by-nvidia
- [S41] Slang 主页：https://shader-slang.org/
- [S42] Slang Feature Matureness：https://shader-slang.org/docs/feature_matureness/
- [S43] HLSL SM 6.8：https://microsoft.github.io/DirectX-Specs/d3d/HLSL_ShaderModel6_8.html
- [S44] HLSL SM 6.9：https://microsoft.github.io/DirectX-Specs/d3d/HLSL_ShaderModel6_9.html
- [S45] WGSL 规范：https://gpuweb.github.io/gpuweb/wgsl/
- [S46] UE Material Shader Map：https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/FMaterialShaderMap
- [S47] UE Material Analyzer：https://dev.epicgames.com/documentation/en-us/unreal-engine/unreal-engine-material-analyzer-tool
- [S48] Vulkan Pipeline Cache：https://docs.vulkan.org/guide/latest/pipeline_cache.html
- [S49] UE Derived Data Cache：https://dev.epicgames.com/documentation/unreal-engine/using-derived-data-cache-in-unreal-engine
- [S50] UE Cooking：https://dev.epicgames.com/documentation/en-us/unreal-engine/cooking-content-in-unreal-engine
- [S51] UE Zen Cooked Snapshot：https://dev.epicgames.com/documentation/unreal-engine/cooked-data-snapshots-with-zen-storage-server-for-unreal-engine
- [S52] UE Cloud DDC：https://dev.epicgames.com/documentation/unreal-engine/how-to-set-up-a-cloud-type-derived-data-cache-for-unreal-engine
- [S53] meshoptimizer：https://github.com/zeux/meshoptimizer/
- [S54] meshoptimizer v1.0：https://meshoptimizer.org/v1.html
- [S55] meshoptimizer v1.2：https://github.com/zeux/meshoptimizer/releases/tag/v1.2
- [S56] Basis Universal：https://github.com/BinomialLLC/basis_universal/
- [S57] KTX Software：https://github.com/KhronosGroup/KTX-Software
- [S58] Arm ASTC Encoder：https://github.com/ARM-software/astc-encoder/
- [S59] AMD Compressonator：https://github.com/GPUOpen-Tools/Compressonator
- [S60] NVIDIA Texture Tools 3：https://developer.nvidia.com/gpu-accelerated-texture-compression
- [S61] UE Virtual Texturing：https://dev.epicgames.com/documentation/en-us/unreal-engine/virtual-texturing-in-unreal-engine
- [S62] UE Runtime VT Build：https://dev.epicgames.com/documentation/unreal-engine/runtime-virtual-texturing-in-unreal-engine
- [S63] glTF 主页与路线图：https://www.khronos.org/gltf/
- [S64] glTF SIGGRAPH 2025：https://www.khronos.org/assets/uploads/developers/presentations/glTF_Innovations_SIGGRAPH_2025.pdf
- [S65] fastgltf/MIT：https://github.com/spnda/fastgltf
- [S66] OpenUSD Core：https://openusd.org/25.05/api/usd_page_front.html
- [S67] OpenUSD Ar 2.0：https://openusd.org/release/wp_ar2.html
- [S68] OpenUSD TOST 许可证：https://github.com/PixarAnimationStudios/OpenUSD/blob/dev/LICENSE.txt
- [S69] MaterialX：https://materialx.org/index.html
- [S70] MaterialX 2025 状态：https://materialx.org/assets/ASWF_OSD2025_MaterialX_Final.pdf

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-02 | 初版：GPU API 与资产管线深度调研成果落盘（G8 计划定稿档输入材料） |
