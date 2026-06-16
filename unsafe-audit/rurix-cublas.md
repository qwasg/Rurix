# unsafe-audit: rurix-cublas(cublas v2 C API FFI 边界)

> 注册依据:AGENTS.md 硬规则 9 / 10 §7.6(无注册条目的 unsafe 是 CI 错误);
> 14 §2 常驻集 unsafe-audit 完整性。M8.2 激活(D-M8-2 cublas 绑定包落地,FFI 边界)。
> 决策依据:D-113(FFI 战略:`extern "C"` + 原始指针,Windows x64 唯一 ABI)、
> 09(cublas 绑定包 raw FFI / safe wrapper / 高层 API)、05 §FFI(复杂类型不透明句柄
> `cublasHandle_t`)、许可红线 r6(cublas runtime DLL 按需附带 Attachment A 白名单,
> 完整 Toolkit/驱动/Nsight 永不捆绑)。M8 契约 `rfc_required: none`(已锁定决策的条款化),
> 新决策面 cublas FFI 边界 unsafe 策略带档位标记 **Mini**(spec/cublas.md 前言),
> 直接实现 + 块级豁免(不另走 RFC)。spec 条款:RXS-0126 ~ RXS-0129。

## 范围与豁免

- crate:`src/rurix-cublas`(`[lints.rust] unsafe_code = "allow"`;`undocumented_unsafe_blocks
  = "deny"` 维持——每个 unsafe 块强制 `// SAFETY:` 注释)。
- 全仓其余新 crate 维持 `unsafe_code = "deny"`(根 workspace 默认),不受影响。
- 全部 unsafe 集中于 `src/rurix-cublas/src/sys.rs`(cublas runtime DLL 动态加载 + v2 C API
  调用)+ `lib.rs` 中借用外部设备指针缓冲处(`from_device_ptr` 调用,实际原语在
  `rurix-rt` U9/U10,见 `unsafe-audit/rurix-rt.md`)。`ffi.rs` C ABI 导出层**无 unsafe 块**
  (设备指针为不透明 `u64` 地址,仅前向给高层 safe API)。

## 原语清单与验证义务(RustBelt 式)

| # | 原语 | 位置 | 验证义务(SAFETY 不变量) |
|---|---|---|---|
| C1 | `LoadLibraryA` / `GetProcAddress` 动态加载 cublas runtime DLL | sys.rs `Cublas::load` | 入参为 `c"..."` NUL 结尾字面量;仅尝试 Attachment A 白名单候选名(`cublas64_*.dll`,RXS-0129);返回地址仅经 `cast_fn` 在 null 校验后转函数指针 |
| C2 | `transmute_copy::<*mut c_void, FnT>` 符号 → 函数指针 | sys.rs `cast_fn` | `raw` 非 null;符号名 ⇔ 类型别名签名 ⇔ cublas v2 C API ABI 逐一对应(D-113);指针宽度相等(debug_assert) |
| C3 | `cublasCreate_v2` / `cublasDestroy_v2` 句柄创建 / 销毁 | sys.rs `Cublas::{create,destroy}` + lib.rs `CublasHandle::create` / `Drop` | 句柄出参有效可写;句柄绑定 current(primary)context(由 rurix-rt `Context::from_primary` 设置,与 PyTorch 共享);create/destroy 配对,`CublasHandle` RAII 独占,Drop 仅销毁一次(`raw` 非 null 早查) |
| C4 | `cublasSgemm_v2` / `cublasSgemv_v2` device 算子调用 | sys.rs `Cublas::{sgemm,sgemv}` + lib.rs `run_gemm` / `run_gemv` | `handle` 有效未销毁;A/B/C/x/y 为 current context 内有效、可读写、容量与 m/n/k 相容的设备地址(经 `from_device_ptr` 借用 + 维度合法性前置校验,RXS-0127);alpha/beta 指向有效主机 `f32`(`CUBLAS_POINTER_MODE_HOST` 默认);device 地址以 `u64` 按值传参(x64 GP 寄存器与 `const float*` 同宽,ABI 等价),FFI 层不解引用 |
| C5 | 借用外部设备指针缓冲(`from_device_ptr`) | lib.rs `run_gemm` / `run_gemv` | 实际原语 = rurix-rt U10(借用缓冲 Drop 不 free,所有权留外部 deleter);调用方(经 torch CUDA 张量设备指针)保证 `ptr` 在本 context 设备上有效、容纳 ≥ `len` 个 `f32`、借用存活期内未被外部 deleter 释放(RXS-0128) |

## 销毁纪律

`CublasHandle::drop` 经 `cublasDestroy_v2` 销毁句柄(`raw` 非 null 早查,错误吞掉,Drop
无 panic)。设备内存所有权在外部框架(PyTorch)deleter——借用缓冲(`from_device_ptr`)
Drop **不** `cuMemFree`(不双重释放,rurix-rt U10)。primary context 经 rurix-rt
`Context::Drop` 走 `cuDevicePrimaryCtxRelease`(引用计数,不 destroy 与 PyTorch 共享的
context;rurix-rt U9)。cublas runtime DLL 进程内单次加载(`OnceLock`),不卸载(进程
生命期);**永不**捆绑完整 Toolkit/驱动/Nsight(许可红线 r6,RXS-0129)。

## 测试

- `cargo test -p rurix-cublas`:RXS-0126~0129 单测锚定(DLL 候选 Attachment A 白名单形态 /
  设备指针 + 维度合法性先于 cublas / C ABI 薄包返回码一致 / loaded_dll 审计内省;host 上
  不触 GPU 的确定性校验)。
- 三层绑定数值对照 + 内建篡改红绿真跑见 `ci/cublas_binding_smoke.py`(步骤 35,torch CUDA
  张量经 ctypes 零拷贝调用)/ M8 CI_GATES §2 步骤 35 / close-out run URL。
