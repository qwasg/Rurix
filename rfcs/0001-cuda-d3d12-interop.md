# RFC-0001 — CUDA–D3D12 interop（`ExternalBuffer` / `ExternalSemaphore`）与软光栅实时窗口呈现

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0001（首个真实 RFC；采用 4 位制，编号永不复用，10 §9.5；见 §9 Q1） |
| 标题 | CUDA–D3D12 interop 与软光栅实时窗口呈现 |
| 档位 | **Full RFC**（10 §3：FFI ABI / 运行时语义 / unsafe 边界 / 内存模型映射；触及 AGENTS 硬规则 5 禁区） |
| 状态 | **Owner Approved（2026-06-18）** — owner 已在本工作会话明确确认 Q1~Q5 与 §4.2/§4.3/§4.4 🔒 文本；批准记录由 Codex 代录，**不是 AI 代签或自行裁决**。FCP-lite 的额外评审/等待窗若适用，仍须按 10 §2.2/§5 完成 |
| 承接里程碑 | G1.1（验收门 **G-G1-1**），首子里程碑 |
| 关联条款 | 拟落 spec **RXS-0140~**（区间随条款数定，§5）；新建 `spec/interop_d3d12.md` |
| 依据决策 | D-002（图形分期）· **D-130**（G1 interop=D3D12 external memory/semaphore）· D-230（运行时=Driver API 薄层）· D-233（WDDM/TDR/HAGS 环境画像强制）· 06 §8.1 通路 · 06 §4.2 内存模型 · 06 §6 三阶段 · spec/softraster.md:153 |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`；`Assisted-by: codex:gpt-5`。Human-in-the-loop（AGENTS 硬规则 1/2）：owner 于 **2026-06-18** 在本工作会话明确回复“确认，给你权限签字”，据此批准 Q1~Q5 与全部 🔒 文本；Codex 仅代录该 owner 决定，不以 AI 身份署名 |
| Owner 批准 | **Approved — 2026-06-18**。批准范围：RFC 全文，特别包括 §4.2 FFI ABI、§4.3 内存模型映射/信号时序、§4.4 安全包络及 §9 Q1~Q5。记录方式：owner 在工作会话中直接确认，由 Codex 写回仓库 |

> **批准记录**：§4.2 / §4.3 / §4.4 的 🔒 表示其内容属于只能由 owner 裁决的禁区；owner 已于 2026-06-18 明确批准。本标记继续保留，用于说明权限来源，而非表示仍待裁。

---

## 1. 摘要

在**不做 shader codegen**（G2 红线）的前提下，经 **CUDA–D3D12 external memory / external semaphore 互操作**把 G0 软光栅 demo 从离屏出图（M7.4，UC-03）升级为**实时窗口呈现**：

```
D3D12 创建 swapchain + shared committed buffer/shared fence（薄 C/C++ shim，不进语言）
   → cuImportExternalMemory / cuImportExternalSemaphore 映射为 CUDA 资源
   → Rurix kernel（G0 软光栅，RXS-0118~0121 语义 0-byte）写共享 f32 RGB buffer
   → D3D12 固定 present pass 读取 buffer、写 swapchain backbuffer
   → 共享 fence 同步 present → 交互式窗口实时刷新
```

语言面新增两个 affine 类型 `ExternalBuffer<'ctx, T>` / `ExternalSemaphore<'ctx>`，并以**生成式 context brand**与 `Ready → Acquired → Presentable` typestate 把 **import 句柄生命周期**、**跨 context 误用**、**信号时序违例**做成编译期约束（rustc 原生诊断，对齐 M8.3 RXS-0130~0134 零新 RX 码先例）。D3D12/DXGI 侧以薄 C/C++ shim 驱动，不进语言（对齐 D-130）。

## 2. 动机

- **D-002 / 06 §6 已批准的图形分期**要求「每一阶段都有出图能力反哺动力」：G0 离屏 → **G1 实时窗口** → G2 原生管线。实时呈现是 G1 的标志性出图升级（spec/softraster.md:153 明确归 G1-1）。
- **D-130 已锁定** G1 interop 走 D3D12 external memory/semaphore（Windows-first 自洽；Vulkan 驱动黑洞实证 H04 §2.3 已否）。本 RFC 是把该已锁方向落到**语言面类型契约 + 工程通路**的具体设计。
- **U5 前奏**（02 §U5 / 06 §8.3）：interop 呈现通路为后续「引擎级 compute pass 集成」（G1.3）验证 Windows 图形互操作的全部驱动现实（WDDM/TDR/HAGS，D-233）。

**为何需要 Full RFC（而非 Direct 条款化）**：尽管 D-130 已锁高层方向，本设计**触及 AGENTS 硬规则 5 / 10 §7.5 禁区**——(a) **FFI ABI**：D3D12/DXGI host shim 的 C ABI 面 + `cuImportExternal*` Driver API 绑定；(b) **内存模型映射**：CUDA↔D3D12 跨 API 信号量的 signal/wait 时序与同步序保证（接 06 §4.2）；(c) **安全包络边界**：external 资源 unsafe 边界划线。这些只能人类经 Full RFC 落笔。owner 经 AskQuestion 裁决为 **Full RFC 前置**（spec 条款 PR 与实现 PR 均门控于本 RFC 合入之后；AI 不自判 Direct，争议向上取严）。

## 3. 指导级解释（用户视角）

软光栅 demo（`uc03-demo`）新增 `--present` 交互窗口模式，保留既有离屏 PPM 序列路径向后兼容。窗口模式下每帧：

1. shim 侧（C++）在与选定 CUDA device **LUID 相同**的 DXGI adapter 上创建 D3D12 device、双缓冲 swapchain、一个 `D3D12_HEAP_FLAG_SHARED` 的 committed **buffer resource** 与一个 `D3D12_FENCE_FLAG_SHARED` fence；
2. Rurix 侧把 committed resource 的 NT HANDLE 按 `CU_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE | CUDA_EXTERNAL_MEMORY_DEDICATED` import，并把逻辑映射区映射为 `ExternalBuffer<'ctx, f32>`；共享 fence import 为 `ExternalSemaphore<'ctx>`；
3. 现有 G0 `sr_tonemap` kernel（语义与源码 0-byte）直接把紧密排列的 `width × height × 3` 个 `f32` 量化刻度值写入共享 buffer；
4. CUDA signal 后，shim 在 D3D12 queue 上 wait 同一 fence 值，以固定、shim 私有的 fullscreen present shader 读取 RGB buffer（`0…255 → 0…1`）并写 `DXGI_FORMAT_R8G8B8A8_UNORM` backbuffer，再 `Present`；
5. D3D12 queue signal 下一偶数 fence 值后，下一帧 CUDA wait 该值取回写权。帧循环不做 `cuCtxSynchronize` / CPU fence wait；仅 shutdown 与诊断路径允许 host wait。

类型系统保证：未经 signal/wait 的句柄**没有**可 present / 可写的形态（编译期拦截信号时序违例，见 §4.1）。

## 4. 参考级设计

### 4.1 语言面 affine 类型与 typestate

`ExternalBuffer` / `ExternalSemaphore` 镜像 `DeviceBuffer` 与 `InFlight` 的 affine 纪律，但**不沿用“普通借用生命周期等于唯一 context 身份”的过强假设**：同一词法作用域中的两个 `&Context` 可以被推断为同一较短生命周期，单独的 `'ctx` 参数不足以证明值级 context 相同。

本 RFC 因此采用**生成式 brand**：`D3D12Presenter::scope` 以高阶闭包生成不可伪造、不可逃逸的新鲜 `'ctx`；interop context、stream、module、external memory 与 semaphore 全部携带不变 brand。两个独立 `scope` 的资源类型不同，不能混用。

```rust
pub fn scope<R>(
    cuda_ordinal: i32,
    render_size: [u32; 2],
    window_size: [u32; 2],
    f: impl for<'ctx> FnOnce(InteropContext<'ctx>, ReadyFrame<'ctx>) -> Result<R>,
) -> Result<R>;

pub struct InteropContext<'ctx> { /* generated invariant brand + CUDA context */ }
pub struct InteropKernel<'ctx> { /* 只能由同 brand context/module 取得 */ }
pub struct InteropKernelArg<'ctx> { /* sealed typed arg; no detachable raw pointer */ }
pub struct ExternalBuffer<'ctx, T: Copy> { /* ext_mem + mapped dptr + invariant brand */ }
pub struct ExternalSemaphore<'ctx> { /* ext_sem + invariant brand */ }

pub struct ReadyFrame<'ctx> { /* owns presenter core; next value = 2n */ }
pub struct AcquiredFrame<'ctx> { /* CUDA wait(2n) issued; exposes writable buffer */ }
pub struct PresentableFrame<'ctx> { /* CUDA signal(2n+1) issued */ }

impl<'ctx> ReadyFrame<'ctx> {
    pub fn wait(self) -> Result<AcquiredFrame<'ctx>>;
}
impl<'ctx> AcquiredFrame<'ctx> {
    pub fn buffer_mut(&mut self) -> &mut ExternalBuffer<'ctx, f32>;
    pub fn launch(
        &mut self,
        kernel: &InteropKernel<'ctx>,
        grid: [u32; 3],
        block: [u32; 3],
        args: &mut [InteropKernelArg<'ctx>],
    ) -> Result<()>;
    pub fn signal(self) -> Result<PresentableFrame<'ctx>>;
}
impl<'ctx> PresentableFrame<'ctx> {
    pub fn present(self) -> Result<ReadyFrame<'ctx>>;
}
```

`AcquiredFrame` 捕获 presenter 私有 CUDA stream；`signal(self)` 不接受另一个 stream 参数，因此 wait、kernel launch 与 signal 必定落在同一 stream 序。safe API **不导出可脱离 frame 的 `CUdeviceptr` 或 stream handle**；external buffer 只能经 `InteropKernelArg<'ctx>` 绑定到 `AcquiredFrame::launch`。需要 raw pointer/自选 stream 的逃生舱不属于 G1.1，后续若开放必须另走 Full RFC。`PresentableFrame::present` 是唯一 present 入口。用户可以放弃并销毁整个状态对象，但不能在继续帧循环时跳过 wait 或 signal。

| 错误类别 | 类型化机制 | 编译期拦截 |
|---|---|---|
| 句柄生命周期 | external 对象与 frame state 均非 `Copy`/非 `Clone`；单一所有权 | move 后再用 `E0382`；重复 clone `E0599` |
| 跨 context | `for<'ctx>` 生成式不变 brand；资源不可逃逸 scope | 两个 scope 的 brand 不匹配，产生借用/类型错误 |
| 信号时序 | `Ready → Acquired → Presentable → Ready` 消费式状态机 | 未 wait 无 writable buffer；未 signal 无 `present` 方法 |

### 4.2 🔒 FFI ABI（owner 已批准，2026-06-18）

#### 4.2.1 D3D12/DXGI shim ABI

shim 使用 C++ 实现、C ABI 导出；Windows x64，`extern "C"` / Rust `extern "C"`（x64 统一调用约定），不得让 C++ 异常越过 ABI。所有函数除 `destroy(NULL)` 外均校验空指针与版本；成功返回 `S_OK == 0`，失败返回原始 `HRESULT` 的 `int32_t` 位模式。Rust safe wrapper 负责把 HRESULT 映射为稳定的 Rurix 运行期诊断并保留十六进制详情。

窗口、窗口类、消息泵、DXGI factory/adapter、D3D12 device/queue/swapchain、固定 present shader 与共享 resource/fence **全部由 shim 拥有**。对象固定在创建线程，除 `rx_d3d12_close_shared_handle` 外的方法跨线程调用返回 `RPC_E_WRONG_THREAD`。

```c
#define RX_D3D12_ABI_VERSION 1u
#define RX_D3D12_PRESENT_VSYNC 0x1u

typedef struct RxD3D12Present RxD3D12Present;

typedef struct RxD3D12InteropExport {
    uint32_t abi_version;       /* = RX_D3D12_ABI_VERSION */
    uint32_t struct_size;       /* = 96 */
    void*    memory_handle;     /* caller-owned NT HANDLE */
    uint64_t allocation_size;   /* GetResourceAllocationInfo.SizeInBytes */
    uint64_t mapping_size;      /* render_width * render_height * 3 * sizeof(float) */
    void*    fence_handle;      /* caller-owned NT HANDLE */
    uint8_t  adapter_luid[8];   /* 与 cuDeviceGetLuid 逐字节相同 */
    uint32_t node_mask;
    uint32_t render_width;
    uint32_t render_height;
    uint32_t window_width;
    uint32_t window_height;
    uint32_t channels;          /* 固定为 3 */
    uint32_t reserved[6];       /* 必须为 0 */
} RxD3D12InteropExport;

int32_t rx_d3d12_present_create(
    uint32_t abi_version,
    const uint8_t cuda_luid[8],
    uint32_t cuda_node_mask,
    uint32_t render_width,
    uint32_t render_height,
    uint32_t window_width,
    uint32_t window_height,
    uint32_t flags,
    RxD3D12Present** out_present,
    RxD3D12InteropExport* out_export);

int32_t rx_d3d12_present_pump(
    RxD3D12Present* present,
    uint32_t* out_should_close);

int32_t rx_d3d12_present_submit(
    RxD3D12Present* present,
    uint64_t cuda_done_value,
    uint64_t d3d_done_value);

int32_t rx_d3d12_present_wait_idle(RxD3D12Present* present);
int32_t rx_d3d12_close_shared_handle(void* handle);
void    rx_d3d12_present_destroy(RxD3D12Present* present);
```

ABI 结构不得 `#pragma pack`；C++ 侧 `static_assert(sizeof(RxD3D12InteropExport) == 96)`，Rust 侧 `#[repr(C)]` + size/offset 单测。未知 `flags` 位返回 `E_INVALIDARG`。`present_submit` 固定执行：queue wait `cuda_done_value` → fullscreen present pass → `Present` → queue signal `d3d_done_value`。

#### 4.2.2 共享资源形态与所有权

- **采纳 `D3D12_RESOURCE`，否决 `D3D12_HEAP`**：创建 `D3D12_HEAP_TYPE_DEFAULT` + `D3D12_HEAP_FLAG_SHARED` 的 committed buffer resource；CUDA import type = `CU_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE`，并强制 `CUDA_EXTERNAL_MEMORY_DEDICATED`。
- `allocation_size` 用 `ID3D12Device::GetResourceAllocationInfo`；CUDA import descriptor 的 `size` 使用该值。`mapping_size` 是逻辑 RGB buffer 字节数；`cuExternalMemoryGetMappedBuffer` descriptor 使用 `offset=0,size=mapping_size`。
- 共享 buffer 布局固定为行主序紧密 `f32 RGB`，分量值 `0…255`，与 RXS-0121 `sr_tonemap` 输出同义。render size 与 window/swapchain size 分离；shim 私有 shader 以 nearest-neighbor 将 render buffer 放大到窗口，只做索引、除以 255、补 alpha=1 与 fullscreen 输出。HLSL 源随 crate 入库、构建期编译为嵌入式固定 DXBC，缺 shader compiler 直接构建失败，不做运行期编译或静默 fallback。它不是 Rurix shader codegen，也不扩张 G2。
- `CreateSharedHandle` 产生的两个 NT HANDLE 在 `create` 成功后转移给 Rust wrapper；CUDA import **不接管** Win32 HANDLE 所有权。每个 HANDLE 无论 import 成败都必须恰好调用一次 `rx_d3d12_close_shared_handle`，正常路径在 import 返回后立即关闭。
- shim 始终持有 committed resource 与 fence 的 COM 强引用，直至 CUDA mapped pointer、external semaphore、external memory 均销毁后才允许 `present_destroy` 释放。

#### 4.2.3 CUDA Driver API ABI

`src/rurix-rt/src/sys.rs` 按现有 `nvcuda.dll` 动态装载模式新增：

`cuDeviceGetLuid`、`cuImportExternalMemory`、`cuExternalMemoryGetMappedBuffer`、`cuDestroyExternalMemory`、`cuImportExternalSemaphore`、`cuSignalExternalSemaphoresAsync`、`cuWaitExternalSemaphoresAsync`、`cuDestroyExternalSemaphore`。

全部入口使用 CUDA 头文件声明的 Windows ABI；上述 external-resource 符号无 `_v2` 后缀。async signal/wait 装载基础符号名（非 `_ptsz`），且只传非空私有 stream。所有 descriptor 以 CUDA 头文件 v1 布局 `#[repr(C)]` 复刻，含 reserved 字段并全零初始化；Windows x64 预期大小分别为 memory handle 104、buffer desc 88、semaphore handle 96、signal params 144、wait params 144 字节，必须由编译期/单测核对。

### 4.3 🔒 内存模型映射 / 信号时序（owner 已批准，2026-06-18）

本 RFC 定义的是**同一物理 adapter、同一进程、单一共享 allocation 的资源移交协议**，不是对 06 §4.2 `System` scope 原子模型的扩张。

共享 fence 初值为 0。第 `n` 帧（从 0 起）只使用以下值：

| 阶段 | fence 值 |
|---|---:|
| CUDA 取得写权并 wait | `acquire(n) = 2n` |
| CUDA 完成写入并 signal | `cuda_done(n) = 2n + 1` |
| D3D12 完成读取/present 并 signal 下一写权 | `d3d_done(n) = 2n + 2` |

值只允许 checked `+1/+2`，严格递增、永不 rewind、永不复用；溢出时确定性停止 presenter。首帧 wait 0 可执行为统一路径。

API 调用顺序固定：

1. shim 已创建 fence(initial=0)；
2. CUDA `cuWaitExternalSemaphoresAsync(acquire(n))`；
3. 同一私有 stream 上执行全部写共享 buffer 的 kernel；
4. CUDA `cuSignalExternalSemaphoresAsync(cuda_done(n))`；
5. **第 4 步 API 已成功返回后**，shim 才提交 D3D12 queue wait `cuda_done(n)`、present pass、`Present`、queue signal `d3d_done(n)`；
6. **第 5 步 queue signal 已成功入队后**，下一帧才提交 CUDA wait `acquire(n+1)`。

绑定保证边界：

- CUDA signal 排在同 stream 的先前 kernel 之后；D3D12 对共享 buffer 的读取排在同 fence 值的 queue wait 之后。因此在 wait 满足后，Rurix **承诺** D3D12 观察到该帧 signal 之前完成的 CUDA 写。该承诺是对 NVIDIA external-memory/external-semaphore 官方互操作模型与 `simpleD3D12` 用法的保守归纳；若某驱动/环境不能满足，feature 必须报告 unavailable/failed，而不是降级为更弱语义。
- D3D12 queue signal 排在该帧 present pass/Present 提交之后；下一 CUDA wait 满足后，CUDA 才重新取得共享 buffer 写权。
- 没有配对 wait/signal、使用不同 fence、不同 allocation、不同 adapter、CPU 同时映射、跨进程、跨 GPU、跨 node、或绕过 typestate 的访问，**均不在 safe 保证内**。
- 本协议不承诺 CUDA `System` scope 原子与 D3D12 shader 原子之间的逐原子一致性；跨 API 只承诺整块资源在 fence handoff 边界的排他访问与完成顺序。
- WDDM/HAGS 可改变调度、批处理与延迟，TDR/device removal 可破坏进度；它们不改变一个**已成功完成** fence handoff 的顺序含义。发生 device removed、TDR、CUDA poisoned context 或 wait/signal 错误后，presenter 进入 poisoned 状态，不再提交帧；重建整个 interop 子树是唯一恢复路径。

### 4.4 🔒 安全包络边界（owner 已批准，2026-06-18）

G1.1 **不提供 public `from_raw_handle`**。唯一 safe 构造入口是 `D3D12Presenter::scope`：它先用 `cuDeviceGetLuid` 取得选定 CUDA device 的 LUID/node mask，再要求 shim 在同 LUID adapter 上创建设备与资源，随后在内部完成 import/map/close-handle。由此，外部调用方不承担“任意 raw HANDLE 是否有效”的证明义务。

内部 unsafe 边界仅包括：

1. C shim FFI 调用与 out-pointer 解码；
2. CUDA external descriptor 的 `#[repr(C)]` 传递；
3. import/map/signal/wait/destroy 与 `cuMemFree(mapped_ptr)`；
4. 生成式 brand wrapper 对现有 runtime raw handle 的封装；
5. 必要的 `Send`/`Sync` 实现（默认不实现；本 RFC presenter/thread state 为 `!Send + !Sync`）。

每个 unsafe 块必须单操作、带 `// SAFETY:` 并引用新增 `unsafe-audit/rurix-rt.md` / `unsafe-audit/rurix-d3d12.md` 条目。`src/rurix-d3d12` 的 Rust 外壳默认 `unsafe_code=deny`；若 FFI 声明所在模块需要豁免，只对该边界 crate/module 最小开放并登记。

正常 shutdown 的强制顺序：

1. 停止消息泵接收新帧，状态机不再产生 `AcquiredFrame`；
2. `rx_d3d12_present_wait_idle` + 私有 CUDA stream synchronize，确认 outstanding wait/signal/draw 均完成；
3. `cuMemFree(mapped_ptr)`；
4. `cuDestroyExternalSemaphore`；
5. `cuDestroyExternalMemory`；
6. 确认两个临时 NT HANDLE 已在 import 后关闭；
7. `rx_d3d12_present_destroy` 释放 fence/resource/queue/swapchain/device/window。

`Drop` 仅作为 best-effort fallback，吞掉错误但维持上述相对顺序；显式 `shutdown(self) -> Result<()>` 是可报告错误的首选路径。mapped pointer 未先 `cuMemFree`、outstanding signal/wait 未完成即 destroy、或 D3D12 COM resource 先于 CUDA import object 释放，均不得出现在 safe 实现中。

## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

新建 `spec/interop_d3d12.md`，自 **RXS-0140** 起续号（最高现存 RXS-0139 @ release.md）。拟定条款（区间随最终条款数定）：

| 条款（拟） | 标题 | 测试锚定计划（每条 ≥1，`//@ spec: RXS-####`） |
|---|---|---|
| RXS-0140 | `ExternalBuffer`/`ExternalSemaphore` affine 类型与 import 句柄生命周期 | `src/rurix-rt` 单测（move-only；shutdown 顺序为 mapped `cuMemFree` → destroy semaphore → destroy memory → shim destroy；D3D12 COM owner 不被 CUDA wrapper释放） |
| RXS-0141 | 生成式 context brand 与跨 context 编译期拦截 | conformance reject + UI golden（两个独立 `scope` 的资源/模块/stream brand 不匹配） |
| RXS-0142 | `Ready → Acquired → Presentable` typestate 与偶/奇 fence 协议 | 单测（0/1/2、2/3/4 值序；溢出拒绝）+ conformance reject（未 wait 无 writable buffer；未 signal 无 present） |
| RXS-0143 | D3D12 committed resource import ABI 与 present pass 布局 | ABI size/offset 单测 + 带 GPU smoke（allocation_size / mapping_size / LUID / RGB 通道与像素对照） |

- **错误码策略（Q4 裁决）**：三类编译期拦截走 rustc 原生诊断，不新增 RX 码。运行期诊断归 7xxx 段位，从 RX7020 起按实现中真实可达、用户可行动的错误类别分配；**RFC 不预留号码、不预造码数**。原始 `CUresult` / `HRESULT` 保留为 detail，不直接充当稳定 RX 码。`registry/error_codes.json` 只追加并同时落 en/zh message-key。
- spec PR 先于实现 PR（AGENTS 硬规则 7）；trace_matrix 维持全锚定。

## 6. feature gate / tracking / 实现序（10 §3 要件）

- **feature gate（Q4 裁决）**：cargo feature 固定为 `d3d12-interop`；未启用时不编译/链接 D3D12 shim。tracking 清单随实现 PR 维护（实现状态/未决问题/测试清单）。
- **栈式 PR**（对齐 M8.3 #51→#52；门控于本 RFC 合入后，从 `origin/main` 切 `feat/g1.1-interop`）：
  - **PR-1 spec 脚手架**：`spec/interop_d3d12.md` 登记文件名 + RXS-0140 预留区间（不落裸条款头）+ README §4 行 + 修订行；`trace_matrix --check` PASS。
  - **PR-2 interop 核心**（步骤 40）：条款体 + 测试锚定；rurix-rt 生成式 brand、`ExternalBuffer`/`ExternalSemaphore`、frame typestate 与 `cuImportExternal*` 绑定；新 `src/rurix-d3d12` shim crate（build.rs + cc + .cpp）；ABI size/offset 测试；conformance reject + UI golden；运行期错误码（如需）+ en/zh；`ci/d3d12_interop_smoke.py` + `milestones/g1/d3d12_interop_evidence_schema.json` + check_schemas 路由 + workflow 步骤 40；写 `evidence/d3d12_interop_*.json`（`interop_ok=true` + D-233 环境画像）→ `g1.counter.d3d12_interop≥1`。
  - **PR-3 实时呈现**（步骤 41）：G0 `sr_*` kernel 语义与源码 0-byte；新增 shim 私有 fullscreen present pass 与 `uc03-demo --present`；`ci/realtime_present_smoke.py` + `realtime_present_evidence_schema.json` + 步骤 41；写 `evidence/realtime_present_*.json`（`present_ok=true`）→ `g1.counter.realtime_present≥1`；无窗口/显示 → 降级 SKIP(exit 0)。
- **真实红绿**（反 YAML-only，CI_GATES §6）：PR-2 篡改 interop 同步时序 / 放行跨 context → 红 → 复原绿；PR-3 present 同步缺失 / 帧像素篡改 → 红 → 复原绿；run URL 归档。
- **NVIDIA 再分发**：D3D12/DXGI 系 Windows SDK 系统组件，**不受 NVIDIA 再分发白名单约束**（G1_CONTRACT §5 / CI_GATES §5.4）。
- **runner**：必须**交互桌面会话**跑（消费卡 WDDM，D3D12 窗口 + CUDA interop 不可用会话 0 服务）。
- **G0 不动**：`src/rurix-rt/kernels/sr_*.rx` 与 RXS-0118~0121 语义面 0-byte；present shader 是 shim 私有固定资产，不进入 Rurix 语言/API/stable 面。

## 7. 备选方案

- **D3D12 FFI 实现**：薄 C/C++ shim（cc + build.rs，**已采纳**，owner 裁决）vs windows-rs crate（把大依赖引入语言侧，与「不进语言」及最小依赖文化有张力）vs Rust 手写 COM vtable FFI（unsafe 量大、易错）。
- **判档**：Full RFC 前置（**已采纳**，owner 裁决）vs 对齐 M8 Direct+Mini（被否：本设计触 FFI ABI/内存模型禁区，争议向上取严）。
- **import 形态**：采纳 shared committed `D3D12_RESOURCE` + dedicated import；否决共享 heap（G1.1 仅一个逻辑 buffer，heap 会扩大 offset/alignment/placed-resource 安全包络）。
- **呈现形态**：采纳共享 `f32 RGB` buffer + shim 私有 fullscreen present pass；否决直接共享 swapchain backbuffer（swapchain 资源不作为本 RFC 的 CUDA import 契约）与 host 往返 pack（违 P-05/零拷贝目标）。
- **context 身份**：采纳 `scope` 生成式 brand；否决仅靠普通 `'ctx` 生命周期或 `Arc` 指针身份声称“编译期跨 context 拦截”（前者不能区分同作用域内两个值，后者只能运行期比较）。

## 8. 不做（范围红线）

G2 原生 D3D12+DXIL（D-131）；AsyncBuffer 流序分配 / Graph API（G1.2，非本子里程碑）；多后端（D-008/SG-003）；Tensor Core 等高级 intrinsics（SG-001/002）；包 registry（SG-007）；多 GPU/VMM。

## 9. Q1~Q5 裁决结果

- **Q1**：采用 4 位 RFC 编号与文件名：`RFC-0001` / `rfcs/0001-cuda-d3d12-interop.md`；编号永不复用。
- **Q2（🔒）**：采用 §4.2 的版本化扁平 C ABI；shim 拥有窗口/消息泵/D3D12 对象；共享 committed `D3D12_RESOURCE` + dedicated CUDA import；临时 NT HANDLE 在 import 后立即关闭；销毁序按 §4.4。
- **Q3（🔒）**：采用单 fence、严格递增偶/奇值 handoff；只承诺同 adapter、同进程、整块资源在配对 fence 边界的排他访问与完成顺序；不扩张 System atomics、跨进程/跨 GPU 语义；WDDM/TDR 只进入进度/失败与环境证据边界。
- **Q4**：feature gate = `d3d12-interop`；编译期错误零新 RX 码；运行期从 RX7020 起按真实实现按需追加，不预留、不预造。
- **Q5**：采用 `D3D12Presenter::scope` 生成式 brand + `ReadyFrame::wait → AcquiredFrame::signal → PresentableFrame::present` 消费式 typestate；私有 stream 被状态对象捕获，调用方不能替换 signal stream。

技术问题已全部收敛，owner 已批准全文。若当前治理阶段要求开源后的 FCP-lite 额外评审与 5–7 天等待窗，该流程仍需独立完成；本批准记录不虚构尚不存在的第二位评审。

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：实现于 feature gate 后 → tracking → 两里程碑无重大修订 → stabilization report → FCP-lite。stable API 面冻结随 **RD-008**（open，G1）届时定义；本 RFC 引入的 affine 类型在首个 stable 发布前不进 stable 面。
- **Provenance**：`Assisted-by: claude-code:claude-opus-4-8`；`Assisted-by: codex:gpt-5`。Draft v0.2 依据仓库现有 runtime 形态、CUDA 13.3 Driver API/Programming Guide、Microsoft D3D12 文档与 NVIDIA `simpleD3D12` 样例收敛。owner 于 2026-06-18 明确批准全文；该决定由 Codex 代录，AI 未自行裁决或冒充 owner 署名。G1 close-out 等后续签署仍分别执行。

## 11. 规范与实现依据

- NVIDIA CUDA Driver API 13.3：External Resource Interoperability（descriptor ABI、mapped buffer 必须 `cuMemFree`、outstanding semaphore 操作完成后方可 destroy）：<https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__EXTRES__INTEROP.html>
- NVIDIA CUDA C++ Programming Guide：Direct3D 12 LUID 匹配、committed resource dedicated import、共享 fence signal/wait：<https://docs.nvidia.com/cuda/cuda-c-programming-guide/>
- Microsoft：`ID3D12Device::CreateSharedHandle` / `CreateCommittedResource` / command queue fence 同步：
  - <https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device-createsharedhandle>
  - <https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device-createcommittedresource>
  - <https://learn.microsoft.com/en-us/windows/win32/direct3d12/user-mode-heap-synchronization>
- NVIDIA CUDA Samples：`simpleD3D12`（LUID 配对、`GetResourceAllocationInfo`、D3D12 resource/fence import 与帧 fence 先例）：<https://github.com/NVIDIA/cuda-samples/tree/master/cpp/5_Domain_Specific/simpleD3D12>

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-06-18 | AI 起草初版（动机/通路/语言面 affine 类型提案 §4.1 + 禁区待裁清单 §4.2~4.4 + 下游条款映射/实现序/备选/未决问题）。**待 owner 人工落笔禁区章节并签署/FCP-lite**。 | Full RFC（Draft） |
| Draft v0.2 | 2026-06-18 | 收敛 Q1~Q5 与 🔒 拟绑定文本：4 位 RFC 编号；shared committed D3D12 resource + dedicated CUDA import；版本化 C ABI；LUID 配对；mapped pointer / semaphore / external memory / shim 强制销毁序；单 fence 偶/奇值 handoff；生成式 context brand；Ready→Acquired→Presentable typestate；`d3d12-interop` gate；RX7020+ 按需分配。修正 v0.1 两处过强假设：普通 `'ctx` 不足以证明 context 值身份，mapped external buffer Drop 不能只 destroy import handle。待 owner 逐节签署。 | Full RFC（Draft） |
| Owner approval | 2026-06-18 | owner 在本工作会话明确确认 RFC 全文并授权记录批准；Codex 仅将该人工决定写回文档，不作 AI 代签。批准覆盖 Q1~Q5 与 §4.2~§4.4 🔒 禁区。FCP-lite 额外评审/等待窗若适用，仍按治理规则独立完成。 | Full RFC（Owner Approved） |
