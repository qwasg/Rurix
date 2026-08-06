# G8_CAPABILITY_MATRIX — UE5 级能力 → Rurix 现状与缺口矩阵

> **所属**：G8 文档集（`milestones/g8/`；计划状态见 [G8_PLAN.md](G8_PLAN.md)——**计划定稿，G8.1 governance-only active / G8.2+ blocked**）。上游输入：[research/R1](research/R1_UE5_RENDERER_PANORAMA.md) · [research/R2](research/R2_PHYSICS_CHAOS_JOLT.md) · [research/R3](research/R3_GPU_API_ASSET_PIPELINE.md) + 本文 §0 仓内实况核对。下游消费者：[G8_CONTRACT](G8_CONTRACT.md) / [G8_CANDIDATE_DECISIONS](G8_CANDIDATE_DECISIONS.md) / [G8_ACCEPTANCE_MAP](G8_ACCEPTANCE_MAP.md)。
> **基准日**：2026-08-02（deferred.json v1.73、spike_gating.json v1.10、G7 active）。
> **纪律**：G8.0 基线零编号；G8.1 只 claim RFC-0019~0021，G7 active 仍可能消费的 RXS/RD/U/RX/数字 CI 空间不占用。不改写 G5/G6/G7 结论。行号 `M##` 为本文档内部定位标识，非 ledger 编号。
> **「承 G7」警告**：G-G7-9 允许 RD-038 仍 open 时收口（见 G7_CONTRACT）。当前全部“承 G7”行均为 **unresolved dependency**，在 RD-038 closed 前不得视为已交付；若 G7 收口而 RD-038 仍 open，按 G8_PLAN §1.0 的互锁终态把遗留行改标 G8.x。
> **条件型 RD**：RD-039/040/041/044 分项不得以「UE5 级目标」静默改写 backfill；进主线须 G8.1 决策表 go 证据或 strategic_override（G8_PLAN §1.2）。
> **图例**：✅ 已交付 · 🟡 部分 · ⬜ 缺失 · 🔬 门控观察；档位 A/B/C/D；优先级 P0~P3；4070Ti = 可否真机验证。

---

## 0. 仓内实况核对方法与关键事实

现状列的证据来自本次逐条核对（2026-08-02），核对面：

- **注册表**：`registry/deferred.json`（RD-034~044 全条目逐字读取）、`registry/spike_gating.json`（SG-002/003/004/005/010）。
- **规格**：`spec/shader_stages.md`（RXS-0242~0245、RXS-0297~0299）、`spec/vulkan_backend.md`（RXS-0246~0248、RXS-0300）、`spec/rhi.md`（RXS-0270 task/RT 条件臂、RXS-0280~0283）。
- **源码 grep**：`src/rurix-rt/src/vk.rs`（独立 compute/graphics queue 句柄在位；`VkPipelineCache`/timeline semaphore/sparse/descriptor buffer/HDR metadata 本次核对零命中）、`src/rurix-physics*`（vehicle/character/soft body 包装零命中）、`src/rurix-render/src/streaming|material`（磁盘 I/O 调用零命中——页流送为内存内驻留模型）。

关键既成事实（矩阵行反复引用，先行钉死）：

1. **RT 面现状**：语言已有 RT 六阶段类型面 + `emit_*_min` SPIR-V 见证（RXS-0242~0248）与 `vk.rs` 最小 RT 运行路径；**RT pipeline + SBT 完整语义/运行时未做**（RD-040 分项字面）；DXIL RT 腿 blocked-on-upstream（RD-034，步骤 69 探针恒跑）；RHI 图形 pass 的 RT 条件臂未立类型面（`spec/rhi.md` RXS-0270）。
2. **RayQuery**：RXS-0297~0300 条款已冻结（RFC-0018 Agent Approved 2026-08-01），codegen/AS descriptor/三效果核 = **G7.2~G7.4 在途**——G8 视之为前置依赖，不重复占用。
3. **task shader**：RXS-0270 字面「task 前置条件臂首期不开放」，评估窗未消费。
4. **render graph 执行面**：单 queue 全序 + 自动 barrier + happens-before（RXS-0236~0241）；transient 别名复用 + DAG 重排调度已兑现（RXS-0280~0283，RD-035 closed）；**async compute 多 queue / split barrier 显式不做**（G3.5 收窄留痕，earmark RD-034+ 家族）。
5. **流送**：G5 页式流送（`PageRequest`/`StreamingBudget`，128KB 页）为**内存内页驻留模型**，无磁盘异步 I/O + 解压链。
6. **物理**：Jolt 5.3.0 刚体底座 ✅（G6，contact/查询/批插/CCD/睡眠）；vehicle/character/ragdoll/soft body/浮力**均未包装**；capture/replay 与网络层不存在。
7. **P3+ 债务已字面登记**：RD-039（几何 P3+：mesh shader 光栅路/HZB/cluster 流送 P4/Foliage 骨骼/细分位移/Assemblies/Mega Geometry）、RD-040（光照 P3+：SMRT/世界辐射缓存/自适应探针/SDF 软追踪/ReSTIR·MegaLights/**RT pipeline+SBT**/SER·OMM/NRD）、RD-041（材质流送时域 P3+：多层 slab/**SVT**/KTX2-BasisU 转码/vendor 超分/FG-MFG/蒙皮 WPO MV/Work Graphs）、RD-042/043（物理研究轨观察）、RD-044（物理 P3+：软体布料流体生产化/Taichi 生产面/Rapier 深造）。**G8 的大量行即这些 RD 分项的承接裁决**。

---

## 1. 渲染：虚拟化几何

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M01 | 离线 cluster/meshlet DAG 构建 | R1 §3.1 | ✅ `src/rurix-geom-build`（meshlet 化 + 分组简化层级 DAG + CPU 参照剔除，host 纯 safe 确定性） | **版本化页格式 ABI**（builder 侧）须在流送消费前冻结 | C/D | P0 | ✔ | **G8.3**（与 M04 同波冻结 ABI） |
| M02 | GPU 两级剔除 + VisBuffer SW/HW 双路光栅 | R1 §3.1 | 🟡 G5 host + RD-038 W1/W2 device；SW/HW 对拍 = G7.5 在途 | persistent culling / occlusion feedback | C | P1 | ✔ | 承 G7（受 §引导「承 G7」警告约束）；遗留→G8.5a |
| M03 | HZB 两阶段遮挡剔除 | R1 §3.1 | ⬜ RD-039（backfill：剔除效率成为 **measured** 瓶颈） | HZB 构建 + 两阶段重投影 | C | P1 | ✔ | G8.5a（**仅** G8.1 决策表 go/override；默认 no-go） |
| M04 | 集群压缩与正式磁盘页格式 | R1 §3.1 / §6.1 | ⬜ RD-039「cluster 流送 P4」；**格式定版**与「超显存」运行时触发分离（见 G8_PLAN §1.2） | 磁盘/内存格式分离、量化、解码 ABI、golden 往返 | C/D | P0 | ✔ | **G8.3**（ABI 冻结；G8.4 M44 只消费不重定） |
| M05 | WPO / 动态位移 tessellation | R1 §3.1 | ⬜ 语言无位移语义；RD-041 蒙皮/WPO MV 分项 open | A：位移/bounds/velocity 语义；C：programmable geometry | A/C | P1 | ✔ | 语义→RFC-0019；实现→G8.5a（决策表） |
| M06 | 骨骼/植被虚拟几何（Skinned/Foliage/Assemblies） | R1 §3.1（5.7/5.8 专项） | ⬜ RD-039 分项 | deformer ABI、skin cache、微实例 | A/B/C/D | P2 | ✔ | G8.7 评估（backfill 条件维持「动态资产面出现时」） |
| M07 | RT fallback/proxy 几何与主几何误差联动 | R1 §3.1 | ⬜ | fallback 构建器 + BLAS 派生数据 | C/D | P1 | ✔ | G8.5a |
| M08 | programmable raster / material binning | R1 §3.1（GDC 2024） | 🟡 classify/resolve host 参考 + G7 真实帧路径在途 | GPU material dispatch 全量化 | C | P1 | ✔ | G8.5a |
| M09 | Mega Geometry / 簇级 BLAS（虚拟几何直接进 RT AS） | R1 §6.1 | ⬜ RD-039 分项 | — | B/C | P2 | ✔ | G8.7 评估（RT 与虚拟几何合流需求出现时，RD-039 backfill 字面） |

## 2. 渲染：光照与 GI

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M10 | 屏幕探针 GI | R1 §3.2 | ✅ G5 `gi::` host 全量（白炉守恒/收敛单测）；device 化 `gi_probe.rx` = G7.4 在途 | — | — | 承 G7 | ✔ | 承 G7 |
| M11 | 世界空间辐射缓存（多级） | R1 §3.2 / §6.3 | ⬜ RD-040（backfill：屏幕探针远场缺失成为画质 **measured** 问题） | GPU hash/grid allocator、probe budget | B/C | P1 | ✔ | G8.5b（**仅**决策表 go/override；默认 no-go） |
| M12 | Surface Cache / cards / 表面缓存 | R1 §3.2 / §6.3 | ⬜ | 离线 card 生成、材质重定向编译、coverage 诊断 | C/D | P2 | ✔ | G8.7 评估 |
| M13 | SWRT 距离场追踪（mesh DF + Global DF） | R1 §3.2 | ⬜ RD-040「SDF 软追踪」相邻面；决策表裁决 | DF 资源与增量更新、DF builder、tiered tracing | B/C/D | P1 | ✔ | G8.5b（决策表） |
| M14 | HWRT hit lighting / Far Field 分层 AS | R1 §3.2 | ⬜（依赖 M50 RT pipeline） | 近/远场 AS、hit 材质求值 ABI | A/B/C | P2 | ✔ | G8.7（M50 后评估） |
| M15 | MegaLights / ReSTIR DI 随机直接光照 | R1 §3.3 / §6.2 | ⬜ RD-040 分项；G7_CONTRACT out_of_scope 字面维持 | reservoir 库、light DB、时空复用+去噪 | A/C | P2 | ✔ | G8.7 条件消费（RD-040 backfill「多灯场景需求出现时」） |
| M16 | 低成本 irradiance field 档位（Lumen Lite 类） | R1 §2（5.8） | ⬜ | probe occlusion 档位化 | C | P2 | ✔ | G8.7 评估 |
| M17 | Path Tracer 参照器（全材质/MIS/累积） | R1 §5.1 | 🟡 `rt::ref_tracer` host + ruridrop 离线 PPM；无完整 RT pipeline 级参照器 | MIS/累积/材质跨路径 golden | C/D | P1 | ✔ | G8.5b（依赖 G8.2 M50 **增量**面，非 RXS-0248 最小见证） |

## 3. 渲染：阴影

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M18 | VSM 页表 + page-mark | R1 §3.4 | ✅ G5 `shadow::vsm` host 全量 + W1 page-mark device；VSM depth/sample device 化 = G7.5 在途（RD-038 字面矩阵分项） | — | — | 承 G7 | ✔ | 承 G7 |
| M19 | VSM 完整页缓存（跨帧 cache/失效分类/clipmap scroll/local light pages/非虚拟几何 caster） | R1 §3.4（16K 虚拟/128×128 页） | ✅ G8.5a `shadow::{events,local,page_cache}` + `uc06 --m19-vsm-page-cache`（16 腿 device） | 物理页池 LRU/age、失效原因分类、multi-view 批量 | B/C | P0 | ✔ | G8.5a |
| M20 | SMRT 软阴影完整版 | R1 §3.4 | ⬜ RD-040（backfill：VSM device 化后可独立 Mini） | 采样端沿光线多采样 | C | P1 | ✔ | G8.5a（依赖 M19/RD-038 VSM device；决策表） |
| M21 | ray query 硬阴影 | — | ✅→G7.4 `hard_shadow.rx` 在途 | — | — | 承 G7 | ✔ | 承 G7 |
| M22 | 海量灯阴影（MegaLights 配套 RT/VSM 阴影统一接口） | R1 §3.3 | ⬜ | 统一阴影查询接口 | C | P2 | ✔ | G8.7（随 M15） |

## 4. 渲染：时域重建与超分

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M23 | TAA/TSR 底座（MV/jitter/历史验证） | R1 §3.5 | ✅ G5 `temporal::` host 全量 + G6 物理 MV 供给；TSR device 化审计 = G7.5 在途（RD-038 分项「TSR 是否仍只有 host reference」） | — | — | 承 G7 | ✔ | 承 G7 |
| M24 | TSR 生产契约（history resurrection / pixel animation 材质语义 / thin geometry / 动态分辨率 / 透明 velocity） | R1 §3.5 | ✅ G8.5b `temporal/contract` + RFC-0019 §4.6.4 `rfc_budget_frozen`（13 腿 device） | A：时域材质语义；C：rejection/resurrection；D：序列回归 | A/C/D | P0 | ✔ | 语义→RFC-0019；实现→G8.5b |
| M25 | vendor 超分插件面（FSR/DirectSR/DLSS 输入 ABI） | R1 §6.4 | ✅ G8.5b UpscalerInputAbi v1 + TSR/CAS 双非 no-op（12 腿；vendor FSR FFI 仍 open 观察） | 标准输入 ABI | B/C | P1 | ✔ | G8.5b（接口不改底座） |
| M26 | 帧生成 FG/MFG | R1 §6.4 | ⬜ RD-041 分项「FG/MFG 为独立层另判」；G7 out_of_scope 字面 | — | — | P3 | 部分 | 不进 G8（RD-041 观察维持） |

## 5. 材质与着色器工程体系

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M27 | 单层 principled 材质闭合 | R1 §4.1 | ✅ G5 `MaterialClosure` 32B（拓扑字段位已预留） | — | — | ✅ | ✔ | — |
| M28 | 多层 closure 材质 IR（Substrate 类：分层/混合/平台降级/跨路径 lowering） | R1 §4.1 | ⬜ RD-041「多层材质 slab」（backfill：单层闭合成为**真实资产瓶颈**） | A：closure 类型/组合/lowering；C：adaptive payload | A/C | P1 | ✔ | **语义边界进 RFC-0019**；决策表 no-go，故 G8 不实现；真实资产瓶颈或逐项 override 后另开 |
| M29 | shader permutation 域/静态裁剪/预算 | R3 §3.4 | ⬜（现状单入口小内核集，无 permutation 体系） | A：specialization 常量、permutation key、域定义；D：analyzer/预算报告 | A/D | P0 | ✔ | G8.2 |
| M30 | PSO precache / pipeline cache / pipeline binary | R3 §3.4 | ⬜（`vk.rs` 本次核对零 `VkPipelineCache` 命中） | B：异步编译服务、分层缓存（源码→IR→shader library→PSO→驱动 binary）；C：collector/precacher/遥测 | B/C | P0 | ✔ | G8.2 |
| M31 | 结构化 reflection / shader interface hash | R3 §3.1 | 🟡 绑定布局推导（RXS-0163~0170）+ artifacts v2 设备描述表（RXS-0290~0292）已有；稳定 reflection schema 与 interface hash 未定义 | 可序列化 reflection schema、hash 进 DDC 键 | A/B | P0 | ✔ | G8.2 |
| M32 | capability/profile 类型化检查 | R3 §2/§3.1（Slang 对标） | 🟡 运行时 fail-closed 能力协商已成体系（KernelWave W1/W2/W3 capability snapshot、feature chain 探测、per-entry SPIR-V 版本策略 RFC-0018 §B）；**语言级** capability/profile 类型检查缺 | A：`capability(...)` 约束进类型检查、profile 驱动 fallback specialization | A | P0 | ✔ | G8.2（语义面经 Full RFC） |
| M33 | shader library / 多 entry 组合链接 / 独立编译 | R3 §3.1 | 🟡 模块系统 + 单产物嵌入已有；shader library 级组合链接面缺 | 可序列化中间 IR、entry-point 组合 | A/B | P1 | ✔ | G8.2 |
| M34 | wave-size range / long vector 等 SM6.8/6.9 对齐面 | R3 §3.2 | ⬜ | `[WaveSize(min,max,preferred)]` 类语义评估 | A | P2 | ✔ | G8.7 评估 |

## 6. 场景、流送与虚拟纹理

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M35 | GPU 场景实例表 + 物理单向同步 | — | ✅ G5 `GpuScene` + G6 `PhysicsBridge`（单向事实源五纪律） | — | — | ✅ | ✔ | — |
| M36 | 页式流送预算模型（内存内） | R1 §4.2 | ✅ G5 `streaming::`（`PageRequest`/`StreamingBudget`/128KB 页/反馈驱动）+ G6 页驻留驱动 body 批插移除 | — | — | ✅ | ✔ | — |
| M37 | 磁盘异步 I/O + 解压 + 上传流送链 | R3 §2.5 | ✅ G8.4 `uc06 --stream-io`（StreamIoPool 真盘读 + 冻结 RXPD decoder + GPU FNV；`queue_mode=single`；迟到页 fallback） | I/O、解压、copy、tile map 分离 timeline；迟到页处理；预算与优先级 | B/C | P0 | ✔ | G8.4 |
| M38 | DirectStorage/GDeflate 与 CPU fallback | R3 §2.5 | ⬜ | D：GDeflate/Zstd packer（GDeflate 参考实现 Apache-2.0 可 vendor）；B：解压调度与 GPU 竞争预算 | B/D | P1 | ✔ | G8.4 |
| M39 | sparse residency / tiled resources | R3 §2.6 | ⬜（`vk.rs` 零 sparse 命中） | sparse binding/residency 通道 | B | P1 | ✔ | G8.4 |
| M40 | SVT 虚拟纹理（运行时 + 烘焙） | R1 §4.2 / R3 §4.4 | ⬜ RD-041 **标题**含 SVT，但 backfill **无独立 SVT 门槛**（G8.1 须补「真实大纹理资产管线」或 strategic_override） | A：虚拟采样；B：residency；C：页表/atlas；D：VT baker | A/B/C/D | P1 | ✔ | G8.4 门-VT（**仅**决策表 go/override；与门-GeomPage 独立，禁止二选一充绿） |
| M41 | sampler feedback（D3D12）/ shader feedback 通用路径 | R3 §2.6 | ⬜ | 通用 feedback image/atomic bitset 路径优先 | B/C | P2 | ✔ | G8.4 增量 |
| M42 | RVT 运行时生成 | R1 §4.2 | ⬜ | — | C | P2 | ✔ | G8.7 评估 |
| M43 | World Partition / HLOD / scene-cell 调度 | R1 §4.2 | ⬜ | C：world scheduler；D：HLOD/cell builder | C/D | P2 | ✔ | G8.7 评估（大世界资产面出现时） |
| M44 | 几何页/BLAS/OMM 派生数据流送闭环 | R3 §5 | ⬜（运行时消费端） | streamer **只消费 G8.3 已冻结的 M04 ABI**；禁止本波重定格式 | B/C | P1 | ✔ | G8.4 门-GeomPage（独立硬门） |

## 7. 后处理、显示与场景类型

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M45 | HDR 场景色管线与 HDR 输出 | R1 §5.1 / R3 §2.7 | ⬜（present 现为 FIFO + RGBA8/BGRA8；`vk.rs` 零 HDR metadata） | 色彩空间协商、HDR metadata、scene-linear | B/C | P1 | ✔ | G8.5b |
| M46 | 后处理栈（bloom/DOF/motion blur/自动曝光/局部曝光/tonemap 分级） | R1 §5.1 | 🟡 仅 soft-raster/uc06 tonemap 级 | 后处理子图、曝光、LUT | C/D | P1 | ✔ | G8.5b |
| M47 | 透明渲染 / OIT 策略 | R1 §5.1 | ⬜ | 透明 pass、排序/OIT、透明 velocity | A/B/C | P1 | ✔ | G8.5b |
| M48 | 体积雾/体积云 | R1 §5.1 | ⬜ | froxel 光照、时域重投影 | C | P2 | ✔ | G8.7 评估 |
| M49 | 水体/毛发/皮肤/地形/贴花 场景类型族 | R1 §5.1 | ⬜ | 各专项渲染器与资产工具 | C/D | P2 | ✔ | G8.7 评估/后续期 |
| M49a | GPU 粒子 VFX 渲染侧 | R1 §5.1 | 🟡 Taichi Vulkan AOT external import spike 成功臂（uc09，G6.5）；粒子渲染模块与 VFX 图缺 | emitter/render 模块、VFX 资产图 | C/D | P2 | ✔ | G8.7 评估（联动 RD-044 Taichi 生产面分项） |
| M49b | present pacing / 低延迟（present_wait / Reflex 类） | R3 §2.7 | ⬜ | present ID/wait、latency marker | B | P2 | ✔ | G8.7 |

## 8. RT 与现代 GPU API 能力

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M50 | **完整 RT pipeline + SBT**（raygen/miss/closest-hit/any-hit/intersection/callable、payload ABI、递归/栈管理、pipeline library） | R1 §5.2 / R3 §2.2 | ✅ **G8.2 M50 增量面 closed**（步骤 103；RXS-0322~0327）：多 hit group + SBT user data + stack + pipeline library device 真跑；冻结子集 RED-GREEN；`vk_rt`/RXS-0248 最小见证不得代绿。RD-040 **总体仍 open**（SER/OMM 等分项未兑现）；DXIL 腿 = RD-034 blocked | G8 退出门增量面已兑现；其余 RD-040 分项留 G8.7 | A/B | **P0** | ✔ | **G8.2**（strategic_override 已兑现；DXIL 不强攻） |
| M51 | inline RayQuery（compute） | — | 🟡 RXS-0297~0300 条款冻结（RFC-0018）+ 前端检查在树；codegen/AS descriptor/三效果核 = G7.2~G7.4 在途 | — | — | 承 G7 | ✔ | 承 G7（受「承 G7」警告；遗留→G8.2） |
| M52 | SER / hit-object 重排 | R3 §2.2 | ⬜ RD-040 分项「SER 与 OMM」 | hit-object intrinsic（依赖 M50） | A/B | P2 | ✔ | G8.7 条件消费 |
| M53 | Opacity Micromap（OMM） | R3 §2.2 | ⬜ RD-040 分项 | B：micromap build/BLAS attach；D：离线烘焙 | A/B/D | P2 | ✔ | G8.7 条件消费 |
| M54 | ray tracing position fetch | R3 §2.4 | ⬜ | — | B | P2 | ✔ | G8.7 |
| M55 | descriptor buffer / shader object / DGC（`VK_EXT_device_generated_commands`） | R3 §2.4 | ⬜（bindless 经 descriptor indexing 已交付 RXS-0231~0235；descriptor buffer/DGC 未用） | DGC 优先（GPU-driven 提交）、descriptor buffer 作高性能后端 | B | P1 | ✔ | G8.7（DGC/descriptor buffer 先行，shader object 评估） |
| M56 | D3D12 Work Graphs（compute nodes；mesh nodes 另计） | R3 §2.1 | ⬜ RD-041 分项「Work Graphs 与 mesh nodes」；报告5 既判 P3+ | A：node/record 语义；B：backing memory | A/B | P2 | ✔ | G8.7 评估（RD-041 backfill 字面「Vulkan 侧对应物成熟且 pass 内部提交单元可替换接缝已预留」维持） |
| M57 | Vulkan AMDX shader enqueue | R3 §2.1 | ⬜ provisional、AMD-only | — | — | P3 | ✘ | 不进 G8 |
| M58 | cooperative vector / 神经着色（NTC/NRC/RTX Neural Shaders） | R1 §6.4 / R3 §2.3 | 🔬 SG-002（Tensor Core 族）conditional not_triggered；DX 原设计已宣布弃用重构（R3 [S4]） | 若未来触发：先 IR 抽象后厂商 intrinsic | A/B | P3 | 部分 | 不进 G8（SG-002 维持；重审条件见 §12） |
| M59 | 多队列 async compute / timeline semaphore | R3 §2.7 | 🟡 `vk.rs` 独立 compute/graphics queue 句柄在位；执行面单 queue 全序；timeline semaphore 零命中 | **语义须进 RFC-0019**（ownership/timeline/跨队列 barrier/无专用队列 fallback）；触 Barrier 冻结面须 RFC 修订行 | B/C | P1 | ✔ | G8.4（**无 RFC-0019 Approved → 强制单队列**）；async compute → G8.5a/b 评估 |
| M60 | 64 位原子 / synchronization2 / ray query 设备能力 | — | ✅ RFC-0016 §4.0-2 能力链 fail-closed（`VK_KHR_ray_query`/`shader_atomic_int64`/synchronization2）+ G7 场景冻结 capability snapshot | — | — | ✅ | ✔ | — |
| M61 | mesh shader 光栅路径（第三路径） | R1 §3.1 | 🟡 语言 mesh/task 类型面 + `run_mesh_offscreen` 最小见证（RXS-0243/0246~0248）；作为光栅第三路径 = RD-039 分项 | mesh 路径与 VisBuffer 合流 | A/B/C | P2 | ✔ | G8.7 条件消费（RD-039 backfill 字面「多厂商行为收敛 + measured 证据」维持） |
| M62 | task shader 开放 | — | ⬜ RXS-0270 字面「task 前置条件臂首期不开放」评估窗未消费 | task 阶段语义评估 | A | P2 | ✔ | G8.2 评估窗（随 M50 RFC 一并裁决开/不开） |
| M63 | VRS（fragment shading rate） | R3 §2.7 | ⬜ | shading-rate builtins + rate image | A/B/C | P2 | ✔ | G8.7 评估 |

## 9. 物理平台

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M64 | 刚体底座（多核/睡眠/批插/CCD/并发查询/接触事件/SyncBudget） | R2 §2.2~2.4 | ✅ G6 `rurix-physics`（Jolt 5.3.0 自维护 JoltC FFI，U33~U43 审计，CI 步骤 88~92） | — | — | ✅ | ✔ | — |
| M65 | Rapier 快路径对拍 | — | ✅ G6.4（parity 七判据，默认 off） | — | — | ✅ | ✔ | — |
| M65b | Rapier 快路径**深造**（真实 workload 采用面） | RD-044 backfill | ⬜ G6.4 仅为对拍见证；生产 workload 未采用 | parity 扩展场景、性能/功能差距闭环 | C | P2 | ✔ | G8.7 穷举（**仅**决策表 go；默认 no-go；RD-044 四拆之一） |
| M66 | physics capture/replay + 状态哈希 + divergence 定位 | R2 §2.13 / §3.1 | ✅ G8.6a materialize：`physics-capture` + 10 场景 corpus + 15 腿 smoke（步骤 120）；divergence 注入定位 | B/C：snapshot delta、body lifecycle journal、重演比对器 | B/C | P0 | ✔ | **G8.6a**（已绿；M73 钉 5.3） |
| M67 | 网络物理层（input/state history、prediction、rollback/resimulation、事件去重、平滑） | R2 §2.12 | ⬜ | C：physics frame ID、快照环、server correction | C | P0 | ✔ | **G8.6b** |
| M68 | 破坏生产链（预破碎资产/connection graph/strain 断键/层级 cluster 激活/cache/VFX 事件桥） | R2 §2.5 | ⬜ Jolt 无内建 fracture；**未被 RD-044 覆盖的新缺口面** | C：GeometryCollection 等价运行时；D：Voronoi/plane fracture cook、interior face、anchor | C/D | P0 | ✔ | **G8.6c** |
| M69 | PhysicsAsset / ragdoll / physical animation | R2 §2.8 | ⬜（§0 核对：rurix-physics 零 character/ragdoll 包装；Jolt Ragdoll/motor 原语在 vendor 内未暴露） | C：骨骼刚体映射、pose motor、partial simulation；D：collider/joint authoring | C/D | P1 | ✔ | G8.6b |
| M70 | 载具产品层 | R2 §2.7 | ⬜（Jolt `VehicleConstraint` 未包装） | C：drivetrain/tire 包装与状态序列化；D：调参资产/telemetry | C/D | P1 | ✔ | G8.6d |
| M71 | 角色控制器（CharacterVirtual 包装） | R2 §3.1 | ⬜ | C：包装 + 状态保存（网络联动） | C | P1 | ✔ | G8.6b |
| M72 | 布料（开放 panel/seam/fabric schema、DCC 导入、碰撞/LOD、独立求解时间线） | R2 §2.6 | ⬜ Jolt soft body 未包装（且官方限制：无 self-collision 等，R2 §3.1）；RD-044 分项 | C：XPBD cloth 或 Jolt soft body 扩展；D：资产 schema + USD 导入验证 | C/D | P1 | ✔ | **G8.6d**（RD-044 Cloth；决策表） |
| M73 | Jolt 5.3→5.6 升级评估（GPU compute 抽象/新摩擦模型/ragdoll motor） | R2 §3.2 | 🟡 wave6a subject：`pin_5_3_honest_stop_loss`（无 JoltC-next，不伪绿 5.6） | B：replay/perf/CCD 回归 + 资产重烘焙规则 | B/D | P1 | ✔ | **G8.6a**（诚实钉 5.3；双二进制 A/B 后置） |
| M74 | Physics Field 等价（统一空间影响） | R2 §2.14 | ⬜ | field evaluator + 资产化 | C/D | P2 | ✔ | G8.7 评估 |
| M75 | 异步物理 tick / physics thread 时间域 | R2 §2.11 | 🟡 固定步 + accumulator 在宿主（RFC-0017 冻结面）；独立 physics thread/异步 tick 未做 | frame-domain 契约、回调时序 | B/C | P2 | ✔ | G8.7 评估 |
| M76 | 软体 Flesh / MPM / FLIP / 神经布料 / 可微物理 | R2 §4 | 🔬 Continuum/Fluid → RD-044 P3 观察；**Differentiable → RD-042**（不进 RD-044 四拆）；RD-043 维持 | — | — | P3 | 部分 | 不进 G8 硬门（见 G8_PLAN §1.4） |
| M77 | 水体/浮力 gameplay 面 | R2 §2.10 | ⬜（Jolt `ApplyBuoyancyImpulse` 原语未包装） | 浮力包装 + water body 面（渲染侧 M49 联动） | C | P2 | ✔ | G8.7 评估 |
| M78 | GPU 主刚体 | R2 §5.1 | 🔬 G6 否决线维持（`G6_PLAN` §0.1；RD-043 观察） | — | — | P3 | — | 不进 G8（五条重审条件登记 §12） |

## 10. 资产管线与确定性构建

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M79 | SourceAsset/ImportRecipe/DerivedArtifact/CookProfile schema + 确定性双构建验证 | R3 §4.1/§4.6 | ⬜（`rurix-pkg` 为包管理非资产管线；lockfile+vendor+checksum+无任意构建脚本纪律为既有优势基座） | D：schema 定义、canonical serialization、双构建 hash 相等 CI | D | P0 | ✔ | G8.3 |
| M80 | 内容寻址派生数据缓存（DDC） | R3 §4.1 | ⬜ | D：artifact key（源+依赖+工具版本+profile 全哈希） | D | P0 | ✔ | G8.3 |
| M81 | glTF 2.0 导入器（锁定扩展集） | R3 §4.5 | ⬜ | D：严格 schema/validator（Rust 自研或审计 crate） | D | P0 | ✔ | G8.3 |
| M82 | meshoptimizer 对接与交叉验证 | R3 §4.2 | ⬜（`rurix-geom-build` 自研在位，无第三方交叉验证基准） | D：作为 partition/simplification 参照与 codec 基准（MIT 可 vendor） | D | P1 | ✔ | G8.3 |
| M83 | 纹理压缩管线（BCn/ASTC/KTX2/Basis；mip/normal/alpha coverage 语义） | R3 §4.3 | ⬜ RD-041 分项「KTX2-BasisU 真转码器接入」（`PagedResource::transcode` 留口已冻结） | D：vendor Basis Universal/KTX-Software/astc-encoder（Apache-2.0）+ 自研确定性 wrapper；Compressonator 需逐目录许可审计、NVTT3 仅可选外部工具 | D | P1 | ✔ | G8.3（承接 RD-041 分项） |
| M84 | VT tile / OMM / BLAS 派生数据烘焙器 | R3 §4.4 | ⬜ | D：VT baker；**OMM baker 仅决策表 go 后做**（未触发禁止抢跑） | D | P1 | ✔ | VT/BLAS → G8.3；OMM → G8.7/条件；消费 → G8.4 |
| M85 | shader/PSO manifest 进 DDC（与 M29~M31 一体） | R3 §3.4 | ⬜ | A/B/D：manifest 格式、合并去重、覆盖率分析 | A/B/D | P0 | ✔ | G8.2 + G8.3 |
| M86 | USD 受限 ingest/export adapter | R3 §4.5 | ⬜（TOST 1.0 许可需法务清单如实标注，非 Apache-2.0） | D：受限集成不内嵌 composition | D | P2 | ✔ | G8.7 评估/后续期 |
| M87 | MaterialX schema/stdlib（→ typed material IR） | R3 §4.5 | ⬜ | D：vendor schema + 自研映射（联动 M28） | D | P2 | ✔ | G8.7 评估 |
| M88 | 打包 chunk/manifest 与流送安装清单 | R3 §4.1 | ⬜ | D：PackageChunk 对齐/压缩/依赖/优先级 | D | P1 | ✔ | G8.3 |

## 10a. 单源图形派发（RD-037 承接）

| 行 | 能力 | UE5 基线 | Rurix 现状（证据锚） | 缺口要点 | 档位 | 优先级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|
| M89 | `.rx` 单源 gfx submit 真派发（声明式 gfx 图零 Rust 宿主出图） | —（Rurix 自有目标：单语言双层模型兑现面） | ✅ **RD-037 closed**（G8.2 M89 materialize）：rurixc lowering gfx pass vs/fs SPIR-V 入 artifacts v2 + cabi VB/IB 绑定面 + `rxrt_rhi_submit` gfx 派发臂；device 真跑 readback/golden（步骤 102） | 判据 = RD-037 backfill 字面已兑现：`.rx` gfx 图零 Rust 宿主 device 真跑 readback 像素断言 | A/B | P0 | ✔ | **G8.2**（正式承接 RD-037；UE5 级渲染器要以 `.rx` 为主语言书写，此为硬前置） |

---

## 11. 汇总

### 11.1 P0 地基清单（G8 必做，18 行）

| 波次 | P0 行 |
|---|---|
| G8.2 shader/RT 基座 | M50 RT **增量** pipeline+SBT · M89 单源 gfx submit（RD-037）· M29 permutation · M30 PSO 缓存 · M31 reflection/hash · M32 capability/profile · M85 shader/PSO manifest（与 G8.3 共担）——均须 `G8_ACCEPTANCE_MAP` 硬门 |
| G8.3 资产闭环 + 页 ABI | M79 schema+确定性 · M80 DDC · M81 glTF · **M01** DAG/builder 页格式 · **M04** 压缩磁盘页格式（ABI 冻结，供 G8.4 消费） |
| G8.4 流送 | M37 磁盘异步 I/O+解压链（门-VT∥门-GeomPage 独立过门） |
| G8.5a 几何/阴影 | M19 VSM 完整页缓存 |
| G8.5b 材质/时域 | M24 TSR 生产契约 |
| G8.6a–c 物理 | M66 capture/replay（6a）· M67 网络（6b）· M68 破坏（6c） |
| G8.8a soak | 全部 P0 硬门绿后 soak；**禁止**条件实现后跳过 soak 直接 close |

### 11.2 统计

- **行数**：92 行（M01~M89 + M49a/M49b + **M65b**）；实施/评估行随决策表浮动。
- **优先级分布**（约数，立项时按决策表复核）：P0 = 18 · P1 ≈ 30 · P2 ≈ 28（含 M65b）· P3/不进 G8 = 5 · ✅/承 G7 = 11（承 G7 在 RD-038 closed 前不算交付）。
- **档位分布**：含 A 档共 23 行——伞形 RFC 语义面主体；其余重心在 B/C/D。
- **4070 Ti 可验证性**：除 M57、M26/M58/M76/M78 外全部可真机验证——与 P-09 兼容。
- **RD 承接映射**：RD-037 → M89（G8.2）；RD-038 遗留 → §G8_PLAN 1.0 接入表；RD-039 → M03/M04/M06/M09/M61（各需决策表）；RD-040 → M11/M15/M20/M50/M52/M53；RD-041 → M05/M25/M26/M28/M40/M56/M83（SVT 补门槛）；RD-044 → M72 Cloth / M65b Rapier 深造 / M49a Taichi / Continuum·Fluid P3；**Differentiable → RD-042**；RD-034/043 维持不承接。

### 11.3 G8 成功判据草案（十条，立项时进 G8_CONTRACT 硬化为验收门）

渲染/平台侧：

1. RT pipeline **增量面**端到端 device 真跑（多 hit group + SBT 用户数据 + stack sizing + pipeline library；any-hit/intersection/callable 按 RFC-0019 子集；**禁止**仅复述 RXS-0248 最小见证）。
2. `.rx` 单源 gfx 图零 Rust 宿主代码 device 真跑 readback 像素断言（RD-037 backfill 字面判据）。
3. 资产管线确定性：同一输入双构建 artifact hash 逐字节相等（CI 门）；**M01/M04 页格式 ABI 在 G8.3 冻结**。
4. 磁盘→GPU 页流送：**门-VT 与 门-GeomPage 各自独立过门**（禁止二选一充绿）+ 迟到页降级可见证；多队列须 RFC-0019 或诚实单队列回退。
5. permutation/PSO/reflection（M29–M32/M85）均有 CI 硬门，不得因「部分实现」假绿 close。

物理侧（承 R2 五门槛）：

6. physics capture 完整重演固定步世界并定位首个 divergence。
7. 网络物理预测/权威修正/rollback-resimulation/事件去重全链见证。
8. 预破碎资产 fracture cook→层级 cluster→strain 断裂→cache→VFX 事件全链见证。
9. PhysicsAsset 完成 ragdoll/physical animation/vehicle/character collider authoring 闭环。
10. 布料开放资产 schema + DCC 导入 + 碰撞 + LOD + 独立求解时间线。

---

## 12. 门控维持与重审条件登记（只读引用，不改写既有注册表）

| 项 | 维持裁决 | 重审条件（字面来源） |
|---|---|---|
| GPU 主刚体 | 否决线维持（G6_PLAN §0.1；RD-043 观察） | R2 §5.1 五条件**同时**成立：① 数千活跃刚体超 CPU Jolt 能力的真实场景；② Vulkan 渲染 profiling 证明稳定 GPU headroom；③ GPU 方案覆盖查询/CCD/回滚/容错；④ end-to-end 帧时间优于加 CPU 核/物理 LOD；⑤ 跨 NVIDIA/AMD 后端策略成立 |
| cooperative vector / 神经着色 | SG-002 conditional not_triggered 维持 | DX 统一线性代数设计 retail 落地 + Vulkan 跨厂商（非 NV-only）扩展批准 + L2/渲染基准证明真实瓶颈（SG-002 trigger 同构） |
| autodiff / 可微渲染·物理 | SG-004 permanent 维持 | 生态包层面探索允许，不动语言核心（离线参数拟合走工具层） |
| kernel fusion / 稀疏结构 | SG-005 permanent 维持 | 同上 |
| Work Graphs / mesh nodes | RD-041 分项维持 | RD-041 backfill 字面：Vulkan 侧对应物成熟 + 「pass 内部提交单元可替换」接缝已预留；G8.7 仅做 D3D12 评估探针不进硬门 |
| MegaLights / ReSTIR | RD-040 分项维持 | RD-040 backfill 字面：多灯场景需求出现时 |
| 帧生成 FG/MFG | RD-041 分项维持 | 独立层另判（vendor 超分插件面 M25 先行） |
| AMDX shader enqueue / GPUDirect Storage | 不进 G8 | provisional AMD-only / Linux-only，均不可在 4070 Ti+Windows 验证 |
| DXIL RT 腿 | RD-034 维持 open | 上游二选一解锁（spirv-cross RT 消费路径或 LLVM 签名钳制解除）；步骤 69 探针恒跑不强攻 |
| 窗口/UI 框架进语言 | SG-010 留续号维持 | 编辑器/工具 UI 若立项走宿主库与外部工具，不进语言 |

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.3 | 2026-08-06 | **G8.6a M66 materialize**：M66→✅；M73→🟡 `pin_5_3_honest_stop_loss`。 |
| v1.2 | 2026-08-02 | 对齐 G8_PLAN v1.2 双门解耦：G8.1 governance-only active、G8.2+ blocked；“承 G7”全记 unresolved；编号仅 RFC-0019~0021，其他共享在途空间零占用；RFC-α 具体化为 RFC-0019；M50 单独 strategic_override，M28 no-go 不实现。 |
| v1.1 | 2026-08-02 | **对齐 G8_PLAN v1.1 评审修订（暂不定稿）**：承 G7/条件型 RD 纪律；M01/M04→G8.3；M28→RFC-α+5b；M40 SVT 门槛；M50 增量退出门；M59 多队列 RFC-α；M65b Rapier 深造；P0 波次与成功判据防假绿；Differentiable→RD-042。 |
| v1.0 | 2026-08-02 | 初版：基于 R1/R2/R3 三份深度调研 + 仓内实况核对建立能力矩阵；P0/RD 映射、成功判据草案、门控重审条件。零编号占用。 |
