# gpu_driven_submit.md — GPU-driven 提交（DGC 抽象层）语义面（G9.2 M102）

> **地位**：GPU-driven 提交（Device-Generated Commands 跨 API 最小公倍数抽象）语义
> 事实源之一（RFC-0023 §4.1/§4.2，Agent Approved 2026-08-09；G9_ACCEPTANCE_MAP
> §2 M102 行）。配套面：render graph 新访问类/新依赖边见
> [render_graph.md](render_graph.md) RXS-0346（🔒 修订行表）；「资源→全局
> descriptor 索引」记录面见 [rendering_platform.md](rendering_platform.md)
> RXS-0347；capability ID 闭集修订行见 [shader_stages.md](shader_stages.md)
> RXS-0349。
>
> **档位**：Full RFC / RFC-0023。
>
> **编号**：RXS-0348（G9.2 spec-first，自合入时 `registry/number_ledger.json` 实测
> `RXS.next_free = 344` 顺位领取之本批第五号；编号永不复用，10 §9.5）。
>
> **新建裁决留痕（G9.2 spec PR）**：RFC-0023 §5 拟条款表把 DGC 抽象层语义面的目标
> spec 冻结为「新 `spec/gpu_driven_submit.md`（候选）」；本 PR 裁定**新建本文件**
> （render_graph.md / rendering_platform.md 新建先例，spec/README.md v1.65/v1.70
> 行）——DGC 类型面/token 闭集/三后端映射与既有 rhi.md（库面）/ render_graph.md
> （推导面）均不同轴，独立成文。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——token 限制违例、
  capability 缺失、Execution Set 不可表达均为编译期/装配期确定性拒绝或库层
  typed `Err`（fail-closed），不设未定义行为。
- 实现锚定：`src/rurix-rt/src/vk.rs`（DGC FFI 面，U 号实现期登记）+
  `src/rurix-rt` safe 类型层包装（`IndirectCmdLayout` / `DgcBuffer`）。
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

## 4. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-09 | 新建（G9.2 spec-first，M102）：RXS-0348 DGC 抽象层语义面——IndirectCmdLayout 声明闭集（token = ExecuteIndirect 语义跨 API 最小公倍数）/ token 限制装配期核验 fail-closed（恰一 dispatch/draw 终止 token 且最后、sequence 内禁 render pass、禁插 barrier、禁绑 descriptor set）/ DgcBuffer 类型层无 host 读接口契约（镜像 RXS-0144~0148 AsyncBuffer 先例；调试 dump 走显式 readback pass；回读计数器 = 0 断言）/ 三后端映射单一事实源（VkIndirectCommandsLayoutEXT+vkCmdExecuteGeneratedCommandsEXT / D3D12 command signature+ExecuteIndirect / NVPTX 不承诺）/ capability snapshot 阻塞性前置（M32 snapshot 机制 RXS-0313，缺 capability fail-closed 禁静默模拟）/ Execution Set 降级律登记（D3D12 CPU 侧 PSO 切换，`submit.execution_set` capability 区分，禁静默模拟）。依据 [RFC-0023](../rfcs/0023-gpu-driven-submission-shading.md)（Agent Approved 2026-08-09）§4.1/§4.2/§5 + G9_ACCEPTANCE_MAP M102 行 | **Full RFC**（RFC-0023） |
