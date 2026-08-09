# RFC-0023 — GPU-driven 提交与着色系统（G9 伞形三章之二）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0023（4 位制，编号永不复用，10 §9.5；编号按 2026-08-09 实测 `registry/number_ledger.json` namespaces.RFC `next_free=22` 起领取，本号由 G9.1 治理波实测领取，非推测） |
| 标题 | GPU-driven 提交与着色系统（G9 伞形三章之二；之一 = RFC-0022 虚拟几何与 GI 语义，之三 = RFC-0024 物理平台修订） |
| 档位 | **Full RFC**（新增 DGC 命令生成运行时语义与类型面、AccessKind 封闭枚举加性扩展并触 G5 Barrier EB 三轴冻结面显式修订行、reflection/manifest 记录面扩张、RT 语言面扩张（SER/HitObject）、mesh shader 可选 geometry pipeline；触及运行时语义、codegen、绑定/描述符物理布局相邻边界及 G5 冻结面，10 §3 / AGENTS 硬规则 5） |
| 状态 | **Agent Approved（2026-08-09）**。§9.1 独立 provenance 对抗性评审第 1 轮完成，4 findings（2 major + 2 minor）全部采纳并修；G9.1 governance-only 交付物，批准只表示语义评审通过，不解锁任何 `src/`、`spec/`、`conformance/` 实现 |
| 承接里程碑 | G9.1 governance-only RFC 交付；实施候选分别归 G9.2（M102/M103/M104，P0）、G9.3（M105/M106/M107，P1）、M108/M109（strategic_override 已定案，裁决 go 后排波，M109 顺序硬约束见 §4.8） |
| 关联条款 | G9.1 **不领取 RXS 数字号**。拟修订/扩展 `spec/render_graph.md`、`spec/rendering_platform.md`、`spec/shader_stages.md`、`spec/vulkan_backend.md` / `dxil_backend.md`，并候选新建 `spec/gpu_driven_submit.md`；actual RXS 从实现期 `number_ledger` 的 `next_free` 逐条领取（§5 拟条款表不预写号） |
| 依据决策 | D-404/D-406 v2.0/D-409 · P-01/P-09/P-11/P-13 · G9.1 立项裁决六项（2026-08-09，含 M52→M108 / M61→M109 strategic_override 定案）· G9.0 不可变 ref `1d9460a1` · [G9_CAPABILITY_MATRIX](../milestones/g9/G9_CAPABILITY_MATRIX.md) §3（M102~M109 行）· [G9_D3 设计草案](../milestones/g9/design/G9_D3_GPU_DRIVEN_SUBMISSION.md) · [R6 调研](../milestones/g9/research/R6_GPU_DRIVEN_SUBMISSION.md) · [RFC-0019](0019-rendering-platform.md)（M29/M30/M31/M32/M85 治理面与 G5 EB 边界先例） |
| Provenance | `Assisted-by: Kimi Code CLI (Kimi) rfc0023-drafter` |
| Agent 批准 | **Agent Approved 2026-08-09**；agent 依 10 §7 / P-13 / D-406 v2.0 完全自主签署；批准范围含 §4.4.3 🔒 禁区修订行（RXS-0236/RXS-0241 字面已随 F-1 补申报）；批准只表示语义评审完成，**不构成实现许可**（实现由 G9.2+ 各实现波 spec-first 与 capability snapshot 硬门决定） |
| 对抗性评审 | **完成**（D-409 第 1 轮，2026-08-09）：评审 provenance `Assisted-by: Kimi Code CLI (Kimi) rfc0023-adversarial-reviewer`（独立实例）≠ 起草 provenance `Assisted-by: Kimi Code CLI (Kimi) rfc0023-drafter`；结论「有条件通过」，4 findings（2 major + 2 minor）全部 disposition = 采纳并修；同工具族独立实例评审偏差如实登记，详见 §9.1 |

---

## 1. 摘要

本 RFC 是 G9 伞形三章之二，定义 G9 模块 D3「GPU-driven 提交与着色器系统深化」的八个相互依赖语义面：

1. M102：DGC 跨 API 最小公倍数抽象——`IndirectCmdLayout` / `DgcBuffer` 类型面、token 闭集语义、限制装配期 fail-closed、DgcBuffer 无 host 读接口的结构性保证；
2. M105：Indirect Execution Set 与 PSO/manifest 衔接——GPU 侧索引切换；D3D12 无对应物时诚实降级 CPU 侧 PSO 切换，capability ID 区分两条路径，禁静默模拟；
3. M103：descriptor buffer 全局表——「资源→全局 descriptor 索引」进 reflection/manifest、与 set/binding 加性并存、索引分配律与回收语义；`VK_EXT_descriptor_heap` 只预留 feature 位不实现；
4. M104：command build compute node 与 render graph 集成——零 CPU 回读结构性保证 + 回读计数器，以及自动 barrier 新依赖边 `StorageWrite→IndirectCommandRead`（对 G5 Barrier EB 三轴冻结面的**唯一**显式修订行，🔒 标注）；
5. M106：shader library IR 函数级组合链接——v1 边界 = 函数级符号链接、禁跨 module 泛型单态化、interface hash 重算进 manifest；
6. M107：变体预算与审计工具语义——工程级总预算门硬失败、死变体检测报告；
7. M108：SER 语言原语——`HitObject` 类型面、`reorderThread` / `hitObjectTraceRay` / `hitObjectInvoke`、capability `rt.ser` 可选、材质 flags coherence hint 位段预留；渲染器集成延后（M52 strategic_override 定案）；
8. M109：mesh shader 可选 geometry pipeline——cluster 流入口、VS 光栅唯一 fallback、`mesh.task` capability 选择律；顺序硬约束 = 排在 meshlet 页格式 v2 与 GPU-driven 剔除之后（M61 strategic_override 定案）。

这些面共享 G8 已冻结的总原则：**编译器产出的 reflection/capability/permutation/pipeline manifest 是运行时装配的单一事实源**；DGC 的产物只是把「谁录制命令」从 CPU 换到 GPU，命令的**合法性核验仍在编译期/装配期完成**，运行时逐字执行，不二次猜测。

```text
.rx source
  ├─ module 符号导出 ── IR 函数级链接 ── 组合物化 SPIR-V/DXIL ── interface hash 重算 ── manifest/DDC
  ├─ reflection v1 加性扩展「资源→全局 descriptor 索引」── 全局表分配律
  └─ compute pre-pass（GPU 写 DgcBuffer）
        │  StorageWrite→IndirectCommandRead（新 AccessKind 边，单 queue 全序内）
        ▼
     ExecuteIndirect / vkCmdExecuteGeneratedCommandsEXT（零 CPU 回读）
        └─ Execution Set：GPU 侧索引切换 shader（D3D12：诚实降级 CPU 侧 PSO 切换）
```

本 RFC 是 **G9.1 governance-only** 交付物，零 `src/`、`spec/`、`conformance/` 改动，零 RXS/RD/U/RX 数字 claim，不创建空 schema 壳/空脚本占位。

## 2. 动机、范围与治理门

### 2.1 为什么需要 Full RFC

G8 已绿单源 gfx submit（M89/RD-037）、render graph 自动 barrier（RXS-0236~0241）、bindless（RXS-0231~0235）、permutation/PSO cache/reflection/capability profile（M29~M32）与 manifest↔DDC（M85）。但 G9 的三级剔除（instance→cluster→triangle）与 cluster 流光栅要求 compute pre-pass 在 GPU 上产出命令并被 indirect 执行消费，全程零 CPU 回读——G8 的命令录制面全部是 CPU 侧，且 render graph 的 AccessKind 封闭枚举没有「storage-write → indirect-command-read」这条依赖边。同时 M33（shader library 组合链接）与 M55（descriptor buffer / DGC）两条 defer-to-G9+ 承接锚、M52/M61 两条 strategic_override 定案都需要语义落笔。这些变更触及：

- 新增命令生成运行时语义与类型面（DgcBuffer 无 host 读接口）；
- AccessKind 封闭枚举加性扩展 → 触 G5 Barrier EB 三轴冻结面，必须显式修订行（§4.4.3）；
- reflection v1 字段闭集（RXS-0304）与 manifest v1（RXS-0317）的加性扩张；
- RT 语言面扩张（HitObject / SER 内建）与 capability ID 闭集（RXS-0311）加性扩展；
- mesh/task stage 消费面从「最小见证」扩为「可选 geometry pipeline」。

均非 Direct/Mini 可安全承载。

### 2.2 治理门：governance-only 与 strategic_override 定案

| 门 | 允许动作 | 禁止动作 |
|---|---|---|
| G9.1 governance-only | 起草/评审/批准本 RFC；引用 P0 key 命名空间（15 行，三方逐字一致，冻结） | 不改 `src/`、`spec/`、`conformance/`、`.github/workflows/`；不 materialize 数字 CI 步骤；不领取 RXS/RD/U/RX 数字号；不创建空 schema 壳/空脚本占位 |
| G9.2+ 实现波 | 按本 RFC §5 拟条款表 spec-first 落条款与 RED 语料，再落实现 | RFC Draft/Approved 状态本身不构成实现许可；capability snapshot 实测确认（fail-closed）为阻塞性前置 |

G9.1 立项裁决（2026-08-09，六项定案之一）对两条 G8 no-go 留档项的改判**已定案，本文只承接不重裁**：

- **M52 SER → M108**：no-go 改判接受，记 strategic_override「语言层原语 + capability 可选」；`registry/deferred.json` RD-040 history 只追加 override 条目，禁静默改判；
- **M61 mesh shader → M109**：no-go 改判接受，记 strategic_override「可选 geometry pipeline」，顺序硬约束 = 排在 meshlet 页格式 v2（M91）与 GPU-driven 剔除链路之后；deferred.json RD-039 history 只追加 override 条目，禁静默改判。

本 RFC 不登记这两条 override 本身（登记由 G9.1 治理波统一落 deferred.json history），只把其语义面冻结为条款依据。

### 2.3 in-scope

| 面 | 本 RFC 冻结内容 | 实施波次 | canonical P0 gate key（命名空间已冻结，本文不新增） |
|---|---|---|---|
| M102 | DGC 抽象层：IndirectCmdLayout/DgcBuffer 类型面、token 闭集、装配期 fail-closed、三后端映射 | G9.2 | `g9.p0.m102.dgc_abstraction`（脚本 `ci/g9_dgc_abstraction_smoke.py --gate g9.p0.m102.dgc_abstraction`；schema 目标 `milestones/g9/g9_m102_dgc_abstraction_evidence_schema.json`） |
| M103 | descriptor buffer 全局表：reflection/manifest 索引记录面、分配律/回收、heap 预留位 | G9.2 | `g9.p0.m103.descriptor_global_table`（同体例） |
| M104 | command build compute node：AccessKind 新边、零 CPU 回读结构性保证、🔒 EB 修订行 | G9.2 | `g9.p0.m104.accesskind_indirect_edge`（同体例） |
| M105 | Execution Set 与 PSO/manifest 衔接、D3D12 诚实降级 | G9.3（P1） | 无 P0 key；gate 归届时验收映射，本文不预造 |
| M106 | shader library IR 函数级组合链接 | G9.3（P1） | 无 P0 key；同上 |
| M107 | 变体预算与审计工具 | G9.3（P1，与 M106 同波不延后） | 无 P0 key；同上 |
| M108 | SER 语言原语 + capability 可选 | 裁决 go 后排波（strategic_override 已定案） | 无 P0 key；同上 |
| M109 | mesh shader 可选 geometry pipeline | 裁决 go 且 cluster 流就绪后（顺序硬约束） | 无 P0 key；同上 |

注：上表 P0 key 与 evidence schema 路径来自 G9.1 已冻结的 P0 key 命名空间（15 行）；本文只引用，只冻结路径，不预建文件。

## 3. 指导级解释（用户视角）

### 3.1 GPU 端生成命令的一次提交

用户在 `.rx` 图中写一个既有形态的 compute pass（command build node），声明 `reads_writes_uav(dgc_buf)`；后续 indirect-draw pass 声明 `reads_indirect(dgc_buf)`。compute pre-pass 在 GPU 上做剔除并把 draw 命令写进 `DgcBuffer`；indirect pass 就地消费，全程无 CPU 回读。`DgcBuffer` 类型**没有 host 读接口**——想看内容必须显式加 readback pass，这让「零 CPU 回读」成为类型层结构性保证，而不是纪律口号。漏写 `reads_indirect` 依赖边时，装配期 strict 拒，不存在「碰巧能跑」。

### 3.2 用 Execution Set 切换材质 shader

同一 pass 状态模板下，材质变体是 Execution Set 的成员：GPU 侧按索引切换 shader，CPU 不重新绑管线。manifest 记录 set 成员枚举，全部离线物化进 DDC。在 D3D12 后端该能力不存在：编译/装配产物显式登记「GPU 侧 shader 索引切换不可表达」，走 CPU 侧选 PSO 再录 ExecuteIndirect 的降级路径——两条路径由 capability ID（拟 `submit.execution_set`）区分，profile 选择律裁定，**绝不静默模拟**。

### 3.3 shader library：函数级组合链接

用户把材质函数、lighting 函数写成独立 module 的稳定符号；链接期按 manifest 声明的拓扑把「材质函数 × lighting 函数 × pass 入口」组合物化成完整 SPIR-V/DXIL。v1 只做函数级符号链接：**不允许跨 module 泛型单态化**；链接后 interface hash 重算并写回 manifest，拓扑可回放（拓扑 → 产物 digest 重算相等）。

### 3.4 SER 与 mesh shader 是可选能力，不是默认承诺

SER 三内建（`reorderThread` / `hitObjectTraceRay` / `hitObjectInvoke`）与 `HitObject` 类型在语言层可用，但只有在 profile 提供 `rt.ser` capability 时才编译通过；无 SER 硬件时按 RXS-0312 选择律走 fallback 变体。mesh shader 路径同理：`mesh.task` capability 缺失时走传统 VS 光栅——VS 是**唯一** fallback，不存在第三条静默路径。

## 4. 参考级设计

### 4.0 跨面不变量

1. **P-11 单源**：compiler manifest/reflection 是 token 闭集、Execution Set 成员、全局 descriptor 索引、链接拓扑、capability 的唯一事实源；runtime 不从后端 blob、源码名或 host struct 再推导第二份语义。
2. **strict-only / fail-closed**：token 限制违反、索引越界/悬空、capability 缺失、链接符号缺失/接口失配、漏声明 indirect 读边，均在编译期或装配期确定性拒绝；不得静默继续、不得最近邻回退。
3. **deterministic**：相同输入得到逐字节相同的 canonical reflection、interface hash、索引映射与链接拓扑。
4. **诚实降级（P-01）**：后端无对应能力时显式登记不可表达并走 capability 裁定的 fallback；禁止静默模拟。
5. **非 stable 物理布局**：DGC buffer 物理字节格式（`vkCmdPreprocessGeneratedCommandsEXT` 产物）、descriptor heap 偏移编码、Execution Set 句柄均为实现确定、非 stable；条款只作存在性/确定性声明，不冻结数值布局（沿 `spec/binding_layout.md` 🔒「descriptor heap 编码不冻结为 stable 语言保证」边界）。

### 4.1 M102 — DGC 跨 API 最小公倍数抽象

#### 4.1.1 类型面

- `IndirectCmdLayout`：声明式模板，描述一个命令 sequence 的 token 序列（vertex/index buffer 绑定、push constant、draw/dispatch 等）。
- `DgcBuffer`：GPU 可写的命令数据 buffer。**类型不提供 host 读接口**（镜像 G8 `AsyncBuffer` 在途态无 host 读接口先例，RXS-0144~0148）——这是零 CPU 回读的结构性保证；调试 dump 走显式 readback pass（`g.readback` 既有面）。
- `ExecutionSet`：同状态仅换 shader 的管线数组（§4.2）。

#### 4.1.2 token 闭集与限制内化

命令 token 集取 `ExecuteIndirect` 语义的**跨 API 最小公倍数**：draw / draw_indexed / dispatch + 少量状态 token；超出子集的 token（如 D3D12 专有）首期不可表达。

DGC 的 API 限制**不进运行时检查，内化为 layout 声明的编译期/装配期核验，fail-closed**（沿 RXS-0237 装配核验先例）：

- 每 sequence 恰一个 dispatch/draw 终止 token 且必须位于最后；
- sequence 内不可开 render pass；
- 不可插 barrier；
- 不可绑 descriptor set。

#### 4.1.3 三后端映射表（单一事实源，镜像 RXS-0238「双后端映射同源」纪律）

| 抽象 | Vulkan | D3D12 | NVPTX |
|---|---|---|---|
| IndirectCmdLayout | `VkIndirectCommandsLayoutEXT` | command signature（`ID3D12CommandSignature`） | 不承诺（仅命令数据生成） |
| DgcBuffer 填充 | GPU compute 直写 + `vkCmdPreprocessGeneratedCommandsEXT` | GPU compute 直写 argument buffer | compute kernel 产出 buffer 数据（既有 launch 语义） |
| 执行 | `vkCmdExecuteGeneratedCommandsEXT` | `ExecuteIndirect` | — |
| Execution Set | `VkIndirectExecutionSetEXT`（GPU 侧索引切换） | 无对应物 → 诚实降级（§4.2.2） | — |

#### 4.1.4 阻塞性前置

`VK_EXT_device_generated_commands` 在目标硬件（4070 Ti 起步 + CI 设备清单）的 capability snapshot 实测确认为阻塞性前置，走 M32 snapshot 核验原语（RXS-0313）既有机制，fail-closed；EXT 较新、非 NVIDIA 驱动实现质量未证，单 vendor 绿不算绿。

### 4.2 M105 — Execution Set 与 PSO/manifest 衔接

#### 4.2.1 语义与衔接

Execution Set = 同一 graphics/compute 状态、仅 shader 不同的管线数组，GPU 侧索引切换。材质变体是自然消费方：同一 pass 状态模板下按 material ID 索引切换 shader。

- 与 M30 PSO cache 衔接：execution set 成员 = PSO cache 条目集合的子集视图；cache key（RXS-0306 pipeline key）加性扩展「execution set 成员身份」字段；
- manifest（RXS-0317）记录 set 成员枚举；DDC 去重（RXS-0318）按既有键律生效；
- hitching 治理分工：execution set 成员**全部离线物化**进 manifest/DDC；运行时 JIT 链接（`VK_EXT_shader_object` / `VK_EXT_graphics_pipeline_library`）只做映射备选面登记，不进承诺。

#### 4.2.2 D3D12 诚实降级

D3D12 无 Execution Set 对应能力 → 降级为 CPU 侧选 PSO 再录 `ExecuteIndirect`：

- 降级路径必须显式登记「GPU 侧 shader 索引切换不可表达」，不静默模拟（P-01）；
- capability profile 新增 ID（拟 `submit.execution_set`，闭集加性扩展走 spec 修订行）区分两路径，profile 选择律（RXS-0312）裁定 fallback；
- 请求 GPU 侧索引切换而 profile 不含该 capability → 显式不可表达诊断，不静默降级为模拟。

### 4.3 M103 — descriptor buffer 全局表

- 单一大表架构：全场景纹理/缓冲经 `VK_EXT_descriptor_buffer` 进全局表，shader 侧以全局 descriptor 索引寻址（NVIDIA 官方推荐 bindless 大表路径）。
- **reflection/manifest 记录面升级**：reflection v1 字段闭集（RXS-0304）加性扩展「资源→全局 descriptor 索引」映射记录，**与 set/binding 对并存不删**（保 M31/M85 digest 链）。沿 RXS-0180 L2 加性演进先例：v1 字段不删，新增字段按下列 0-drift 机制加性。
- **0-drift 序列化机制（评审 F-2 补齐）**：「资源→全局 descriptor 索引」为 canonical 序列化的**尾随可选字段**——缺省时序列化字节 ≡ 字段不存在，既有产物字节流 **0-byte**；不得以「空编码为 count 0」冒充 0-byte（count 0 仍会改变既有字节）。该机制列为 **RXS-0305 CanonW 律的加性修订点**，随 §5 rendering_platform.md 条款行落地，验收面强制「既有 reflection golden 0-byte 恒跑」。
- **索引分配律与回收语义**：全局索引的分配、streaming 换入换出时的回收/复用进 spec（不单靠实现约定）；同输入同映射逐字节等值；索引越界/悬空索引 → fail-closed 诊断，不静默回退；索引泄漏以计数器断言。索引空间预算进 capability profile。
- `VK_EXT_descriptor_heap`：**只预留 feature 位不实现**——capability profile v1 ID 闭集加性预留占位 ID（拟 `bindless.descriptor_heap`），profile JSON schema 相应加性扩展；跟踪其规范化进展，现在实现等于绑定未冻结面。
- descriptor 全局索引的物理布局（heap 偏移编码）不冻结为 stable 语言保证（§4.0-5）。

### 4.4 M104 — command build compute node 与 render graph 集成

#### 4.4.1 图内表达形态

- compute pre-pass（既有 compute pass kind）声明 `reads_writes_uav(dgc_buf)`；
- 后续 indirect-draw pass 声明新访问类 `reads_indirect(dgc_buf)`；
- DGC 缓冲的「GPU 端生成 → GPU 端消费」是 **RXS-0239 单 queue 全序内的数据流**，不引入 pass 内重排语义；首期 indirect pass 仍受单 queue 全序裁定。

#### 4.4.2 零 CPU 回读结构性保证

- DgcBuffer 无 host 读接口（§4.1.1），compute pre-pass → ExecuteIndirect 全程不经 CPU；
- 验收面配「回读计数器 = 0」断言：任何隐式回读（如调试路径）必须经计数器显式记账，计数器非零即红。

#### 4.4.3 🔒 G5 Barrier EB 三轴冻结面显式修订行

自动 barrier 系统需新增依赖边类型 `StorageWrite→IndirectCommandRead`。这是对 G5 冻结面（`Barrier { sync_before/after, access_before/after, layout_before/after }` 三轴结构及其推导规则，RFC-0016/RFC-0019 §4.8.4 口径）的**唯一修订**，按 14 §1 / 07 §5 纪律逐条给出：

| 修订点 | 原冻结面（逐字口径） | 修订后 | 零漂移证明计划 |
|---|---|---|---|
| 🔒 AccessKind 封闭枚举 | G5/G8 既有 AccessKind 封闭枚举无 indirect-command-read 访问类 | AccessKind 封闭枚举**加性**扩展一个访问类 `IndirectCommandRead`；新依赖边类型 `StorageWrite→IndirectCommandRead` 进入 `graph.rs` 推导与 spec/render_graph.md | 既有 barrier 推导 golden 全部 0-byte 恒跑；新边类型的 barrier 序列作为新增 golden 只加不改 |
| 双后端映射新行 | RXS-0238 双后端映射单一事实源表无对应行 | 新增行：Vulkan `SHADER_WRITE`→`INDIRECT_COMMAND_READ`；D3D12 `UNORDERED_ACCESS`→`INDIRECT_ARGUMENT`，同居单一事实源 | 同上映射表 golden 扩展 |
| EB 三轴结构 | `Barrier { sync_before/after, access_before/after, layout_before/after }` 字段、枚举含义、推导规则 | **0-byte 不动**：新访问类只是 access 轴取值域的加性扩展，不是第四轴，也不是新 barrier 结构 | 三轴结构相关既有 golden 0-byte |
| RXS-0239 单 queue 全序 | 「单 queue；声明序=提交序=pass 粒度完成序」 | **字面 0-byte 不动**：DGC 数据流是全序内的数据流，不扩承诺面、不引入多队列与 pass 内重排 | RXS-0239 既有判据 0-byte 恒跑 |
| RXS-0236 访问声明集封闭枚举（评审 F-1 补申报） | `spec/render_graph.md:62-66`：访问声明集封闭枚举（`writes_rt → ColorAttachmentWrite` / `writes_depth → DepthAttachmentWrite` / `reads → ShaderRead` / `reads_writes_uav → UavReadWrite` / `readback → CopySrcReadback + CopyDstReadback` / present 终端胶水 `→ PresentHandoff`），本面「不支持即不可表达」 | **加性扩展** `reads_indirect → IndirectCommandRead`；既有五类 + present 字面 **0-byte** | 既有访问声明集判据 0-byte 恒跑；`reads_indirect` 合法化与非法访问类拒绝作为新增判据只加不改 |
| 🔒 RXS-0241 cabi tag 域（评审 F-1 补申报） | `spec/render_graph.md:220`：cabi `rxrt_graph_declare(pass, resource, access: u32)`，`access = AccessKind u32 tag（0..=6）` | **字面 0-byte**：首期 cabi **不暴露**新访问类，该限制在此显式登记（备选 = 扩 tag 域 0..=7；本 RFC 取前者，cabi 面扩展另案裁决） | cabi tag 域既有判据 0-byte 恒跑；「cabi 侧声明 indirect 读访问类 → 不可表达诊断」新增 RED 语料 |

配套 strict 判据：**漏声明 indirect 读边（indirect pass 消费 DgcBuffer 但未声明 `reads_indirect`）→ 装配期 strict 拒**（沿 RX6029 族装配诊断先例，实号实现期领取）。

G3.5 首期不可表达清单（`spec/render_graph.md` §4.0-3：bindless 表 / storage image / mesh·RT pass kind 登记 RD-034+）中，本模块消费「bindless 表 + indirect buffer」两项出列，spec 修订行明确登记；mesh/RT pass kind 是否同步出列归后续波次裁决，本文不预裁。

若实现期发现上表任一行无法在不改既有 golden 的前提下落地，必须停止实现并先修订本 RFC。

### 4.5 M106 — shader library IR 函数级组合链接

- **主轴 = 编译期 IR 链接**：在 rurixc 自有 IR（TBIR/MIR 层）做函数级组合链接——module 级着色函数以稳定符号导出，链接期按 manifest 声明把「材质函数 × lighting 函数 × pass 入口」组合物化为完整 SPIR-V/DXIL 产物。Slang 生态（Khronos 托管）仅作语义对标与互操作评估对象，不做运行时依赖。
- **分工纪律**：离线组合解决组合爆炸（数量）；运行时链接解决 hitching（延迟）。本 RFC 承诺离线侧；运行时链接面（§4.2.1 备选）登记不承诺。
- **v1 边界（写进条款）**：函数级符号链接；**禁跨 module 泛型单态化**；链接拓扑必须 manifest 显式声明，禁隐式全图链接。
- **链接合法性**：跨 module 函数链接的类型契约 = 既有阶段间接口契约（RXS-0155）+ reflection 接口事实同一提取律（单一事实源）；链接后 **interface hash 重算并写回 manifest**，manifest 记录链接拓扑（哪个 module 的哪个符号进哪个变体），保证审计可回放（拓扑 → 产物 digest 重算相等）。
- **与 permutation 的关系**：IR 链接发生在 permutation 求解之后——变体 key 确定 → manifest 查链接拓扑 → 组合物化 → artifact digest 进 DDC；`--permutation-select` 路径（RXS-0310）自然承载。
- **fail-closed**：符号缺失 / 类型契约失配 / 接口失配 → 编译期确定性诊断（无最近邻回退，沿 RXS-0310 选择律先例）；循环链接 → 拒。

### 4.6 M107 — 变体预算与审计工具

- 承接 M29（domain digest / 预算）+ M30（PSO cache）+ M85（manifest merge/dedup），新增**变体审计工具**：
  - 全工程变体枚举报告：按 axis 贡献分解、按 module/pass 归属分解、按 DDC 命中率分解；
  - 预算门：per-entry budget（既有）+ 新增**工程级总预算**——超预算**硬失败**（非警告；诊断码实现期从工具段按实际可达类别领取，不预造）；
  - 死变体检测：manifest 声明但无 workload 引用的变体清单（报告字段，不自动删——删除是人的决定）。
- 报告产物 schema 沿 `rurix.permutation-report.v1` 先例新建 `rurix.variant-audit-report.v1`。
- 审计恒等式：`enumerated == pruned + emitted`（沿 RXS-0310 先例）工程级成立；manifest 声明变体 ∪ DDC 产物闭合（无声明外产物）。
- 交付纪律：审计工具与 M106 IR 链接**同波交付，不延后**（变体爆炸治理闸不能晚于产能工具）。

### 4.7 M108 — SER 语言原语（M52 strategic_override 承接）

- **改判事实**：M52 由 G8 no-go 留档改判接受，记 strategic_override「语言层支持 + capability 可选」（G9.1 立项裁决定案；deferred.json RD-040 history 只追加 override，禁静默改判；登记由治理波统一落，本文不登记）。依据：双 API 标准化完成（`VK_EXT_ray_tracing_invocation_reorder` 2025-11 + DXR 1.2/SM 6.9，语法强制、实现可选）。
- **语言面**：
  - `HitObject` 类型：仅 RT 阶段签名/局部变量合法（沿 AccelStruct RXS-0245 先例）；逃逸出 RT 阶段 → 编译期拒；
  - 内建原语：`reorderThread(hint: u32, bits: u32)` / `hitObjectTraceRay` / `hitObjectInvoke`；阶段合法性矩阵冻结进条款（非 RT 阶段使用 → 编译期拒）；
  - capability ID 新增（拟 `rt.ser`）进 RXS-0311 闭集加性扩展；`#[requires("rt.ser")]` 与 profile fallback 机制（RXS-0312）原样生效——无 SER 硬件时编译期选择 fallback 变体；无 SER 硬件且无 fallback 映射 → 拒。
- **材质 flags coherence hint 位段预留**：材质 flags 预留 2~4 bit coherence hint 编码位段，编码值域冻结进 spec，**消费端延后**；RT payload 遵循最小 live state 原则（RXS-0244 契约面加性注释，不改既有条款字面）。
- **不承诺性能、渲染器集成延后**：SER 原语落地只承诺「语言可表达 + capability 可选 + codegen 双后端物化」；材质 hint 消费、coherence 测量、渲染器默认开启均属后续专项。理由：收益证据目前集中 NVIDIA（glTF path tracer +47%、warp coherence 23.0%→54.2%；R6 §四，Khronos 官方案例 [S16][S19]），跨厂商证据不足，不做默认承诺。验收口径为**正确性等价**（SER 变体与 fallback 变体像素级一致），不比性能。

### 4.8 M109 — mesh shader 可选 geometry pipeline（M61 strategic_override 承接）

- **改判事实**：M61 由 G8 no-go 留档改判接受，记 strategic_override「可选 geometry pipeline」（G9.1 立项裁决定案；deferred.json RD-039 history 只追加 override，禁静默改判；登记由治理波统一落，本文不登记）。依据：`VK_EXT_mesh_shader` 跨厂商收敛按公开证据实质成立（活跃驱动 GPU 95.95%，Sawicki 站点 2025 年末统计），RD-039「measured」条件可在本机 4070 Ti + CI 设备 measured 补齐（实现期前置，当前无 measured artifact）。
- **顺序硬约束**：mesh shader 入口数据 = cluster 流，故本路径**必须排在 meshlet 页格式 v2（M91）与 GPU-driven 剔除链路（渲染器主体模块）之后**。本模块只落「cluster 流 → mesh shader 入口 → DGC 提交」的通道条款；剔除算法本体不在本 RFC。
- **VS 光栅为唯一 fallback**：不做双套全功能并行；mesh 路径缺失时走既有 VS 路径。capability profile 已有 `mesh.task` ID（RXS-0311 十项之一），选择律（RXS-0312）原样生效：`mesh.task` 缺失且请求 mesh 路径 → fallback 或拒，不静默走第三路。
- **cluster 流输入格式契约**与 G8.3 M01/M04 meshlet 页格式 ABI 对接，声明-反射相等（沿 RXS-0237 先例）。
- **task shader 维持不开放**：M62 随 M61 重估后结论不变——cluster fan-out 由 DGC 承担，task 的 Amplification 语义在当前架构无消费方；RXS-0270「task 前置条件臂首期不开放」字面与 RXS-0243 task 入口契约不动，task 入口使用维持既有字面拒（M62 不开放回归门）。

## 5. 下游 spec 条款映射（拟条款表，10 §3 要件）

G9.1 **不领取 RXS 数字号**。下表条款号一律记 `RXS-####`（拟）；actual RXS 在实现波 spec PR 时读取 `number_ledger` actual `next_free` 逐条领取，每条 materialize 时至少一个 `//@ spec: RXS-实际号` 锚点，trace_matrix 全锚定。**spec 条款 PR 先于实现 PR**（硬规则 7）。**禁止沿用 design/G9_D3 草案 §⑨ 的建议编号区间**（其为未批准建议值）。

| 条款（拟） | 目标 spec | 规范 diff | 测试锚定计划（每条 ≥1） |
|---|---|---|---|
| RXS-#### | `spec/render_graph.md`（修订 + 加性） | 🔒 §4.4.3 修订行落地：AccessKind 加性扩展 `IndirectCommandRead` + 新边类型 + 双后端映射新行；「bindless 表 + indirect buffer」出首期不可表达清单（§4.0-3 修订行）；RXS-0239 单 queue 全序字面不动声明 | 新边 barrier 推导 golden 新增；既有 golden 0-byte；漏声明 indirect 读边 → 装配期 strict 拒 RED |
| RXS-#### | `spec/rendering_platform.md`（加性） | reflection v1 字段闭集加性扩展「资源→全局 descriptor 索引」（尾随可选字段，缺省序列化 ≡ 字段不存在；RXS-0305 CanonW 律加性修订点）；manifest v1（RXS-0317）加性记录 execution set 成员枚举与 shader library 链接拓扑；interface hash 重算律 | 声明-反射双向精确相等；索引分配确定性双构建逐字节等值；**既有 reflection golden 0-byte 恒跑**（缺省字段产物字节流不变） |
| RXS-#### | 新 `spec/gpu_driven_submit.md`（候选） | DGC 抽象层语义面：IndirectCmdLayout 声明闭集 / token 限制装配期核验（恰一 dispatch token 且最后、禁 render pass、禁插 barrier、禁绑 descriptor set）/ DgcBuffer 无 host 读接口类型契约 / Execution Set capability 降级律 / 三后端映射单一事实源 | token 限制违反逐类装配期 RED；DgcBuffer host 读不可构造（类型层）；D3D12 请求 GPU 侧索引切换 → 显式不可表达诊断 RED |
| RXS-#### | `spec/shader_stages.md`（加性） | SER 原语：`HitObject` 类型面（仅 RT 阶段，沿 RXS-0245 先例）/ `reorderThread` / `hitObjectTraceRay` / `hitObjectInvoke` 签名与阶段合法性矩阵 / RXS-0311 闭集加 `rt.ser` / 材质 flags coherence hint 位段编码值域 / payload 最小 live state 注释 | 非 RT 阶段使用 → 编译期 RED；hit object 逃逸 → RED；`#[requires("rt.ser")]` 推导 + fallback 选择律绿 |
| RXS-#### | `spec/shader_stages.md` + `spec/vulkan_backend.md` / `dxil_backend.md`（加性） | mesh shader 可选路径：cluster 流 → mesh 入口数据契约（对接 M01/M04 页格式 ABI）/ mesh ↔ VS fallback 选择律（`mesh.task` 既有 ID）/ task 维持不开放（RXS-0270 字面重申 + M62 留档引用）；SPIR-V/DXIL 编码面复用 RXS-0246 基建加性扩展 | `mesh.task` 缺失 → fallback 或拒（不静默第三路）；task 入口使用 → 维持 RXS-0270 字面拒；cluster 流契约声明-反射相等 |
| RXS-#### | `spec/rendering_platform.md` 或新 `spec/shader_library.md`（候选） | shader library IR 链接：module 符号导出 / 链接拓扑 manifest 声明律 / 组合物化与 interface hash 重算 / v1 边界（函数级符号链接、禁跨 module 泛型单态化）/ 链接诊断 fail-closed | 符号缺失/接口失配/循环链接 → 编译期 RED；拓扑 → 产物 digest 重算相等 golden |
| RXS-#### | `spec/toolchain.md`（加性）或 Mini-RFC 分拆 | 变体审计工具：`rurix.variant-audit-report.v1` schema / 工程级总预算门硬失败 / 死变体检测报告律 | 总预算超限 → 硬失败 RED；注入已知死变体 → 报告必须列出（漏报即红）；恒等式 enumerated == pruned + emitted |
| （不加条款） | `spec/README.md` §4 登记 | capability profile ID 闭集加性：拟 `submit.execution_set` / `rt.ser` / `bindless.descriptor_heap`（预留位）/ profile JSON schema 加性扩展；`VK_EXT_descriptor_heap` 关注登记（不实现） | 随上述各条款行 |

**错误码策略**：G9.1 零 RX claim。实现波优先复用已冻结的接口/资源/装配诊断类别（RX6029 族装配诊断、RX3020 类 capability 不足等先例）；只有实现证明出现新的、用户可行动、可独立到达的诊断类别（如工程级总预算超限的工具段类别）时，才按当时各段 `next_free` 只追加，并同步 en/zh message key；不预留、不预造。

## 6. feature gate / tracking / 实现序

### 6.1 Gates

本 RFC **不新造 gate 命名空间**。M102/M103/M104 的 canonical P0 key 由 G9.1 已冻结命名空间给出（§2.3 表）；M105~M109 当前无 P0 key，实现波由 G9 验收映射统一登记，本文不预造 gate、不在 workflow 放空步骤。

### 6.2 真实 RED/GREEN（反 YAML-only）

| 面 | RED（必须先可复现） | GREEN（不得以较弱见证替代） |
|---|---|---|
| M102 DGC | token 限制违反（多 dispatch token / 嵌 render pass / 绑 descriptor set）装配期拒；host 读 DgcBuffer 不可构造 | compute pre-pass 填充 → ExecuteIndirect 出图与 CPU 录制等价场景像素级 golden；全程回读计数器 = 0 |
| M105 Execution Set | D3D12 请求 GPU 侧 shader 索引切换 → 显式不可表达诊断 | GPU 侧索引切换 device 见证 + D3D12 降级路独立成门，双路绿 |
| M103 descriptor 全局表 | 索引越界/悬空索引 → fail-closed；capability 缺失 → profile fallback 或拒 | 全局表 ≥ 65536 条目场景出图正确；streaming 换入换出后索引回收无泄漏（计数器断言）；映射表逐字节锚定；**既有 reflection golden 0-byte 恒跑**（尾随可选字段 0-drift 机制，评审 F-2） |
| M104 新依赖边 | 漏声明 indirect 读边 → 装配期 strict 拒 | graph 推导 golden 扩展（新边 barrier 序列逐条锚）+ 既有 golden 0-byte + 同图双跑逐字节等值 |
| M106 IR 链接 | 符号缺失/类型契约失配/接口失配/循环链接 → 编译期拒 | ≥3 module 组合材质 × lighting 变体出图正确；DDC 命中逐字节复用；拓扑 → digest 重算相等 |
| M107 变体审计 | 工程级总预算超限 → 硬失败；注入死变体漏报 → 红 | `rurix.variant-audit-report.v1` 逐字节锚定；恒等式工程级成立 |
| M108 SER | 非 RT 阶段使用 / hit object 逃逸 / payload 超最小 live state → 编译期拒；无 SER 硬件且无 fallback → 拒 | capability 具备设备上 SER 变体与非 SER fallback 变体像素级一致（正确性等价，不比性能） |
| M109 mesh 路径 | `mesh.task` 缺失且请求 mesh 路径 → fallback 或拒；task 入口使用 → 维持 RXS-0270 字面拒 | cluster 流 → mesh shader → DGC 提交最小见证出图；VS fallback 设备出图等价 |

### 6.3 栈式实现序（均门控于本 RFC 合入后）

1. **PR-Spec**：按 §5 拟条款表落 actual RXS 与 RED 语料（🔒 §4.4.3 修订行先行，附 G5 golden 零漂移证明）；capability snapshot 实测确认 DGC/descriptor buffer 可用性（fail-closed）。
2. **PR-DescriptorTable**：M103 reflection/manifest 记录面 + 全局表分配律。
3. **PR-AccessKind**：M104 AccessKind 新边 + graph 推导扩展。
4. **PR-DGC**：M102 Vulkan 原生路 + compute pre-pass → ExecuteIndirect 全链路 device 真跑 + D3D12 降级路。
5. **PR-ShaderLibrary**：M106 IR 链接 + M107 审计工具（同波）+ M105 Execution Set 衔接。
6. **PR-Conditional**：M108 SER 语言原语；M109 mesh 通道待 cluster 流就绪后接线（顺序硬约束）。
7. **PR-Evidence**：P0 evidence schema 落 `milestones/g9/`（路径 §2.3 已冻结，实现波才建文件）；目标硬件真跑，禁 YAML-only 与 host substitution。

## 7. 备选方案

| 方案 | 裁决 | 理由 |
|---|---|---|
| 以 D3D12 `ExecuteIndirect` 为 DGC 抽象基准 | 否决 | 丢 Execution Set 能力；`VK_EXT_device_generated_commands` 语义集有 Proton/DXVK 转译层背书，是跨 API 最小公倍数的正确边 |
| D3D12 静默模拟 GPU 侧 shader 索引切换 | 否决 | 违 P-01 诚实降级；必须 capability ID 区分 + 显式不可表达登记 |
| descriptor 记录面直接替换 set/binding 对 | 否决 | 破坏 M31/M85 既有 digest 链；加性并存不删 |
| 现在实现 `VK_EXT_descriptor_heap` | 否决 | 提案早期未冻结；只预留 feature 位 |
| token 限制放运行时检查 | 否决 | 装配期 fail-closed 沿 RXS-0237 先例；运行时检查留下「碰巧能跑」窗口 |
| 跨 module 泛型单态化进 v1 | 否决 | 链接范围失控风险；v1 边界 = 函数级符号链接，写进条款 |
| 只做 per-entry 预算、不设工程级总预算 | 否决 | 组合爆炸无工程闸（UE 变体治理教训）；总预算门为硬失败非警告 |
| SER 全量渲染器集成 / 默认开启 | 否决 | 收益集中 NVIDIA，跨厂商证据不足；承诺面止步语言层 + capability 可选 |
| mesh/VS 双套全功能并行 | 否决 | 维护成本翻倍；VS 为唯一 fallback |
| 借 DGC 开多队列 / Work Graphs | 否决 | 无 measured 收益证据 + 触冻结面；M56/M59 no-go 维持，RXS-0239 字面不动 |

## 8. 不做（范围红线）

- **Work Graphs 任何实现**（M56 维持 no-go）：仅 render graph 节点 schema 预留「GPU 端 fan-out」表达能力字段，字段命名带 `reserved_` 前缀 + spec 注释「预留不接线」，不接线、不产生 gate 绿；
- **async compute / 多队列第二腿**（M59 维持 no-go）：DGC 全在 RXS-0239 单 queue 全序内表达，不引入多队列与 pass 内重排；
- **task shader 开放**（M62 维持不开放）：RXS-0270 字面不动；
- **D3D12 Execution Set 模拟**：无对应物即显式不可表达 + CPU 侧 PSO 切换降级，禁静默模拟；
- 三级剔除算法本体、meshlet 格式、cluster 流构建（归渲染器主体模块；本 RFC 只提供提交通道与依赖边类型）；
- RT 渲染器中 SER 的实际调度集成（材质 hint 消费、coherence 测量、默认开启属后续专项）；
- `VK_EXT_descriptor_heap` 实现、`VK_EXT_shader_object` 全量切换、`VK_EXT_graphics_pipeline_library` 生产接线（映射备选面/关注项，非承诺）；
- 传统 VS 光栅路径的任何删减（唯一 fallback 地位不动）；
- 不稳定化 DGC buffer 物理字节格式、descriptor heap 偏移编码、Execution Set 句柄、driver stack query 值；
- 不在 G9.1 改 `src/`、`spec/`、`conformance/`、`.github/workflows/`，不 materialize 数字 CI 步骤，不领取 RXS/RD/U/RX 数字号，不创建空 schema 壳/空脚本占位；
- 不登记 M52/M61 的 strategic_override 本身（deferred.json history 追加由 G9.1 治理波统一落），不重裁立项裁决六项。

## 9. 未决问题 / 关键裁决

下表裁决提案经 D-409 第 1 轮对抗性评审（4 findings 全部采纳并修）后，随 Agent Approved（2026-08-09）逐行冻结；后续变更须走本 RFC 加性修订行，不得静默改字面。

| ID | 问题 | Draft 裁决 |
|---|---|---|
| Q1 | DGC 抽象取哪个语义集？ | `VK_EXT_device_generated_commands` 语义集为基准，token 取跨 API 最小公倍数 |
| Q2 | D3D12 无 Execution Set 怎么办？ | 诚实降级 CPU 侧 PSO 切换 + capability ID（拟 `submit.execution_set`）区分路径；显式不可表达登记，禁静默模拟 |
| Q3 | descriptor 记录面替换还是并存？ | 加性扩展「资源→全局 descriptor 索引」，set/binding 对并存不删；分配律/回收进 spec |
| Q4 | 新依赖边是否触 G5 冻结面？ | 触；§4.4.3 🔒 显式修订行为唯一修订，EB 三轴结构与 RXS-0239 字面 0-byte 不动，漏声明读边装配期 strict 拒 |
| Q5 | shader library 主轴？ | 编译期 IR 函数级链接为主轴，API 期链接为后端映射备选；v1 禁跨 module 泛型单态化 |
| Q6 | 工程级总预算门强度？ | 硬失败（非警告）；审计工具与 M106 同波交付 |
| Q7 | SER 承诺面？ | 语言原语 + capability 可选 + 双后端物化；不承诺性能，渲染器集成延后（M52 strategic_override 承接） |
| Q8 | mesh shader 地位与顺序？ | 可选 geometry pipeline；VS 唯一 fallback；顺序硬约束 = meshlet 页格式 v2 与 GPU-driven 剔除之后（M61 strategic_override 承接） |
| Q9 | task shader？ | 维持不开放；RXS-0270 字面不动（M62 留档） |
| Q10 | RFC Approved 是否解锁实现？ | 不；G9.1 governance-only，实现由 G9.2+ 各实现波 spec-first 与 capability snapshot 硬门决定 |

## 9.1 对抗性评审记录（10 §3 / §7 · D-409）

第 1 轮评审已完成，结论「有条件通过」；全部条件（4 findings）已在正文实改，逐条 disposition 如下。

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: Kimi Code CLI (Kimi) rfc0023-adversarial-reviewer`（独立实例；**≠** 起草 provenance `Assisted-by: Kimi Code CLI (Kimi) rfc0023-drafter`） |
| 评审轮次 | 第 1 轮，2026-08-09；结论：有条件通过（4 findings：2 major + 2 minor，全部 disposition = 采纳并修） |

**Findings 与 disposition**：

| # | Finding（评审者提出） | 严重度 | Disposition |
|---|---|---|---|
| F-1 | §4.4.3/§5：新访问类还触两处未申报字面——`spec/render_graph.md:62-66` RXS-0236 访问声明集封闭枚举（「不支持即不可表达」）与 `spec/render_graph.md:220` RXS-0241 🔒 cabi `rxrt_graph_declare` access u32 tag 域 0..=6 | major | **采纳并修 §4.4.3**：修订点表追加两行——① RXS-0236 访问声明集封闭枚举字面：加性扩展 `reads_indirect → IndirectCommandRead`，既有五类 + present 字面 0-byte；② RXS-0241 🔒 cabi tag 域：首期 cabi 不暴露新访问类，字面 0-byte 并显式登记该限制（备选 = 扩 tag 域 0..=7，本 RFC 取前者） |
| F-2 | §4.3/§5/§6.2：「资源→全局 descriptor 索引」是 RXS-0304 v1 闭集之外新字段，按现行 canonical 序列化规则即使空编码为 count 0 也会改变全部既有产物字节 → 与「保 M31/M85 digest 链」「空编码兼容既有 digest 链」「回归 digest 不变」内部矛盾 | major | **采纳并修 §4.3/§5/§6.2**：显式写明 0-drift 机制——新字段为尾随可选字段，缺省序列化 ≡ 字段不存在、既有字节流 0-byte，列为 RXS-0305 CanonW 律的加性修订点；§5 rendering_platform.md 行测试锚与 §6.2 M103 GREEN 补「既有 reflection golden 0-byte 恒跑」 |
| F-3 | §4.8：「RD-039『measured』条件以本机 4070 Ti + CI 设备 measured 补齐」丢失「可」字，有被误读为已发生之嫌 | minor | **采纳并修 §4.8**：改为「可在本机 4070 Ti + CI 设备 measured 补齐（实现期前置，当前无 measured artifact）」 |
| F-4 | §4.7：SER 收益数字（+47%/23.0%→54.2%）内联未标出处 | minor | **采纳并修 §4.7**：补出处「（R6 §四，Khronos 官方案例 [S16][S19]）」 |

**偏差说明**：首选跨工具评审者在本环境不可得，本轮评审由同工具族独立实例执行（评审 provenance ≠ 起草 provenance），按 RFC-0015 §9.1 / number_ledger v1.29 先例如实登记，不构成对 D-409 字面之外效力的声称。

## 10. 稳定化与 provenance

- **特性生命周期**：RFC Agent Approved 只是语义评审完成；随后仍需实现波 spec-first/RED → gated implementation → tracking evidence → 至少两个里程碑无重大语义修订 → stabilization report → FCP-lite。
- **稳定面候选**：DGC 抽象类型面与 token 闭集语义、AccessKind 新访问类语义、「资源→全局 descriptor 索引」记录 schema、链接拓扑 manifest 律、`rt.ser` capability ID 语义；是否 stable 由未来 stabilization report 裁决。
- **明确非 stable**：DGC buffer 物理字节格式、descriptor heap 偏移编码、Execution Set 句柄、driver stack query 值、变体预算数值阈值。
- **Provenance**：`Assisted-by: Kimi Code CLI (Kimi) rfc0023-drafter`。

## 11. 规范与实现依据

- 仓库内：[RFC-0019](0019-rendering-platform.md)（M29/M30/M31/M32 着色治理、G5 EB 0-byte 边界与单源先例）；[G9_CAPABILITY_MATRIX](../milestones/g9/G9_CAPABILITY_MATRIX.md) §3（M102~M109 行字面）；[G9_D3 设计草案](../milestones/g9/design/G9_D3_GPU_DRIVEN_SUBMISSION.md)（G9.0 冻结引用，内容事实源）；[R6 调研](../milestones/g9/research/R6_GPU_DRIVEN_SUBMISSION.md)（调研依据 1~6 与来源复核）；`milestones/g8/G8_P2_DECISIONS.md` M33/M52/M55/M56/M59/M61/M62 承接锚行。
- 现有基础（只读引用）：`spec/render_graph.md` RXS-0236~0241；`spec/binding_layout.md` RXS-0233；`spec/shader_stages.md` RXS-0231~0232/0242~0245/0311；`spec/rendering_platform.md` RXS-0304~0318；`spec/rhi.md` RXS-0270。
- 外部规范（实现期以仓库 pin 的工具链为准）：`VK_EXT_device_generated_commands`、`VK_EXT_descriptor_buffer`、`VK_EXT_descriptor_heap`（关注不实现）、`VK_EXT_ray_tracing_invocation_reorder`、`VK_EXT_mesh_shader`、DXR 1.2 / Shader Model 6.9；真实 device/driver capability snapshot 随 evidence 归档。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-09 | AI 起草初版：冻结 M102~M109 八语义面（DGC 抽象 / Execution Set / descriptor 全局表 / command build 节点与 🔒 G5 EB 唯一修订行 / IR 函数级链接 / 变体审计 / SER 原语 / mesh 可选路径）；§5 拟条款表不预写号；§9.1 第 1 轮对抗性评审待进行。G9.1 governance-only：零 `src/`/`spec/`/`conformance/` 改动、零编号 claim、零 CI materialize；M52→M108 / M61→M109 按立项裁决 strategic_override 定案承接，不重裁、不登记 | Full RFC（Draft） |
| v1.0 | 2026-08-09 | **Agent Approved**：D-409 第 1 轮对抗性评审完成（评审 provenance `Kimi Code CLI (Kimi) rfc0023-adversarial-reviewer` 独立实例 ≠ 起草），结论「有条件通过」，4 findings（2 major + 2 minor）全部采纳并修：F-1 §4.4.3 修订点表补 RXS-0236 访问声明集与 RXS-0241 🔒 cabi tag 域两行（cabi 首期不暴露新访问类，字面 0-byte）；F-2 §4.3/§5/§6.2 补尾随可选字段 0-drift 序列化机制（RXS-0305 CanonW 律加性修订点，既有 reflection golden 0-byte 恒跑）；F-3 §4.8「可…补齐（实现期前置，当前无 measured artifact）」措辞；F-4 §4.7 SER 收益数字补出处（R6 §四，[S16][S19]）。同工具族独立实例评审偏差按 RFC-0015 §9.1 / number_ledger v1.29 先例如实登记（§9.1）。头部状态翻 Agent Approved；零 RXS/RD/U/RX 数字 claim；批准不解锁实现 | Full RFC（Agent Approved） |
