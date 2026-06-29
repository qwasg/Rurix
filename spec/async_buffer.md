# Rurix 语言规范 — 流序分配 `AsyncBuffer<'stream,T>` 类型契约语义面（G1.2 起）

> 条款:RXS-0144 起续号预留(G1.2 流序分配语义面:`AsyncBuffer<'stream,T>` affine 所有权与流序分配/释放 RAII / 分配未完成访问被 stream 序排除 / 释放后访问编译期生命周期错误 / 跨 stream 经 `share_with(other,event)` 显式时序边 / 三类流序分配生命周期错误编译期拦截判据)。**复用 M8.3 RXS-0130~0134 affine 资源 + `InFlight` 流序分配类型化先例,新增仅补流序分配器(`cuMemAllocAsync`)缺口**。体例见 [README.md](README.md)。
> 依据:06 §5.4(`AsyncBuffer<'stream,T>` 三规则设计预留,D-122);08 §2.2(运行时内存分配策略:G1 = stream-ordered allocator `cuMemAllocAsync` + `CUmemoryPool`;VMM G2,D-232);13 D-122(流序分配类型推迟 G1)/ D-232(运行时内存分配策略);05 §1(device⊂host 安全子集:所有权/借用/生命周期规则在 device 路径同义延拓)+ 05 §(affine 资源所有权:GPU 资源 move-only、单一所有权、RAII 销毁纪律);08 §1/§2(`rurix-rt` 运行时对象,D-230~D-234)。授权:[../milestones/g1/G1_CONTRACT.md](../milestones/g1/G1_CONTRACT.md)(`in_scope: async_buffer` / `spec_g1_clauses`,D-G1-2,G-G1-2)+ [../milestones/g1/G1_PLAN.md](../milestones/g1/G1_PLAN.md) §2 G1.2 第 1~2 项 + [../rfcs/mini-0001-async-buffer.md](../rfcs/mini-0001-async-buffer.md)(MR-0001,owner 2026-06-19 批准)。
> 档位:**Mini-RFC**(MR-0001;owner 2026-06-19 经 AskUserQuestion 批准)。本文是对 06 §5.4 / 08 §2.2 已锁定决策(`AsyncBuffer<'stream,T>` 时序契约类型化 / stream-ordered allocator)的条款化;**「AsyncBuffer API 具体形态 + `share_with` 时序边」为 G1 执行期新决策面**(G1_CONTRACT YAML 头第 8 行明列),带档位标记 **Mini**(纯类型级 typestate + 生成式 `'stream` brand + `#[must_use]`,**不改 rustc/MIR 借用检查器、不触内存模型映射**,镜像 M8.3 RXS-0132 `InFlight` 先例;`cuMemAllocAsync`/`cuMemFreeAsync`/`cuMemPool*` 为稳定 CUDA Driver API 薄层绑定,与 M8.3 `cuEvent*`/`cuMemcpy*Async` 同类,非新 FFI ABI 契约)。**agent 自主判档**,判档争议向上取严。任何偏离已锁定决策、或触及 **MIR 借用检查扩展 / 内存模型映射(06 §4.2)/ FFI ABI / 安全包络**(AGENTS 硬规则 5 / 10 §7.5 禁区)的条款,必须**停手标注「需人工升 Full RFC」**,不在本文件自行落笔(向上取严)。**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5):流序分配资源所有权与生命周期以 **affine 所有权 + 确定性诊断(rustc 编译期拦截)** 定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.29 pipeline.md / v1.32 interop_d3d12.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0144 起)与每条 ≥1 测试锚定随 G1.2 实现 PR(步骤 42)同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **G1.2 流序分配类型契约语义面**(D-G1-2)。覆盖语义面:

- **`AsyncBuffer<'stream,T>` affine 所有权与流序分配/释放 RAII**:运行时流序分配器(`cuMemAllocAsync` 入 `CUmemoryPool` + `cuMemFreeAsync` 流序释放,Driver API 薄层,D-232)分配的设备缓冲为 **move-only**(非 `Copy` / 非 `Clone`)、单一所有权;携不变 `'stream` brand 编码其所属 stream(产出 / 排队它的 stream);`Drop` = `cuMemFreeAsync` 入所属 stream(流序销毁,不引入隐式主机同步,P-05 薄层)。复用 M8.3 affine 资源纪律(RXS-0130),新增仅补流序分配器缺口。
- **分配未完成访问被 stream 序排除**:流序分配在 stream 上排队;同 `'stream` 上的后续设备操作(launch / copy)经 stream 序天然排在分配之后——「分配未完成访问」由 stream 序结构性排除。`AsyncBuffer` 在途态**无 host 读接口**(镜像 M8.3 RXS-0132 `InFlight`);取回 host 数据须经显式同步(消费 / 重 brand),跳过同步即无可读句柄。
- **释放后访问 = 编译期生命周期错误**:`AsyncBuffer` 单一所有权 + `'stream` 生命周期不晚于其 stream——`move`(消费 / 释放)后再用 → 编译期 use-after-free(rustc `E0382`);试 `.clone()` → `E0599`。释放(`cuMemFreeAsync`)经 `Drop` 单点发生,不双重释放。
- **跨 stream 使用经 `share_with(other,event)` 显式时序边**:`AsyncBuffer<'stream,T>` 跨 stream 使用**必须** `buf.share_with(other_stream, event)` 显式建立时序边(`cuEventRecord` + `cuStreamWaitEvent`),**消费** self 并重 brand 到 `'other`;缺 `share_with` 直接在他 stream 上使用 → 编译期类型错误(无重 brand 句柄 / brand 失配,`E0599` / `E0277` / lifetime),而非运行期数据竞争 / UB(CUDA.jl #780 事故类)。复用 M8.3 RXS-0131/0132 event 同步 + 流序分配类型化骨架。
- **三类流序分配生命周期错误编译期拦截判据**:预设流序分配生命周期错误类别——**分配未完成访问**(stream 序排除 + 在途态无读接口)/ **释放后访问**(affine move 后再用)/ **跨 stream 未同步访问**(缺 `share_with`)——**100% 编译期拦截**(rustc affine move / 生命周期 brand / 方法存在性原生诊断),G-G1-2 验收判据;reject 类别覆盖以 compile-fail 样例(应拦截却放行即红)核对,device 路径纳入 Compute Sanitizer racecheck+memcheck nightly(CUDA.jl #780 事故类永久回归项)。

全部流序分配编排**复用 `src/rurix-rt` 运行时对象**(SharedContext/SharedStream/SharedEvent/DeviceBox,RXS-0130~0133)+ 本里程碑补缺的流序分配器(`cuMemAllocAsync`/`cuMemFreeAsync`/`cuMemPool*`);device 分发维持 **PTX-only**(07 §7);流序分配资源所有权 / 生命周期以 **affine 所有权 + 确定性诊断(rustc 编译期拦截)** 定义,**不以 UB 表述**(§4)。**资源生命周期错误类别由 Rust 类型系统原生拦截,本里程碑大概率不新增 RX 错误码**(§3;rustc 编译期诊断而非 RX#### 段位码,「如需」按需——本轮判定为不需要)。

**编号区间**:本文件条款自 **RXS-0144** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0143 @ [interop_d3d12.md](interop_d3d12.md))。本轮计划落地 **RXS-0144 ~ RXS-0148**(`AsyncBuffer` affine 所有权与流序分配/释放 RAII / 分配未完成访问被 stream 序排除 / 释放后访问编译期生命周期错误 / 跨 stream 经 `share_with` 显式时序边 / 三类流序分配生命周期错误编译期拦截判据),每条 ≥1 测试锚定(`//@ spec: RXS-####`,`src/rurix-rt` crate 单测)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。Legality 违例由 **rustc 原生编译期诊断**拦截(affine move / 生命周期 brand / 方法存在性),**不引用新 RX 段位码**(§3)。流序分配资源所有权 / 生命周期以 **affine 所有权 + 确定性诊断**定义,不以 UB 表述。条款体复用 `src/rurix-rt` 既有运行时对象(SharedContext/SharedStream/SharedEvent/DeviceBox,RXS-0130~0133)+ 本里程碑补缺的流序分配器(`cuMemAllocAsync`/`cuMemFreeAsync`),镜像 RXS-0132 `InFlight` 流序分配类型化先例。

### RXS-0144 `AsyncBuffer<'stream,T>` affine 所有权与流序分配/释放 RAII

**Syntax**(运行时流序分配 API,`src/rurix-rt`):

```
SharedStream::alloc_async<T>(len) -> AsyncBuffer<'stream, T>   // cuMemAllocAsync 入 ordered pool
AsyncBuffer<'stream, T>                                        // #[must_use];in-flight,无读接口
AsyncReady<'stream, T>                                         // share_with 重 brand 后:可读/写/取址
```

**Legality**:

- `AsyncBuffer` / `AsyncReady` 为 **move-only**(非 `Copy` / 非 `Clone`,单一所有权):`move` 后再用 → `E0382`(use-after-free 类别,RXS-0146/0148);试 `.clone()` → `E0599`。违例由 rustc 拦截,不设 RX 段位码。
- 携不变 `'stream` brand(借用产出 stream,`PhantomData<(&'stream SharedStream, T)>`),生命周期不晚于其 stream;`!Send`(持裸 stream 句柄,线程内使用,对齐 `SharedStream`)。

**Dynamic Semantics**:

- **流序分配**:`cuMemAllocAsync` 把 `len` 个 `T` 分配入产出 stream 的 ordered memory pool(默认 = 设备默认 `CUmemoryPool`,D-232);分配在该 stream 排队,同 stream 后续操作经 stream 序排在其后(RXS-0145)。薄层不引入隐式同步 / 自动池化(P-05)。
- **流序释放**:`Drop` = `cuMemFreeAsync` 入所属 stream(流序释放回 pool);释放责任由内部 RAII 载体 `PoolAlloc`(独占 `Drop`)承载,`share_with` 经 `move` 单点转移(wrapper 无 `Drop`),不双重释放(RXS-0146)。

**Implementation Requirements**:

- 流序分配器 FFI(`cuMemAllocAsync` / `cuMemFreeAsync`)落 `src/rurix-rt` 既有豁免 + 每 unsafe 块 `// SAFETY:` + `unsafe-audit/rurix-rt.md` 注册(U19/U20,AGENTS 硬规则 9);safe 类型层对上全 safe。符号为 **Option 字段非致命解析**(CUDA 11.2+;老驱动缺失 → `DriverUnavailable`,核心 CUDA 不受影响)。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`async_buffer_alloc_and_pool_raii`:alloc_async / AsyncBuffer / AsyncReady / PoolAlloc 类型与 RAII 面)。

### RXS-0145 分配未完成访问被 stream 序排除(in-flight 无读接口)

**Legality**:

- in-flight `AsyncBuffer`(`#[must_use]`)**无 `device_ptr` / `copy_to_host` / `copy_from_host` 读写接口**;读 / 写 / 取址接口**仅 `AsyncReady` 提供**(经 `share_with` 同步重 brand 后)。直接对 `AsyncBuffer` 取址 / 读 → 方法不存在 `E0599`(RXS-0148),而非运行期未就绪访问 / UB。

**Dynamic Semantics**:

- 同 `'stream` 后续设备操作经 stream 序排在分配之后,「分配未完成访问」由 stream 序**结构性排除**;取回可读 `AsyncReady` 须经 `share_with`(显式时序边,RXS-0147)同步重 brand——跳过同步即无可读句柄。镜像 RXS-0132 `InFlight` 先例。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`async_buffer_inflight_no_read_interface`:读/写/取址接口仅 AsyncReady 提供)。

### RXS-0146 释放后访问 = 编译期生命周期错误

**Legality**:

- `AsyncBuffer` / `AsyncReady` 单一所有权(非 `Copy`):`move`(消费 / `share_with`)后再用 → 编译期 `E0382`(use-after-free 类别,RXS-0148);非 `Clone`,试复制句柄 → `E0599`(double-free 类别)。`'stream` 生命周期不晚于其 stream(brand 借用)。

**Dynamic Semantics**:

- 释放(`cuMemFreeAsync`)由内部 `PoolAlloc` 的单点 `Drop` 发生;`share_with` 经 `move` 转移 `PoolAlloc`(释放责任随之转移到目标 stream),不双重释放(RXS-0144)。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`async_buffer_affine_move_only_use_after_free`:AsyncBuffer/AsyncReady 单一所有权 + PoolAlloc 单点释放)。

### RXS-0147 跨 stream 经 `share_with(other,event)` 显式时序边

**Syntax**(typestate 转移,`src/rurix-rt`):

```
AsyncBuffer::share_with<'o>(self, other: &'o SharedStream, event: &SharedEvent) -> AsyncReady<'o, T>
AsyncReady::share_with<'o>(self, other: &'o SharedStream, event: &SharedEvent) -> AsyncReady<'o, T>  // 再跨 stream
```

**Legality**:

- 跨 stream 使用**必经** `share_with`:消费 self,在产出(所属)stream `record` `event`、在 `other` stream `wait_event` 建立流序依赖,**重 brand** 到 `'other` → 可在 `other` 上读 / 写 / launch 的 `AsyncReady`。缺 `share_with` 直接跨 stream 读 `AsyncBuffer`(无读接口)→ `E0599`(RXS-0145/0148),而非运行期数据竞争 / use-after-free(CUDA.jl #780 事故类)。

**Dynamic Semantics**:

- `share_with` 内插 `cuEventRecord`(产出 stream)+ `cuStreamWaitEvent`(`other`);其后 `AsyncReady` 在 `other` 上读 / launch 合法,流序释放改到 `other`。复用 RXS-0131 event 记录·等待跨 stream 同步骨架 + RXS-0132 重 brand 类型化。`InFlight` 跨 stream 转移与 `acquire` 重 brand 为同源先例。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`async_buffer_share_with_cross_stream_edge`:share_with 时序边 + 端到端 fn 面)。

### RXS-0148 三类流序分配生命周期错误编译期拦截判据

**Legality**(G-G1-2 验收判据:**三类流序分配生命周期错误 100% 编译期拦截**):

| 类别 | 触发 | 编译期拦截机制 | rustc 诊断 |
|---|---|---|---|
| 分配未完成访问 | in-flight `AsyncBuffer` 取址 / 读 | in-flight 无 `device_ptr` / 读接口(RXS-0145) | `E0599`(方法不存在) |
| 释放后访问 | `move`(`share_with`)后再用 / 试 `.clone()` | 单一所有权(非 `Copy` / 非 `Clone`,RXS-0146) | `E0382` / `E0599` |
| 跨 stream 未同步访问 | 缺 `share_with` 跨 stream 读 | `AsyncBuffer` 无读接口(RXS-0147) | `E0599`(方法不存在) |

**Dynamic Semantics**:

- 三类**全部于编译期拦截**,无运行期 UB / use-after-free / 数据竞争路径;**不新增 RX 错误码**(§3,rustc 原生诊断)。reject 类别覆盖以 compile-fail 样例核对(`src/rurix-rt/compile-fail/async_buffer_*.rs`,冒烟步骤 42 断言每个应失败者均被 rustc 拒绝);**真实红绿**:放行任一违例(使应拦截者编译通过)→ 红;复原 → 绿,run URL 归档(反 YAML-only)。

**Implementation Requirements**:

- 拦截判据以 **affine 类型 + rustc 诊断**表达,**严禁 UB 节**(10 §7.5);流序分配 device 路径纳入既有 Compute Sanitizer racecheck+memcheck nightly(运行期无 use-after-free 佐证,CUDA.jl #780 事故类**永久回归项**,CI_GATES §4)。三 stream 流序分配端到端真跑佐证见 `three_stream_async_pipeline`(冒烟步骤 42 device 段,`g1.counter.async_buffer_pipeline`)。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`async_buffer_lifecycle_classes_compile_intercepted`:三类编译期拦截判据 + reject 样例引用)。

## 3. 错误码引用汇总

> **本里程碑大概率不新增 RX 错误码**(对齐 M8.3 RXS-0134 先例)。G1.2 预设流序分配生命周期错误类别(分配未完成访问 / 释放后访问 / 跨 stream 未同步)由 **Rust 类型系统原生编译期拦截**(affine move → `E0382`、在途态无读接口 / 无 `Clone` → `E0599`、跨 stream brand 失配 → `E0277`·lifetime 等 rustc 诊断),**而非 RX#### 段位码**;`registry/error_codes.json` 与 `src/rurixc/src/messages/*.messages` **本里程碑不动**(零追加)。
>
> 运行期失败(`cuMemAllocAsync` / `cuMemFreeAsync` / pool 创建的驱动错误)沿用 `rurix-rt` 既有 `CudaError` 错误值面 + poisoned 状态机(RXS-0077),不新增 RX 段位码。
>
> 若实现期发现某流序分配生命周期类别**无法以纯 Rust 类型拦截**而确需编译器侧 RX 诊断 / 运行期段位码(如 7xxx RX7020+),则**停手标注「需升档」**(§4),不在本文件自行预造错误码。

## 4. 升档 / 禁区留痕

- **AsyncBuffer API 形态 + `share_with` 时序边(新决策面,档位 Mini-RFC / MR-0001)**:G1_CONTRACT YAML 头第 8 行明列「AsyncBuffer API 具体形态」为 G1 执行期新决策面;owner 2026-06-19 经 AskUserQuestion 裁为 **Mini-RFC**(MR-0001)。口径:纯类型级 typestate + 生成式 `'stream` brand + `#[must_use]`,**不改 rustc/MIR 借用检查器、不触内存模型映射**;`src/rurix-rt` 的 unsafe 边界维持 `undocumented_unsafe_blocks=deny`,流序分配器 FFI(`cuMemAllocAsync`/`cuMemFreeAsync`/`cuMemPool*`)落 `src/rurix-rt` 既有豁免 + 每 unsafe 块 `// SAFETY:` + `unsafe-audit/rurix-rt.md` 注册条目(U19+,AGENTS 硬规则 9),safe 类型层对上全 safe。**agent 自主判档**,判档争议向上取严。
- **流序分配生命周期错误类别 100% 编译期拦截(G-G1-2 验收判据)**:分配未完成访问 / 释放后访问 / 跨 stream 未同步三类以 **affine 类型 + rustc 原生编译期诊断**拦截(§3);reject 类别覆盖以 compile-fail 样例核对(应拦截却放行即红,反 YAML-only,run URL 归档);device 路径纳入 Compute Sanitizer racecheck+memcheck nightly(运行期无 use-after-free 佐证,CUDA.jl #780 事故类永久回归项,CI_GATES §4)。**不以 UB 表述**(10 §7.5)。
- **MIR 借用检查扩展 / 内存模型映射(06 §4.2)/ FFI ABI / 安全包络(AGENTS 硬规则 5 / 10 §7.5 禁区)**:若实现期发现流序分配三规则**无法以纯类型拦截**而确需 MIR 借用检查器扩展(stream-region 分析)/ 内存模型映射 / 新 FFI ABI 契约 / 安全包络扩展,则**停手标注「需人工升 Full RFC」**,不在本文件自行落笔(向上取严,MR-0001 §3 升档守卫)。
- **VMM / 多 GPU(G2 评估,08 §2.2 / D-232)**:本里程碑流序分配仅 `cuMemAllocAsync` + `CUmemoryPool`;VMM(`cuMemAddressReserve` 族)/ 多 GPU / NVLink / MIG → G2(A-06 单机单 GPU 语义边界);触及即停下标注「需升档」。
- **Graph API(评估,非立项,08 §2.2 / D-232)**:本里程碑仅产 Graph API spike report(与流序分配交互 / CUB-Thrust 对标 / 立项决策树);**立项与否 agent 裁决留痕,触发新扩张方向才登记 `registry/spike_gating.json` SG-###(AI 不自行立项)**。
- **UB 节禁区**:流序分配资源所有权 / 生命周期以 **affine 所有权 + 确定性诊断(rustc 编译期拦截)** 定义,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-19 | 新建 spec/async_buffer.md(G1.2 流序分配 `AsyncBuffer<'stream,T>` 类型契约语义面起始文件):登记编号区间 RXS-0144 起续号预留 + 文件级前言 / 范围(`AsyncBuffer<'stream,T>` affine 所有权与流序分配/释放 RAII / 分配未完成访问被 stream 序排除 / 释放后访问编译期生命周期错误 / 跨 stream 经 `share_with(other,event)` 显式时序边 / 三类流序分配生命周期错误编译期拦截判据;**复用 M8.3 RXS-0130~0134 affine 资源 + `InFlight` 流序分配类型化先例,新增仅补流序分配器缺口**;复用 src/rurix-rt 运行时对象、PTX-only、affine 所有权不设 UB、不触 VMM/多 GPU、Graph API 仅评估不立项)/ 依据与授权(06 §5.4 + 08 §2.2 + 13 D-122/D-232 + 05 §1/§ + 08 §1/§2;G1_CONTRACT D-G1-2 / G-G1-2 + G1_PLAN §2 + MR-0001 agent 批准)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0144 affine 所有权与流序分配/释放 RAII / RXS-0145 分配未完成访问被 stream 序排除 / RXS-0146 释放后访问编译期生命周期错误 / RXS-0147 跨 stream 经 share_with 显式时序边 / RXS-0148 三类流序分配生命周期错误编译期拦截判据)/ 错误码说明(§3:**本里程碑大概率不新增 RX 码**——流序分配生命周期类别由 Rust 类型系统原生编译期拦截 rustc 诊断,registry/error_codes.json 与 messages 零追加;确需编译器侧 RX 诊断则停手升档)/ 升档·禁区留痕(§4:AsyncBuffer API 形态新决策面带档位标记 Mini-RFC/MR-0001、流序分配生命周期 100% 编译期拦截 G-G1-2 判据、MIR 借用检查/内存模型映射/FFI ABI/安全包络人工升档守卫、VMM/多 GPU G2、Graph API 仅评估不立项、UB 节禁区)。**沿 README v1.29 pipeline.md / v1.32 interop_d3d12.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 G1.2 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | **Mini-RFC**（MR-0001） |
| v1.1 | 2026-06-19 | §2 落地带编号条款体 RXS-0144 ~ RXS-0148(G1.2 实现 PR,条款体随实现 + 测试锚定同落,去计划骨架):RXS-0144 `AsyncBuffer<'stream,T>` affine 所有权与流序分配/释放 RAII(`cuMemAllocAsync` 入 ordered pool + `cuMemFreeAsync` 流序释放;`PoolAlloc` 单点 Drop 转移释放责任不双重释放)/ RXS-0145 分配未完成访问被 stream 序排除(in-flight `AsyncBuffer` 无 `device_ptr`/读接口,读写取址仅 `AsyncReady`,镜像 RXS-0132 `InFlight`)/ RXS-0146 释放后访问编译期生命周期错误(affine move-only 非 Copy/非 Clone → `E0382`)/ RXS-0147 跨 stream 经 `share_with(other,event)` 显式时序边(`cuEventRecord`+`cuStreamWaitEvent` 重 brand → `AsyncReady`)/ RXS-0148 三类流序分配生命周期错误编译期拦截判据(分配未完成访问 `E0599` / 释放后访问 `E0382` / 跨 stream 未同步 `E0599`;reject 样例锚定 + 真实红绿 + Compute Sanitizer nightly,CUDA.jl #780 事故类回归)。每条 ≥1 `src/rurix-rt/src/pipeline.rs` 单测锚定(trace_matrix 维持全锚定 143→148)。§2 计划骨架升格为条款体。**本里程碑不新增 RX 码**(§3:rustc 原生编译期诊断,registry/error_codes.json 与 messages 零追加)。实现裁决:流序分配器 FFI(`cuMemAllocAsync`/`cuMemFreeAsync`)落 `src/rurix-rt` 既有豁免 + 每块 `// SAFETY:` + unsafe-audit U19/U20;`AsyncBuffer` 随 `rurix-rt` **始终编译**(镜像 `InFlight`,无可选依赖,默认 workspace 网 build+clippy+test 全覆盖且不依赖 device 而绿,device 仅运行期检测),区别于 G1.1 因 `rurix-d3d12` C++ 依赖而 feature 门控;PTX-only、不触 VMM/多 GPU(G2,D-232)、Graph API 仅评估不立项。档位 **Mini-RFC**(MR-0001,owner 2026-06-19 批准),agent 自主判档,判档争议向上取严 | **Mini-RFC**（MR-0001） |
