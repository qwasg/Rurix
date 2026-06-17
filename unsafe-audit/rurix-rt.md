# unsafe-audit: rurix-rt(CUDA Driver API FFI 边界)

> 注册依据:AGENTS.md 硬规则 9 / 10 §7.6(无注册条目的 unsafe 是 CI 错误);
> 14 §2 常驻集 unsafe-audit 完整性。M4.3 激活(D-M4-4 运行时落地,首个 unsafe 边界)。
> 决策依据:D-113(FFI 战略:`extern "system"` + `#[repr(C)]` + 原始指针,Windows x64
> 唯一 ABI)、D-230(运行时 = Driver API 薄层)、D-231/D-232/D-234(对象模型/内存
> 三件套/装载协商)。M4 契约 `rfc_required: none`(已锁定决策的条款化),会话授权
> 直接实现 + 块级豁免(不另走 RFC)。

## 范围与豁免

- crate:`src/rurix-rt`(`[lints.rust] unsafe_code = "allow"`;`undocumented_unsafe_blocks
  = "deny"` 维持——每个 unsafe 块强制 `// SAFETY:` 注释)。
- 全仓其余 crate(`rurixc`)维持 `unsafe_code = "deny"`(根 workspace 默认),不受影响。
- 全部 unsafe 集中于 `src/rurix-rt/src/sys.rs`(FFI 边界)+ `lib.rs` 中少量裸指针
  构造(launch 实参数组 / JIT 选项数组 / pinned 切片视图),逐块 `// SAFETY:` 在位。

## 原语清单与验证义务(RustBelt 式)

| # | 原语 | 位置 | 验证义务(SAFETY 不变量) |
|---|---|---|---|
| U1 | `LoadLibraryA` / `GetProcAddress` 动态加载 | sys.rs `Cuda::load` | 入参为 `c"..."` NUL 结尾字面量;返回地址仅经 `cast_fn` 在 null 校验后转函数指针 |
| U2 | `transmute_copy::<*mut c_void, FnT>` 符号 → 函数指针 | sys.rs `cast_fn` | `raw` 非 null;符号名 ⇔ 类型别名签名 ⇔ CUDA Driver API(`_v2`)ABI 逐一对应(D-113);指针宽度相等(debug_assert) |
| U3 | Driver API 函数指针调用(cuInit/cuCtx*/cuMem*/cuModule*/cuLaunchKernel/...) | sys.rs `Cuda::*` 方法 | 句柄(ctx/stream/module/function/deviceptr)有效且未释放,由上层所有权类型(Context/Stream/DeviceBuffer/Module/Kernel)RAII 维持;出参指针有效可写;字节范围在分配内 |
| U4 | `CStr::from_ptr`(cuGetErrorName/String) | sys.rs `error_name`/`error_string` | 成功返回时驱动写入进程生命期静态 NUL 结尾字符串 |
| U5 | `cuModuleLoadDataEx` 平行选项数组 + NUL 结尾 PTX image | lib.rs `Context::load_module` | image 为 `CString`(NUL 结尾);opts/vals 长度 4 平行有效;日志缓冲 `info_buf`/`err_buf` 调用期存活 |
| U6 | H2D/D2H 拷贝裸指针 | lib.rs `DeviceBuffer::copy_*` | 主机切片 `bytes` 字节有效;设备地址范围在分配内;`assert` 守长度 ≤ 容量 |
| U7 | `cuLaunchKernel` 实参指针数组 | lib.rs `Stream::launch` | `params` 各元素指向调用方维持的有效实参存储,长度与 kernel 形参匹配(编译期 launch_check 裁决,RXS-0074) |
| U8 | `slice::from_raw_parts(_mut)` pinned 视图 | lib.rs `PinnedBuffer::as_(mut_)slice` | ptr 为 cuMemAllocHost 返回的 `len*size_of::<T>()` 字节锁页内存,对齐满足;`&self`/`&mut self` 约束生命期与别名 |
| U9 | primary context retain/release/set_current(`cuDevicePrimaryCtxRetain` / `cuDevicePrimaryCtxRelease_v2` / `cuCtxSetCurrent`) | sys.rs `Cuda::{primary_ctx_retain,primary_ctx_release,ctx_set_current}` + lib.rs `Context::from_primary` / `Drop` | `device` 来自 `device_get`;retain/release 配对(引用计数,Drop 仅 release 一次);`ctx_set_current` 入参为刚 retain 成功的有效 context;set_current 失败回滚 release(M8.1 互操作零拷贝:与 PyTorch runtime API 共享 primary context,RXS-0125) |
| U10 | 借用外部设备指针构造缓冲(`from_device_ptr`,Drop 不 free) | lib.rs `Context::from_device_ptr` / `DeviceBuffer::drop`(`!owned` 早返) | 调用方(互操作 FFI 边界,经 `__cuda_array_interface__` v3 / DLPack capsule 取得)保证 `ptr` 在本 context 设备上有效、可读写、容纳 ≥ `len` 个 `T`,借用存活期内未被外部 deleter 释放;`owned=false` 故 Drop **不** `cuMemFree`(所有权留外部 deleter,不双重释放,M8.1 / RXS-0123/0124) |
| U11 | event 跨 stream 同步 FFI(`cuEventCreate` / `cuEventRecord` / `cuEventDestroy_v2` / `cuEventSynchronize` / `cuStreamWaitEvent`) | sys.rs `Cuda::{event_create,event_record,event_destroy,event_synchronize,stream_wait_event}`(M8.3 UC-02,RXS-0131) | event/stream 句柄有效、未销毁、同 current context,由上层所有权类型(`SharedEvent`/`SharedStream`)RAII 维持;出参指针有效可写;`event_record` 前 stream 有效,`stream_wait_event` 建立流序依赖不解引用数据 |
| U12 | 异步搬运 FFI(`cuMemcpyHtoDAsync_v2` / `cuMemcpyDtoHAsync_v2`) | sys.rs `Cuda::{memcpy_htod_async,memcpy_dtoh_async}`(M8.3 UC-02,RXS-0131) | 设备地址范围在分配内;主机端(宜锁页)`src`/`dst` ≥ `bytes` 字节,且**在 stream 异步操作完成前保持有效**(由 `InFlight` 持 `PinnedBox` 保活至同步,杜绝悬垂);`stream` 有效 |
| U13 | 跨线程共享 primary context(`SharedContext`/`SharedInner` 的 `unsafe impl Send + Sync` + 跨线程 `cuCtxSetCurrent` 重绑 + retain/release) | pipeline.rs `SharedInner`(Send/Sync/Drop)、`SharedContext::{from_primary,bind}`、各 affine 资源 `Drop`(重绑 current 后释放)(M8.3 UC-02,RXS-0133) | primary context 为**进程级**对象,多线程各自 `cuCtxSetCurrent` 后共享合法(Driver 线程模型);`SharedInner` 持句柄/设备序号纯数据,`Arc` 单点配对 retain/release(最后引用 Drop 仅 release 一次);任意线程的资源 Drop 先 `ctx_set_current(inner.raw)`(`Arc` 存活保证句柄有效)再释放 |
| U14 | 跨线程 event 句柄转移(`SharedEvent` 的 `unsafe impl Send`) | pipeline.rs `SharedEvent`(`unsafe impl Send` / Drop)(M8.3 UC-02,RXS-0133) | `cuEvent` 为绑 context 的进程级驱动对象,跨线程 `move` 合法(持有者线程 current 为同一 context;`Arc<SharedInner>` 保证 context 存活,Drop 前重绑 current);仅 `Send`(move 转移),不 `Sync`(不跨线程共享 `&`) |
| U15 | 跨 stream 流序依赖(`SharedStream::wait_event` 经 `cuStreamWaitEvent`) | pipeline.rs `SharedStream::wait_event` / `acquire`(M8.3 UC-02,RXS-0132 流序分配类型化) | `stream`/`event` 有效且同 current context;`acquire` 消费 `InFlight` 插入 wait 后重 brand 回 `DeviceBox`——「跨 stream 未同步访问」由类型系统(`InFlight` 无读接口)编译期拦截,不解引用未就绪数据 |
| U16 | 异步搬运裸指针 + pinned 经 `InFlight` 保活(`SharedStream::{upload,download}`) | pipeline.rs `SharedStream::{upload,download}`(M8.3 UC-02,RXS-0131/0132) | `upload` move 入 `PinnedBox` 并随 `InFlight` 存活至同步(异步 H2D 期 pinned 源不悬垂);`download` 末 `cuStreamSynchronize` 后方返回(异步 D2H 期 pinned 目标存活);设备地址范围在分配内,`assert` 守长度 ≤ 容量 |

## 销毁纪律(D-231)

`Context::drop` 先 `cuCtxSynchronize`,再按种类释放:独占 context(`cuCtxCreate`)走
`cuCtxDestroy`,primary context(`from_primary` retain)走 `cuDevicePrimaryCtxRelease`
(引用计数,不 destroy 与 PyTorch 共享的 context)。Stream/Module/PinnedBuffer 的 Drop 在
各自资源上调用 free/unload;`DeviceBuffer` 的 Drop **仅当 `owned`** 才 `cuMemFree`——借用
缓冲(`from_device_ptr`,零拷贝互操作)所有权在外部 deleter,Drop 不释放(不双重释放)。
错误吞掉(Drop 无 panic)。生命周期 brand(`'ctx`)保证资源不晚于 context(借用检查 +
反向 Drop 序)。

**M8.3 UC-02 shared 族销毁纪律(`pipeline.rs`,RXS-0130/0133)**:`SharedContext` 经 `Arc`
引用计数包裹 primary context,`Clone` 仅 `Arc` +1(不重复 retain);`DeviceBox`/`PinnedBox`/
`SharedStream`/`SharedEvent`/`SharedModule` 各持一份 `Arc<SharedInner>` 克隆,故 `SharedInner`
(及其 `primary_ctx_release`)在**全部资源 Drop 之后**才发生(context 不早于其资源,跨线程亦
然)。各资源 Drop 在**任意持有线程**先 `cuCtxSetCurrent(inner.raw)` 重绑本 context 再
free/destroy/unload(`Arc` 存活保证句柄有效),Drop 仅一次(单一所有权、非 `Clone`,不双重
释放)。current context 线程绑定守卫 `Bound` 为 `!Send`(`PhantomData<*const ()>`),不得跨线程
转移;可跨线程的仅 `SharedContext`(`Send+Sync`)/ `DeviceBox`(`Send`)/ `SharedEvent`(`Send`)。

## 测试

- `cargo test -p rurix-rt`(子进程隔离 GPU 真跑,14 §6):装载→launch→拷回逐元素核对。
- 全链路真跑红绿见 M4 CI_GATES §2 步骤 21(M4.3 接入)/ close-out run URL。
