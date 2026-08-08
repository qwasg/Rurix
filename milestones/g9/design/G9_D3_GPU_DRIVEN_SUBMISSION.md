# G9-D3 设计草案：GPU-driven 提交与着色器系统深化

> **DRAFT 设计提案——G9 未立项，不构成契约/验收承诺**。本文件为 G9 模块 D3 的设计输入草案，
> 供 G9 立项（G9_CONTRACT / G9_PLAN）时裁剪、裁决与硬化。所有波次、验收门、RXS 区间均为**建议值**，
> 立项裁决前不产生任何契约效力。G8 承接锚以 `milestones/g8/G8_P2_DECISIONS.md`（v1.0，2026-08-06）
> 与 `milestones/g8/G8_CAPABILITY_MATRIX.md` 字面为准。
>
> 版本：v0.1（2026-08 起草）；作者：G9 模块设计子 agent；状态：**未批准**。
> **G9.0 冻结引用**：2026-08-08 起，本文作为 G9.0 文档集不可变基线附件被 [G9_PLAN.md](../G9_PLAN.md) 冻结引用；正文 0-byte，后续变更只追加修订记录（追加于文末）。

## ① 定位与承接锚

D3 是 G9「UE5 级渲染器」的**提交面与着色器治理底座**：把 G8 已绿的单源 gfx submit（M89）、
render graph 自动 barrier（RXS-0236~0241）、bindless（RXS-0231~0235）、permutation（M29）、
PSO cache（M30）、reflection hash（M31）、capability profile（M32）、manifest↔DDC（M85）
升级为 **GPU-driven submission**（compute pre-pass 零 CPU 回读产出命令）与**工业级 shader
library/变体治理**。没有 D3，G9 的三级剔除（instance→cluster→triangle）、cluster 流光栅、
大规模材质变体都缺少合法提交通道——D3 是渲染器主体（D4+）的前置底座。

**法定承接锚**（G8_P2_DECISIONS.md 行字面）：

| 锚 | G8 裁决 | 行字面要点 | D3 承接方式 |
|---|---|---|---|
| **M55** descriptor buffer / DGC | defer-to-G9+ | 「P1 无 G8 硬门；属建造期渲染器主体」→「G9+ GPU-driven 提交」；矩阵行字面「DGC 优先（GPU-driven 提交）、descriptor buffer 作高性能后端」 | 本模块主体，波次 1/2 |
| **M33** shader library 组合链接 | defer-to-G9+ | 「M85 manifest/DDC 已覆盖打包主需求；完整 library 组合链接未交付、无独立 workload」→「G9+ shader library 深化」 | 本模块主体，波次 3 |
| **M52** SER | no-go 留档 | RD-040 高分歧 RT workload 未触发 | §⑤ 处置裁决建议：**调整为语言层原语 + capability 可选** |
| **M56** Work Graphs | no-go 留档 | RD-041 双条件（Vulkan 对应物成熟 + 接缝预留）字面未满足 | §⑤ 处置裁决建议：**维持 no-go**，render graph schema 预留 fan-out 接缝 |
| **M59** async compute 第二腿 | no-go 留档 | G8.4 默认单队列；无 measured 收益证据 | §⑤ 处置裁决建议：**维持 no-go** |
| **M61** mesh shader 第三光栅 | no-go 留档 | RD-039 双条件（跨厂商收敛 + measured）未成立 | §⑤ 处置裁决建议：**调整为可选 geometry pipeline**，波次 4 |
| **M62** task shader 开放 | no-go 留档（不开放） | RXS-0270「task 前置条件臂首期不开放」；G8.2/M50 实况维持不开放 | §⑤ 处置裁决建议：**随 M61 重估，仍默认不开放** |

**调研结论（设计依据，以下§④/⑤引用为「依据 N」）**：

1. DGC 跨厂商化：`VK_EXT_device_generated_commands`（2024 底自 NV 版升级，Vulkanised 2025 重点，
   Ricardo Garcia/Igalia/Valve）= Indirect Commands Layout 模板 + DGC buffer（GPU compute 可填充）+
   Indirect Execution Set（同状态仅换 shader 的管线数组，GPU 侧索引切换——DX12 无对应能力）；
   限制：每 sequence 恰一个 dispatch token 且最后、不可开 render pass/插 barrier/绑 descriptor set。
   DGC 已成 ExecuteIndirect 语义的跨 API 最小公倍数（Proton/DXVK 底层）。Work Graphs 仍 D3D12 独占
   （活跃驱动 GPU 74.1%，几乎无 shipped 游戏，Sawicki 2025 估计首款硬需约 3 年），Vulkan 无对应物。
2. `VK_EXT_descriptor_buffer` 是大规模 bindless 现行标准（NVIDIA 官方推荐 bindless 大表）；
   2026 新动向 `VK_EXT_descriptor_heap` 提案（免 image view、统一常量数据、真 heap 化，
   Sascha Willems 2026-06 示例）——关注不实现，capability profile 预留 feature 位；反射/manifest
   记录「资源→全局 descriptor 索引」映射而非 set/binding 对。
3. SER 双 API 标准化完成（`VK_EXT_ray_tracing_invocation_reorder` 2025-11 + DXR 1.2/SM 6.9
   语法强制、实现可选）；硬件收益目前集中 NVIDIA（glTF PT +47%，coherence 23%→54%）。
4. shader library 工业答案 = 编译期 IR 链接（Slang 式模块系统，Khronos 托管，有官方 Rust crate）
   为主轴 + API 期链接（VK_EXT_graphics_pipeline_library / VK_EXT_shader_object / RT pipeline
   collection / execution set）为后端映射；离线组合解决组合爆炸（数量），运行时链接解决 hitching
   （延迟），分工不替代。UE permutation 治理 = manifest 精确枚举 + DDC 去重 + 变体审计工具
   （案例 1300 万→400 万变体）。RuriX 差异化点：自有编译器可在自家 IR 层做函数级组合链接，
   物化输出 SPIR-V/DXIL。
5. `VK_EXT_mesh_shader` 跨厂商收敛完成（活跃驱动 GPU 95.95%，Sawicki 2025）；mesh shader 入口
   数据 = cluster 流，顺序必须在 meshlet 格式与 GPU-driven 剔除之后；保留传统 VS 光栅为唯一
   fallback，不做双套全功能并行。
6. GPU-driven submission 范式：compute pre-pass 在 GPU 上产出命令缓冲零 CPU 回读（DGC 解决
   compute pre-pass 结果回传 stall）；三级剔除是入场券；自动 barrier 系统需新增「buffer 从
   storage-write 变 indirect-command-read」依赖边类型。

## ② 范围 in / out

**in（D3 主体）**：

- DGC 抽象层与三后端映射（Vulkan `VK_EXT_device_generated_commands` 原生 / D3D12
  `ExecuteIndirect` + command signature / NVPTX 侧仅做命令 buffer 数据生成的 compute 编码，
  不承诺 NVPTX 消费路径）。
- descriptor buffer 全局表（`VK_EXT_descriptor_buffer` 单一大表 + 「资源→全局 descriptor 索引」
  反射/manifest 记录面；`VK_EXT_descriptor_heap` 只预留 feature 位）。
- command build compute node 与 render graph 集成（新增 AccessKind 依赖边类型：
  indirect-command-read；compute pre-pass → DGC buffer → ExecuteIndirect 的图内表达）。
- Indirect Execution Set 与 PSO 体系衔接（同状态仅换 shader 的管线数组，GPU 侧索引切换；
  与 M30 PSO cache / M85 manifest 的 key 组成扩展）。
- shader library IR 链接（自有 IR 函数级组合链接，物化 SPIR-V/DXIL；API 期链接作为后端映射
  备选面）。
- 变体预算工具（manifest 精确枚举 + DDC 去重 + 变体审计/报告，承接 M29/M30/M85 治理面）。
- SER 语言原语（hit object / reorderThread(hint,bits) 内建 + capability 可选 + 材质 flags 位段
  预留 coherence hint；渲染器集成延后）。
- mesh shader 可选 geometry pipeline 路径（承接 M61 重估；以 cluster 流为入口数据）。

**out（明确不做，防范围蔓延）**：

- Work Graphs 任何实现（M56 维持 no-go；仅 render graph 节点 schema 预留「GPU 端 fan-out」
  表达能力字段，不接线）。
- async compute / 多队列（M59 维持 no-go；RXS-0239「单 queue 全序」承诺字面不动）。
- 三级剔除算法本体、meshlet 格式、cluster 流构建（归渲染器主体模块；D3 只提供其提交通道
  与依赖边类型）。
- RT 渲染器中 SER 的实际调度集成（材质 hint 消费、重排收益测量属后续专项；D3 只落语言原语
  与 capability 面）。
- `VK_EXT_descriptor_heap` 实现、`VK_EXT_shader_object` 全量切换、`VK_EXT_graphics_pipeline_library`
  生产接线（均为映射备选面/关注项，非本模块承诺）。
- 传统 VS 光栅路径的任何删减（唯一 fallback 地位不动）。

## ③ 依赖前置

| 前置 | 状态 | 证据 / 条款锚 |
|---|---|---|
| render graph 自动 barrier + AccessKind 单源 | ✅ G8 全绿 | `spec/render_graph.md` RXS-0236~0241；`src/rurix-rt/src/graph.rs`；`src/rurix-render/src/graph/`（compile/sync/types 四趟编译 + EB 三轴映射） |
| bindless 无界数组 + 动态索引 | ✅ | `spec/binding_layout.md` RXS-0233；`spec/shader_stages.md` RXS-0231~0232 |
| permutation 域/canonical key/预算 | ✅ M29 | `src/rurixc/src/permutation.rs`（RXS-0308~0310，RX3019/RX7023）；`g8_m29_shader_permutation_evidence_schema.json` |
| PSO cache | ✅ M30 | `g8_m30_pso_cache_evidence_schema.json` |
| reflection v1 + interface hash + pipeline key | ✅ M31 | `src/rurixc/src/reflection.rs`（RXS-0304~0307）；`g8_m31_reflection_hash_evidence_schema.json` |
| capability profile + `#[requires]` ID 闭集 | ✅ M32 | `src/rurixc/src/capability_check.rs`（RXS-0311~0313，v1 冻结十项 ID）；`g8_m32_capability_profile_evidence_schema.json` |
| shader/PSO manifest v1 + merge/dedup + DDC 往返 | ✅ M85 | `spec/rendering_platform.md` RXS-0317/0318；`g8_m85_shader_manifest_ddc_evidence_schema.json` |
| mesh/task 语言入口契约 + SPIR-V 编码 | ✅（最小见证） | `spec/shader_stages.md` RXS-0243；`spec/vulkan_backend.md` RXS-0246；`run_mesh_offscreen` 见证 |
| RT payload/attribute/callable 类型契约 + capability ID | ✅ M50 绿 | `spec/shader_stages.md` RXS-0244/0245；RXS-0311 `rt.*` ID |
| 单源 gfx submit | ✅ M89 | `g8_m89_single_source_gfx_submit_evidence_schema.json`（RD-037 兑现） |
| G8.4 单队列冻结面 | ✅ 约束输入 | RXS-0239「单 queue；声明序=提交序=pass 粒度完成序」字面不动 |

**阻塞性前置（D3 开工前必须成立）**：G9 立项裁决 D3 进合同；`VK_EXT_device_generated_commands` /
`VK_EXT_descriptor_buffer` 在目标硬件（4070 Ti 起步 + CI 设备清单）的 capability snapshot 实测
确认（走 M32 snapshot 核验原语 RXS-0313 既有机制，fail-closed）。

## ④ 模块分解

### D3.1 DGC 抽象层与三后端映射

- **核心抽象**：`IndirectCmdLayout`（Indirect Commands Layout 模板）声明式描述一个命令 sequence
  的 token 序列（vertex/index buffer 绑定、push constant、draw/dispatch indexed 等）；`DgcBuffer`
  为 GPU 可写的命令数据 buffer；`ExecutionSet` 为同状态仅换 shader 的管线数组（依据 1）。
- **限制内化为类型约束**（依据 1）：每 sequence 恰一个 dispatch token 且最后、不可开 render
  pass、不可插 barrier、不可绑 descriptor set——这些限制不进运行时检查，而在 layout 声明的
  编译期/装配期核验中 fail-closed（沿 RXS-0237 装配核验先例）。
- **三后端映射表**（单一事实源，镜像 RXS-0238「双后端映射同源」纪律）：

| 抽象 | Vulkan | D3D12 | NVPTX |
|---|---|---|---|
| IndirectCmdLayout | `VkIndirectCommandsLayoutEXT` | command signature（`ID3D12CommandSignature`） | 不承诺（仅命令数据生成） |
| DgcBuffer 填充 | GPU compute 直写 + `vkCmdPreprocessGeneratedCommandsEXT` | GPU compute 直写 argument buffer | compute kernel 产出 buffer 数据（既有 launch 语义） |
| 执行 | `vkCmdExecuteGeneratedCommandsEXT` | `ExecuteIndirect` | — |
| Execution Set | `VkIndirectExecutionSetEXT`（GPU 侧索引切换） | 无对应物→降级：CPU 侧选 PSO 再录 ExecuteIndirect（诚实降级，不伪造） | — |

- D3D12 降级路径必须显式登记「GPU 侧 shader 索引切换不可表达」，不静默模拟（P-01 纪律；
  依据 1：DX12 无此能力）。
- 命令 token 集取 `ExecuteIndirect` 语义的**跨 API 最小公倍数**（draw/draw_indexed/dispatch +
  少量状态 token），超出子集的 token（如 DX12 专有）首期不可表达（依据 1）。

### D3.2 descriptor buffer 全局表

- 单一大表架构：全场景纹理/缓冲经 `VK_EXT_descriptor_buffer` 进全局表，shader 侧以全局
  descriptor 索引寻址（依据 2；NVIDIA 官方推荐 bindless 大表）。
- **反射/manifest 记录面升级**：reflection v1 字段闭集（RXS-0304）加性扩展「资源→全局
  descriptor 索引」映射记录，取代（并存于）set/binding 对记录（依据 2）。沿 RXS-0180 L2 加性
  演进：v1 字段不删，新增字段带确定性空编码先例（M31 既有纪律）。
- 全局索引的分配律与生命周期（streaming 换入换出时的索引回收/复用）归 D3 与 streaming 模块
  的接缝条款；索引空间预算进 capability profile。
- `VK_EXT_descriptor_heap`：只关注不实现——capability profile v1 ID 闭集加性预留 feature 位
  （如 `bindless.descriptor_heap` 占位 ID），profile JSON schema 相应加性扩展（依据 2；
  RXS-0311 闭集为加性冻结，扩 ID 走 spec 修订行）。

### D3.3 command build compute node 与 render graph 集成

- **新增 AccessKind 依赖边类型**（依据 6）：`StorageWrite`（compute 写 DgcBuffer）→
  `IndirectCommandRead`（ExecuteIndirect 消费）。进入 `graph.rs` `AccessKind` 封闭枚举的加性
  扩展 + 双后端映射表新行（Vulkan：`SHADER_WRITE`→`INDIRECT_COMMAND_READ`；D3D12：
  `UNORDERED_ACCESS`→`INDIRECT_ARGUMENT`），同居单一事实源（RXS-0238 纪律）。
- 图内表达形态：compute pre-pass（既有 compute pass kind）声明 `reads_writes_uav(dgc_buf)`，
  后续 indirect-draw pass 声明新访问类 `reads_indirect(dgc_buf)`。首期 indirect pass 仍受
  RXS-0239 单 queue 全序裁定——DGC 缓冲的「GPU 端生成 → GPU 端消费」是全序内的数据流，
  不引入 pass 内重排语义。
- 零 CPU 回读为**结构性保证**：DgcBuffer 类型不提供 host 读接口（镜像 `AsyncBuffer` 在途态
  无 host 读接口先例，RXS-0144~0148）；调试 dump 走显式 readback pass（`g.readback` 既有面）。
- G3.5 首期不可表达面（render_graph.md §4.0-3：bindless 表/storage image/mesh·RT pass kind
  登记 RD-034+）中，本模块消费「bindless 表 + indirect buffer」两项出不可表达清单，spec
  修订行明确登记；mesh/RT pass kind 是否同步出列归 D3.8 波次裁决。

### D3.4 Execution Set 与 PSO 体系

- Execution Set = 同一 graphics/compute 状态、仅 shader 不同的管线数组，GPU 侧索引切换
  （依据 1）。材质变体是它的自然消费方：同一 pass 状态模板下按 material ID 索引切换 shader。
- 与 M30 PSO cache 衔接：execution set 成员 = PSO cache 条目集合的子集视图，cache key 组成
  （RXS-0306 pipeline key）加性扩展「execution set 成员身份」字段；manifest（RXS-0317）记录
  set 成员枚举，DDC 去重（RXS-0318）按既有键律生效。
- D3D12 无对应物 → CPU 侧 PSO 切换 fallback（D3.1 表）；capability profile 新增 ID
  （建议 `submit.execution_set`）区分两路径，profile 选择律（RXS-0312）裁定 fallback。
- hitching 治理分工（依据 4）：execution set 成员**全部离线物化**进 manifest/DDC；运行时
  JIT 链接（VK_EXT_shader_object / graphics_pipeline_library）只做映射备选面登记，不进承诺。

### D3.5 shader library IR 链接

- **主轴 = 编译期 IR 链接**（依据 4）：在 rurixc 自有 IR（TBIR/MIR 层）做函数级组合链接——
  module 级着色函数以稳定符号导出，链接期按 manifest 声明把「材质函数 × lighting 函数 ×
  pass 入口」组合物化为完整 SPIR-V/DXIL 产物。这是 RuriX 差异化点：自有编译器不需要外挂
  Slang 也能做函数级链接；Slang 生态（Khronos 托管、官方 Rust crate）仅作为语义对标与
  互操作评估对象，不做运行时依赖（依据 4）。
- **分工纪律**（依据 4）：离线组合解决组合爆炸（数量）；运行时链接解决 hitching（延迟）。
  D3 承诺离线侧；运行时链接面（D3.4 备选）登记不承诺。
- 链接合法性：跨 module 函数链接的类型契约 = 既有阶段间接口契约（RXS-0155）+ reflection
  接口事实同一提取律（单一事实源）；链接后 interface hash 重算，manifest 记录链接拓扑
  （哪个 module 的哪个符号进哪个变体），保证审计可回放。
- 与 permutation 的关系：IR 链接发生在 permutation 求解之后——变体 key 确定 → manifest
  查链接拓扑 → 组合物化 → artifact digest 进 DDC。`--permutation-select` 路径（RXS-0310）
  自然承载。

### D3.6 变体预算工具

- 承接 M29（domain digest / budget RX7023）+ M85（manifest merge/dedup）+ M30（PSO cache），
  新增**变体审计工具**（依据 4：UE 案例 1300 万→400 万变体）：
  - 全工程变体枚举报告：按 axis 贡献分解、按 module/pass 归属分解、按 DDC 命中率分解；
  - 预算门：per-entry budget（既有）+ 新增**工程级总预算**（超预算硬失败，工具段 7xxx 新码
    候选）；
  - 死变体检测：manifest 声明但无 workload 引用的变体清单（报告字段，不自动删——删除是
    人的决定）。
- 报告产物 schema 沿 `rurix.permutation-report.v1`（permutation.rs REPORT_SCHEMA_ID）先例
  新建 `rurix.variant-audit-report.v1`。

### D3.7 SER 语言原语

- 处置建议（§⑤）：M52 从 no-go 调整为「语言层支持 + 运行时 capability 可选」（依据 3：
  双 API 标准化完成；VK_EXT_ray_tracing_invocation_reorder 2025-11 + DXR 1.2/SM 6.9 语法强制、
  实现可选）。
- 语言面：hit object 类型（`HitObject` 仅 RT 阶段签名/局部，沿 AccelStruct RXS-0245 先例）+
  内建原语 `reorderThread(hint: u32, bits: u32)` / `hitObjectTraceRay` / `hitObjectInvoke`；
  capability ID 新增（建议 `rt.ser`）进 RXS-0311 闭集加性扩展，`#[requires("rt.ser")]` 与
  profile fallback 机制（RXS-0312）原样生效——无 SER 硬件时编译期选择 fallback 变体。
- 材质 flags 位段预留 coherence hint 编码（依据 3；2~4 bit 位段，编码值域冻结进 spec，
  消费端延后）；RT payload 遵循最小 live state 原则（RXS-0244 契约面加性注释，不改既有条款
  字面）。
- **渲染器集成延后**：SER 原语落地只承诺「语言可表达 + capability 可选 + codegen 双后端
  物化」；材质 hint 消费、coherence 测量、渲染器默认开启均属后续专项（收益目前集中 NVIDIA：
  glTF PT +47%、coherence 23%→54%，跨厂商证据不足——依据 3，故不做默认承诺）。

### D3.8 mesh shader 可选路径

- 处置建议（§⑤）：M61 从 no-go 调整为「DXIL/SPIR-V 后端的可选 geometry pipeline」（依据 5：
  VK_EXT_mesh_shader 跨厂商收敛完成，活跃驱动 GPU 95.95%，Sawicki 2025——RD-039「跨厂商
  收敛」条件按公开证据已实质成立，「measured」条件以本机 4070 Ti + CI 设备 measured 补齐）。
- **顺序硬约束**（依据 5）：mesh shader 入口数据 = cluster 流，故本路径**必须排在 meshlet
  格式（G8.3 M01/M04 已绿）与 GPU-driven 剔除（渲染器主体模块）之后**；D3 内只落「cluster
  流 → mesh shader 入口 → DGC 提交」的通道条款，剔除算法本体不在 D3。
- 传统 VS 光栅为**唯一 fallback**（依据 5）：不做双套全功能并行；mesh 路径缺失时走既有
  VS 路径（capability profile 已有 `mesh.task` ID，RXS-0311 十项之一，选择律原样生效）。
- M62 随 M61 重估（§⑤）：task shader 维持默认不开放——meshlet 构建在 CPU/streaming 侧，
  GPU 端 cluster fan-out 由 DGC 承担，task 的 Amplification 语义在当前架构无消费方；保留
  RXS-0270 字面与 RXS-0243 task 入口契约不动。

## ⑤ 关键设计决策表（含留档项处置裁决建议）

| # | 决策点 | 建议 | 依据 | 备选与理由 |
|---|---|---|---|---|
| D3-Q1 | DGC 抽象层取哪个语义集 | `VK_EXT_device_generated_commands` 语义集为主，token 取跨 API 最小公倍数 | 依据 1（Proton/DXVK 底层共识；ExecuteIndirect 最小公倍数） | DX12 ExecuteIndirect 为基准：丢 Execution Set 能力，否 |
| D3-Q2 | Execution Set 缺后端时 | D3D12 显式降级 CPU 侧 PSO 切换 + capability ID 区分，不伪造 GPU 索引 | 依据 1（DX12 无对应物） | 静默模拟：违 P-01，否 |
| D3-Q3 | descriptor 记录面 | 反射/manifest 加性扩展「资源→全局 descriptor 索引」，set/binding 对并存不删 | 依据 2 | 直接替换：破坏 M31/M85 既有 digest 链，否 |
| D3-Q4 | `VK_EXT_descriptor_heap` | 关注不实现；profile 预留 feature 位 | 依据 2（2026-06 提案早期） | 现在实现：提案未冻结，否 |
| D3-Q5 | **M52 SER 处置** | **no-go → 调整为「语言层原语 + capability 可选」**；渲染器集成延后 | 依据 3（双 API 标准化完成；语法强制/实现可选；收益集中 NVIDIA） | 维持 no-go：标准化已完成，RD-040 触发条件可被 capability 机制精确表达；全量集成：跨厂商收益未证，否 |
| D3-Q6 | **M56 Work Graphs 处置** | **维持 no-go**；render graph 节点 schema 预留「GPU 端 fan-out」表达能力字段（不接线） | 依据 1（D3D12 独占 74.1%，Vulkan 无对应物，首款硬需约 3 年）；RD-041 双条件字面维持 | 立项实现：无 Vulkan 对应物即违 RD-041 字面，否 |
| D3-Q7 | **M59 async compute 处置** | **维持 no-go**；DGC 全在单 queue 全序内表达，不引入多队列 | 依据 6；G8.4 单队列冻结面；RXS-0239 字面不动 | 借 DGC 开多队列：无 measured 收益证据 + 触 Barrier 冻结面，否 |
| D3-Q8 | **M61 mesh shader 处置** | **no-go → 调整为「可选 geometry pipeline」**；顺序在 meshlet 格式与 GPU-driven 剔除之后；VS 为唯一 fallback | 依据 5（VK_EXT_mesh_shader 收敛 95.95%，Sawicki 2025） | 维持 no-go：RD-039 跨厂商条件已实质成立，measured 可本机补齐；双套全功能并行：维护成本翻倍，否 |
| D3-Q9 | **M62 task shader 处置** | **维持不开放**（随 M61 重估后结论）；RXS-0270 字面不动 | 依据 5/6（cluster fan-out 由 DGC 承担，Amplification 无消费方） | 开放 task：无消费方 + 增加第三套调度语义，否 |
| D3-Q10 | shader library 主轴 | 编译期 IR 链接为主轴，API 期链接为后端映射备选 | 依据 4（数量 vs 延迟分工不替代） | Slang 运行时依赖：外挂依赖违自主可控路线，仅作对标 |
| D3-Q11 | 变体治理 | manifest 精确枚举 + DDC 去重 + 审计工具 + 工程级总预算门 | 依据 4（UE 1300 万→400 万案例） | 无总预算只 per-entry：组合爆炸无工程闸，否 |
| D3-Q12 | SER hint 编码 | 材质 flags 位段预留 2~4 bit coherence hint，值域冻结进 spec，消费端延后 | 依据 3 | 现在定义消费语义：渲染器集成延后，空转语义，否 |

## ⑥ 波次建议

> 波次编号对齐 G9 立项后的 G9.x 子段；以下为 D3 内部排序建议，实际并入 G9_PLAN 时可能与
> 渲染器主体模块交错。

| 波次 | 内容 | 出口判据（概要） |
|---|---|---|
| **D3-W1（建议 G9 早期）** | spec/RFC 先行：RXS 条款落笔（§⑨）+ capability snapshot 实测确认 DGC/descriptor buffer 可用性 + D3.2 descriptor buffer 全局表 + D3.3 AccessKind 新边类型 | 条款 PR 先于实现；snapshot 核验 fail-closed 绿；graph.rs 推导 golden 扩展 |
| **D3-W2** | D3.1 DGC 抽象层 Vulkan 原生路 + D3.3 compute pre-pass → ExecuteIndirect 全链路 device 真跑 + D3D12 降级路 | 零 CPU 回读结构性断言 + device golden + D3D12 降级显式登记 |
| **D3-W3** | D3.5 shader library IR 链接 + D3.6 变体预算/审计工具 + D3.4 Execution Set 与 PSO/manifest 衔接 | IR 链接物化产物 digest 稳定 + 审计报告 schema 绿 + execution set fallback 双路绿 |
| **D3-W4** | D3.7 SER 语言原语（capability 可选）+ D3.8 mesh shader 可选路径通道（待渲染器主体 cluster 流就绪后接线） | SER reject/accept 语料 + capability fallback 绿 + mesh 通道最小见证 |

排序理由：W1/W2 是渲染器主体的提交入场券（三级剔除依赖 DGC）；W3 是材质变体规模化的
治理闸；W4 两项均有外部依赖（SER 收益证据 / cluster 流）故殿后。

## ⑦ 验收门草案

> 全部 evidence schema 命名沿 G8 体例（`milestones/g8/g8_*_evidence_schema.json`），G9 立项后
> 落 `milestones/g9/`。RED 臂 = 负例必须红（防假绿）；golden = 逐字节锚定；device 真跑 =
> 目标硬件实测非模拟。

**G-G9-D3-1（descriptor buffer 全局表）**
- 断言：reflection/manifest 中「资源→全局 descriptor 索引」映射与 shader 实际索引**双向精确
  相等**（沿 RXS-0237 声明-反射相等先例）；索引分配确定性（同输入同映射逐字节等值）。
- device 真跑：全局表规模 ≥ 65536 条目场景绑定渲染出图正确；streaming 换入换出后索引回收
  无泄漏（计数器断言）。
- golden：全局索引映射表 JSON 逐字节锚定。
- RED 臂：索引越界/悬空索引 → fail-closed 诊断（不静默回退）；capability 缺失设备 → profile
  fallback 或 RX3020 类拒。
- evidence schema：`g9_d3_descriptor_buffer_table_evidence_schema.json`

**G-G9-D3-2（DGC 抽象层 + compute 全链路）**
- 断言：DgcBuffer 无 host 读接口（类型层结构性保证，镜像 AsyncBuffer 在途态先例）；layout
  声明违反 token 限制（多 dispatch token / 嵌 render pass / 绑 descriptor set）→ 装配期拒。
- device 真跑：compute pre-pass 填充 → ExecuteIndirect 出图与 CPU 录制等价场景**像素级
  golden 一致**；全程零 CPU 回读（回读计数器 = 0 断言）。
- golden：graph.rs 推导计划扩展 golden（新 AccessKind 边类型的 barrier 序列逐条锚）；同图
  双跑逐字节等值。
- RED 臂：漏声明 indirect 读边 → 装配期 strict 拒（RX6029 族扩展）；D3D12 路请求 GPU 侧
  shader 索引切换 → 显式不可表达诊断（不静默降级）。
- evidence schema：`g9_d3_dgc_indirect_submit_evidence_schema.json`

**G-G9-D3-3（shader library IR 链接）**
- 断言：链接产物 interface hash = 组合各 module 接口事实的确定性函数；manifest 链接拓扑记录
  可回放（拓扑 → 产物 digest 重算相等）。
- device 真跑：≥3 module 组合的材质 × lighting 变体出图正确；DDC 命中时产物逐字节复用。
- golden：SPIR-V/DXIL 物化产物 digest golden；链接拓扑 canonical 序列化 golden。
- RED 臂：符号缺失/类型契约失配/接口失配 → 编译期确定性诊断（fail-closed，无最近邻回退，
  沿 RXS-0310 选择律先例）；循环链接 → 拒。
- evidence schema：`g9_d3_shader_library_link_evidence_schema.json`

**G-G9-D3-4（变体预算与审计）**
- 断言：审计报告恒等式（enumerated == pruned + emitted，沿 RXS-0310 先例）工程级成立；
  manifest 声明变体 ∪ DDC 产物闭合（无声明外产物）。
- golden：`rurix.variant-audit-report.v1` 报告逐字节锚定。
- RED 臂：工程级总预算超限 → 硬失败（7xxx 工具段新码候选）；死变体清单注入已知死变体 →
  报告必须列出（漏报即红）。
- evidence schema：`g9_d3_variant_budget_audit_evidence_schema.json`

**G-G9-D3-5（SER 语言原语）**
- 断言：`reorderThread`/hit object 内建的阶段合法性矩阵（非 RT 阶段使用 → 3xxx 拒）；
  `#[requires("rt.ser")]` 隐式推导 + 调用图并集（RXS-0311 机制）正确。
- device 真跑：capability 具备设备上 SER 变体出图与非 SER fallback 变体**像素级一致**（正确性
  等价，不比性能）；capability 缺失 → fallback 选择律绿。
- RED 臂：hit object 逃逸出 RT 阶段/payload 超最小 live state 约束 → 编译期拒；无 SER 硬件
  且无 fallback 映射 → RX3020 类拒。
- evidence schema：`g9_d3_ser_language_primitives_evidence_schema.json`

**G-G9-D3-6（mesh shader 可选路径通道）**
- 断言：mesh 路径与 VS fallback 路径 capability 选择律（RXS-0312）裁定正确；cluster 流输入
  格式契约（与 G8.3 M01/M04 meshlet 页格式 ABI 对接）声明-反射相等。
- device 真跑：cluster 流 → mesh shader → DGC 提交最小见证出图；VS fallback 设备出图等价。
- RED 臂：`mesh.task` capability 缺失且请求 mesh 路径 → fallback 或拒（不静默走第三路）；
  task 入口使用 → 维持 RXS-0270 字面拒（M62 不开放回归门）。
- evidence schema：`g9_d3_mesh_pipeline_optional_evidence_schema.json`

## ⑧ 风险与止损

| 风险 | 等级 | 止损方案 |
|---|---|---|
| `VK_EXT_device_generated_commands` 驱动成熟度参差（EXT 较新，非 NVIDIA 驱动实现质量未证） | 高 | W1 capability snapshot fail-closed 先行；Vulkan 路不可用时 D3D12 ExecuteIndirect 降级路独立成门（D3-Q2）；CI 设备清单双 vendor 覆盖，单 vendor 绿不算绿 |
| descriptor buffer 全局索引生命周期与 streaming 换页竞争 | 中 | 索引分配律进 spec（不单靠实现约定）；换入换出压力测试进 G-G9-D3-1；索引泄漏计数器断言 |
| shader library IR 链接范围失控（跨 module inline/单态化边界模糊） | 高 | W3 限定「函数级符号链接、禁跨 module 泛型单态化」为 v1 边界（写进 RXS 条款）；链接拓扑必须 manifest 声明，禁隐式全图链接 |
| 变体爆炸复现 UE 教训（1300 万变体级） | 中 | 工程级总预算门（D3-Q11）为硬失败非警告；审计工具 W3 与 IR 链接同波交付，不延后 |
| SER 收益不可移植（依据 3：收益集中 NVIDIA） | 中 | 只做语言层 + capability 可选，承诺面不含性能；渲染器默认不开启；跨厂商 measured 证据出现前不进默认路径 |
| mesh shader 路径与渲染器主体 cluster 流时序错配 | 中 | W4 殿后 + 顺序硬约束写进条款（依据 5）；cluster 流未就绪时 mesh 通道只交最小见证不交集成 |
| Work Graphs 预留字段被误读为承诺 | 低 | schema 字段命名带 `reserved_` 前缀 + spec 注释「预留不接线」；M56 no-go 字面进 G9 决策表留档 |
| D3D12 降级路被当成二等公民腐化 | 低 | 降级路独立验收门 + 显式不可表达诊断清单；双后端映射单一事实源纪律（RXS-0238 先例） |

## ⑨ spec / RFC 需求

> 编号区间为**建议值**：现存最高 RXS-0321（rhi.md），G9 D3 建议自 **RXS-0322** 起分配
> （立项时按 spec/README.md §4 登记纪律核实冲突）。档位预判：本模块触 codegen 新面 +
> 绑定/描述符物理布局边界 + RT 语言面扩张，主体 **Full RFC**（建议 RFC-0022 或并入 G9 伞形
> RFC），审计/工具面可 Mini-RFC 分拆。规范先行硬规则：条款 commit 先于实现 commit。

| 建议条款 | 落点文件 | 内容 | 档位建议 |
|---|---|---|---|
| RXS-0322+ | spec/render_graph.md（修订 + 加性条款） | AccessKind 封闭枚举加性扩展：`StorageWrite→IndirectCommandRead` 依赖边类型 + 双后端映射新行；「bindless 表 + indirect buffer」出首期不可表达清单（§4.0-3 修订行）；RXS-0239 单 queue 全序字面不动声明 | Full RFC（触 Barrier EB 三轴冻结面修订边界） |
| RXS-0326+ | spec/rendering_platform.md（加性） | reflection v1 字段闭集加性扩展「资源→全局 descriptor 索引」映射记录（确定性空编码先例）；manifest v1（RXS-0317）加性记录 execution set 成员枚举与 shader library 链接拓扑 | Full RFC（RFC-0019 伞形修订行） |
| RXS-0330+ | 新 spec/gpu_driven_submit.md 候选 | DGC 抽象层语义面：IndirectCmdLayout 声明闭集 / token 限制的装配期核验（恰一 dispatch token 且最后、禁 render pass、禁插 barrier、禁绑 descriptor set）/ DgcBuffer 无 host 读接口类型契约 / Execution Set 与 capability 降级律 / 三后端映射单一事实源 | Full RFC（新 codegen/运行时面） |
| RXS-0336+ | spec/shader_stages.md（加性） | **SER 原语条款**：`HitObject` 类型面（仅 RT 阶段，沿 AccelStruct RXS-0245 先例）/ `reorderThread(hint,bits)` / `hitObjectTraceRay` / `hitObjectInvoke` 内建签名与阶段合法性矩阵 / RXS-0311 capability ID 闭集加 `rt.ser` / 材质 flags coherence hint 位段编码值域冻结 / payload 最小 live state 注释 | Full RFC（RT 语言面扩张） |
| RXS-0340+ | spec/shader_stages.md + spec/vulkan_backend.md / dxil_backend.md（加性） | **mesh shader 可选路径条款**：cluster 流 → mesh 入口数据契约（对接 G8.3 M01/M04 页格式 ABI）/ mesh 路径 ↔ VS fallback 选择律（`mesh.task` capability 既有 ID）/ task 维持不开放（RXS-0270 字面重申 + M62 留档引用）；SPIR-V/DXIL 编码面复用 RXS-0246 基建加性扩展 | Full RFC |
| RXS-0344+ | spec/rendering_platform.md 或新 spec/shader_library.md | **shader library IR 链接条款**：module 符号导出/链接拓扑 manifest 声明律 / 组合物化与 interface hash 重算 / v1 边界（函数级符号链接、禁跨 module 泛型单态化）/ 链接诊断 fail-closed | Full RFC |
| RXS-0348+ | spec/toolchain.md（加性）或 Mini-RFC | 变体审计工具：`rurix.variant-audit-report.v1` schema / 工程级总预算门（7xxx 新码候选）/ 死变体检测报告律 | Mini-RFC 可分拆 |
| （不加条款） | spec/README.md §4 登记 | capability profile ID 闭集加性：`submit.execution_set` / `rt.ser` / `bindless.descriptor_heap`（预留位）/ profile JSON schema 加性扩展；`VK_EXT_descriptor_heap` 关注登记（不实现，留 RD 编号候选） | 随上述各 RFC 行 |

**需向上取严确认的点**（判档争议向上取严纪律）：① RXS-0239 单 queue 全序承诺下表达 DGC
是否需要 🔒 禁区（RFC-0013 §4.D4 同级）修订行——建议按「全序内数据流、不扩承诺面」处理，
但立项时需 owner/RFC 裁决确认；② descriptor 全局索引的物理布局（heap 偏移编码）触碰
binding_layout.md 🔒「descriptor heap 编码不冻结为 stable 语言保证」边界——条款只作存在性/
确定性声明，不冻结数值布局。

## 附：引用锚点汇总

- G8 承接锚：`milestones/g8/G8_P2_DECISIONS.md` M33/M52/M55/M56/M59/M61/M62 行（v1.0，
  2026-08-06）；`milestones/g8/G8_CAPABILITY_MATRIX.md` M55/M61/M33 行。
- 现有基础：`spec/render_graph.md` RXS-0236~0241；`spec/binding_layout.md` RXS-0163~0166/0233；
  `spec/shader_stages.md` RXS-0231~0232/0242~0245/0311；`spec/rendering_platform.md`
  RXS-0304~0318；`spec/rhi.md` RXS-0270~0276/0280~0283；`src/rurixc/src/permutation.rs` /
  `reflection.rs` / `capability_check.rs` / `manifest.rs`；`src/rurix-render/src/graph/`。
- 调研依据 1~6：见 §①（VK_EXT_device_generated_commands 2024 / Vulkanised 2025 / Sawicki 2025 /
  VK_EXT_descriptor_buffer NVIDIA 推荐 / VK_EXT_descriptor_heap Sascha Willems 2026-06 /
  VK_EXT_ray_tracing_invocation_reorder 2025-11 + DXR 1.2 / glTF PT +47% / Slang Khronos /
  UE 1300 万→400 万变体 / VK_EXT_mesh_shader 95.95%）。
