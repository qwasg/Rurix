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

## 销毁纪律(D-231)

`Context::drop` 先 `cuCtxSynchronize` 再 `cuCtxDestroy`;Stream/Module/DeviceBuffer/
PinnedBuffer 的 Drop 在各自资源上调用 free/unload,错误吞掉(Drop 无 panic)。生命周期
brand(`'ctx`)保证资源不晚于 context(借用检查 + 反向 Drop 序)。

## 测试

- `cargo test -p rurix-rt`(子进程隔离 GPU 真跑,14 §6):装载→launch→拷回逐元素核对。
- 全链路真跑红绿见 M4 CI_GATES §2 步骤 21(M4.3 接入)/ close-out run URL。
