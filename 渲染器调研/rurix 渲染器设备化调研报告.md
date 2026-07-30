# rurix 渲染器设备化调研报告

**——着色器供给路线（方向一深潜）与方向二至九精要**

调研基准日期：2026-07-29。锚定前提：rurix-render 七个方向的宿主参考实现为黄金标准（239 个单元测试）；设备化阻塞项 RD-038——rurixc 的 vulkan_codegen 当前只支持 CAP_SHADER / SPIR-V 1.0，无 Int64、零 OpAtomic\*、无 OpTypeImage lowering、零 ray query。

---

## 摘要

本报告对九个调研方向给出结论，其中方向一（着色器供给）为完整深潜，方向二至九为精要级核实与移植契约。核心裁决如下。

**裁决一（供给路径）：以"扩展 rurixc MIR lowering"（路径 a）为主线，以"程序化 SPIR-V 直写"（路径 b）为补充通道，以"外部交叉编译"（路径 c）为有条件采纳的加速手段。** 关键依据：rust-gpu 作为与 rurixc 同构的 MIR→SPIR-V 后端，已公开证明该路径的工程可行性（包括 ray query 支持） [(rust-gpu.github.io)](https://rust-gpu.github.io/changelog/) ；WGSL/naga 路线因规范不支持 ray query 与 64 位整数而被排除 [(The Khronos Group)](https://www.khronos.org/developers/linkto/wgsl) ；DXC/Slang 的 SPIR-V 后端可用但有记录在案的正确性缺陷史，引入必须附带验证纪律 [(Github)](https://github.com/microsoft/DirectXShaderCompiler/issues/4407) 。

**裁决二（能力波次）：按"零新能力 → 两个能力 → 版本门槛 → 可选增强"分四波。** 32 位缓冲原子操作在 SPIR-V 1.0 / CAP_SHADER 内即可解锁（OpAtomicIAdd/IMin/IMax 等，无需任何新 capability） [(xjbcode.fun)](http://www.xjbcode.fun/Notes/004-3d-rendering/vulkan_html/guide.html) ；VisBuffer 的 u64 atomicMax 需要 Int64 + Int64Atomics 两个能力及 VK_KHR_shader_atomic_int64 设备扩展 [(vulkan.net.cn)](https://docs.vulkan.net.cn/guide/latest/atomics.html) ；ray query 由于 Vulkan API 层面对 SPIR-V 版本的硬性要求，迫使 rurixc 从 SPIR-V 1.0 升级到 SPIR-V 1.4，这是整条路线的最大单项门槛 [(The Khronos Group)](https://www.khronos.org/blog/vulkan-ray-tracing-final-specification-release) 。

**裁决三（ray query 与 SBT 的分工）：所有"只问可见性、不问材质"的内核（probe GI、RTAO、硬阴影）一律走 ray query；只有"命中点需要多样化材质着色"时才引入 RT pipeline + SBT，且必须清醒认识 Vulkan 的 shader record 是只读常量，没有 DXR local root signature 的对等物** [(vkdoc.net)](https://vkdoc.net/chapters/ray-tracing) 。材质多样性在 Vulkan 上的正解是 record 常量 + RuntimeDescriptorArrayEXT 无绑定索引 + gl_InstanceCustomIndexEXT 间接寻址 [(vulkan.org)](https://docs.vulkan.org/spec/latest/chapters/raytracing.html) 。

### 用户前提核实总表

调研启动前，用户随任务给出的 12 项前提表述被逐条与公开一手资料对表核实：9 项确认、1 项确认但需修正归因（Alan Wake 2 的 39% 为 SER+OMM 合并收益）、2 项未能在公开渠道核实（已登记入 §10.2，建议内部确认后再入库）。

| 用户前提 | 核实结论 | 关键证据 |
|---|---|---|
| ray query 需 SPV_KHR_ray_query、Int64 + shaderBufferInt64Atomics | **确认**，且 ray query 额外强制 SPIR-V ≥ 1.4 |  [(The Khronos Group)](https://github.khronos.org/SPIRV-Registry/extensions/KHR/SPV_KHR_ray_query.html)  |
| Vulkan SBT record 只读常量 vs DXR local root signature | **确认**，无语义对等物 |  [(vkdoc.net)](https://vkdoc.net/chapters/ray-tracing)  |
| 黑神话 SER 约 3.7× 收益 | **确认**：ReSTIR GI pass 15.10ms→4.08ms（RTX 4070Ti @4K DLSS），相干性 20.5%→69.9% |  [(The Khronos Group)](https://www.khronos.org/blog/boosting-ray-tracing-performance-with-shader-execution-reordering-introducing-vk-ext-ray-tracing-invocation-reorder)  |
| Alan Wake 2 SER 约 39% | **确认但需修正归因**：39% 是 SER+OMM 合并收益（16.8ms→10.2ms，RTX 4090） |  [(The Khronos Group)](https://www.khronos.org/blog/boosting-ray-tracing-performance-with-shader-execution-reordering-introducing-vk-ext-ray-tracing-invocation-reorder)  |
| ReSTIR PT Enhanced（I3D 2026）2–3× | **确认**：2.08–3.05×，I3D 2026 最佳论文（并列） |  [(techvogue.blog)](https://techvogue.blog/blog/nvidia-restir-pt-enhanced-algorithm-making)  |
| VSM 页面命中率 >95%、≤3ms@60Hz 预算 | **确认可行**：移动光源无效化 0.4–0.8ms，阴影总预算 2.5–3.5ms |  [(StraySpark)](https://www.strayspark.studio/blog/virtual-shadow-map-optimization-open-worlds-ue5-7)  |
| SMRT RayCount=0 退化为硬阴影 | **确认** |  [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/virtual-shadow-maps-in-unreal-engine?lang=en-US)  |
| Substrate 参数混合 108→28 字节/像素 | **确认**：官方文档同一材质精确数字 |  [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/overview-of-substrate-materials-in-unreal-engine?lang=en-US)  |
| Fortnite 高峰 3 万 PSO、常驻约 1 万 | **确认**：官方技术博客原话 |  [(Unreal Engine)](https://www.unrealengine.com/tech-blog/game-engines-and-shader-stuttering-unreal-engines-solution-to-the-problem)  |
| Fortnite PSO"排列减半" | **未核实**：公开渠道无此口径，仅有"预编译子集远小于百万级全组合空间" |  [(Unreal Engine)](https://www.unrealengine.com/tech-blog/game-engines-and-shader-stuttering-unreal-engines-solution-to-the-problem)  |
| UE5.5 RHI 并行翻译 2×/省 7ms | **确认**：发行说明原话；另有异步 RDG 省 0.4ms |  [(Tom Looman)](https://tomlooman.com/unreal-engine-5-5-performance-highlights/)  |
| Mlakar（CGF）文献 | **未核实**：网络与学术检索均无此人此文；最接近的是 Benthin & Peters 2023（CGF，Micro-Poly RT HLOD），建议内部确认出处 | — |

需要强调：两项"未核实"均按"如实登记、不采信、不删除"处理——它们不影响本报告任何裁决的成立（各裁决均有独立公开证据支撑），但在评审引用时应标注待确认状态，避免把未核实口径当作既定事实进入 RD-038 的排期依据。

---

## 第一章 方向一：效果内核着色器供给路线（深潜）

### 1.1 现状与卡点：RD-038 的真实构成

rurixc 的 vulkan_codegen 今天的能力边界是 CAP_SHADER + SPIR-V 1.0：这意味着 32 位标量与向量算术、常规纹理采样、storage buffer 读写可用；而七个效果内核所依赖的四组能力——64 位整数及其原子、存储图像写入、texel 指针寻址、光线查询——全部缺失。卡点不是孤立的四个缺口，而是一条依赖链：VisBuffer 的 64 位 atomicMax 以 Int64 为前提，Int64Atomics 又依赖于 Int64；ray query 以加速结构为前提，加速结构扩展又依赖 buffer device address 与描述符索引 [(vulkan.org)](https://docs.vulkan.org/refpages/latest/refpages/source/VK_KHR_buffer_device_address.html) 。因此"补能力"不能按内核逐个点菜，必须按依赖序组织成波次（见 1.7 节与能力依赖图）。

![SPIR-V 能力与扩展依赖图](research/charts/capability_graph.png)

上图把七类内核的能力需求收敛为一张依赖图：青色节点（32 位缓冲原子、存储图像写入、OpImageTexelPointer）在 SPIR-V 1.0 内即可落地；蓝色链路（Int64→Int64Atomics→VisBuffer）只需两个新能力；琥珀色链路（SPIR-V 1.4→加速结构）是 ray query 的强制性前置；红色与紫色节点才是光线类内核本体。一个容易被忽视的事实是：Vulkan 自 1.0 起就支持 storage buffer 上的 32 位原子与 storage image 上的 32 位图像原子，二者都不需要声明任何新 capability——rurixc 缺的不是"能力"，而是这几条 OpAtomic\* 指令的 lowering 路径 [(xjbcode.fun)](http://www.xjbcode.fun/Notes/004-3d-rendering/vulkan_html/guide.html) 。这直接决定了第一波次可以零风险启动。

### 1.2 七类效果内核的最小指令集清单

下列清单按"最小可运行"口径给出：每行只列该内核确实需要的能力、扩展与指令，可选增强单列。能力的官方编号与依赖关系以 SPIR-V 注册表与 Vulkan 规范为准 [(Khronos Registry)](https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html) 。

**内核 1：两级剔除 compute（视锥 + HZB 遮挡）**

| 项目 | 最小集合 |
|---|---|
| SPIR-V 能力 | Shader（现有） |
| 设备扩展 | 无（可选 VK_KHR_buffer_device_address 用于剔除参数直传） [(vulkan.org)](https://docs.vulkan.org/refpages/latest/refpages/source/VK_KHR_buffer_device_address.html)  |
| 关键指令 | OpAtomicIAdd / OpAtomicUMin（计数器与紧凑化）、OpGroupNonUniform\* / subgroup 表决（波内前缀，可选加速）、OpControlBarrier |
| 说明 | 32 位原子在 SPIR-V 1.0 即可用；subgroup 属 SPIR-V 1.3/Vulkan 1.1，若坚持 1.0 可用共享内存替代 [(Github)](https://github.com/KhronosGroup/Vulkan-Guide/blob/main/chapters/versions.adoc)  |

**内核 2：VisBuffer u64 atomicMax 软件光栅**

| 项目 | 最小集合 |
|---|---|
| SPIR-V 能力 | Int64（11）+ Int64Atomics（12，依赖 Int64） [(Khronos Registry)](https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html)  |
| 设备扩展 | VK_KHR_shader_atomic_int64（特性 shaderBufferInt64Atomics） [(Github)](https://github.com/hamu77/Vulkan-Guide_Khronos/blob/main/chapters/atomics.adoc)  |
| 关键指令 | OpAtomicUMax（64 位，storage buffer）、OpIAddCarry 等 64 位算术、OpShiftRightLogical |
| 打包格式 | u64 = 深度 30 \| cluster 27 \| triangle 7（Nanite 口径，见第二章） [(Unbiased Gamer)](https://unbiased-gamer.com/the-mental-model-for-unreal-engines-nanite-virtualized-geometry-and-cluster-culling/)  |
| 可选增强 | Int64ImageEXT + VK_EXT_shader_image_atomic_int64（若 VisBuffer 落 R64_UINT 图像而非缓冲） [(Github)](https://github.com/KhronosGroup/glslang/issues/2975)  |

**内核 3：classify-resolve（VisBuffer → 材质分类解析着色）**

| 项目 | 最小集合 |
|---|---|
| SPIR-V 能力 | Shader（现有）；若 resolve 直接写 UAV 图像则需 StorageImageWriteWithoutFormat [(vulkan.net.cn)](https://docs.vulkan.net.cn/spec/latest/chapters/interfaces.html)  |
| 设备扩展 | 无（descriptor indexing 可选，用于材质表无绑定化） |
| 关键指令 | OpShiftRightLogical/OpBitwiseAnd（解包 u64）、OpImageFetch（读 VisBuffer texel）、OpImageWrite（写解析结果）、间接分派准备缓冲写入 |
| 说明 | 该内核是"读缓冲 + 查表 + 写图像"结构，天然落在第一波次 |

**内核 4：VSM 页面标记 / 深度光栅**

| 项目 | 最小集合 |
|---|---|
| SPIR-V 能力 | Shader（现有）+ StorageImageWriteWithoutFormat（页面表为 R32_UINT 图像时） [(vulkan.net.cn)](https://docs.vulkan.net.cn/spec/latest/chapters/interfaces.html)  |
| 设备扩展 | 无 |
| 关键指令 | OpAtomicOr（页面表置位，32 位）、OpImageWrite（深度页写入）、OpConvertFToU（深度量化） |
| 说明 | 页面标记本质是 32 位原子或普通写入；物理页分配若用缓冲 freelist 则同样落在 32 位原子域 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/virtual-shadow-maps-in-unreal-engine?lang=en-US)  |

**内核 5：probe GI ray query**

| 项目 | 最小集合 |
|---|---|
| SPIR-V 版本 | **≥ 1.4（硬性门槛，见 1.3 节）** [(The Khronos Group)](https://www.khronos.org/blog/vulkan-ray-tracing-final-specification-release)  |
| SPIR-V 能力 | RayQueryKHR；扩展 SPV_KHR_ray_query [(The Khronos Group)](https://github.khronos.org/SPIRV-Registry/extensions/KHR/SPV_KHR_ray_query.html)  |
| 设备扩展链 | VK_KHR_ray_query → VK_KHR_acceleration_structure →（VK_KHR_buffer_device_address + VK_EXT_descriptor_indexing + VK_KHR_deferred_host_operations） [(The Khronos Group)](https://github.khronos.org/SPIRV-Registry/extensions/KHR/SPV_KHR_ray_query.html)  |
| 关键指令 | OpRayQueryInitializeKHR、OpRayQueryProceedKHR、OpRayQueryGetIntersectionTypeKHR、OpRayQueryGetIntersectionTKHR、OpRayQueryGetIntersectionBarycentricsKHR、OpRayQueryGetIntersectionInstanceCustomIndexKHR |
| 说明 | probe 着色只需交点属性与实例索引，不需要任何 hit shader，这是 ray query 相对 RT pipeline 的成本优势所在 [(The Khronos Group)](https://github.khronos.org/SPIRV-Registry/extensions/KHR/SPV_KHR_ray_query.html)  |

**内核 6：RTAO / 硬阴影 ray query**

| 项目 | 最小集合 |
|---|---|
| 同内核 5 的全部门槛 | 同上 [(The Khronos Group)](https://github.khronos.org/SPIRV-Registry/extensions/KHR/SPV_KHR_ray_query.html)  |
| 关键指令 | OpRayQueryInitializeKHR（配 gl_RayFlagsTerminateOnFirstHitKHR 语义）、OpRayQueryConfirmIntersectionKHR、OpRayQueryGetIntersectionTKHR；阴影射线用不透明标志短路 |
| 说明 | 硬阴影只需布尔可见性，ray query 的 terminate-on-first-hit 是延迟最低的形式；这也是 MegaLights"短屏幕射线→世界射线"策略的直接参照 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/megalights-in-unreal-engine?lang=en-US)  |

**内核 7：TAA/TSR 全屏 pass**

| 项目 | 最小集合 |
|---|---|
| SPIR-V 能力 | Shader（现有） |
| 设备扩展 | 无 |
| 关键指令 | OpImageFetch / OpImageSampleImplicitLod（历史重投影采样）、OpFClamp（邻域钳制）、OpVectorTimesScalar 等常规向量运算 |
| 说明 | 全屏时序通道在能力上零门槛，其难点在契约（运动矢量、抖动、历史格式）而非指令；TSR 的历史双倍分辨率、样本计数、异步计算档位都是纯算法层旋钮 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/temporal-super-resolution-in-unreal-engine?lang)  |

### 1.3 版本门槛：为什么 ray query 迫使 SPIR-V 1.0 → 1.4

Khronos 的光线追踪最终规范明确要求：**使用 ray query 或 ray tracing pipeline 的 SPIR-V 模块版本不得低于 1.4** [(The Khronos Group)](https://www.khronos.org/blog/vulkan-ray-tracing-final-specification-release) 。这不是 rurixc 自己的选择，而是 Vulkan API 的有效性规则——即使 rurixc 能在 SPIR-V 1.0 里编码出 OpRayQueryProceedKHR，驱动也有权拒绝加载。版本对应关系是刚性的：Vulkan 1.0 消费 SPIR-V 1.0，Vulkan 1.1 消费 1.3（subgroup 即随 1.3 而来），Vulkan 1.2 同时放开 SPIR-V 1.4 与 1.5 [(Github)](https://github.com/KhronosGroup/Vulkan-Guide/blob/main/chapters/versions.adoc) 。

这一门槛的工程含义比"改一个版本号"深得多。SPIR-V 1.4 放开了 Logical 寻址模型下的若干限制（最典型的是 OpCopyLogical 与跨接口变量的放宽），并且是 Khronos 为光线追踪系列指令统一选定的基线；rurixc 的版本升级因此应当作为一个独立波次（W3a）对待：先升级版本并回归 239 个宿主单测的黄金输出，再叠加加速结构与 ray query。把"版本升级"与"新指令落地"混在同一提交里，是 RD-038 这类长阻塞项最典型的失控路径。值得同步注意：buffer device address（SPV_KHR_physical_storage_buffer）虽在 1.3 时代即可用，但它与 ray query 同为加速结构扩展的前置依赖，放进同一波次一并验证可以减少一次全量回归 [(vulkan.org)](https://docs.vulkan.org/refpages/latest/refpages/source/VK_KHR_buffer_device_address.html) 。

### 1.4 三条供给路径对比与裁决

三条路径的本质差异在于"谁对最终 SPIR-V 负责"。路径 a（扩展 rurixc MIR lowering）让 rurixc 继续作为唯一真相源，所有指令选择、能力声明、验证规则都在仓库内可审计；路径 b（手写/程序化 SPIR-V 直写）把个别内核的指令序列固化为可审查的文本构件，以"生成器脚本 + 黄金反汇编 diff"为纪律；路径 c（Slang/HLSL→SPIR-V 离线交叉编译）把指令选择外包给上游编译器，换取生态借力，但引入上游缺陷与版本漂移风险。

| 维度 | a · rurixc MIR lowering | b · 程序化 SPIR-V 直写 | c · 外部交叉编译（Slang/DXC） |
|---|---|---|---|
| 审计可控性 | 全链路仓库内，diff 即审计 | 构件即文本，可反汇编比对，但游离于类型系统外 | 黑盒段不可审，只能审输出 |
| 能力上限 | 取决于实现投入，无理论上限（rust-gpu 已证 ray query 可达） [(rust-gpu.github.io)](https://rust-gpu.github.io/changelog/)  | 任意指令可达，含尚未建模的能力 | 受上游支持度限制（Slang 能力原子机制覆盖较全） [(shader-slang.org)](https://docs.shader-slang.org/en/latest/external/slang/docs/user-guide/a3-02-reference-capability-atoms.html)  |
| 工程成本 | 最高：每能力一组 lowering + 验证 | 中：生成器 + 一次性纪律建设 | 最低：接入即用 |
| 依赖/合规风险 | 零外部依赖 | 零外部依赖（工具链自选 rspirv/SPIRV-Tools） [(rust-gpu.github.io)](https://rust-gpu.github.io/changelog/)  | 引入上游许可证与版本锁定；DXC SPIR-V 后端有缺陷史 [(Github)](https://github.com/microsoft/DirectXShaderCompiler/issues/4407)  |
| 演进速度/生态借力 | 慢，全靠自己 | 慢 | 快，随上游获得 SER/OMM/Cooperative Vector 等新特性 [(shader-slang.org)](https://shader-slang.org/)  |
| 验证可复用性 | 与 239 单测黄金标准同源 | 需新建"构件-参考"对拍 | 需新建"上游输出-参考"对拍与版本门禁 |

![三条供给路径对比雷达图](research/charts/path_radar.png)

**裁决：a 为主线、b 为补充、c 有条件采纳。** 路径 a 的可行性不由信仰支撑：rust-gpu 是与 rurixc 几乎同构的 Rust MIR→SPIR-V 后端，它在开源状态下完成了从基础光栅到 ray query 的全谱系落地，证明"MIR 直接 lowering 到 SPIR-V"这条路没有原理性障碍，只有工程量问题 [(rust-gpu.github.io)](https://rust-gpu.github.io/changelog/) 。路径 b 的定位是"能力探针与紧急通道"：当某个内核（如 VisBuffer 的 OpAtomicUMax）急需先行联调而 rurixc lowering 尚未就绪时，用生成器直写该内核的 SPIR-V，附强制性的反汇编黄金比对，事后可被 a 路径产物无损替换。路径 c 仅在两种情形下采纳：其一，需要快速验证某内核算法本身（此时 DXC/Slang 产物作为一次性原型，不进主干）；其二，长期引入 Slang 作为内容侧着色语言——这属于另一个立项，其合规成本（上游缺陷史、版本门禁、Slang/DXC 双后端的差异管理）必须单独评审 [(Github)](https://github.com/microsoft/DirectXShaderCompiler/issues/4407) 。WGSL/naga 路线被明确排除：WGSL 规范当前没有 ray query，也不支持 64 位整数类型，naga 与 Tint 在复杂着色器上的翻译缺陷亦有公开记录 [(The Khronos Group)](https://www.khronos.org/developers/linkto/wgsl) 。

### 1.5 ray query vs RT pipeline + SBT：Vulkan 与 DXR 的语义差异

两种机制的适用面由"命中之后做什么"决定。ray query 是内联在任何着色阶段里的"只问遍历、不问着色"原语：着色器自己驱动遍历状态机（OpRayQueryProceedKHR），自己决定接受哪个候选交点，交点信息以指令返回值给出——它适合可见性查询（AO、硬阴影）、probe 追踪（取 T、重心坐标、实例索引后自己查表着色）这类"命中点处理同质化"的负载 [(The Khronos Group)](https://github.khronos.org/SPIRV-Registry/extensions/KHR/SPV_KHR_ray_query.html) 。RT pipeline 则是"命中点处理异质化"的解：GPU 遍历到命中后回调 per-geometry 的 closest-hit/any-hit shader，着色代码与材质绑定，通过 Shader Binding Table 按 (instance, geometry, ray type) 索引派发。

用户的前提在规范层面成立：Vulkan 的 shader record 存放在 ShaderRecordBufferKHR 存储类中，该存储类被规范定义为**只读、仅常量数据**——每条 record 是一段内联字节（通常放材质 ID、纹理索引等小而固定的数据），没有 DXR local root signature 那种"每条 record 一套根参数（可含描述符表指针、动态常量）"的对等物 [(vkdoc.net)](https://vkdoc.net/chapters/ray-tracing) 。RT Gems II 的移植章节与 NVIDIA 的 HLSL→Vulkan RT 指南给出的 Vulkan 正解是三段式：record 常量放小尺寸键值；真正的材质数据放全局缓冲，用 record 里的索引或 gl_InstanceCustomIndexEXT / gl_GeometryIndexEXT 间接寻址；纹理等描述符用 RuntimeDescriptorArrayEXT（VK_EXT_descriptor_indexing 的 runtimeDescriptorArray 特性）做无绑定数组 [(CSDN博客)](https://blog.csdn.net/pizi0475/article/details/131688441) 。NVRHI 等跨 API 抽象层正是按这个公约数建模的 [(Github)](https://github.com/h-mathias/nvrhi/blob/main/doc/ProgrammingGuide.md) 。

**"何时必须 SBT"的判据因此可以写死：当命中点的着色逻辑需要按材质分派到不同代码路径（不同 BRDF 求值、不同纹理集合、alpha 测试的 any-hit 语义），且这些代码路径无法在调用方着色器里用一个查表分支收敛时，引入 RT pipeline + SBT。** 反向判据同样明确：probe GI、RTAO、硬阴影这三类内核的命中后处理是统一的（取属性→累加辐射或置位可见性），ray query 严格更优——少一条管线、少一套 SBT 内存管理、遍历控制流全在本地，MegaLights 甚至特意把 alpha-mask 命中的 any-hit 求值拆成"先内联 ray query，命中待求材实例时再续发一条开 any-hit 的 continuation ray"，以保住内联遍历的性能 [(TISTORY)](https://techartnomad.tistory.com/659) 。这条经验对 rurix 的直接推论是：即便未来引入 SBT，ray query 内核也不应回退改写，二者是互补而非替代。

### 1.6 业界参照：Bevy/wgpu、Dawn、gfx-rs、FidelityFX 的着色器供给

四家代表了四种供给哲学，恰好构成 rurix 决策的坐标系。**wgpu/gfx-rs 系**以 WGSL 为内容侧语言、naga 为翻译器，wgpu v22 起允许 Vulkan 后端直收 SPIR-V 原始模块（passthrough），等于官方承认"内容语言表达能力不足时放行原生二进制"的泄压阀 [(SourceForge)](https://sourceforge.net/projects/wgpu.mirror/files/v22.0.0/) ；Bevy 在其上用 naga_oil 做 WGSL 的模块组合（import/条件编译），把"一门受限语言 + 组合器"用到极致，但 ray query 与 64 位整数的缺席使其无法承载 rurix 的光线类内核 [(Zenn)](https://zenn.dev/omini/articles/f06a1b1af514a9) 。**Dawn/Tint** 是 Chrome 的 WebGPU 实现，单语言（WGSL）单翻译器，优化最保守，工程化最强；它对 rurix 的启示是反向的——Tint 的严格性来自浏览器安全模型，这个约束对自研引擎并不成立 [(python | DeepWiki)](https://deepwiki.com/google/dawn/4-tint-shader-compiler) 。

**FidelityFX** 走的是内容侧 HLSL + 自研编译器（FidelityFX_SC，基于 LLVM 下游定制）+ 运行时预编译库的路，本质是路径 c 的"自闭环"变体：AMD 自己控制编译器，把交叉编译的审计风险内部化 [(juandiegomontoya.github.io)](https://juandiegomontoya.github.io/porting_fsr2.html) 。这对 rurix 的启示在于组合方式而非照搬：FidelityFX 证明了"效果内核以预编译构件 + 明确 ABI 交付"在量产管线里成立，rurix 的路径 b 构件可以借这套 ABI 思维（内核入口契约、常量缓冲布局、能力声明清单全部随构件入库）。**Slang** 作为 Khronos 托管的着色语言则展示了另一条曲线：以"能力原子"机制把目标 API 的特性矩阵编码进语言层，同一份源码按目标能力自动降级或报错 [(shader-slang.org)](https://docs.shader-slang.org/en/latest/external/slang/docs/user-guide/a3-02-reference-capability-atoms.html) ；若 rurix 未来把内容侧着色语言外包，Slang 是比裸 DXC 更结构化的选择，但引擎内核实色（本报告七类）仍应留在 a/b 路径内，避免把帧率关键路径绑在上游发布节奏上。

### 1.7 分波次实施路线图

综合 1.2 的指令清单与 1.3 的版本门槛，实施按收益/成本比排序为四波。每波给出：解锁内核、新增能力/扩展、验证要求、预估量级（以 rurixc 现有代码规模外推的相对人月，非绝对承诺）。

![分波次实施路线图](research/charts/wave_roadmap.png)

| 波次 | 内容 | 解锁内核 | 新增能力/扩展 | 验证要点 |
|---|---|---|---|---|
| **W1**（M0–M1.5） | 32 位缓冲原子 lowering + 存储图像写入 | 两级剔除、classify-resolve、VSM 页面标记、TAA/TSR | 0 新能力（StorageImageWriteWithoutFormat 视写入路径而定） [(xjbcode.fun)](http://www.xjbcode.fun/Notes/004-3d-rendering/vulkan_html/guide.html)  | OpAtomicIAdd/UMin/UMax 与参考实现的逐位对拍；图像写入格式无关路径的合规扫描 |
| **W2**（M1–M3） | Int64 + Int64Atomics | VisBuffer u64 atomicMax 软光栅 | Int64、Int64Atomics + VK_KHR_shader_atomic_int64 [(vulkan.net.cn)](https://docs.vulkan.net.cn/guide/latest/atomics.html)  | u64 打包域（深度30/cluster27/triangle7）边界单测；与 host 光栅的 VisBuffer 全等比对 [(Unbiased Gamer)](https://unbiased-gamer.com/the-mental-model-for-unreal-engines-nanite-virtualized-geometry-and-cluster-culling/)  |
| **W3**（M2.5–M7） | SPIR-V 1.4 升级 → 加速结构 → ray query | probe GI、RTAO、硬阴影 | RayQueryKHR + SPV_KHR_ray_query；AS 三依赖（BDA、descriptor indexing、deferred host ops） [(The Khronos Group)](https://github.khronos.org/SPIRV-Registry/extensions/KHR/SPV_KHR_ray_query.html)  | 版本升级单独一波（W3a）先回归；ray query 遍历结果与 host 参考 BVH 的交点一致性（容差内） |
| **W4**（M7–M11，按需） | RT pipeline + SBT；SER/OMM | 命中点多样化材质着色；增强 | RayTracingKHR、ShaderRecordBufferKHR；VK_EXT_ray_tracing_invocation_reorder、VK_EXT_opacity_micromap [(vkdoc.net)](https://vkdoc.net/chapters/ray-tracing)  | SBT record 布局 ABI 审查；SER/OMM 仅在硬件覆盖审查后启用，OMM 对 alpha 植被收益先行实测 [(The Khronos Group)](https://www.khronos.org/blog/boosting-ray-tracing-performance-with-shader-execution-reordering-introducing-vk-ext-ray-tracing-invocation-reorder)  |

波次间有两个刻意的重叠：W3a（SPIR-V 1.4）在 W2 收尾时即可启动，因为版本升级与 Int64 lowering 无耦合；W3c（ray query 内核）不必等 W3b 的加速结构管理面全部完工，可先用最小 TLAS（每帧全量重建）联调遍历正确性，再替换为正式的 BLAS 治理（见第五章）。W4 整体标注"按需"：第一章 1.5 节的判据未触发前（命中点着色仍同质），不引入 SBT；SER/OMM 属于优化项，黑神话与 Alan Wake 2 的数据证明其收益真实但依赖内容特征（相干度低、alpha 植被密集） [(The Khronos Group)](https://www.khronos.org/blog/boosting-ray-tracing-performance-with-shader-execution-reordering-introducing-vk-ext-ray-tracing-invocation-reorder) 。

---

## 第二章 方向二（精要）：Nanite 软件光栅与两级剔除

### 2.1 VisBuffer 与 32px 阈值

Nanite 的核心观察是：当三角形投影到屏幕后普遍小于约 32 像素时，硬件光栅化器的固定功能流水线（三角形装配、粗粒度 2×2 quad 着色）效率急剧下降，软件扫描转换 + 64 位原子最大值写入反而更快。VisBuffer 把每个像素的可见性压缩进一个 u64——**深度 30 位 | cluster 索引 27 位 | 三角形索引 7 位**——光栅化即对每个覆盖像素执行一次 InterlockedMax（深度反序编码使"更近=更大"），随后一个 classify-resolve pass 解包该值、反查 cluster 与三角形、执行真正的材质着色 [(Unbiased Gamer)](https://unbiased-gamer.com/the-mental-model-for-unreal-engines-nanite-virtualized-geometry-and-cluster-culling/) 。对 rurix 的移植契约是双重的：打包域布局必须作为 ABI 冻结（它同时是 W2 波次的验收标准）；resolve pass 的间接分派（按材质聚类后每类一次 draw/dispatch）是 classify 内核存在的意义，宿主参考实现的 239 单测里该结构应已有对应断言。

Bevy 0.14 的实验性 meshlet 渲染器为"自研复现这套管线"提供了第二参照系：其在公开基准中把可见性光栅 + 两级剔除控制在亚毫秒级（特定场景约 0.49ms），证明该架构不绑定于 Unreal 的具体实现 [(jms55.github.io)](https://jms55.github.io/posts/2024-06-09-virtual-geometry-bevy-0-14/) 。nvpro 的 vk_lod_clusters 样例则给出第三条证据：基于 meshoptimizer 生成 cluster/LOD 数据 + 连续 LOD + 按需流式加载的完整 Vulkan 实现，且该样例同时支持光栅与 RTX MegaGeometry 的 cluster 加速结构路径——这正是第五章 BLAS 治理与 rurix cluster 管线的交汇点 [(Github)](https://github.com/nvpro-samples/vk_lod_clusters/blob/main/README.md) 。

### 2.2 两级 HZB 与流式预算

两级 HZB 剔除的时序是：第一遍用上一帧的深度金字塔测试全部 cluster，通过者直接渲染；随后用新深度重建本帧 HZB，对被遮挡者做第二遍复测，漏判者补渲——它把"上一帧答案"与"本帧正确性"解耦，代价是 HZB 的两次构建与一次额外的剔除 dispatch [(illinois.edu)](https://cs418.cs.illinois.edu/website/text/nanite.html) 。

UE5.7 在该方向上的两个新旋钮值得 rurix 直接借入：**r.Nanite.PrimeHZB**（用上一帧可见集预填充 HZB，降低首遍误判）与 **r.Nanite.Culling.MinLOD**（剔除时强制 LOD 下限，压住远景 cluster 数量） [(Tom Looman)](https://tomlooman.com/unreal-engine-5-7-performance-highlights/) 。流式方面，Nanite 以 128KB 页（32KB 根页常驻）为粒度、默认约 2GB 池上限做按需加载；这个"根页常驻 + 页粒度请求 + 硬池上限"三元组是预算化设计的直接模板，与第六章 FastGeo 的时间预算化形成 CPU/GPU 两侧呼应 [(Bing)](https://www.bing.com/ck/a?!=&fclid=35273c6b-4d66-6b12-20e2-29984cb76afa&hsh=4&ntb=1&p=8c4f73e51f1e0c94d83824b84e132c7f75f33ed4728e65f637fd8f9118a29afeJmltdHM9MTc0Nzc4NTYwMA&psq=Nanite 瓦片化网格分块&ptn=3&u=a1aHR0cHM6Ly96aHVhbmxhbi56aGlodS5jb20vcC83OTY1ODI0NDI&ver=2) 。

### 2.3 Foliage 与 tessellation 警示

Nanite Foliage（UE5.7 实验性）把植被拆成三层处理：Voxels（远处体素近似）、Assemblies（中景实例化装配）、Skinning（近景蒙皮摆动），其前提是承认"密集 alpha 植被"在传统 Nanite 管线（小三角 + alpha 测试）与 RT 管线（any-hit 风暴）下都是最坏情况 [(TweakTown)](https://www.tweaktown.com/news/107858/unreal-engine-5-7-preview-now-out-with-production-ready-procedural-content-generation-framework/index.html) 。对 rurix 的启示有二：植被应作为独立内容类走独立剔除与着色路径，不应假设一套 VisBuffer 参数通吃；OMM（第五章）在 Vulkan 侧是 alpha 植被阴影/遮挡的官方解法，但需硬件支持审查 [(Github)](https://github.com/NVIDIA-RTX/OMM) 。另一个警示来自 tessellation：UE 的 Nanite tessellation 知识库与 5.4 时代的实测都表明位移细分会把 cluster 内存与光栅成本推高一个量级，除非艺术需求刚性，不建议作为 rurix 首期目标 [(Epic Dev)](https://dev.epicgames.com/community/learning/knowledge-base/8yy8/unreal-engine-hardware-tessellation-support?locale=zh-cn) 。

把本章三件事串起来看，方向二对 RD-038 的实际诉求被压缩得很小：VisBuffer 主路径只需要 32 位原子加与无格式存储图像写（W1 波次即可覆盖），64 位原子最大值仅在走 u64 深度打包变体时才需要（W2），两级 HZB 与流式预算纯是调度与内存策略、不产生任何新着色器能力。换言之，Nanite 化不是 rurixc 能力扩展的挡路石，真正的版本压力全部来自第五、七章涉及的 ray query 一族。

---

## 第三章 方向三（精要）：VSM 设备化与逐灯虚拟化

### 3.1 页面经济与命中目标

Virtual Shadow Maps 把每盏灯的影子放进一张虚拟 16K² 纹理，按 128² texel 的页切片，只有被屏幕像素实际采样的页才被标记、分配物理页并光栅化深度——"按采样驱动分配"使其内存占用与灯数、场景规模解耦 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/virtual-shadow-maps-in-unreal-engine?lang=en-US) 。

用户设定的目标（页面命中率 >95%、阴影总耗时 ≤3ms@60Hz）在公开实测口径下是可达的：社区复现与调优记录显示，移动平行光源引发的页面无效化成本约 0.4–0.8ms/帧，而完全缓存命中时页面管理成本可低至约 0.05ms 量级；典型 60Hz 阴影预算 2.5–3.5ms 内可同时容纳 VSM 页面管理、深度光栅与采样 [(StraySpark)](https://www.strayspark.studio/blog/virtual-shadow-map-optimization-open-worlds-ue5-7) 。命中率 >95% 的工程含义是：池必须大到覆盖工作集峰值（UE 默认池为共享式，逐灯独占池会浪费），且页面表查找必须在采样器侧一次完成 [(Epic Developer Community Forums)](https://forums.unrealengine.com/t/preventing-nanite-pop-in-issues-in-2d-rendering-use-cases/2512906/7) 。

### 3.2 SMRT 与逐灯虚拟化的分工

SMRT（Sparse Multi-resolution Ray Tracing / 阴影射线追踪，UE 文档内置于 VSM 体系）用每像素 8 条射线、每射线 4–8 个样本在 VSM 页内做遮挡估计，**RayCount=0 时整体退化为硬阴影**——这个旋钮把"软阴影质量"变成纯运行时参数，且官方调优指引证实它是性能第一旋钮 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/virtual-shadow-maps-in-unreal-engine?lang=en-US) 。

逐灯虚拟化的现代形态则由 MegaLights 定义：所有局部灯统一进入随机直接光照路径，阴影来源可按灯在"RT（默认）/ VSM"间选择，VSM 回退目前仅推荐给平行光——原因是密集植被 any-hit 或大规模动画实例的 BVH 维护成本在 HWRT 下不可控时，VSM 的"只对采样页做工作"反而更便宜 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/megalights-in-unreal-engine?lang=en-US) 。对 rurix 的契约：页面表与物理页分配器落在 W1 波次（32 位原子域），SMRT 式软阴影作为采样侧算法层后续叠加；灯类型 → 阴影路径（RT/VSM/硬）的路由表应设计为数据驱动。

### 3.3 无效化治理

VSM 的成本中心从来不是采样而是无效化：几何移动、灯移动、Nanite LOD 切换都会使页面失效并触发重新分配与重光栅。Epic 在 Unreal Fest 的分享与社区实测给出的治理手段是三条——按页追踪依赖（只无效化真正受影响的页）、动画几何的无效化频率上限（降采样更新）、静态内容的跨帧缓存命中率监控 [(ドクセル)](https://www.docswell.com/s/EpicGamesJapan/K6E4V7-2025-10-14-222925) 。UE5.4 时代 tessellation 与 VSM 的交互（细分几何导致页面内容剧变）是已知坏案例，再次印证 2.3 节"细分缓行"的判断 [(Epic Dev)](https://dev.epicgames.com/community/learning/tutorials/bOda/unreal-engine-nanite-tessellation-displacement-ue-5-4-step-by-step-tutorial-any-asset-not-just-landscapes) 。rurix 的验收口径建议直接继承用户目标并仪器化：页面命中率、每帧无效化页数、页面管理 GPU 时长三个计数器进第八章的性能硬门。

能力维度上，VSM 是七类内核里最"便宜"的一类：页标记用 32 位原子位或、页表用无格式存储图像读写，深度光栅复用既有管线即可，全部落在 W1 的零新能力包内；只有"逐灯 16K² 虚拟页表 + 物理页池"的地址折算需要 64 位整数做页号运算，这在 W2 的 Int64 能力下自然解决。因此 VSM 设备化可以与 Nanite 主路径并行推进，不必等待 ray query 波次。

---

## 第四章 方向四（精要）：Lumen 世界辐射缓存与自适应探针

### 4.1 Surface Cache 与多反弹的真相

Lumen 的间接光照建立在两级缓存上：每个 mesh 预生成的 Surface Cache（沿包围盒六向投影的卡片，约 12 张/mesh，存材质属性与直接光照）供射线命中时直接读取出射辐射；屏幕空间探针（1/16 分辨率的 16×16 tile）与间接探针（4×4 间距/4×4 tile 双档）负责空间复用 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/lumen-technical-details-in-unreal-engine?lang=en-US) 。所谓"多反弹"不是逐帧递归追踪，而是 **Surface Cache 的跨帧反馈**：第 N 帧的间接结果被回写进 Surface Cache，第 N+2 帧的射线读取时便包含了上一次间接——用两帧延迟换取任意次反弹的稳态，这是"Lumen 多反弹"的准确语义（SIGGRAPH 2022 官方讲义） [(Advances in Real-Time Rendering in Games)](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Lumen-Wright%20et%20al.pdf) 。

Far Field / HLOD 层把远处对象替换为简化代理参与追踪，控制长射线的成本 [(nvidia.com)](https://dlss.download.nvidia.com/uebinarypackages/Documentation/UE5 Raytracing Guideline v5.4.pdf#:~:text=To enable raytraced shadows in UE5, simply enable the Ray) 。

### 4.2 UE5.6 自适应探针与世界辐射缓存

UE5.6 的 Lumen 性能提升来自两个可移植思想：**自适应探针放置**（按场景几何密度与光照变化分配探针，而非均匀网格）与**天空快速通道**（探针射线出 scene 后直接读天空，跳过完整材质求值） [(Tom Looman)](https://tomlooman.com/unreal-engine-5-6-performance-highlights/) 。

学术侧的对应物是 DDGI 及其重采样变体（Majercik et al. 2019 JCGT；2022 CGF 的 DDGI Resampling，把 reservoir 重采样引入探针更新）与 Neural Radiance Cache（Müller et al. 2021，以微型 MLP 替代显式缓存结构）——rurix 的"世界辐射缓存"选型应显式在这三条路线间做裁决：显式缓存（Lumen 式，可控可调试）、重采样探针（DDGI+ReSTIR 式，质量/成本比优）、神经缓存（NRC 式，表达力最强但引入推理依赖与确定性风险）。用户目标 <2ms@1080p 在 Lumen 官方口径内属于"可达成但需 HWRT 共享"的量级：UE5.5 起 Lumen 已以 60Hz 主机 HWRT 为目标优化，且官方建议 MegaLights 与 Lumen HWRT 共用同一 RT Scene 以摊薄遍历与加速结构维护成本 [(Tom Looman)](https://tomlooman.com/unreal-engine-5-5-performance-highlights/) 。

### 4.3 对 rurix 的移植契约

GI 内核落在 W3c 波次（ray query），但其缓存结构应在 W1 就冻结 ABI：探针 atlas 布局、probe→世界映射、Surface Cache 卡片的离线路线。两条纪律来自 UE 的公开教训：其一，屏幕空间与间接空间两级探针必须可独立降级（4×4/4×4 是质量档，16×16 screen probe 是性能档），这与 SMRT RayCount 的设计同构；其二，多反弹的跨帧反馈意味着 GI 结果天然含两帧延迟，TAA/TSR 的历史契约（第七章）必须把 GI 信号的时序特性纳入邻域钳制策略，否则会出现"阴影追光"类时序瑕疵 [(Althera Games)](https://altheragames.com/en/blog/ue5-lumen-guide) 。ReSTIR 系（第五章）作为 GI 的备选高端路线，其 2.08–3.05× 的公开提速针对的是路径追踪参照系，不应与 Lumen 式缓存管线混排预算 [(techvogue.blog)](https://techvogue.blog/blog/nvidia-restir-pt-enhanced-algorithm-making) 。

排期结论因此清晰：W1 冻结缓存 ABI 与卡片离线生成（零新能力、纯数据布局工作），W3c 落地 ray query 版 GI 内核（与 RTAO/硬阴影共用同一条 SPIR-V 1.4 → AS → RayQuery 扩展链，边际成本极低），自适应探针与 world radiance cache 作为其后的质量迭代项。GI 是第一章路线图里"一波投入、多内核摊销"论点成立的最大受益者。

---

## 第五章 方向五（精要）：SER / OMM / NRD / RTXDI / BLAS 治理

### 5.1 SER：从厂商扩展到跨 API 标准

Shader Execution Reordering 在 2025–2026 年完成了从 NVIDIA 私有到行业标准的两级跳：Vulkan 侧 VK_NV_ray_tracing_invocation_reorder 演进为多厂商的 **VK_EXT_ray_tracing_invocation_reorder（2025-11-18 随 Khronos 官方博客发布）**；D3D12 侧随 DXR 1.2 公告（GDC 2025）并在 **2026 年 2 月的 Agility SDK 1.619 / Shader Model 6.9 中转正为必需特性** [(The Khronos Group)](https://www.khronos.org/news/categories/Updated%20Basemark%20GPU%20with%20DX12,%20OpenGL%204.5,%20Enhanced%20Linux%20Support/www.ceatec.com/The%20Opera%20TV%20Store%20now%20packs%20in%20support%20for%20WebGL,%20the%20emerging%20web%20standard%20that%20has%20excited%20many%20publishers%20of%20graphics-rich%20content.%20As%20an%20example,%20game%20developers%20can%20finally%20bring%20their%20WebGL-based%20games%20to%20connected%20TVs%20running%20the%20Opera%20TV%20Store./streamcomputing.eu/P50) 。

实测收益的三个公开锚点：黑神话悟空的 ReSTIR GI pass 经 SER 重排后 15.10ms→4.08ms（3.7×，RTX 4070Ti @4K DLSS，线程相干性 20.5%→69.9%）；Alan Wake 2 的 SER+OMM 组合把光追成本 16.8ms→10.2ms（约 39%，RTX 4090，37M rays/帧）；Indiana Jones 经存活状态压缩把 SER 收益从 11% 提到 24% [(The Khronos Group)](https://www.khronos.org/blog/boosting-ray-tracing-performance-with-shader-execution-reordering-introducing-vk-ext-ray-tracing-invocation-reorder) 。对 rurix 的三条推论：SER 只对"低相干 + 高存活状态"的射线负载有意义，硬阴影这类 terminate-on-first-hit 负载收益有限；收益数字高度内容相关，立项前必须用自有场景做相干性画像；SER 的 API 形态（hit object + 显式重排点）要求着色器结构为其预留插入点，W4 波次若引入应作为独立特性开关 [(Vulkan)](https://www.vulkan.org/user/pages/09.events/vulkanised-2025/T49-Eric-Werness-NVIDIA.pdf) 。

### 5.2 OMM：alpha 植被的官方解法

Opacity Micromap 把 alpha 测试几何的遮挡信息预烘焙成微三角形级 1/2/4 状态掩码，使遍历器无需回调 any-hit shader 即可判定遮挡，Ada 架构起硬件加速约 2× alpha 遍历 [(Github)](https://github.com/NVIDIA-RTX/OMM) 。NVIDIA OMM SDK（1.8.0 起支持 DXR 1.2）提供 CPU 离线烘焙与 GPU 运行时烘焙两条路，Vulkan 侧对应 VK_EXT_opacity_micromap；3DMark 已上线 OMM 特性测试，MediaTek Dimensity 9400 成为首个支持该扩展的移动 SoC，标志其向移动端渗透 [(Github)](https://github.com/NVIDIAGameWorks/Opacity-MicroMap-SDK) 。

OMM 与 MegaLights 的教训要并读：UE 没有依赖 OMM 解决植被阴影，而是给平行光保留 VSM 回退，因为"艺术家自由编写 alpha 遮罩"与"预烘焙掩码"之间存在内容管线鸿沟 [(TISTORY)](https://techartnomad.tistory.com/659) 。rurix 的裁决建议：OMM 列为 W4 可选增强，仅对经内容审核的 alpha 植被资产启用离线烘焙；any-hit 兜底路径必须保留。

### 5.3 NRD：降噪器的集成契约

NRD（NVIDIA Real-time Denoisers，REBLUR/ReLAX 双算法族）的集成契约可以精确到资源表：REBLUR_DIFFUSE_SPECULAR 与 RELAX_DIFFUSE_SPECULAR 共用五个必需输入——**去调制后的漫反射辐射+二次命中距离、去调制后的高光辐射+命中距离、3D 运动矢量、NRD 格式打包的法线+线性粗糙度（含材质 ID）、线性视深 ViewZ**，输出对应的两个去噪后纹理 [(Github)](https://github.com/nvpro-samples/vk_denoise_nrd) 。调优经验三条最具迁移价值：命中距离必须用 REBLUR_FrontEnd_GetNormHitDist 归一化后传入（归一化参数经 HitDistanceParameters 回传用于反归一化）；累积帧数上限应按 FPS 缩放（maxAccumulatedFrameNum = 累积秒数 × FPS），且保持 maxAccumulatedFrameNum > maxFastAccumulatedFrameNum > historyFixFrameNum 的序关系；去噪前先做材质去调制（albedo 等不可模糊量必须移出信号），去噪后再调制回来 [(Github)](https://github.com/dylanblokhuis/NVIDIA_RayTracingDenoiser) 。

瓣分离（diffuse/specular 独立历史、独立命中距离）是质量基石，单一滤波强度必然顾此失彼 [(Github)](https://github.com/shaderjp/D3D12LookDevPT/blob/main/docs/rendering-pipeline.md) 。对 rurix：NRD 契约直接定义了 GI/AO 内核的输出布局（辐射+命中距离双通道），W3c 内核设计应"为降噪器而设计"，而非先出噪声图再想办法糊。

### 5.4 RTXDI 与 MegaLights：多光源的两种哲学

RTXDI（RIS/ReSTIR 的 NVIDIA SDK 实现）与 UE MegaLights 回答同一个问题——每帧从数百到数千盏灯里选哪几盏——但给出相反答案。RTXDI 坚持 reservoir 时空复用：候选采样 → RIS → 时空 reservoir 合并 → 可见性复验 [(shikihuiku.github.io)](https://shikihuiku.github.io/post/rtxdi_first_step/) 。

MegaLights 的 SIGGRAPH 2025 讲义明确拒绝了这条路的三个环节：初始候选就评估含可见性的 target function 太贵；所有复用样本重验可见性 UE 付不起；reservoir 直接复用样本会让样本模式呈 AABBCC 式重复，降噪器不喜欢（ABCABC 才友好）——于是 MegaLights 改为"不复用样本，复用历史可见性来引导采样"：可见灯表全评估 + 灯网格随机子集评估 + 棋盘/4-rooks 抖动模式 + 随机双线性上采样 + 时序方差驱动的空间滤波 [(TISTORY)](https://techartnomad.tistory.com/659) 。性能含义是官方承诺的"恒定成本曲线"：延迟着色是质量恒定、成本随灯数涨；MegaLights 是成本恒定（主旋钮 r.MegaLights.DownsampleMode 与 r.MegaLights.NumSamplesPerPixel）、质量随局部灯复杂度降 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/megalights-in-unreal-engine?lang=en-US) 。面光源用 2×2 bitmask 记录象限可见性（硬编码，计划改自适应层级），重复射线去重、预积分光照拆分阴影与着色是进一步的成本闸门 [(TISTORY)](https://techartnomad.tistory.com/659) 。rurix 的多光源路线裁决建议：默认复刻 MegaLights 哲学（与降噪器协同、恒定预算），把 RTXDI/ReSTIR 作为高端质量档备选；ReSTIR PT Enhanced（I3D 2026，2.08–3.05×：互惠邻居选择减半空间复用成本、足迹式重连判据、重复图、统一直接+GI reservoir、去遮挡降噪）表明学术前沿正在收敛"production-ready"，但其对照系是路径追踪 [(techvogue.blog)](https://techvogue.blog/blog/nvidia-restir-pt-enhanced-algorithm-making) 。

### 5.5 BLAS 治理与 RTX MegaGeometry

加速结构维护的规范级规则是：update（refit）只允许改实例定义、变换矩阵、顶点/AABB 位置，其余（几何数量、顶点/图元数、索引格式、geometry flags、active 状态）一律禁止，违反必须全量重建；作为交换，update"显著快于"build [(vulkan.org)](https://docs.vulkan.org/spec/latest/chapters/accelstructures.html) 。Arm 的最佳实践指南把工程策略说全了：静态 BLAS 用 PREFER_FAST_TRACE，TLAS 与动态 BLAS 用 PREFER_FAST_BUILD + ALLOW_UPDATE；蒙皮是 RT 的成本大户，应限制数量、按距离启发式降频更新、允许跨帧复用旧 AS（多数场景无可见瑕疵） [(arm.com)](https://learn.arm.com/learning-paths/mobile-graphics-and-gaming/ray_tracing/rt04_acceleration_structure/) 。RTX MegaGeometry 把这条曲线再推一档：CLAS（Cluster-level AS）把空间相邻的最多约 256 个三角形批量预组织成簇级加速结构并可落盘缓存，使 BVH 构建输入量降约两个数量级——"100× 更多光追三角形"的官方口径即源于此；NvRTX 5.6 分支与 Alan Wake 2（实装后 RTX 20/30 系受益最大，帧率 5–20% 提升、显存省约 300MB）是两个实装锚点 [(搜狐)](https://www.sohu.com/a/856847231_122180097) 。Vulkan 侧对应 VK_NV_cluster_acceleration_structure（nvpro 四样例：vk_animated_clusters / vk_tessellated_clusters / vk_lod_clusters / vk_partitioned_tlas），目前仍是 NVIDIA 厂商扩展 [(Github)](https://github.com/nvpro-samples/vk_lod_clusters/blob/main/README.md) 。rurix 的治理表建议写成三级：静态 cluster → 预构建 CLAS 式簇 BLAS（与第二章 cluster 管线同源）；动态实例 → TLAS 每帧 FAST_BUILD；蒙皮 → 数量预算 + refit 降频，超限回退 VSM/光栅阴影 [(arm.com)](https://learn.arm.com/learning-paths/mobile-graphics-and-gaming/ray_tracing/rt04_acceleration_structure/) 。

对 rurix 的排序建议：NRD 契约与 BLAS 治理纪律属于"现在就做"的零成本项（反推内核输出布局、写进 executor 的 AS 生命周期管理）；SER 与 OMM 依赖 VK_EXT_ray_tracing_invocation_reorder 与 VK_EXT_opacity_micromap 的实机覆盖率，列入 W4 可选波次并以 gpuinfo 数据做启动门禁；MegaGeometry/CLAS 目前绑 NVIDIA 厂商扩展，只做架构预留（cluster 数据结构与第二章同源即可），不做排期承诺。

---

## 第六章 方向六（精要）：材质、管线与资产管线

### 6.1 Substrate：参数混合的内存经济学

UE Substrate 的 layered BSDF 按"每像素每层一次光照求值"计价：两个 slab 的光照求值成本翻倍，内存也按层数线性增长。官方文档给出的精确案例与用户口径一致——四 slab 混合材质（两个 Vertical Layer + 一个 Coverage Weight）内存 **108 字节/像素**，开启 Use Parameter Blending 后合并为单 slab，降至 **28 字节/像素** [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/overview-of-substrate-materials-in-unreal-engine?lang=en-US) 。参数混合的语义是"先混合参数、后一次求值"：以单 slab 近似多层效果，移动端编译器自动启用，中间平台自下而上逐层引入以守住预算 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/overview-of-substrate-materials-in-unreal-engine?lang=en-US) 。

对 rurix 材质系统的裁决：把"参数混合"设为默认路径、"逐层求值"设为高质量档；材质内存（字节/像素）应成为与 PSO 数量并列的 CI 指标。一个容易踩的坑是混合在法线/粗糙度强对比层间的视觉损失——Substrate 官方也承认混合是"保持外观的近似"，验收需配图像对拍而非参数对拍 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/overview-of-substrate-materials-in-unreal-engine?lang=en-US) 。

### 6.2 PSO 预缓存与 GPL：编译卡顿的系统解法

Epic 官方技术博客给出 Fortnite 的精确口径：**每场比赛预编译约 30,000 个 PSO，实际使用约 10,000 个，而全组合空间是百万级**；驱动缓存为空时进入对局要多等 20–30 秒 [(Unreal Engine)](https://www.unrealengine.com/tech-blog/game-engines-and-shader-stuttering-unreal-engines-solution-to-the-problem) 。用户"排列减半"的说法在公开渠道未见出处，报告如实标注未核实；已核实的机制是"按材质×顶点工厂×网格 pass 处理器预测子集"，预编译后即弃、依赖驱动压缩缓存回填，保留内存方案要多付 1GB+ 内存 [(Unreal Engine)](https://www.unrealengine.com/tech-blog/game-engines-and-shader-stuttering-unreal-engines-solution-to-the-problem) 。控制台变量族已成型：r.PSOPrecaching/.Components/.Resources、ProxyCreationWhenPSOReady（PSO 未就绪则延迟 proxy，默认开）、ProxyCreationDelayStrategy（0=跳过绘制，1=回退默认材质）、KeepInMemoryUntilUsed 系列；排障用 r.PSOPrecache.Validation=2 与 -clearPSODriverCache 冷启动测试 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/pso-precaching-for-unreal-engine?lang=en-US) 。

Vulkan 侧的对等武器是 **VK_EXT_graphics_pipeline_library（GPL）**：把管线拆成顶点输入/预光栅/片段/输出四段库分别预编译，最终链接在 graphicsPipelineLibraryFastLinking 设备上"成本与录制一条命令相当"，可录制期按需链接；该扩展源自 VK_KHR_ray_tracing_pipeline 的 pipeline library 机制，Valve 在 Source 2 上用它消除了绘制期编译卡顿 [(Github)](https://github.com/KhronosGroup/Vulkan-Docs/blob/main/proposals/VK_EXT_graphics_pipeline_library.adoc) 。实战组合策略：材质加载时建库 → 无 LINK_TIME_OPTIMIZATION 快速链接先出图 → 后台编译优化版替换 [(Github)](https://github.com/KhronosGroup/Vulkan-Docs/blob/main/proposals/VK_EXT_graphics_pipeline_library.adoc) 。对 rurix：PSO 问题必须在 W1 之前立 CI（运行时编译计数硬门），GPL 评估与 rurixc 管线对象模型同步设计。

### 6.3 KTX2 / DirectStorage / FastGeo / SVT：资产管线的四块拼图

**KTX2 + Basis Universal** 是纹理压缩的当前最优折中：ETC1S 模式 0.3–3bpp（常用 0.3–1.25bpp，适合 albedo 等颜色纹理），UASTC 模式固定 8bpp（BC7 级质量，法线/数据纹理必选），UASTC 叠 RDO + Zstandard 超压缩后等效约 4–6bpp；运行时按设备转码 BC7/ASTC/ETC2，显存内保持压缩，相比 RGBA8 省 4–8× 显存 [(Github)](https://github.com/Oefenweb/ktx-basis-universal) 。**DirectStorage** 的版本阶梯已清晰：1.1（2022-10）引入 GDeflate GPU 解压，1.3 引入 EnqueueRequests 精细控制，**1.4 公开预览（2026-03-11，GDC 2026）加入 Zstd 编解码 + Game Asset Conditioning Library（GACL）**——构建期 shuffle/BLER/CLER（ML 引导的熵缩减）可把 Zstd 比率再提最高 50%，运行时由 DirectStorage 自动逆变换（现支持 BC1/3/4/5，BC7 后续），微软同时开源了 Zstd GPU 解压 compute shader（针对 ≤256KB 块优化） [(Microsoft Developer Blogs)](https://devblogs.microsoft.com/directx/directstorage-api-downloads/) 。Linux/通用侧没有 DirectStorage，等价物是 io_uring + GPU 直传（自研或 GDeflate/Zstd 计算着色器），rurix 的抽象层应按"请求队列 + 编解码器插拔 + 完成事件"建模 [(AMD GPUOpen)](https://gpuopen.com/download/GDC-2023-DirectStorage-optimizing-load-time-and-streaming.pdf) 。

**FastGeo（UE5.6 实验性，与 CDPR 共创）**把不变静态几何的注册/注销从 Actor/Component 体系里剥出来，用更轻量的路径进出渲染与物理场景，且全程预算化：Epic 官方 City Sample 内部配置给出 s.LevelStreamingActorsUpdateTimeLimit=1ms、s.UnregisterComponentsTimeLimit=1ms、FastGeo.AsyncRenderStateTask.TimeBudgetMS=1ms、ParallelWorkerCount=4 的参考档，UE5.7 又把 FastGeoContainer 的 PSO 预缓存移出游戏线程 [(Epic Dev)](https://dev.epicgames.com/community/learning/knowledge-base/r6wl/unreal-engine-world-building-guide) 。"预算化 AddToWorld/RemoveFromWorld"的准确语义由此明确：不是单次调用变快，而是每帧给流式任务固定时间片、超额排队——rurix 流式系统应把"毫秒预算 + 工作队列 + 可观测积压"作为一等公民。**SVT（Streaming Virtual Texturing）**侧，UE 的反馈分辨率因子直接刻画了核心权衡：反馈越密，CPU/GPU 开销越大，但流式延迟越低 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/virtual-texturing-reference?application_version=4.27) ；Vulkan 硬件底座是 sparse residency——部分驻留图像、mip tail 合并绑定、未驻留读取在 residencyNonResidentStrict 设备上返回零（否则未定义但保证不崩溃），且 vkQueueBindSparse 是队列级操作、与提交同级同步，大量小页绑定的成本不可小觑 [(The Khronos Group)](https://github.khronos.org/Vulkan-Site/spec/latest/chapters/sparsemem.html) 。rurix 若做 SVT，页表更新频率与反馈分辨率应并入与 VSM 页表相同的治理框架。

---

## 第七章 方向七（精要）：超分、帧生成与速度契约

### 7.1 速度矢量是第一公民

本章所有技术共享同一份输入契约：每帧的深度、运动矢量、曝光与亚像素抖动——契约错误是质量事故的第一来源（DLSS 官方开发者论坛的排障帖里，抖动是否并入 MV、mvecScale 符号、深度来源三个问题占了集成问题的大半） [(NVIDIA Developer Forums)](https://forums.developer.nvidia.com/t/dlss-ray-reconstruction-persistent-swimming-flowing-artifact-during-camera-motion-even-at-16-spp/361209) 。rurix 在 W1 波次落地 TAA 时就应把这份契约冻结为渲染图的标准通道：蒙皮网格与 WPO（世界位置偏移）顶点动画必须输出真实速度而非相机差分，UE 的 velocity pass 与 TSR 的 WeightClamping 系列旋钮都以这个前提成立 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/temporal-super-resolution-in-unreal-engine?lang) 。

这份契约一旦冻结，FSR/DLSS/XeSS/DirectSR/TSR 就都只是"契约的消费者"，可插拔。

### 7.2 四家方案的现状与约束

超分与帧生成市场在 2024–2026 年间完成了从"三家 SDK 各自集成"到"统一 API 收敛"的转折：FSR 3.1 把帧生成从超分中解耦并以签名 DLL 承载可升级性，DirectSR 试图以单一 API 抹平三家差异 [(Github)](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/releases) 。DLSS 4 把整条链路换成 Transformer 模型并引入多帧生成，XeSS 2.1 则把帧生成与低延迟开放给非 Intel 硬件 [(pcoptimizedsettings.com)](https://pcoptimizedsettings.com/the-4-technologies-driving-nvidia-dlss-4-multi-frame-generation/) 。四条路线的现状、约束与对 rurix 的适配性汇总如下。

| 方案 | 现状（2026-07） | 关键约束 | 证据 |
|---|---|---|---|
| FSR 3.1 | FidelityFX SDK 1.1.x，超分与帧生成解耦，FSR API 仅 5 个导出函数 + 签名 DLL，原生 DX12/Vulkan 后端 | Vulkan 帧生成交换链需额外应用数据；FG 建议基线 60+FPS |  [(Github)](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/releases)  |
| DirectSR | Agility SDK 预览（1.714→1.715），统一 API 下运行时选择 DLSS-SR/XeSS（驱动级）与内置 FSR（2.2→3.1） | **只覆盖超分，不含帧生成**；D3D12 限定 |  [(Developer Tech News)](https://www.developer-tech.com/news/microsoft-unveils-directsr-unify-super-resolution-technologies/)  |
| DLSS 4 | Transformer 时空模型（自/交叉注意力），MFG 的 AI 光流取代 OFA：FG 快 40%、省 30% 显存、最多 3 中间帧 | 黑盒 SDK + Streamline；输入含深度/MV/曝光/抖动矩阵；硬件 Flip Metering 依赖 Blackwell |  [(gamegpu.com)](https://en.gamegpu.com/test-video-cards/Quality-Performance-and-Mfg-Big-Review-of-DLSS-4-on-a-Laptop)  |
| XeSS 2.x | SDK 2.1（2025-08）起 XeSS-FG/XeSS-LL 开放给任意 SM6.4 GPU（DP4a 计算路径；Arc 走 XMX） | **FG 仅 D3D12，无 Vulkan**；非 Intel 最多插 1 帧；XeLL 与 FG 绑定 |  [(jonpeddie.com)](https://www.jonpeddie.com/news/intels-xess-2-supports-ai-driven-frame-generation-on-all-gpus/)  |

对 Vulkan 优先的 rurix，结论直接：超分层 FSR 3.1（开源 MIT、Vulkan 原生、质量持续迭代）为默认，DLSS 经 Streamline 为高端增强；DirectSR 与 XeSS-FG 因 API 边界暂不进入 Vulkan 主线的首屏清单，但契约层不为其设障 [(Github)](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/releases) 。TSR 作为 UE 内建参照系提供了最完整的旋钮集合：r.TSR.History.ScreenPercentage=200（Nyquist-Shannon 驱动的历史双倍分辨率，历史更新成本 ×4）、r.TSR.History.SampleCount 8–32（默认 16）、r.TSR.AsyncCompute 0–3（默认 2，只放无关键路径依赖的 pass 进异步队列）、Resurrection 与 ShadingRejection 系列 [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/temporal-super-resolution-in-unreal-engine?lang) 。rurix 自研 TAA/TSR 时，这些旋钮的含义与默认值可直接移植为初始设计稿。

### 7.3 帧生成的延迟纪律

三家 FG 的公开文档共同指向同一条纪律：帧生成放大吞吐但不降低（甚至略增）输入延迟，因此基线帧率门槛必须强制执行——AMD 建议 60+FPS、Intel 要求 40FPS 最低/60FPS 推荐、且 Intel 把低延迟方案（XeLL）与 FG 绑死 [(VideoCardz.com)](https://videocardz.com/newz/amd-fidelityfx-sdk-1-1-brings-fsr-3-1-with-improved-upscaling-and-frame-generation-working-with-dlss-xess) 。rurix 的 FG 开关逻辑应内置：基线 FPS 低于门槛时 UI 层禁用 FG；全屏菜单/暂停时停插帧；运动模糊与 FG 的相互作用按 Intel 指南处理（关闭或改造） [(Github)](https://github.com/intel/xess/blob/main/doc/xess_fg_developer_guide_english.md) 。这些不是优化而是正确性行为，进验收清单。

落地顺序上，FG 严格排在超分稳定之后：FSR 3.1 的解耦设计允许"先只接 upscale 函数、后接 FG"两步走 [(Github)](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/releases) ，rurix 应在 W1 冻结速度矢量契约后先验收纯超分路径（误差图红线），再把 FG 作为独立功能位灰度放量；DLSS 4 的 MFG（多帧生成）因绑 Streamline 与 NVIDIA 驱动栈，仅作高端档可选项，不进入基线验收矩阵 [(pcoptimizedsettings.com)](https://pcoptimizedsettings.com/the-4-technologies-driving-nvidia-dlss-4-multi-frame-generation/) 。

---

## 第八章 方向八（精要）：执行器、时间戳与性能硬门

### 8.1 sync2 与执行器原语

VK_KHR_synchronization2（已入 Vulkan 1.3 核心）对执行器设计的三项实质改进：VkDependencyInfoKHR 把三类屏障收进单结构、stage/access 同处声明使依赖关系自洽可机检；VK_IMAGE_LAYOUT_ATTACHMENT_OPTIMAL / READ_ONLY_OPTIMAL 两个"做正确的事"布局消灭了颜色/深度布局分流样板；vkCmdSetEvent2KHR 允许事件携带屏障 [(vulkan.org)](https://docs.vulkan.org/guide/latest/extensions/VK_KHR_synchronization2.html) 。队列家族所有权转移（QFO）的"分裂屏障"模式——src 队列提交只填 srcAccess 的释放屏障、dst 队列提交只填 dstAccess 的获取屏障、两侧都填对方的 stage 与信号/等待信号量配对——是异步计算/拷贝队列间移交资源的规范姿势，Khronos 官方同步示例逐条给出 [(Github)](https://github.com/khronosgroup/vulkan-docs/wiki/synchronization-examples) 。

rurix 的 executor（帧图）应把"屏障 = 资源×阶段×访问三元组的边"作为 IR 的自然语义，sync2 恰好是这个 IR 的 API 形态；异步计算的收益不要先验承诺——UE 只把 TSR 中无关键路径依赖的 pass 默认放进异步队列（r.TSR.AsyncCompute=2），MegaLights/Lumen 则以共享 RT Scene 摊薄而非盲目堆队列，说明工业界的共识是"可重叠才重叠" [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/temporal-super-resolution-in-unreal-engine?lang) 。

### 8.2 CPU 侧并行：UE5.5 的量级证据

UE5.5 的渲染线程优化给出一个公开量级锚点：**RHI 命令列表向平台命令列表的并行翻译，最高 2× 性能提升（部分平台省 7ms）**；同版本异步 RDG execute 任务再省约 0.4ms 关键路径，并让约一半 Slate 渲染异步化 [(Tom Looman)](https://tomlooman.com/unreal-engine-5-5-performance-highlights/) 。

对自研 executor 的推论：提交路径（rhi translate）与录制路径（rdg execute）是两个独立的并行化战场，各自都有毫秒级收益；线程优先级（UE5.7 把 Game/Render/RHI 提为 AboveNormal）这类"零代码"项也应早期纳入基线 [(Tom Looman)](https://tomlooman.com/unreal-engine-5-7-performance-highlights/) 。GPU 自主化的更激进形态是 D3D12 Work Graphs：正式版随 Agility SDK 1.614 发布（Epic 在发布公告中背书其对应 Nanite 式 GPU 驱动渲染的 CPU 减负），而 mesh nodes 自 1.715-preview（2024-07）起至 1.719-preview（2026-02）仍停留在预览通道——**截至本报告日期，mesh nodes 未进零售 SDK，Vulkan 侧更无对等物，rurix 不应将其纳入首期依赖** [(Microsoft Developer Blogs)](https://devblogs.microsoft.com/directx/directx12agility/) 。

### 8.3 GPU 时间戳与性能硬门

GPU 侧计时的规范做法：VkQueryPool（VK_QUERY_TYPE_TIMESTAMP）+ vkCmdWriteTimestamp，时间戳周期由物理设备 limits.timestampPeriod 给出；实践经验是只用 TOP_OF_PIPE / BOTTOM_OF_PIPE 两端点——中间阶段的戳因 GPU 流水重叠而无意义，跨队列的戳不可互减，且 vkCmdWriteTimestamp 本身带有类似屏障的执行依赖副作用（插多了会扰动被测对象） [(pavelsmejkal.net)](https://pavelsmejkal.net/Posts/GPUTimingBasics) 。另一个反直觉陷阱：GPU 在低负载时会降频，CPU-bound 场景里测得的 GPU 时长会虚长——性能 CI 必须配合锁频（稳定功耗状态）或在足够负载下采样 [(pavelsmejkal.net)](https://pavelsmejkal.net/Posts/GPUTimingBasics) 。性能硬门的落地形态建议三层：帧内每层预算计数器（VSM 页管理/GI/降噪/超分各挂时间戳域）；场景级基准（固定相机路径 + 锁频）进 CI，回归阈值按毫秒而非 FPS 表达（"快了多少 ms"才是线性量） [(pavelsmejkal.net)](https://pavelsmejkal.net/Posts/GPUTimingBasics) ；设备级矩阵（首批：NV 40/50 系、AMD RX 7000、Intel Arc B 系，外加一款 Vulkan 1.3 移动 GPU 守门 ray query 基线）。

最后一条纪律是把时间戳数据流接到第九章的验证体系：性能硬门与画质红线共用同一套 CI 捕获（renderdoc 注入 + 固定场景重放），性能计数器随捕获一并归档，使"画质回归"与"性能回归"在一次 CI 运行内同判。这样第八章与第九章共享基础设施，验证体系的建设成本只计一次。

---

## 第九章 方向九（精要）：验证体系——指标、确定性与 CI

### 9.1 图像指标的分工

PSNR/SSIM 与 FLIP 不是竞争而是分层：前者是全保真数值度量（回归门槛、缓冲 ABI 校验），后者是感知度量（"玩家能不能看出来"的裁决）。FLIP（Andersson et al. 2020，HPG）为渲染误差专门建模——对比敏感度与空间频率驱动、输出 0–1 误差加空间误差图，LDR 与 HDR 双变体，BSD-3 开源、pip 可得（flip-evaluator），并有 RT Gems II 的工具章配套 [(Github)](https://github.com/NVlabs/flip) 。

其在渲染界的采用已是事实标准（近年论文普遍 PSNR/SSIM/LPIPS/FLIP 并列报告） [(arXiv.org)](https://arxiv.org/html/2505.21925v1) 。rurix 的指标纪律：黄金比对第一级用逐位/容差比对（VisBuffer 等结构化缓冲必须逐位），第二级用 PSNR/SSIM 门槛（回归红线），第三级用 FLIP 均值 + 误差图（画质评审与放行裁决）；host-device 比对（方向九的核心诉求）走第二、三级，不走第一级——原因见下。

### 9.2 确定性的真实边界

"确定性渲染"必须精确表述：**同 GPU 架构 + 同驱动 + 同库版本 + 同提交序**内，GPU 计算可以做到逐位复现；跨架构、跨驱动版本、跨 host/device，浮点舍入与调度序差异使逐位一致在原理上不成立——NVIDIA 自身文档（PhysX/Isaac、TAO）都明确"可复现性是按硬件计的"，GPU 工作调度的次序变化会传导到最低有效位 [(isaac-sim.github.io)](https://isaac-sim.github.io/IsaacLab/main/source/features/reproducibility.html) 。

rurix 的确定性契约因此应写成两档：设备内确定性（同机同驱动，用于性能 CI 与黄金图回归，要求原子序与随机种子固定——VisBuffer 的 u64 atomicMax 天然满足交换结合律，是少数跨调度序仍逐位稳定的原子模式）；host-device 等价性（黄金标准断言以容差 + 感知度量表述，239 单测的断言库需要按此分层改造，凡是隐含位等的断言都要标记为"仅 host 域"）。这为 RD-038 之后的每一次内核移植提供了统一的验收语言。

### 9.3 renderdoc 化 CI 与黄金图流水线

验证流水线的推荐形态：引擎内置 renderdoc 注入式捕获（命令行触发、按帧触发、带 API 验证层开关），CI 对固定场景集自动捕获 → 重放 → 导出关键 render target → 三级指标比对 → 误差图归档为构建产物 [(RenderDoc)](https://renderdoc.org/docs/getting_started/faq.html) 。截图比对进 CI 是行业通行做法（从 Applitools 到自研 diff 脚本），RenderDoc 的重放侧 GPU 选择、验证层受控等机制已文档化，可直接依赖 [(moldstud.com)](https://moldstud.com/articles/p-troubleshooting-unity-for-cross-platform-apps-a-developers-comprehensive-guide) 。三条工程纪律：黄金图必须与驱动版本绑定入库（驱动升级 = 黄金图主动重基线，而非被动变红）；误差图（FLIP map）而非标量分作为评审入口，标量阈值只做红线；每次内核移植（W1–W4 每波）以"host 黄金输出 → device 输出"的误差图 diff 作为合并门槛，与 239 单测并列 [(nvidia.com)](https://research.nvidia.com/sites/default/files/node/3260/FLIP_Paper.pdf) 。高级图形工程师岗位描述里"自动化性能测试 + 捕获比对 + CI 回归阈值"已是明文交付物，说明这套体系是行业基线而非超额设计 [(Devopsschool.com)](https://www.devopsschool.com/blog/senior-graphics-engineer-role-blueprint-responsibilities-skills-kpis-and-career-path/) 。

体系建设的触发点建议与波次路线图绑定：W1 波次落地时只需 host 域位等与 PSNR/SSIM 红线两级（此时 device 内核尚未引入新能力，误差源有限）；W3 引入 ray query 后必须补齐 FLIP 感知裁决——随机采样类内核（RTAO、GI）的逐像素位等既不现实也无意义，只有感知指标能区分"无差异的随机种子扰动"与"真实回归"。FLIP 的 BSD-3 许可与 pip 可装性使其没有引入门槛 [(Github)](https://github.com/NVlabs/flip) 。

---

## 第十章 跨方向汇总：裁决、路线与待确认项

### 10.1 一页裁决表

九个方向的调研结论压缩为一张裁决表：每行给出该方向的最终取舍、以及支撑该取舍的关键数字锚点与证据出处，供评审会直接引用。

| # | 方向 | 核心裁决 | 关键数字锚点 |
|---|---|---|---|
| 1 | 着色器供给 | a 主线 + b 补充 + c 有条件；四波推进；ray query 强制 SPIR-V 1.4 | W1 零新能力；W2 两能力；W3 版本门槛 + 3 扩展链  [(The Khronos Group)](https://www.khronos.org/blog/vulkan-ray-tracing-final-specification-release)  |
| 2 | Nanite | VisBuffer u64 布局冻结为 ABI；两级 HZB + PrimeHZB/MinLOD 借入；植被独立路径；细分缓行 | 32px 阈值；128KB 页 / 2GB 池；Bevy 0.49ms  [(Unbiased Gamer)](https://unbiased-gamer.com/the-mental-model-for-unreal-engines-nanite-virtualized-geometry-and-cluster-culling/)  |
| 3 | VSM | 采样驱动分配；SMRT RayCount 为运行时旋钮；灯→阴影路径数据驱动 | 命中率 >95%；移动灯无效化 0.4–0.8ms；预算 ≤3ms@60Hz  [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/virtual-shadow-maps-in-unreal-engine?lang=en-US)  |
| 4 | Lumen | 显式缓存为主线（DDGI+重采样备选、NRC 暂缓）；多反弹 = Surface Cache 跨帧反馈（N+2） | <2ms@1080p 目标与 HWRT 共享 RT Scene 绑定  [(Tom Looman)](https://tomlooman.com/unreal-engine-5-6-performance-highlights/)  |
| 5 | RT 增强 | SER/OMM 列 W4 可选；NRD 契约反推内核输出布局；多光源走 MegaLights 哲学 | 黑神话 3.7×；AW2 39%（SER+OMM）；ReSTIR PT Enhanced 2.08–3.05×  [(The Khronos Group)](https://www.khronos.org/blog/boosting-ray-tracing-performance-with-shader-execution-reordering-introducing-vk-ext-ray-tracing-invocation-reorder)  |
| 6 | 资产管线 | 参数混合默认化；PSO 硬门 + GPL 评估；KTX2 双模式；FastGeo 式时间预算 | 108→28 B/px；3 万/1 万 PSO；ETC1S 0.3–3bpp / UASTC 8bpp  [(Epic Dev)](https://dev.epicgames.com/documentation/unreal-engine/overview-of-substrate-materials-in-unreal-engine?lang=en-US)  |
| 7 | 超分/FG | 速度契约 W1 冻结；FSR 3.1 默认 + DLSS 高端；FG 基线帧率门禁 | FSR API 5 函数；DLSS4 FG 快 40%/省 30% 显存；XeSS-FG 无 Vulkan  [(Github)](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/releases)  |
| 8 | 执行器 | sync2 即帧图 IR 形态；提交/录制双路并行；时间戳只取顶/底端点 | UE5.5 并行翻译 2×/7ms；异步 RDG 0.4ms  [(Tom Looman)](https://tomlooman.com/unreal-engine-5-5-performance-highlights/)  |
| 9 | 验证 | 位等（host 域）/数值（红线）/感知（FLIP 裁决）三级；确定性按硬件计 | FLIP BSD-3 可 pip；renderdoc 捕获进 CI  [(Github)](https://github.com/NVlabs/flip)  |

读表口径：方向 1 的四波推进是其余八个方向的前置约束——方向 2–7 的设备化内核分别落在 W1–W3 的能力包内，方向 8–9 的基础设施（帧图同步、验证分级）则与波次并行建设、不占用着色器供给的关键路径。

### 10.2 待确认项（如实登记）

三项用户输入在本轮调研中未能从公开渠道核实，建议内部确认后再入库：其一，**"Mlakar（CGF）"文献**——网络检索与学术检索（按作者、按主题）均无匹配，最接近的公开文献是 Benthin & Peters 2023（Computer Graphics Forum，微多边形 RT 的 HLOD）与 Pusch 2026（Nanite LOD-meshlet 学位论文），疑似内部引用口径或拼写变体。

其二，**Fortnite PSO"排列减半"**——官方口径只有"预编译 3 万/用 1 万/全组合百万级"，无"减半"表述 [(Unreal Engine)](https://www.unrealengine.com/tech-blog/game-engines-and-shader-stuttering-unreal-engines-solution-to-the-problem) ；其三，**各扩展的硬件普及率**（VK_KHR_shader_atomic_int64、ray query、SER/OMM 的实机覆盖率）——vulkan.gpuinfo.org 统计页本轮未能取数，W2/W3/W4 各波次启动前应以其当时数据做门禁。

### 10.3 收尾：RD-038 的解阻定义

回到起点：RD-038 的解除不应以"ray query 能跑"为终点，而应以四层验收定义——rurixc 对 Int64/Int64Atomics/StorageImageWriteWithoutFormat/RayQueryKHR 等能力的 lowering 各有黄金对拍（W1–W3 每波交付物）；七类内核的 device 输出对 host 黄金标准在约定容差与 FLIP 阈值内（方向九流水线承载）；性能硬门在首批设备矩阵上达标（VSM ≤3ms、GI <2ms@1080p 等方向三/四目标）；扩展能力以特性位而非编译期分支管理（对应 Slang 能力原子机制的设计启发，在 rurixc 内部自建轻量版） [(shader-slang.org)](https://docs.shader-slang.org/en/latest/external/slang/docs/user-guide/a3-02-reference-capability-atoms.html) 。做到这一层，着色器供给从"阻塞项"变为"可预期产能"，后续八个方向的移植才有节奏可言。

---

*本报告基于 2026-07-29 前的公开资料与学术文献整理，所有性能数字均为来源方在特定硬件/场景下的实测口径，移植到 rurix 前应以自有场景复测为准。*
