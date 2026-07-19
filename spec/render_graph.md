# render graph 声明式宿主库语义面（G3.5，RFC-0013 §4.D）

> 条款：**RXS-0236 ~ RXS-0241**（G3.5，验收门 G-G3-5）。体例见 [README.md](README.md)。
> 承 [RFC-0013](../rfcs/0013-industrial-rendering.md)（Agent Approved 2026-07-18，§4.D 全文批准，含
> 🔒 禁区 §4.D4 / §4.D7）。**RD-020 兑现**：RFC-0006 §9 Q-Barrier 首期裁决（手动 barrier 编排、
> 自动推导 defer）自此升级——RXS-0169 手动核验器不废除，转为推导产物的独立复核门（D6）。

> 规范先行（AGENTS.md 硬规则第 7 条）：**条款 commit 先于实现 commit**。`ci/trace_matrix.py --check`
> 要求每条 `### RXS-####` ≥1 测试锚定（`//@ spec: RXS-####`）；本文件条款的锚定测试（`graph.rs`
> host 单测 + `uc04-demo` D6 互证 + `rurix-rt-cabi` 符号单测 + conformance/host_orch graph 语料 +
> `ci/render_graph_smoke.py`）随实现 commit 同 PR 落。stable 快照因条款计数增长同 PR 重 bless
> （RXS-0180 L2 加性演进）。

## 1. 范围与编号区间

**声明式宿主库面，无新语法**。`Graph` / `PassBuilder` / `GraphResource` 为编译器已知签名的 lang-item
宿主类型（RXS-0189 lang-item + RXS-0190 已知签名分支先例，零新文法产生式）；pass 以五类访问集方法
声明读写面；**声明序 = 提交序**（不做重排）；自动资源状态推导为 rurix-rt 纯 host safe 模块
[`graph.rs`](../src/rurix-rt/src/graph.rs)（always-on、零 unsafe、零后端调用）；pass 边界 happens-before
语义本体为 🔒 条款；D3D12 / Vulkan 双执行器消费**同一**推导产物；uc04 手动 `plan_barriers` 保留为
独立复核门。用户样例见 RFC-0013 §3.4。

- **RXS-0236**：Graph/Pass 宿主库类型面与访问声明集。
- **RXS-0237**：声明序 = 提交序、图合法性装配核验、声明-反射双向精确相等。
- **RXS-0238**：自动资源状态推导状态机（纯函数）。
- **RXS-0239**：🔒 pass 边界 happens-before 语义本体（禁区子节，本 RFC 全文批准对象）。
- **RXS-0240**：双后端 barrier 映射与执行器语义（`run_graph`）。
- **RXS-0241**：`rxrt_graph_*` C ABI 与手动复核门（🔒 FFI 延伸）。

**编号区间**：本文件条款自 **RXS-0236** 起（RFC-0013 伞形分配，续 G3.4 的 RXS-0235）；区间登记于
[README.md](README.md) §4 文件清单。

**首期不可表达面（§4.0-3）**：bindless 表声明、storage image（`TextureRw2D`）资源、mesh/RT pass kind
均不在访问声明封闭枚举内——凡含此三者的 pass 首期不可经 graph 表达，显式登记 §8（RD-034+），不静默；
storage image barrier 首期走 RXS-0169 手动路。

## 2. 条款（RXS-0236 ~ RXS-0241）

### RXS-0236 Graph/Pass 宿主库类型面与访问声明集

**Syntax**（render graph 宿主类型与方法集，lang items）：

```
Graph<C> / GraphResource<C> / PassBuilder<C>              // 非 Copy affine 句柄结构
Graph::create(&ctx) -> Graph<C>                           // affine 图本体；brand = Context（单 brand）
g.color_target(w: u32, h: u32) -> GraphResource<C>        // color attachment 资源
g.depth_target(w: u32, h: u32) -> GraphResource<C>        // depth attachment 资源
g.pass() -> PassBuilder<C>                                // 声明序 = 提交序
pb.writes_rt(t) / writes_depth(t) / reads(t) / reads_writes_uav(t) -> PassBuilder<C>   // 五类访问声明
g.readback(t)                                             // 源 CopySrc + 自动 readback 目的 buffer
g.execute()                                               // 装配核验 → 状态推导 → 单 queue 顺序提交
```

**Legality**：

- 类型为编译器 lang items（`Graph` / `GraphResource` / `PassBuilder`，追加于既有 lang items 之后，
  DefId 编号稳定），用户同名定义优先遮蔽、语义不变（兜底纪律沿 RXS-0189）。全部句柄类型为**非 Copy
  affine**：move 后再用 / 重复 move / 借用冲突等违例**复用 RXS-0054 与 RXS-0057~0061 既有裁决**
  （零新借用码）。资源实参（`writes_rt(t)` 等的 `t`）与图/pass 接收者的方法调用为**调用期短借用、
  不 move**（镜像 launch/register Buffer 实参纪律，RXS-0191/0235）——资源句柄可跨 pass 复用（同一
  target 被一个 pass 写、另一个 pass 读）；`writes_*`/`reads*` 消费并返回 `PassBuilder`（builder 链）。
- **访问声明集（封闭枚举——本面「不支持即不可表达」）**：`writes_rt → ColorAttachmentWrite` /
  `writes_depth → DepthAttachmentWrite` / `reads → ShaderRead` / `reads_writes_uav → UavReadWrite`
  （唯一合法读写合并）/ `readback → CopySrcReadback（源）+ CopyDstReadback（目的 buffer）` / present
  终端胶水 `→ PresentHandoff`（D5c）。枚举单一事实源见 [`graph.rs`](../src/rurix-rt/src/graph.rs)
  `AccessKind`（D3/D5 双后端映射同居其一处）。
- **宿主 API 着色合法性**：render graph 宿主类型的构造与方法调用**仅 host 着色上下文合法**；出现在
  `kernel` / `device fn` 体内 → **RX3015**（coloring 层，与 RX3001 同点位，承 RXS-0189）。
- **brand 契约**：`Graph::create(&ctx)` 沿单 brand 方案（`Context` 自身即 brand 类型，RFC-0009 §9
  Q-Brand）；跨 context 资源误用裁决（RX3006）复用 RXS-0074，原样生效。
- **方法签名编译器已知**（RXS-0190 口径）：元数/类型/方法名不符 → RX2003/RX2001/RX2004 复用（不另立
  新码）。方法名终形随实现 PR 在已知签名纪律内定案（RFC-0009 §4.7 先例），语义面以本章条款为准。

**Dynamic Semantics**：

- 图建面（资源创建 + pass 声明）为 host 侧记账；`execute()`（或显式 `seal()`）触发装配核验（RXS-0237）
  + 状态推导（RXS-0238）+ 单 queue 顺序提交（RXS-0239/0240 执行器）。affine 句柄 drop 无附加运行期
  语义（图本体销毁经 `rxrt_graph_destroy`，RXS-0241）。

**Implementation Requirements**：

- 句柄为编译器合成布局（`handle: u64` + brand 幽灵参数，`is_gpu_handle` 单 i64 标量）；方法集经 typeck
  编译器已知签名分支表达（[`typeck`](../src/rurixc/src/typeck.rs) `gpu_host_method` / `check_gpu_method`，
  先例 TextureTable 分支）；`Graph::create` 关联构造镜像 `Context::create` 解析锚点
  （[`resolve`](../src/rurixc/src/resolve.rs)）；降级为 `rxrt_graph_*` 字面符号
  （[`mir_build`](../src/rurixc/src/mir_build.rs)，镜像 `rxrt_table_*`）。

> 测试锚定：conformance/host_orch/accept/graph_deferred_three_pass（0 诊断，deferred 三 pass 图声明
> lowering 落 `rxrt_graph_*`）+ reject/graph_in_kernel（kernel 体内宿主构造/方法 → RX3015）+
> `host_orch_corpus` 单测。

### RXS-0237 声明序 = 提交序与图合法性

**Legality**（装配期确定性核验，strict-only）——`execute()`（或显式 `seal()`）时全量判定，违例 → 6xxx
strict 拒（RFC-0006 §9 Q-Err「装配期可预测错误续用 6xxx」先例；新码 ×2 见 §3 码表 RX6029/RX6030）：

- **声明全序**：声明序即提交全序，不做重排（pass 重排 / 依赖驱动调度 out_of_scope，§8）；资源依赖边逆序
  即后向边 = 环——声明序构造下经典环不可构造，**条款措辞锁 use-before-write 可达形态**：消费读
  （`reads` / `readback` 源 / present 终端）的资源若无先前 pass 的写（`writes_rt` / `writes_depth` /
  `reads_writes_uav` / readback 目的），即「读未写」→ **RX6029**。
- **写写冲突**：同 pass 对同资源重复声明写 / 同资源同 pass 既 `reads` 又 `writes_rt`（= feedback）→ 拒
  （**每资源每 pass 至多一条声明**；`reads_writes_uav` 为唯一合法读写合并，以单条 `UavReadWrite`
  表达）→ **RX6029**。跨 pass 顺序重写（ping-pong）合法（由 D4 全序覆盖）。
- **声明-反射双向精确相等**：pass 的管线绑定反射面（RXS-0163~0166 单一事实源）与声明集**双向精确相等**
  核验（Q-G-OverDeclare），**相等域 = 首期封闭枚举资源面**（§4.0-3）；漏声明或声明未用 → **RX6030**。
- **生命周期误用**：空图 `execute` / `seal` 后追加 pass / 重复 `execute` → **RX6029**。

**Implementation Requirements**：

- 图合法性核验为 host 侧 `graph.rs` 本体（纯函数 `seal()`；[`graph.rs`](../src/rurix-rt/src/graph.rs)），
  与状态推导（RXS-0238）同居；错误携 `rx_code()`（RX6029/RX6030）→ error_codes.json 单一事实源
  （message_key `runtime.graph_structure` / `runtime.graph_reflection_mismatch`，en/zh 成对）。

> 测试锚定：`graph.rs` reject 单测 ×4 族（读未写 / 写写冲突 / 读写同 pass / 生命周期误用 → RX6029）+
> 声明-反射失配（RX6030）+ accept（反射精确相等）。**RX6029/RX6030 为 host 装配期错误码**（镜像 uc04
> RXS-0169/RX6021 host 核验器锚定先例，非 rurixc 编译诊断 conformance）。

### RXS-0238 自动资源状态推导状态机

**Dynamic Semantics**（rurix-rt `graph.rs`，纯 host safe；D3）：

- **输入** = 已 seal 图；**输出** = 确定性 barrier 计划：逐资源状态机（初态按创建类别——color/depth
  attachment 创建即处写态、buffer 创建即 `COMMON`）沿声明全序推进，下一使用点所需状态 ≠ 当前状态即在
  该 pass 边界产出一条转换。**推导为纯函数**：同图 → 逐字节相同计划（golden 可锚）；模块零 unsafe、零
  后端调用、无 GPU 依赖，单测恒跑。
- **资源类别分立（状态机诚实性）**：三种 barrier 形态——`Transition`（image/attachment：D3D12 states /
  Vulkan layout 迁移）、`BufferSync`（buffer：无 layout，仅 stage+access）、`UavSync`（同资源相邻 UAV
  写-写/写-读：D3D12 UAV barrier / Vulkan memory barrier，状态不变亦发）。不把 buffer/UAV 硬套 image
  迁移模型。
- **双后端映射同源**：`AccessKind → D3D12_RESOURCE_STATES` 与 `AccessKind → (VkImageLayout,
  VkPipelineStageFlags, VkAccessFlags)` 两张映射表同居 `graph.rs` 一处（P-11 单一事实源）；映射锚点
  `ColorAttachmentWrite → RENDER_TARGET / COLOR_ATTACHMENT_OPTIMAL`、`ShaderRead →
  PIXEL_SHADER_RESOURCE / SHADER_READ_ONLY_OPTIMAL`（RXS-0176 跨 pass RT→SRV 既有裁决的推广）、
  `CopySrcReadback → COPY_SOURCE / TRANSFER_SRC_OPTIMAL`、`UavReadWrite → UNORDERED_ACCESS / GENERAL`、
  `PresentHandoff → PRESENT / PRESENT_SRC_KHR`。执行器只**逐字重放**，禁后端侧二次推导或语义重映射
  （含 shim C++ 侧）。

**Implementation Requirements**：

- Vulkan 数值常量（layout / stage / access）与 [`vk.rs`](../src/rurix-rt/src/vk.rs) 逐值一致（执行器逐字
  重放的单一事实源）；推导计划 [`PlannedBarrier`] 同时携 D3D12 视图（前/后态）与 Vulkan 视图
  （old/new layout + src/dst stage + src/dst access），供两执行器分别消费。

> 测试锚定：`graph.rs` 推导计划 golden 单测（deferred 三 pass 图恰 5 条 barrier 逐条锚）+ 同图双跑逐字节
> 等值断言 + depth（`DEPTH_WRITE→PSR`）/ UAV（相邻 UavSync）独立路由单测。

### RXS-0239 🔒 pass 边界 happens-before 语义本体

> 🔒 禁区子节（RFC-0013 §4.D4，本 RFC 全文批准对象）：RD-020 明记「barrier 并发/可见性/内存序语义
> 本体另归 Full RFC（硬规则 5）」——该本体在此落笔。

**Dynamic Semantics**（承诺面，**且仅此面**）：

- 单 queue；声明序 = 提交序 = pass 粒度完成序。对任意 i < j，pass i 的全部 device 内存效应（RT/depth/UAV
  写、copy 写）在 pass j 的任何访问**之前发生且可见**——**每个 pass 边界是全序同步点**。RAW / WAW / WAR
  三类跨 pass 冲突全部被该全序裁定，可见性保证仅在 **pass 粒度**给出。
- **pass 内不承诺、不触碰**：pass 内跨线程可见性/内存序仍由既有条款独占管辖——RXS-0079（shared/barrier
  一致性）、RXS-0080（scoped atomics）、RXS-0068（barrier uniform 可达性）。本条不新增、不削弱、不重述
  任何 pass 内语义；pass 内 RT 自读（feedback loop）不可表达（RXS-0236 封闭枚举 + 写写冲突拒）。

**Implementation Requirements**：

- 该保证由 RXS-0238 推导计划兑现；首期取**最保守 sound 同步掩码**（Vulkan：pass 边界 barrier 以覆盖
  生产/消费全阶段的保守 stage/access 掩码录制；D3D12：legacy ResourceBarrier 语义自含同步）。掩码窄化
  属性能优化，**不属本条承诺**（§8）。条款措辞与执行器实参须逐字段可对照。

**严禁 UB**：本面**无 UB 节、无实现自由竞争窗口**。承诺面之外的一切构造走编译期诊断（复用）或装配期
6xxx strict 拒（RXS-0237）；运行期后端 API 失败走确定性诊断 + 终止 + poisoned 传播（RXS-0193/0194），
无静默降级（P-01）。多 queue / async compute / split barrier 不在承诺面（§8），其不存在性即由本条全序
措辞封死——条款不为未来扩张预留弱化措辞。

> 测试锚定：**D6 互证恒跑单测**（[`uc04-demo` d6_crosscheck](../src/uc04-demo/tests/d6_crosscheck.rs)：
> deferred 三 pass 图经 `graph.rs` 推导的 barrier 集 == uc04 `barrier::plan_barriers` RXS-0169 手动锚点集，
> 集合相等双向断言）+ 步骤 65 device 数据流红绿（漏声明 read → strict 拒 RED）。

### RXS-0240 双后端 barrier 映射与执行器语义（`run_graph`）

**Dynamic Semantics**（双执行器消费同一推导产物）：

- **(a) D3D12 执行器**（uc04 shim，gate `d3d12-runtime`）：推导产物以 pass 数组 + 逐边界 barrier 数组经
  shim ABI **数值透传下发**（枚举数值即 D3D12 原生常量，C++ 侧零状态映射逻辑，防第二事实源）；执行 =
  逐 pass set RT/DSV → draw → 重放该边界 barrier 数组。与步骤 48 offscreen 路径同判据（G-G3-5）。
- **(b) Vulkan 执行器**（rurix-rt `vk.rs`，gate `vulkan`）：新入口 **`run_graph`**（命名律 §4.0-5）多
  pass command buffer 录制（逐 pass render pass begin/end + 边界 `vkCmdPipelineBarrier`，layout/stage/access
  全取自 RXS-0238 同源表），承 RXS-0207/RXS-0210 执行语义地基；现 `run_graphics_offscreen` /
  `run_graphics_present` 手写定点 barrier 路径 **0-byte 保留**。新 FFI unsafe 逐块 `// SAFETY:` 折叠进
  U27 扩注（graphics FFI 边界内，0 新号）。
- **(c) present 终端 pass 胶水**：终端 pass 为 `PresentHandoff` 时，graph 只做 pre-present 状态迁移
  （推导产物之一）并把 backbuffer 交回 §4.A present 会话（C++ shim present 链，SC-5），**不**吸收 present
  会话生命周期、不建第二个 present 状态机；窗/泵/交换链维持 C++ shim（**D-130 0-byte 红线**）。

**Implementation Requirements**：

- 执行器**逐字重放** `graph.rs` 推导产物，禁二次推导（含 shim C++ 侧数值透传）；既有手写 barrier 路
  零回归（步骤 48 offscreen 入口 / vk.rs 既有入口 0-byte）。D3D12 shim ABI 增量走**新增独立入口**
  （pass/barrier 数组下发，聚合版顺延，§6.3）；shim C++ 改动大时诚实标注留后续（Vulkan `run_graph` +
  host 推导 + D6 互证为本面核心，device 段真跑归主循环活驱动）。

> 测试锚定：`graph.rs` 映射一致性单测（每 AccessKind 的 D3D12/Vulkan 映射单一事实源 + u32 tag round-trip）+
> 步骤 65（uc04 迁 Graph 重跑步骤 48 同判据 + Vulkan 同图）。

### RXS-0241 `rxrt_graph_*` C ABI 与手动复核门（🔒 FFI 延伸）

> 🔒 FFI 延伸（RFC-0013 §4.D7）：`rxrt_graph_*` 为 RXS-0194 符号面的**只追加**延伸（含义冻结、布局不
> 冻结为语言 ABI，RXS-0180 L3 口径）。

**Syntax**（cabi 符号面，粒度 Q-G-CabiGranularity）：

```
rxrt_graph_create(ctx: u64) -> u64                        // 图句柄；handle-0 = 失败
rxrt_graph_resource(g: u64, class: u32) -> u64            // 资源句柄；class 0/1/2/3 = color/depth/uav/readback
rxrt_graph_pass(g: u64) -> u64                            // pass 句柄（声明序 = 提交序）
rxrt_graph_declare(pass: u64, resource: u64, access: u32) -> i32   // access = AccessKind u32 tag（0..=6）
rxrt_graph_readback(g: u64, src: u64) -> i32              // 源 CopySrc + 自动 readback 目的 buffer
rxrt_graph_execute(g: u64) -> i32                         // 装配核验 + 状态推导（负值 → 终止）
rxrt_graph_destroy(g: u64)                                // affine 消费式，清表
```

**Dynamic Semantics / Implementation Requirements**：

- u64 句柄表 / handle-0 = 失败 / `diag` 失败行格式 / poisoned 传播为跨后端不变式**原样生效**；既有
  `rxrt_* / rxp_* / rxio_* / rxrt_table_*` 符号含义**零漂移**（`rxrt_launch` 及既有符号面字节不变，
  RXS-0194）。`GraphResource<C>` / `PassBuilder<C>` 为 u64 affine 句柄（cabi 映射至图内资源/pass 下标）；
  `rxrt_graph_declare` 跨 graph 误用 / 未知句柄 / 未知 access tag → 确定性 `diag` + `RXRT_FAIL`
  （编译器注入检查 → `rxrt_trap` 终止，RXS-0193）。整图序列化单符号下发否决（diag 无法定位违例 pass，
  RFC §7-6）。装配核验（RX6029/RX6030）与状态推导本体归 `graph.rs`（P-11 单一事实源），cabi 面纯 safe。
- **手动复核门（D6，🔒 互证金标准）**：uc04 `barrier::plan_barriers`（RXS-0169 / RX6021，main 已冻结，
  先于 `graph.rs` 存在）**永续保留、条款 0-byte 不动**；`graph.rs` 推导**禁止 import barrier.rs 任何推导
  逻辑**（oracle 独立性）。host 互证金标准（G-G3-5，恒跑纯 host，无 GPU）：uc04 三 pass 图
  （`deferred::plan_deferred_passes` / RXS-0168 结构）经 `graph.rs` 推导的 barrier 集，与 RXS-0169
  `required_transitions` 手动锚点集**集合相等断言（双向）**。

> 测试锚定：`rurix-rt-cabi` cabi 单测（`rxrt_graph_*` 符号面 + handle-0/未知句柄失败路 + 增量建面 →
> execute 装配核验）+ D6 互证 set-equality 恒跑单测（[`uc04-demo` d6_crosscheck](../src/uc04-demo/tests/d6_crosscheck.rs)）。

## 3. 错误码引用汇总

| 码 | 段 | 语义 | 条款 |
|---|---|---|---|
| RX3015 | 3xxx 着色 | render graph 宿主 API 出现在 `kernel` / `device fn` 体内（宿主 API 着色违例，与 RX3001 同点位，复用） | RXS-0236（承 RXS-0189） |
| RX2001 / RX2003 / RX2004 | 2xxx 类型 | 方法实参类型 / 元数 / 方法名不符（编译器已知签名核验，复用，零新码） | RXS-0236（RXS-0190 口径） |
| RX3006 | 3xxx 着色 | 跨 context brand 误用（复用 RXS-0074，零新码） | RXS-0236 |
| RX4001 / RX4003 | 4xxx 借用 | affine 句柄 move 后再用 / 经引用消费（复用 RXS-0054/0057~0061，零新借用码） | RXS-0236 |
| **RX6029** | 6xxx 装配 | **图结构违例族**（装配期 strict：环/读未写 use-before-write 可达形态 / 写写或读写冲突 / 生命周期误用；graph.rs 装配核验） | RXS-0237 |
| **RX6030** | 6xxx 装配 | **声明-反射失配族**（装配期 strict：声明集 ↔ 绑定反射面双向精确相等核验失败，漏声明/声明未用；相等域 = 首期封闭枚举资源面） | RXS-0237 |

新码 ×2（RX6029 / RX6030，en/zh 成对，message_key `runtime.graph_structure` /
`runtime.graph_reflection_mismatch`）；段内分配制递增不复用、含义冻结（10 §6）。**运行期后端失败不占
RX 段位**（RXS-0193 口径，确定性诊断 + `rxrt_trap` 终止）。

## 4. 升档 / 禁区留痕

- **🔒 §4.D4（RXS-0239）pass 边界 happens-before 语义本体**：RFC-0013 全文批准对象（RD-020 兑现）。本面
  **无 UB 节**——承诺面外走编译期诊断 / 装配期 6xxx strict 拒 / 运行期确定性失败 + 终止 + poisoned；
  多 queue / async compute / split barrier / 掩码最小化不在承诺面（§8，RD-034+），其不存在性由全序措辞
  封死，不为未来扩张预留弱化措辞。掩码窄化属性能优化，与 D4 措辞联动修订方可放开。
- **🔒 §4.D7（RXS-0241）`rxrt_graph_*` FFI 面**：符号面含义冻结、布局不冻结为语言 ABI（RXS-0180 L3）；
  新 unsafe 折叠 U27 扩注（0 新号），cabi 面本体纯 safe。
- **D6 手动复核门（RXS-0169 / RX6021）0-byte 不动**：uc04 `barrier.rs` `plan_barriers` /
  `required_transitions` 为独立复核 oracle，`graph.rs` 禁 import 其推导逻辑（互证独立性硬约束）。
- **首期不可表达面（§4.0-3）**：bindless 表 / storage image 资源 / mesh·RT pass kind 出封闭枚举，显式
  登记 RD-034+ 非静默；storage image barrier 首期走 RXS-0169 手动路（在 §4.D 自动推导域之外，G-RED-2）。
- **present 终端胶水（D5c，SC-5）**：graph 只做 pre-present 状态迁移，不吸收 present 会话生命周期；窗/泵/
  交换链维持 C++ shim（D-130 0-byte 红线），RXS-0197 present typestate 维持不动。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-07-19 | 新建 spec/render_graph.md（G3.5，PR-G1 条款先行）：带编号条款体 `### RXS-0236 ~ ### RXS-0241`（FLS 体例，按需分 Syntax/Legality/Dynamic Semantics/Implementation Requirements，**严禁 UB 节**；🔒 禁区 RXS-0239/RXS-0241 全文批准）——RXS-0236 Graph/Pass 宿主库类型面与访问声明集（Graph/GraphResource/PassBuilder lang items 非 Copy affine，五类访问封闭枚举 + PresentHandoff，kernel 体内 → RX3015 承 RXS-0189）/ RXS-0237 声明序=提交序与图合法性（装配期 strict：环/读未写/写写冲突/生命周期 → RX6029，声明-反射双向精确相等 → RX6030）/ RXS-0238 自动资源状态推导状态机（纯函数，三 barrier 形态 Transition/BufferSync/UavSync，AccessKind 双后端映射单一事实源）/ RXS-0239 🔒 pass 边界 happens-before 语义本体（单 queue 全序同步点，pass 粒度可见性，严禁 UB）/ RXS-0240 双后端 barrier 映射与执行器语义（run_graph；D3D12 数值透传 / Vulkan vkCmdPipelineBarrier；present 胶水交 §4.A 链 D-130 0-byte）/ RXS-0241 rxrt_graph_* C ABI 只追加 + 手动复核门（D6 互证金标准，graph.rs 禁 import barrier.rs）。配套 host 侧 safe 推导 `src/rurix-rt/src/graph.rs`（always-on 零 unsafe 零后端调用）+ cabi `rxrt_graph_*`（rurix-rt-cabi）+ 前端 Graph/PassBuilder/GraphResource lang items（resolve/typeck/mir_build/hir）+ D6 互证 `src/uc04-demo/tests/d6_crosscheck.rs`。错误码 RX6029/RX6030（en/zh 成对，bilingual 99→101）；每条 ≥1 `//@ spec` 测试锚定随实现 commit 同 PR 落，trace_matrix 全锚定；stable 快照同 PR 重 bless（RXS-0180 L2）。承 [RFC-0013](../rfcs/0013-industrial-rendering.md)（Agent Approved 2026-07-18，§4.D 全文批准，RD-020 兑现）。 | **Full RFC**（RFC-0013 / §4.D / PR-G1） |
