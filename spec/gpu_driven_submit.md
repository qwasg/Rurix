# gpu_driven_submit.md — GPU-driven 提交（DGC 抽象层）语义面（G9.2 M102 / G9.3 M105~M107 P1）

> **地位**：GPU-driven 提交（Device-Generated Commands 跨 API 最小公倍数抽象）语义
> 事实源之一（RFC-0023 §4.1/§4.2，Agent Approved 2026-08-09；G9_ACCEPTANCE_MAP
> §2 M102 行）。**G9.3 P1 面**（command build node 全链路零 CPU 回读 /
> Execution Set 与 PSO 衔接 / shader library IR 链接与变体预算）承 RFC-0023
> §4.4/§4.2/§4.5/§4.6，登记行 = G9_ACCEPTANCE_MAP §3 M105/M106/M107 行（G9.3 波
> P1 全进裁决，G9_CONTRACT §8.1 裁决①）；M105~M107 承接面按 G9.3 执行波口径
> （G9_PLAN §2 G9.3 D3 链路行），与 RFC-0023 §4.2/§4.4 章节题的 M## 分述差异以
> G9_CANDIDATE_DECISIONS v1.2 校准注登记。配套面：render graph 新访问类/新依赖边见
> [render_graph.md](render_graph.md) RXS-0346（🔒 修订行表）；「资源→全局
> descriptor 索引」记录面见 [rendering_platform.md](rendering_platform.md)
> RXS-0347；capability ID 闭集修订行见 [shader_stages.md](shader_stages.md)
> RXS-0349（`submit.execution_set` 预留位由本文件 RXS-0355 转正消费）。
>
> **档位**：Full RFC / RFC-0023。
>
> **编号**：RXS-0348（G9.2 spec-first，自合入时 `registry/number_ledger.json` 实测
> `RXS.next_free = 344` 顺位领取之本批第五号；编号永不复用，10 §9.5）+
> RXS-0354~0356（G9.3 spec-first P1 批，自合入时实测 `RXS.next_free = 353`
> 顺位领取之本批第二~四号〔首号 0353 落 virtual_geometry.md〕，0354~0356
> 连续不跳号）。
>
> **新建裁决留痕（G9.2 spec PR）**：RFC-0023 §5 拟条款表把 DGC 抽象层语义面的目标
> spec 冻结为「新 `spec/gpu_driven_submit.md`（候选）」；本 PR 裁定**新建本文件**
> （render_graph.md / rendering_platform.md 新建先例，spec/README.md v1.65/v1.70
> 行）——DGC 类型面/token 闭集/三后端映射与既有 rhi.md（库面）/ render_graph.md
> （推导面）均不同轴，独立成文。**目标 spec 合并裁决留痕（G9.3 P1 spec PR）**：
> RFC-0023 §5 映射行「shader library IR 链接」的候选目标 spec 为
> `spec/rendering_platform.md` 或新 `spec/shader_library.md`（候选）；本波裁定
> **合并落本文件**——IR 链接/变体预算与 Execution Set、command build node 同属
> D3 GPU-driven 提交链语义，与 RXS-0348 同卷；`shader_library.md` 不新建、
> `rendering_platform.md` 本体 0-byte（沿 virtual_geometry.md G9.3 合并裁决先例，
> 头注留痕）。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——token 限制违例、
  capability 缺失、Execution Set 不可表达均为编译期/装配期确定性拒绝或库层
  typed `Err`（fail-closed），不设未定义行为。
- 实现锚定：`src/rurix-rt/src/vk.rs`（DGC FFI 面，U 号实现期登记）+
  `src/rurix-rt` safe 类型层包装（`IndirectCmdLayout` / `DgcBuffer`）。
- 实现锚定（G9.3 P1 面）：`src/rurixc/src/capability_check.rs` ID 闭集表
  `submit.execution_set` 预留转正（RXS-0355，本批同落）+ command build node
  生产者与 host 参照构建器、Execution Set 构建/失效重建面、IR 链接器与变体
  审计工具（实现期命名模块）。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定（traceability 矩阵全锚定，10 §4）。

## 2. 术语

- **DGC**：Device-Generated Commands——compute pre-pass 在 GPU 上产出命令数据、
  由 indirect 执行消费，全程零 CPU 回读。
- **IndirectCmdLayout**：声明式模板，描述一个命令 sequence 的 token 序列
  （vertex/index buffer 绑定、push constant、draw/dispatch 等）。
- **DgcBuffer**：GPU 可写的命令数据 buffer；类型层**无 host 读接口**。
- **Execution Set**：同状态仅换 shader 的管线数组，GPU 侧索引切换。

---

## 3. 条款（RXS-0348）

### RXS-0348 DGC 抽象层：IndirectCmdLayout token 闭集装配期核验、DgcBuffer 无 host 读接口与三后端映射

**Syntax**（声明闭集，首期 token 集 = `ExecuteIndirect` 语义的**跨 API 最小公
倍数**，RFC-0023 §4.1.2 逐字）：

```
token ::= bind_vertex_buffer | bind_index_buffer | push_constants
        | draw | draw_indexed | dispatch        // 终止 token 三选一,恰一且最后
```

draw / draw_indexed / dispatch + 少量状态 token 即全部；超出子集的 token（如
D3D12 专有）首期**不可表达**。

**Legality**

1. **token 限制装配期核验（fail-closed，RFC-0023 §4.1.2 逐字）**：DGC 的 API
   限制**不进运行时检查，内化为 layout 声明的编译期/装配期核验**（沿 RXS-0237
   装配核验先例）——
   - 每 sequence **恰一个** dispatch/draw 终止 token 且必须位于**最后**；
   - sequence 内**不可开 render pass**；
   - **不可插 barrier**；
   - **不可绑 descriptor set**。

   任一违例 = 装配期确定性拒绝（沿 RX6029 族装配诊断先例，实号实现期领取），
   fail-closed，不存在「碰巧能跑」（token 限制放运行时检查方案否决，RFC-0023 §7）。
2. **DgcBuffer 无 host 读接口类型契约**（RFC-0023 §4.1.1 逐字；镜像 RXS-0144~0148
   `AsyncBuffer` 在途态无读接口先例）：`DgcBuffer` 类型**不提供** host 读接口——
   这是「零 CPU 回读」的**类型层结构性保证**而非纪律口号；host 侧读/写/取址接口
   不存在 = 方法不存在编译期拦截（结构性断言，非运行期错误）。调试 dump 走
   **显式 readback pass**（`g.readback` 既有面，RXS-0236）。配套验收面（RFC-0023
   §4.4.2）：「回读计数器 = 0」断言——任何隐式回读（如调试路径）必须经计数器
   显式记账，计数器非零即红。
3. **capability snapshot 阻塞性前置**（RFC-0023 §4.1.4 逐字）：
   `VK_EXT_device_generated_commands` 在目标硬件（4070 Ti 起步 + CI 设备清单）的
   capability snapshot 实测确认为**阻塞性前置**，走 M32 snapshot 核验原语
   （RXS-0313）既有机制，**fail-closed**；缺 capability → 装载期/装配期确定性
   拒绝，**禁静默模拟**；EXT 较新、非 NVIDIA 驱动实现质量未证，单 vendor 绿
   不算绿。
4. **Execution Set 降级律登记**（RFC-0023 §4.2.2 逐字）：D3D12 无 Execution Set
   对应能力 → **诚实降级 CPU 侧 PSO 切换**再录 `ExecuteIndirect`——降级路径必须
   显式登记「GPU 侧 shader 索引切换不可表达」，**不静默模拟**（P-01）；两条路径
   由 capability ID（`submit.execution_set`，预留位见 RXS-0349）区分，profile
   选择律（RXS-0312）裁定 fallback；请求 GPU 侧索引切换而 profile 不含该
   capability → **显式不可表达诊断**，不静默降级为模拟。
5. **三后端映射单一事实源**（RFC-0023 §4.1.3 表逐字；镜像 RXS-0238「双后端映射
   同源」纪律）：

| 抽象 | Vulkan | D3D12 | NVPTX |
|---|---|---|---|
| IndirectCmdLayout | `VkIndirectCommandsLayoutEXT` | command signature（`ID3D12CommandSignature`） | 不承诺（仅命令数据生成） |
| DgcBuffer 填充 | GPU compute 直写 + `vkCmdPreprocessGeneratedCommandsEXT` | GPU compute 直写 argument buffer | compute kernel 产出 buffer 数据（既有 launch 语义） |
| 执行 | `vkCmdExecuteGeneratedCommandsEXT` | `ExecuteIndirect` | — |
| Execution Set | `VkIndirectExecutionSetEXT`（GPU 侧索引切换） | 无对应物 → 诚实降级（本条第 4 款） | — |

6. **非 stable 物理布局**（RFC-0023 §4.0-5 逐字）：DGC buffer 物理字节格式
   （`vkCmdPreprocessGeneratedCommandsEXT` 产物）、Execution Set 句柄均为实现
   确定、**非 stable**；本条款只作存在性/确定性声明，不冻结数值布局。

**Implementation Requirements**

- 实现锚定 `src/rurix-rt/src/vk.rs` DGC FFI 面（照 KHR AS 段范式逐值核对；U 号
  实现期自 ledger 实测 next_free 顺位登记）+ `src/rurix-rt` safe 类型层
  （`DgcBuffer` 无 host 读接口结构性断言 + `IndirectCmdLayout` 装配期核验）。
- RED 锚定计划（实现 PR 落）：token 限制违反逐类装配期 RED（多终止 token /
  终止 token 非最后 / 嵌 render pass / 插 barrier / 绑 descriptor set）+
  DgcBuffer host 读不可构造（类型层）+ D3D12 请求 GPU 侧索引切换 → 显式不可
  表达诊断 RED + capability 缺失 fail-closed RED。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/gpu_driven_submit/reject/dgc_layout_double_terminator.rx`（条款
  锚定占位，inert 锚定口径与转正路径见该文件头注释）；锚点目标文件（实现 PR
  转正）= `src/rurix-rt` DGC 类型层/装配核验单测。

---

## 4. 条款（RXS-0354，G9.3 M105 command build node）

### RXS-0354 command build node 图节点语义、全链路零 CPU 回读结构约束与构建产物确定性

**Legality**

1. **图节点语义**（RFC-0023 §4.4.1 逐字）：command build node = render graph 内
   compute pre-pass（既有 compute pass kind，声明 `reads_writes_uav(dgc_buf)`）
   在 GPU 上产出命令数据，后续 indirect-draw pass 声明 `reads_indirect(dgc_buf)`
   （RXS-0346 已冻结新访问类，字面 0-byte 复用不重定）消费；「GPU 端生成 → GPU
   端消费」是 RXS-0239 单 queue 全序内的数据流，不引入 pass 内重排语义。
2. **零 CPU 回读结构性强约束**（RFC-0023 §4.1.1/§4.4.2 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M105 行）：host 侧对 DgcBuffer 命令数据的读接口
   **不存在**（RXS-0348 L2 类型层契约——方法不存在 = 编译期结构性拦截，非运行期
   错误）；**回读计数器恒 0**——任何隐式回读（含调试路径）必须经计数器显式
   记账，计数器非零即红；调试 dump 唯一通道 = 显式 readback pass（`g.readback`
   既有面，RXS-0236）。
3. **构建产物确定性**（RFC-0023 §4.0-3/§4.4 逐字；判据逐字引 G9_ACCEPTANCE_MAP
   §3 M105 行）：同一输入图 + 同一编译器/构建器版本下，command build node 产出的
   命令数据内容流与 host 参照**逐字节一致**；同输入双构建 digest 相等。DGC
   buffer 物理字节格式（preprocess 产物）为实现确定、非 stable（RXS-0348 L6
   同口径）——本条款只冻结内容流一致性与确定性，不冻结数值布局。
4. **全链路断言面**（判据逐字引 G9_ACCEPTANCE_MAP §3 M105 行）：compute pre-pass
   产 → indirect pass 消费全链路零 CPU 回读由**结构性断言 + readback_counter=0
   机器核验**双承担；M104 P0 门已冻结的 AccessKind 新边与装配期 strict 面
   （RXS-0346）不降格、不重复冻结。

**Implementation Requirements**

- 实现锚定（实现期命名）：`src/rurix-rt` DGC 类型层/图编排面（RXS-0346/0348
  消费）+ command build node 生产者与 host 参照构建器（纯 safe 方向）；unsafe
  确需时按当时 `U.next_free` 实测顺位登记 unsafe-audit。
- RED 锚定计划（实现 PR 落）：host 侧读 DgcBuffer 的 variant → 类型层不可构造 /
  回读计数器非零 RED；构建产物与 host 参照逐字节 golden。
- 本 spec PR 落锚定语料
  `conformance/gpu_driven_submit/reject/command_build_host_readback.rx`（条款锚定
  最小 RED 语料，`//@ spec: RXS-0354`）；锚点目标（实现 PR 转正）=
  `ci/g9_command_build_node_smoke.py` 门（symbolic key
  `g9.p1.m105.command_build_node`，G9.3 波 P1 登记字面不动）。

---

## 5. 条款（RXS-0355，G9.3 M106 Execution Set）

### RXS-0355 Execution Set 语义、`submit.execution_set` capability 预留转正与 PSO 衔接/失效律

**Legality**

1. **Execution Set 语义与 PSO/manifest 衔接**（RFC-0023 §4.2.1 逐字）：Execution
   Set = 同一 graphics/compute 状态、仅 shader 不同的管线数组，GPU 侧索引切换；
   材质变体为自然消费方（同一 pass 状态模板下按 material ID 索引切换 shader）。
   成员 = PSO cache 条目集合的子集视图（cache key RXS-0306 pipeline key 加性扩展
   「execution set 成员身份」字段）；manifest（RXS-0317）记录 set 成员枚举，
   **全部离线物化**进 DDC；运行时 JIT 链接（`VK_EXT_shader_object` /
   `VK_EXT_graphics_pipeline_library`）只作映射备选面登记，不进承诺。
2. **`submit.execution_set` capability 预留转正**（RXS-0349 L1 预留位兑现；
   RXS-0311 加性修订行纪律）：capability ID `submit.execution_set` 自本条款起由
   预留位（只预留不实现）转为**实位**——`#[requires("submit.execution_set")]`
   为正当接收（不再是闭集外 ID，RX3023 不适用）；profile v1 三集
   （required/optional/forbidden，RXS-0312）可引用该 ID，选择律裁定 fallback；
   闭集后续加性演进仍走 RXS-0311 加性修订行（禁静默扩）；另一预留位
   `bindless.descriptor_heap` 维持预留不动。
3. **失效与重建确定性**（判据逐字引 G9_ACCEPTANCE_MAP §3 M106 行）：Execution
   Set 句柄失效（设备丢失/显式重建场景）后的重建序列对同输入确定（重建产物成员
   枚举与 manifest 逐位一致）；句柄物理值为实现确定、非 stable（RFC-0023 §4.0-5
   逐字）。
4. **capability 缺失 fail-closed**（RFC-0023 §4.2.2 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M106 行）：D3D12 无 Execution Set 对应能力 → 诚实降级
   CPU 侧选 PSO 再录 `ExecuteIndirect`，编译/装配产物显式登记「GPU 侧 shader
   索引切换不可表达」，**不静默模拟**（P-01）；请求 GPU 侧索引切换而 profile 不含
   该 capability → 显式不可表达诊断，不静默降级为模拟。

**Implementation Requirements**

- 实现锚定：`src/rurixc/src/capability_check.rs` ID 闭集表（`submit.execution_set`
  预留转正，RX3023 钉死测试同步更新，**本 spec PR 同落**）+ Execution Set
  构建/失效重建面（`src/rurix-rt/src/vk.rs` `VkIndirectExecutionSetEXT` FFI，U 号
  实现期按实测 `U.next_free` 顺位登记）。
- RED 锚定计划（实现 PR 落）：profile 缺 capability 请求 GPU 侧索引切换 → 显式
  不可表达诊断 RED；失效重建对同输入双运行 digest golden。
- 测试锚定：`src/rurixc/src/capability_check.rs` `//@ spec: RXS-0355` 单测（本 PR
  落）；锚点目标（实现 PR 转正）= `ci/g9_execution_set_pso_smoke.py` 门
  （symbolic key `g9.p1.m106.execution_set_pso`，G9.3 波 P1 登记字面不动）。

---

## 6. 条款（RXS-0356，G9.3 M107 shader library IR 链接与变体预算）

### RXS-0356 shader library IR 函数级组合链接、interface hash 确定性与变体工程级总预算硬失败

**Legality**

1. **IR 链接主轴与 v1 边界**（RFC-0023 §4.5 逐字）：编译期 IR 链接——module 级
   着色函数以稳定符号导出，链接期按 manifest 显式声明的拓扑把「材质函数 ×
   lighting 函数 × pass 入口」组合物化为完整 SPIR-V/DXIL 产物；v1 只做**函数级
   符号链接**，**禁跨 module 泛型单态化**，禁隐式全图链接；Slang 生态仅作语义
   对标与互操作评估对象，不做运行时依赖。
2. **interface hash 确定性**（RFC-0023 §4.5 逐字；判据逐字引 G9_ACCEPTANCE_MAP
   §3 M107 行）：链接后 **interface hash 重算并写回 manifest**（RXS-0306 定义面
   不变），manifest 记录链接拓扑（哪个 module 的哪个符号进哪个变体）；**同输入
   双构建 interface hash 相等**；拓扑 → 产物 digest 重算相等（审计可回放）。IR
   链接发生在 permutation 求解之后（变体 key 确定 → manifest 查拓扑 → 组合物化
   → artifact digest 进 DDC，`--permutation-select` 路径 RXS-0310 承载）。
3. **链接合法性 fail-closed**（RFC-0023 §4.5 逐字）：跨 module 函数链接的类型
   契约 = 既有阶段间接口契约（RXS-0155）+ reflection 接口事实同一提取律（单一
   事实源）；符号缺失/类型契约失配/接口失配/循环链接 → 编译期确定性诊断（无
   最近邻回退，沿 RXS-0310 选择律先例），不设 UB。
4. **变体工程级总预算硬失败**（RFC-0023 §4.6 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M107 行）：per-entry budget（RXS-0310 既有）之外新增
   **工程级总预算门**——超预算**装配期硬失败**（非警告；诊断码实现期从工具段按
   实际可达类别领取，不预造）；变体审计报告 schema 沿 `rurix.permutation-report.v1`
   先例新建 `rurix.variant-audit-report.v1`（按 axis 贡献/module·pass 归属/DDC
   命中率分解）；审计恒等式 `enumerated == pruned + emitted` 工程级成立，
   manifest 声明变体 ∪ DDC 产物闭合（无声明外产物）；**死变体只报告不自动删**
   （删除是人的决定）。

**Implementation Requirements**

- 实现锚定（实现期命名）：`src/rurixc` IR 链接器（TBIR/MIR 层函数级组合）+
  manifest 链接拓扑记录面（RXS-0317 消费）+ 变体审计工具（工程级总预算门）。
- RED 锚定计划（实现 PR 落）：符号缺失/接口失配/循环链接 → 编译期 RED；双构建
  interface hash golden；工程级总预算超限 → 装配期硬失败 RED。
- 本 spec PR 落锚定语料
  `conformance/gpu_driven_submit/reject/variant_budget_exceeded.rx`（条款锚定最小
  RED 语料，`//@ spec: RXS-0356`）；锚点目标（实现 PR 转正）=
  `ci/g9_shader_library_ir_link_smoke.py` 门（symbolic key
  `g9.p1.m107.shader_library_ir_link`，G9.3 波 P1 登记字面不动）。

---

## 7. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-09 | 新建（G9.2 spec-first，M102）：RXS-0348 DGC 抽象层语义面——IndirectCmdLayout 声明闭集（token = ExecuteIndirect 语义跨 API 最小公倍数）/ token 限制装配期核验 fail-closed（恰一 dispatch/draw 终止 token 且最后、sequence 内禁 render pass、禁插 barrier、禁绑 descriptor set）/ DgcBuffer 类型层无 host 读接口契约（镜像 RXS-0144~0148 AsyncBuffer 先例；调试 dump 走显式 readback pass；回读计数器 = 0 断言）/ 三后端映射单一事实源（VkIndirectCommandsLayoutEXT+vkCmdExecuteGeneratedCommandsEXT / D3D12 command signature+ExecuteIndirect / NVPTX 不承诺）/ capability snapshot 阻塞性前置（M32 snapshot 机制 RXS-0313，缺 capability fail-closed 禁静默模拟）/ Execution Set 降级律登记（D3D12 CPU 侧 PSO 切换，`submit.execution_set` capability 区分，禁静默模拟）。依据 [RFC-0023](../rfcs/0023-gpu-driven-submission-shading.md)（Agent Approved 2026-08-09）§4.1/§4.2/§5 + G9_ACCEPTANCE_MAP M102 行 | **Full RFC**（RFC-0023） |
| v1.1 | 2026-08-11 | 加性扩写（G9.3 spec-first P1 批，M105/M106/M107，硬规则 7 条款先行）：**RXS-0354**（command build node 图节点语义 + 全链路零 CPU 回读结构性强约束〔host 读接口不存在 + 回读计数器恒 0，调试 dump 唯一通道 = 显式 readback pass〕+ 构建产物与 host 参照逐字节一致 + 双构建 digest 相等）/ **RXS-0355**（Execution Set 语义与 PSO/manifest 衔接 + **`submit.execution_set` capability 由 RXS-0349 预留位转正**〔`#[requires]` 正当接收、profile 三集可引用，`bindless.descriptor_heap` 维持预留〕+ 失效重建对同输入确定 + capability 缺失 fail-closed〔D3D12 诚实降级显式登记，禁静默模拟〕；capability_check.rs 闭集转正与 RX3023 钉死测试同步本 PR 同落）/ **RXS-0356**（shader library IR 函数级组合链接〔v1 边界：禁跨 module 泛型单态化、manifest 显式拓扑〕+ interface hash 重算写回 manifest、同输入双构建相等 + 链接拓扑可回放 + 链接违例 fail-closed + 变体工程级总预算装配期硬失败 + `rurix.variant-audit-report.v1` + 死变体只报告不自动删）。**目标 spec 合并裁决**：RFC-0023 §5「shader library IR 链接」候选目标 spec（rendering_platform.md / 新 shader_library.md）裁定合并落本文件，两候选文件本体 0-byte。条款号自 ledger 实测 `RXS.next_free=353` 顺位领取本批第二~四号（0354~0356 连续不跳号，首号 0353 落 virtual_geometry.md）。conformance 锚定语料两件（reject/command_build_host_readback.rx + reject/variant_budget_exceeded.rx）同 PR 落；symbolic key `g9.p1.m105/m106/m107.*`（G9.3 波 P1 全进裁决登记，G9_ACCEPTANCE_MAP §3 / CI_GATES §4A）。零新 RX 码（诊断码实现期按实际可达类别领取，不预造）、零新 U/RD/SG、零 workflow 步骤。依据 [RFC-0023](../rfcs/0023-gpu-driven-submission-shading.md)（Agent Approved 2026-08-09）§4.2/§4.4/§4.5/§4.6 + G9_ACCEPTANCE_MAP §3 M105/M106/M107 行（判据逐字）。既有条款 RXS-0348 字面 0-byte | **Full RFC**（RFC-0023） |
