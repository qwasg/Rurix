# unsafe-audit — `rurix-d3d12`（D3D12/DXGI present 薄 C/C++ shim 边界）

> 地位:`src/rurix-d3d12` crate 的 unsafe 原语注册表(AGENTS 硬规则 9 / 10 §7.6)。G1.1
> CUDA–D3D12 互操作(RFC-0001 §4.2/§4.4,RXS-0143)。crate `unsafe_code = "allow"`(块级豁免),
> 维持 `undocumented_unsafe_blocks = deny`(每 unsafe 块携 `// SAFETY:`)。

## 范围与豁免

- **默认(stub,无 feature)**:**零 unsafe**——`Presenter`/`close_shared_handle` 全部入口返回
  `RX_D3D12_E_NOTIMPL`,不触 FFI。无 Windows SDK D3D12 环境亦编译(常驻回归网绿)。
- **feature `real-shim`**:经 `build.rs` + `cc` 编译 `shim/rx_d3d12_shim.cpp`(D3D12/DXGI COM +
  固定 present pass),Rust 侧仅经下列 `extern "C"` 扁平面 FFI 调用。D3D12/DXGI COM 复杂度全部
  留 C++,**不进语言**(D-130)。

## 原语清单与验证义务(RustBelt 式;real-shim 段)

| # | 原语 | 位置 | 验证义务 |
|---|---|---|---|
| D1 | `rx_d3d12_present_create` extern "C"(出参 `**RxD3D12Present` / `*InteropExport`) | lib.rs `Presenter::create` | `abi_version == RX_D3D12_ABI_VERSION`;`cuda_luid` 为 8 字节有效只读数组指针;出参 `raw`/`export` 为有效可写本地存储;`InteropExport` `#[repr(C)]` 与 C++ `static_assert(sizeof==96)` 双向核对;成功 `rc==0 && raw!=null`,否则 Err(HRESULT 位码) |
| D2 | `rx_d3d12_present_pump` / `submit` / `wait_idle` extern "C" | lib.rs `Presenter::{pump,submit,wait_idle}` | `self.raw` 为 `create` 成功且未销毁的 shim 句柄;`pump` 出参 `should_close` 有效可写;`submit` fence 值由调用方按 RFC-0001 §4.3 偶/奇协议给出;对象固定创建线程(跨线程 → `RPC_E_WRONG_THREAD`) |
| D3 | `rx_d3d12_present_destroy` extern "C" | lib.rs `Presenter::drop` | `self.raw` 为 `create` 成功、本类型独占(非 Clone)的句柄,Drop 仅一次;shim 内部按 RFC-0001 §4.4 释放 fence/resource/queue/swapchain/device/window;`null` 早返(不调用) |
| D4 | `rx_d3d12_close_shared_handle` extern "C" | lib.rs `close_shared_handle` | `handle` 为 shim `create` 经 out-export 移交、尚未关闭的 NT HANDLE;每个 handle 恰好一次(RFC-0001 §4.2.2) |

## C++ shim 侧 COM 所有权纪律(RFC-0001 §4.2.1/§4.2.2/§4.4)

- 窗口 / 窗口类 / 消息泵 / DXGI factory·adapter / D3D12 device·queue·swapchain / 固定 present
  shader(构建期 HLSL→DXBC 嵌入)/ 共享 resource·fence **全部由 shim 拥有**,固定创建线程。
- 共享 committed `D3D12_RESOURCE`(`D3D12_HEAP_FLAG_SHARED`)+ 共享 `D3D12_FENCE`;`CreateSharedHandle`
  产出两 NT HANDLE 在 `create` 成功后移交 Rust wrapper(import 后各 close 一次)。
- shim 持 committed resource 与 fence 的 COM 强引用,直至 CUDA mapped pointer / external semaphore /
  external memory 均销毁后才允许 `present_destroy` 释放(销毁序由 Rust 侧 U17/U18 + 本侧配合)。
- C++ 异常不得越过 C ABI;成功返回 `S_OK==0`,失败返回原始 `HRESULT` 的 `int32_t` 位模式。

## 测试

- 默认 stub:`cargo test -p rurix-d3d12`(`interop_export_abi_layout` 96 字节 + 字段偏移;
  `stub_reports_unavailable_without_real_shim` 返回 `RX_D3D12_E_NOTIMPL`)。
- real-shim:设备真跑见步骤 40/41(交互桌面会话 + Windows SDK D3D12),run URL 随回填。
