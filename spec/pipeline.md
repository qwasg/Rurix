# Rurix 语言规范 — UC-02 流水线类型化语义面(affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化;M8.3 起)

> 条款:RXS-0130 起续号预留(M8.3 UC-02 三 stream 重叠流水线类型化语义面:affine Context/Stream/Event/Buffer 所有权与销毁纪律 / Event 记录·等待与跨 stream 同步 / 流序分配类型化 / 跨线程所有权转移 / 资源生命周期错误类别与编译期拦截判据)。**复用既有 device/运行时条款(RXS-0066 着色 / RXS-0074 launch 类型契约 / RXS-0077 poisoned context 状态机),新增仅补缺口**。体例见 [README.md](README.md)。
> 依据:02 §U2(UC-02 三 stream 重叠流水线:H2D / compute / D2H 三 stream 重叠 + 资源生命周期错误类别 100% 编译期拦截);05 §1(device⊂host 安全子集:所有权 / 借用 / 生命周期规则在 device 路径同义延拓)+ 05 §(affine 资源所有权:GPU 资源 move-only、单一所有权、RAII 销毁纪律);06(GPU 执行模型:context / stream / event / 设备内存);08 §1/§2(`rurix-rt` 运行时对象:Context affine 根 / Stream / Buffer / 装载协商 / poisoned 状态机,D-230~D-234);01 §6(MVP 成功判据:UC-02 端到端 + 预设资源生命周期错误类别 100% 编译期拦截);07 §7(device codegen 分发:M8 维持 PTX-only)。授权:[../milestones/m8/M8_CONTRACT.md](../milestones/m8/M8_CONTRACT.md)(`in_scope: uc02_stream_pipeline` / `spec_m8_clauses`,D-M8-3,G-M8-3 / G-M8-7,`rfc_required: none`)+ [../milestones/m8/M8_PLAN.md](../milestones/m8/M8_PLAN.md) §3 M8.3 第 1 项。
> 档位:**Direct**(条款体)。本文是对 02/05/06/08 已锁定决策(UC-02 三 stream 重叠流水线 / affine 资源所有权 / context·stream·event·buffer 运行时对象 / 资源生命周期错误类别编译期拦截)的初版条款化、纯追加且尚无 stable 面;**agent 自主判档**,判档以 M8_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。本里程碑识别一处新决策面——**跨线程所有权转移的类型化机制(Send 化 primary-context 共享句柄 + 流序分配类型化)**:带档位标记 **Mini**(对齐 M8_CONTRACT §5 guardrail 已锁口径:`src/rurix-rt` 的 unsafe 边界维持 `undocumented_unsafe_blocks=deny`、FFI 凡落 unsafe 须每块 `// SAFETY:` + `unsafe-audit/` 注册;Event / 跨线程 `cuCtxSetCurrent` 新 FFI 落 `src/rurix-rt` 既有豁免 + 注册;safe 类型层对上全 safe,资源生命周期错误以 affine 类型 + rustc 原生诊断拦截)。任何偏离已锁定决策、或触及 **Python 原生嵌入(红线 1,SG-008 永久红线,仅 C ABI/PYD 通道)** / **cubin/fatbin 真分发(G1,M8 维持 PTX-only)** / **Tensor Core/WGMMA/TMA·cluster·动态并行·cooperative groups(11 §2 红线,SG-001/SG-002)** / **const 泛型值运行期单态化(RD-007)** / **device 原子 lowering(D-406/RD-008 agent 自主落笔的高敏面)** 的条款,必须停下标注「需升档」,不在本文件自行落笔(10 §3,M8_CONTRACT §6 / out_of_scope)。**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5):跨 stream / 跨线程资源所有权与生命周期以 **affine 所有权 + 确定性诊断(rustc 编译期拦截)** 定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md / v1.27 cublas.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0130 起)与每条 ≥1 测试锚定随 M8.3 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **UC-02 流水线类型化语义面**的语义条款(M8.3+,D-M8-3)。覆盖语义面:

- **affine Context/Stream/Event/Buffer 所有权与销毁纪律**:GPU 资源(Context affine 根 / Stream / Event / DeviceBuffer / PinnedBuffer)为 **move-only**(非 `Copy` / 非 `Clone`)、单一所有权;生命周期 brand(`'ctx`)编码资源归属层级(跨 context 误用 / 逃逸为编译期借用错误,复用 RXS-0077 既有形态);RAII 确定性销毁序(stream / event 先于 context;owned buffer Drop 释放,borrowed buffer Drop 不释放,对齐 RXS-0124)。**Event 为本里程碑补缺的新运行时对象**(`cuEventCreate` / `cuEventRecord` / `cuEventDestroy`),其余复用 `rurix-rt` 既有对象。
- **Event 记录·等待与跨 stream 同步语义**:Event 在某 stream 上 `record`(`cuEventRecord`),另一 stream `wait`(`cuStreamWaitEvent`)以建立**跨 stream 流序依赖**——三 stream 重叠流水线(H2D stream record `evt_h2d` → compute stream wait `evt_h2d` → record `evt_compute` → D2H stream wait `evt_compute`)的同步骨架;不引入隐式同步 / 自动调度(P-05 薄层)。
- **流序分配类型化(stream-ordered allocation typing)**:缓冲句柄携 **stream brand**(产出 / 最后写入它的 stream);跨 stream 读取须经 `event.wait` 取得**重 brand** 后的句柄;**跳过同步即无重 brand 句柄 → 编译期类型错误**——使「跨 stream 未同步访问」成为 100% 编译期拦截的资源生命周期错误类别(而非运行期 UB / 数据竞争)。
- **跨线程所有权转移(Send 化 primary-context 共享句柄)**:`Context`(独占 / current context 线程绑定)维持 `!Send`;新增并行的 **shared primary-context 句柄**(`Send + Sync`,引用计数 retain/release),经其分配的 Buffer / Event 句柄为 **Send**,可经 affine `move` 跨线程转移(producer→compute→D2H 线程),worker 线程内 `cuCtxSetCurrent` 重绑 current context;单一所有权 + move 语义使 **use-after-free / double-free** 由 rustc 编译期排除,**跨线程非法转移**(送线程绑定的 `!Send` 守卫)为编译期 `Send` 约束错误。
- **资源生命周期错误类别与编译期拦截判据**:预设资源生命周期错误类别——**use-after-free**(move 后再用)/ **double-free**(重复 move·重复释放)/ **跨 stream 未同步访问**(缺 event 同步)/ **跨线程非法转移**(`!Send` 资源越界)——**100% 编译期拦截**(rustc affine move / 生命周期 brand / `Send`-`Sync` 约束原生诊断),MVP 验收判据(01 §6 / G-M8-3);reject 类别覆盖以 compile-fail 样例(应拦截却放行即红)核对。

全部 UC-02 流水线编排**复用 `src/rurix-rt` 运行时对象**(Context/Stream/DeviceBuffer/PinnedBuffer/Module/Kernel)+ 本里程碑补缺的 Event;device 分发维持 **PTX-only**(07 §7);跨 stream / 跨线程资源所有权 / 生命周期以 **affine 所有权 + 确定性诊断(rustc 编译期拦截)** 定义,**不以 UB 表述**(§4)。**资源生命周期错误类别由 Rust 类型系统原生拦截,本里程碑不新增 RX 错误码**(§3;rustc 编译期诊断而非 RX#### 段位码,「如需」按需——本轮判定为不需要)。

**编号区间**:本文件条款自 **RXS-0130** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0129 @ [cublas.md](cublas.md))。本轮计划落地 **RXS-0130 ~ RXS-0134**(affine Context/Stream/Event/Buffer 所有权与销毁纪律 / Event 记录·等待与跨 stream 同步 / 流序分配类型化 / 跨线程所有权转移 / 资源生命周期错误类别与编译期拦截判据),每条 ≥1 测试锚定(`//@ spec: RXS-####`,`src/rurix-rt` crate 单测)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。Legality 违例由 **rustc 原生编译期诊断**拦截(affine move / 生命周期 brand / `Send`-`Sync` 约束),**不引用新 RX 段位码**(§3)。跨 stream / 跨线程资源所有权 / 生命周期以 **affine 所有权 + 确定性诊断**定义,不以 UB 表述。条款体复用 `src/rurix-rt` 既有运行时对象(Context/Stream/DeviceBuffer 等,RXS-0066/0074/0077),新增仅补缺口(Event / shared 跨线程族 / 流序分配类型化)。

### RXS-0130 affine Context/Stream/Event/Buffer 所有权与 RAII 销毁纪律

**Syntax**(运行时 affine 资源,`src/rurix-rt`):

```
SharedContext ::= Arc-wrapped primary context            // Send + Sync,Clone = 引用计数 +1
DeviceBox<T>  ::= 设备内存缓冲                            // Send(T: Send);非 Copy / 非 Clone
SharedEvent   ::= 跨 stream 同步事件(cuEvent)           // Send;非 Copy / 非 Clone
SharedStream  ::= 提交队列(cuStream)                    // !Send;非 Copy / 非 Clone
PinnedBox<T>  ::= 锁页主机缓冲(cuMemAllocHost)          // !Send;非 Copy / 非 Clone
```

**Legality**:

- 全部 affine 资源为 **move-only**(非 `Copy` / 非 `Clone`,单一所有权):`move` 后再用 → `E0382`(use-after-free 类别,RXS-0134);试 `.clone()` → `E0599`(double-free 类别)。违例由 rustc 拦截,不设 RX 段位码。
- 生命周期不晚于其 context:单线程族 `Stream<'ctx>`/`DeviceBuffer<'ctx, T>` 以生命周期 brand `'ctx` 借用 `&'ctx Context`(复用 RXS-0077;跨 context 误用 / 逃逸为编译期借用错误);shared 族以 `Arc<SharedInner>` 持有 context 引用,保证 context 在**全部资源 Drop 之后**才 `cuDevicePrimaryCtxRelease`。

**Dynamic Semantics**:

- **确定性销毁序**:`Context::drop` 先 `cuCtxSynchronize` 再按种类释放(独占 `cuCtxDestroy` / primary `cuDevicePrimaryCtxRelease`,RXS-0077 既有);shared 族各资源 Drop 在**任意持有线程**先 `cuCtxSetCurrent(inner.raw)` 重绑本 context 再 free/destroy/unload,Drop 仅一次(单一所有权,不双重释放);`SharedInner`(及 release)在最后一个 `Arc` 引用 Drop 时发生(context 不早于其资源)。
- 借用外部设备指针缓冲(`from_device_ptr`,M8.1)Drop **不** free(所有权留外部 deleter,RXS-0124),不在本条新增。

**Implementation Requirements**:

- Event 为本里程碑补缺的新运行时对象(`cuEventCreate` / `cuEventRecord` / `cuEventDestroy_v2` / `cuStreamWaitEvent`);FFI unsafe 落 `src/rurix-rt` 既有豁免 + 每块 `// SAFETY:` + `unsafe-audit/rurix-rt.md` 注册(U11~U16)。
- safe 类型层对上全 safe(`uc02-demo` 默认 `unsafe_code=deny`)。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`affine_resources_are_move_only`:DeviceBox/SharedEvent/SharedStream 单一所有权类型存在)。

### RXS-0131 Event 记录·等待与跨 stream 同步语义

**Syntax**(event API,`src/rurix-rt`):

```
Bound::create_event()              -> SharedEvent
SharedStream::record_event(&e)     -> Result<()>     // cuEventRecord(e, stream)
SharedStream::wait_event(&e)       -> Result<()>     // cuStreamWaitEvent(stream, e)
SharedStream::upload(box,pin,e)    -> InFlight<T>     // cuMemcpyHtoDAsync + record e
SharedStream::download(inflight,d) -> PinnedBox<T>    // wait + cuMemcpyDtoHAsync + sync
```

**Legality**:

- `record_event` / `wait_event` 的 event 与 stream 须同 current context(由 `Arc<SharedInner>` 同源 + worker `bind` 维持);跨 context 误用为运行期驱动错误(`CudaError::Driver`),不设编译期 RX 码。

**Dynamic Semantics**:

- 三 stream(H2D / compute / D2H)重叠经 event 流序依赖编排:H2D stream `record(evt_h2d)` → compute stream `wait_event(evt_h2d)` → launch → `record(evt_compute)` → D2H stream `wait_event(evt_compute)` → 异步回拷。**薄层不引入隐式同步 / 自动调度**(P-05);异步搬运(`cuMemcpyHtoDAsync` / `cuMemcpyDtoHAsync`)经锁页 staging,源 / 目标在 stream 操作完成前保持有效(由 `InFlight` 持 `PinnedBox` 保活,RXS-0132 / U16)。
- `wait_event` 仅建立 stream 间排队依赖,不阻塞主机;`Bound::synchronize`(`cuCtxSynchronize`)/ `SharedStream::synchronize`(`cuStreamSynchronize`)为显式主机阻塞点。

**Implementation Requirements**:

- event 默认标志 `CU_EVENT_DEFAULT`;`cuStreamWaitEvent` 标志 0;复用 `rurix-rt` `nvcuda.dll` 动态加载,不依赖 CUDA Toolkit。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`event_sync_api_surface`:record/wait/upload 事件同步 + 异步搬运 API 面)。

### RXS-0132 流序分配类型化(stream-ordered allocation typing)

**Syntax**(typestate,`src/rurix-rt`):

```
InFlight<T>                                  // #[must_use],无读接口(跨 stream 在途态)
SharedStream::upload(box,pin,e) -> InFlight<T>            // DeviceBox → InFlight(在途)
SharedStream::acquire(inflight) -> (DeviceBox<T>, Option<PinnedBox<T>>)  // wait 重 brand → 可读
SharedStream::commit(box,pin,e) -> InFlight<T>            // 本 stream 操作后 record → 在途
```

**Legality**:

- 缓冲在某 stream 上排队异步操作后被封为 **`InFlight<T>`**(`#[must_use]`,**无 `copy_to_host` / `device_ptr` 等读接口**);读 / 跨 stream 操作的接口**仅 `DeviceBox<T>` 提供**。从 `InFlight` 取回 `DeviceBox` **必经** `SharedStream::acquire`(内插 `cuStreamWaitEvent`)——跳过同步即无可读句柄,**「跨 stream 未同步访问」为编译期类型错误**(直接读 `InFlight` → 方法不存在 `E0599`,RXS-0134),而非运行期数据竞争 / UB。

**Dynamic Semantics**:

- `acquire` 在**消费** stream 上 `wait_event(inflight.event)` 建立流序依赖,**重 brand** 回 `DeviceBox`(连同保活的 pinned 源);此后该 stream 上读 / launch 合法。`InFlight` 跨 stream 转移即「流序分配」的类型载体:其完成事件携带产出 stream 的排队点,目标 stream 经 `acquire` 接入依赖。
- 异步搬运期 pinned 源 / 目标由 `InFlight` 持 `PinnedBox` 保活至同步,杜绝悬垂(U16)。

**Implementation Requirements**:

- `InFlight` 字段私有(`boxed` / `pinned` / `event`),无 `pub` 读访问器——类型系统强制「先同步后读」;不以运行期断言 / RX 码表达。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`stream_ordered_typing_gates_reads`:acquire 重 brand + copy_to_host 仅 DeviceBox 提供)。

### RXS-0133 跨线程所有权转移(Send 化 primary-context 共享句柄)

**Syntax**(shared 跨线程族,`src/rurix-rt`):

```
SharedContext : Send + Sync + Clone          // Arc<SharedInner> 包裹 primary context
SharedContext::from_primary(ord) -> SharedContext
SharedContext::bind() -> Bound<'_>           // cuCtxSetCurrent;Bound: !Send + !Sync
DeviceBox<T> : Send  (T: Send)               // 可 move 跨线程
SharedEvent  : Send                          // 可 move 跨线程(不 Sync)
```

**Legality**:

- `Context`(独占 / current context 线程绑定)与 `Bound`(线程绑定守卫)为 **`!Send`**(`PhantomData<*const ()>`):送入另一线程为编译期 `Send` 约束错误 `E0277`(跨线程非法转移类别,RXS-0134)。
- 可跨线程 `move` 的仅:`SharedContext`(`Send + Sync`)/ `DeviceBox`(`Send`,`T: Send`)/ `SharedEvent`(`Send`)。`SharedStream` / `PinnedBox` / `SharedModule` 为 `!Send`(裸句柄 / 裸指针,线程内创建·使用·销毁)。

**Dynamic Semantics**:

- primary context 为**进程级**对象,多线程各自 `cuCtxSetCurrent` 后共享合法(Driver 线程模型);每个 worker 线程先 `SharedContext::bind()` 重绑 current,再使用转移来的 `DeviceBox` / `SharedEvent`。`SharedContext::Clone` 仅 `Arc` 引用计数 +1(不重复 retain);retain/release 由 `Arc` 单点配对(最后引用 Drop 仅 release 一次)。
- producer 线程 `record` 事件 → `move` `DeviceBox` + `SharedEvent` 跨线程 → consumer 线程 `bind` + `wait_event` + 读;所有权经 affine `move` 单点转移,use-after-free / double-free 由 rustc 排除(RXS-0130/0134)。

**Implementation Requirements**:

- `SharedInner` 的 `unsafe impl Send + Sync`、`SharedEvent` 的 `unsafe impl Send` 经裁决最小开 + `// SAFETY:` + `unsafe-audit/rurix-rt.md`(U13/U14)注册;档位 Mini(§4),agent 自主判档。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`cross_thread_transfer_send_bounds`:SharedContext Send+Sync、DeviceBox/SharedEvent Send)。

### RXS-0134 资源生命周期错误类别与编译期拦截判据

**Legality**(MVP 验收判据,01 §6 / G-M8-3:**预设资源生命周期错误类别 100% 编译期拦截**):

| 类别 | 触发 | 编译期拦截机制 | rustc 诊断 |
|---|---|---|---|
| use-after-free | affine 资源 `move` 后再用 | 单一所有权(非 `Copy`) | `E0382` |
| double-free | 重复 `move` / 试 `.clone()` | 非 `Clone` + move | `E0382` / `E0599` |
| 跨 stream 未同步访问 | 缺 `acquire`(event wait)即读在途缓冲 | 流序分配类型化(`InFlight` 无读接口,RXS-0132) | `E0599`(方法不存在) |
| 跨线程非法转移 | 送 `!Send` 线程绑定守卫 `Bound` 入他线程 | `Send` 约束(RXS-0133) | `E0277`(not `Send`) |

**Dynamic Semantics**:

- 四类**全部于编译期拦截**,无运行期 UB / 数据竞争路径;**不新增 RX 错误码**(§3,rustc 原生诊断)。reject 类别覆盖以 compile-fail 样例核对(`src/uc02-demo/compile-fail/*.rs`,冒烟步骤 36 断言每个应失败者均被 rustc 拒绝);**真实红绿**:放行任一违例(使应拦截者编译通过)→ 红;复原 → 绿,run URL 归档(反 YAML-only)。

**Implementation Requirements**:

- 拦截判据以 **affine 类型 + rustc 诊断**表达,**严禁 UB 节**(10 §7.5);UC-02 多 stream device 路径纳入既有 Compute Sanitizer racecheck+memcheck nightly(运行期无数据竞争佐证,CI_GATES §4)。

> 锚定测试:`src/rurix-rt/src/pipeline.rs`(`resource_lifecycle_error_classes_compile_intercepted`:四类编译期拦截 + 正向类型/Send 边界)。

## 3. 错误码引用汇总

> **本里程碑不新增 RX 错误码**(M8_PLAN §3 / 提示词「资源生命周期诊断错误码(如需)」——本轮判定为不需要)。UC-02 预设资源生命周期错误类别(use-after-free / double-free / 跨 stream 未同步 / 跨线程非法转移)由 **Rust 类型系统原生编译期拦截**(affine move → `E0382`、`Send` 约束 → `E0277`、生命周期 brand → 借用 / 生命周期错误、无 `Clone` → `E0599` 等 rustc 诊断),**而非 RX#### 段位码**;`registry/error_codes.json` 与 `src/rurixc/src/messages/en.messages` **本里程碑不动**(零追加)。
>
> 运行期失败(Event 创建 / 记录 / 等待的驱动错误)沿用 `rurix-rt` 既有 `CudaError` 错误值面 + poisoned 状态机(RXS-0077),不新增 RX 段位码。
>
> 若实现期发现某资源生命周期类别**无法以纯 Rust 类型拦截**而确需编译器侧 RX 诊断 / 运行期段位码,则**停手标注「需升档」**(§4),不在本文件自行预造错误码。

## 4. 升档 / 禁区留痕

- **跨线程所有权转移的类型化机制(新决策面,档位 Mini)**:Send 化 primary-context 共享句柄 + 流序分配类型化为本里程碑新决策面;口径已由 M8_CONTRACT §5 guardrail 锁定——`src/rurix-rt` 的 unsafe 边界维持 `undocumented_unsafe_blocks=deny`,Event / 跨线程 `cuCtxSetCurrent` 新 FFI 落 `src/rurix-rt` 既有豁免 + 每 unsafe 块 `// SAFETY:` + `unsafe-audit/rurix-rt.md` 注册条目(AGENTS 硬规则 9),safe 类型层对上全 safe(`uc02-demo` 默认 `unsafe_code=deny`)。**agent 自主判档**,该决策面带档位标记 Mini 落笔,判档争议向上取严。
- **资源生命周期错误类别 100% 编译期拦截(MVP 验收判据)**:use-after-free / double-free / 跨 stream 未同步 / 跨线程非法转移四类以 **affine 类型 + rustc 原生编译期诊断**拦截(§3);reject 类别覆盖以 compile-fail 样例核对(应拦截却放行即红,反 YAML-only,run URL 归档)。**不以 UB 表述**(10 §7.5)。
- **Python 原生嵌入(永久红线 1,SG-008)**:UC-02 流水线仅保留 **C ABI / 自研 kernel 通道**;Python 解释器宿主 / 原生嵌入为死亡路线红线,**永不实现**(SG-008 维持 not_triggered)。触及即停下标注「需升档」。
- **cubin/fatbin 真分发(G1,PTX-only)**:M8 维持 **PTX-only** 开发期产物(07 §7);UC-02 复用 `rurix-rt` PTX 装载路径,不改 device codegen 分发形态;cubin/fatbin 真分发 → G1(M8 out_of_scope)。
- **高级 GPU intrinsics(11 §2 红线,SG-001/SG-002)**:UC-02 三 stream 重叠为**经典 stream/event 并发**编排,不触 Tensor Core / WGMMA / TMA / cluster / 动态并行 / cooperative groups;触及即停下标注「需升档」。
- **const 泛型值运行期单态化(RD-007)**:UC-02 流水线作用面若触发数组长度类 const 泛型运行期单态化——**非 M8 验收门**(M8_CONTRACT out_of_scope / §6,inherited);本文件**不实现 RD-007**,亦不改 [consteval.md](consteval.md) RXS-0064 语义。遇硬需求**停下标注「需升档」**,按 14 §4 处置。
- **device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 agent 自主落笔的高敏面)**:UC-02 跨 stream 同步以 host 侧 event(`cuEventRecord` / `cuStreamWaitEvent`)表达,不触 device 原子 lowering;触及即停下标注「需升档」。
- **UB 节禁区**:跨 stream / 跨线程资源所有权 / 生命周期以 **affine 所有权 + 确定性诊断(rustc 编译期拦截)** 定义,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-17 | 新建 spec/pipeline.md(M8.3 UC-02 流水线类型化语义面起始文件):登记编号区间 RXS-0130 起续号预留 + 文件级前言 / 范围(affine Context/Stream/Event/Buffer 所有权与销毁纪律 / Event 记录·等待与跨 stream 同步 / 流序分配类型化 / 跨线程所有权转移 / 资源生命周期错误类别与编译期拦截判据;**复用既有 RXS-0066/0074/0077 device·运行时条款,新增仅补缺口**;复用 src/rurix-rt 运行时对象、永不 Python 原生嵌入、PTX-only、affine 所有权不设 UB、不触高级 GPU intrinsics)/ 依据与授权(02 §U2 + 05 §1/§FFI + 06 + 08 §1/§2 + 01 §6 + 07 §7 M8;M8_CONTRACT D-M8-3 / G-M8-3 / G-M8-7 `rfc_required: none` + M8_PLAN §3)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0130 affine 所有权与销毁纪律 / RXS-0131 Event 记录·等待与跨 stream 同步 / RXS-0132 流序分配类型化 / RXS-0133 跨线程所有权转移 / RXS-0134 资源生命周期错误类别与编译期拦截判据)/ 错误码说明(§3:**本里程碑不新增 RX 码**——资源生命周期类别由 Rust 类型系统原生编译期拦截 rustc 诊断,registry/error_codes.json 与 en.messages 零追加;确需编译器侧 RX 诊断则停手升档)/ 升档·禁区留痕(§4:跨线程所有权转移类型化机制带档位标记 Mini、资源生命周期 100% 编译期拦截 MVP 判据、Python 原生嵌入红线 1/SG-008、PTX-only/G1、高级 GPU intrinsics SG-001/002、RD-007、D-406/RD-008、UB 节禁区)。**沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md / v1.27 cublas.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 M8.3 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
| v1.1 | 2026-06-17 | 落地带编号条款体 RXS-0130 ~ RXS-0134(M8.3 实现 PR,条款体随实现 + 测试锚定同落):RXS-0130 affine Context/Stream/Event/Buffer 所有权与 RAII 销毁纪律(move-only·单一所有权·生命周期 brand·确定性销毁序;Event 补缺;shared 族 `Arc<SharedInner>` 跨线程销毁纪律)/ RXS-0131 Event 记录·等待与跨 stream 同步语义(`cuEventCreate`/`cuEventRecord`/`cuEventDestroy_v2`/`cuStreamWaitEvent` + `cuMemcpy*Async` 异步搬运;三 stream 重叠流序依赖;薄层无隐式同步)/ RXS-0132 流序分配类型化(`InFlight` typestate 无读接口,`acquire` 插 `cuStreamWaitEvent` 重 brand 回 `DeviceBox`;跨 stream 未同步访问 = `E0599` 编译期拦截)/ RXS-0133 跨线程所有权转移(`SharedContext` Send+Sync、`DeviceBox`/`SharedEvent` Send、`Bound`/`Context` !Send;worker `cuCtxSetCurrent` 重绑;`unsafe impl Send/Sync` 档位 Mini + unsafe-audit U13/U14)/ RXS-0134 资源生命周期错误类别与编译期拦截判据(use-after-free E0382 / double-free E0382·E0599 / 跨 stream 未同步 E0599 / 跨线程非法转移 E0277 四类 100% 编译期拦截 + reject 样例锚定 + 真实红绿)。每条 ≥1 锚定(`src/rurix-rt/src/pipeline.rs` 单测:affine move-only / event 同步 API 面 / 流序类型门 / 跨线程 Send 边界 / 四类拦截;trace_matrix 维持全锚定 129→134)。§2 计划骨架升格为条款体。**本里程碑不新增 RX 码**(§3:rustc 原生编译期诊断,registry/error_codes.json 与 en.messages 零追加)。实现裁决:Event / shared 跨线程族 FFI unsafe 落 `src/rurix-rt` 既有豁免 + 每块 `// SAFETY:` + unsafe-audit 注册(U11~U16);`uc02-demo` 默认 `unsafe_code=deny`;PTX-only、不触 RD-007 / D-406 / 红线 1 / 高级 GPU intrinsics。新决策面跨线程所有权转移类型化机制档位 **Mini**(口径 M8_CONTRACT §5 锁定),agent 自主判档,判档争议向上取严。授权:02 §U2 + 05 §1/§FFI + 06 + 08 §1/§2 + 01 §6 + 07 §7 M8,M8_CONTRACT D-M8-3 / G-M8-3 / G-M8-7 `rfc_required: none` | Direct |
