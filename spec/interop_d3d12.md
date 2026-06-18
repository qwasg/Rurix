# Rurix 语言规范 — CUDA–D3D12 互操作呈现语义面（`ExternalBuffer` / `ExternalSemaphore` / 实时窗口呈现；G1.1 起）

> 条款:RXS-0140 起续号预留(G1.1 CUDA–D3D12 互操作呈现语义面:`ExternalBuffer`/`ExternalSemaphore` affine 类型与 import 句柄生命周期 / 生成式 context brand 与跨 context 编译期拦截 / `Ready→Acquired→Presentable` typestate 与共享 fence 偶奇值 handoff / D3D12 committed resource import ABI 与 present pass 布局)。体例见 [README.md](README.md)。
> 依据:**[RFC-0001](../rfcs/0001-cuda-d3d12-interop.md)**(CUDA–D3D12 interop 与软光栅实时窗口呈现,**owner 已批准定稿**,2026-06-18);06 §8.1(D-130:D3D12 创建 swapchain + 共享堆 → `cuImportExternalMemory`/`cuImportExternalSemaphore` 映射 backbuffer 等价纹理 → Rurix kernel 写入 → 信号量同步 present;`ExternalBuffer`/`ExternalSemaphore` affine 类型,D3D12 侧薄 C FFI 不进语言);06 §4.2(内存模型映射边界);06 §6(三阶段图形路线 G0→G1→G2);08 §1/§2(`rurix-rt` 运行时对象 / Driver API 薄层,D-230);spec/softraster.md:153(实时窗口呈现归 G1-1);01 §6(每阶段必有出图)。授权:[../milestones/g1/G1_CONTRACT.md](../milestones/g1/G1_CONTRACT.md)(`in_scope: d3d12_interop` / `realtime_present` / `spec_g1_clauses`,D-G1-1,G-G1-1 / G-G1-6)+ [../milestones/g1/G1_PLAN.md](../milestones/g1/G1_PLAN.md) §1 G1.1 第 1 项。
> 档位:**Full RFC**(RFC-0001;10 §3:本设计触 FFI ABI / 运行时语义 / unsafe 边界 / 内存模型映射,AGENTS 硬规则 5 / 10 §7.5 禁区,只能人类经 Full RFC 落笔)。RFC-0001 已由 owner 于 2026-06-18 人工落笔/批准 🔒 禁区章节(§4.2 FFI ABI / §4.3 内存模型映射·信号时序 / §4.4 安全包络 + Q1~Q5);spec 条款 PR 与实现 PR 均门控于 RFC-0001 合入之后。**AI 无权自判 Direct**,判档以 RFC-0001 与 G1_CONTRACT 授权为据,判档争议向上取严。任何偏离 RFC-0001 已批准设计、或触及 **G2 原生 D3D12+DXIL(D-131)** / **多后端(D-008/SG-003)** / **Python 原生嵌入(红线 1,SG-008)** / **const 泛型值运行期单态化(RD-007)** / **device 原子 lowering(D-406/RD-008)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):import 句柄生命周期 / 跨 API 信号时序 / external 资源所有权以 **affine 所有权 + 生成式 brand + typestate + 确定性诊断**定义,不以 UB 表述(RFC-0001 §4.3/§4.4)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.25 interop.md / v1.29 pipeline.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0140 起)与每条 ≥1 测试锚定随 G1.1 实现 PR(步骤 40)同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **CUDA–D3D12 互操作呈现语义面**的语义条款(G1.1+,D-G1-1)。覆盖语义面(RFC-0001 §4):

- **`ExternalBuffer<'ctx, T>` / `ExternalSemaphore<'ctx>` affine 类型与 import 句柄生命周期**:import 自 D3D12 共享 committed resource / 共享 fence 的 affine(move-only、非 `Copy`/非 `Clone`)资源;Drop 强制销毁序(mapped pointer `cuMemFree` → `cuDestroyExternalSemaphore` → `cuDestroyExternalMemory` → shim 销毁;D3D12 COM owner 不被 CUDA wrapper 释放)。镜像 `src/rurix-rt` 既有 `DeviceBuffer`/`InFlight` affine 纪律。
- **生成式 context brand 与跨 context 编译期拦截**:`D3D12Presenter::scope` 以高阶 `for<'ctx>` 闭包生成不可伪造、不可逃逸的新鲜 `'ctx`;interop context / stream / module / external memory / semaphore 全携不变 brand;两个独立 `scope` 的资源类型不同,跨 context 误用为编译期类型/借用错误(不依赖普通 `'ctx` 生命周期或 `Arc` 指针运行期身份)。
- **`Ready→Acquired→Presentable` typestate 与共享 fence 偶/奇值 handoff**:消费式状态机——`ReadyFrame::wait`(CUDA `cuWaitExternalSemaphoresAsync(2n)`)→ `AcquiredFrame`(暴露可写 backbuffer + `launch`)→ `signal`(CUDA `cuSignalExternalSemaphoresAsync(2n+1)`)→ `PresentableFrame::present`(shim queue wait `2n+1` → present pass → signal `2n+2`)→ `ReadyFrame`;未 wait 无可写 buffer、未 signal 无 `present`;私有 stream 被状态对象捕获,wait/launch/signal 同 stream 序。
- **D3D12 committed resource import ABI 与 present pass 布局**:`D3D12_RESOURCE` + `CUDA_EXTERNAL_MEMORY_DEDICATED` 导入(否决 HEAP);共享 buffer 固定行主序紧密 `f32 RGB`、分量 `0…255`(与 RXS-0121 `sr_tonemap` `ViewMut<global, f32>` 输出逐字节同义);shim 私有 fullscreen present pass 读 buffer 写 `R8G8B8A8_UNORM` backbuffer(`/255`),非 Rurix shader codegen、不扩张 G2。

全部 D3D12/DXGI 经**薄 C/C++ shim**(`cc` + `build.rs`,COM 留 C++,Rust 仅见版本化扁平 `extern "C"`,RFC-0001 §4.2)驱动,**不进语言**(D-130);device 分发维持 **PTX-only**(07 §7);G0 软光栅 kernel(`src/rurix-rt/kernels/sr_*.rx`,RXS-0118~0121)语义面 **0-byte**,仅新增呈现通路。import 句柄生命周期 / 跨 API 信号时序 / external 资源所有权以 **affine 所有权 + 生成式 brand + typestate + 确定性诊断**定义,**不以 UB 表述**(§4)。

**编号区间**:本文件条款自 **RXS-0140** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0139 @ [release.md](release.md))。本轮(实现 PR,步骤 40)落地 **RXS-0140 ~ RXS-0143** 带编号条款体 + 每条 ≥1 `//@ spec: RXS-####` 测试锚定(`src/rurix-rt/src/interop.rs`;trace_matrix 维持全锚定)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(10 §7.5;import 句柄生命周期 / 跨 API 信号时序 / external 资源所有权以 affine 所有权 + 生成式 brand + typestate + 确定性诊断定义,RFC-0001 §4.3/§4.4)。Legality 三类编译期拦截由 **rustc 原生诊断**拦截(无新 RX 码,§3);运行期诊断为 [`InteropError`] Rust 值(RX7020+ 按需,§3)。

### RXS-0140 `ExternalBuffer` / `ExternalSemaphore` affine 类型与 import 句柄生命周期

**Syntax**(`src/rurix-rt` interop,feature `d3d12-interop`):

```
ExternalBuffer<'ctx, T: Copy>   // CUexternalMemory import + mapped CUdeviceptr,'ctx 不变 brand
ExternalSemaphore<'ctx>         // CUexternalSemaphore import,'ctx 不变 brand
```

**Legality**:

- `ExternalBuffer` / `ExternalSemaphore` 为 **affine**(非 `Copy`/非 `Clone`、单一所有权);move 后再用 → `E0382`;试 `.clone()` → `E0599`(rustc 原生,无 RX 码)。
- 唯一安全构造入口为 [`scope`](RXS-0141);**无 public `from_raw_handle`**(RFC-0001 §4.4),外部不承担「裸 HANDLE 是否有效」证明义务。

**Dynamic Semantics**:

- `ExternalBuffer` 经 `cuImportExternalMemory`(`CU_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE` + `CUDA_EXTERNAL_MEMORY_DEDICATED`)+ `cuExternalMemoryGetMappedBuffer` 映射 D3D12 共享 committed resource 为 CUDA 设备地址;`ExternalSemaphore` 经 `cuImportExternalSemaphore`(`..._D3D12_FENCE`)。
- **Drop 强制销毁序**(RFC-0001 §4.4):mapped pointer `cuMemFree` → `cuDestroyExternalSemaphore` → `cuDestroyExternalMemory` → shim destroy。`ExternalBuffer::Drop` 先 `cuMemFree(mapped)` 再 `cuDestroyExternalMemory`;**不释放 D3D12 resource**(COM owner 留 shim 侧,对齐 RXS-0124 借用缓冲 owned=false 纪律)。单一所有权使 Drop 仅一次(不双重释放)。

**Implementation Requirements**:

- import/map/signal/wait/destroy 为 FFI unsafe 边界,每块 `// SAFETY:` + `unsafe-audit/rurix-rt.md`(U17/U18)/ `unsafe-audit/rurix-d3d12.md` 注册;safe wrapper 对上全 safe(签名无 `unsafe`)。

> 锚定测试:`src/rurix-rt/src/interop.rs`(`external_resources_are_affine_move_only`:affine 类型存在性 + 单一所有权;`scope_reports_unavailable_outside_device_session`:无设备/stub 确定性不可用,不 panic/不 UB)。

### RXS-0141 生成式 context brand 与跨 context 编译期拦截

**Syntax**(唯一安全入口):

```
D3D12Presenter::scope<R>(ordinal, render, window,
    f: impl for<'ctx> FnOnce(InteropContext<'ctx>, ReadyFrame<'ctx>) -> Result<R>) -> Result<R>
```

**Legality**:

- `scope` 以高阶 `for<'ctx>` 闭包生成**不可伪造、不可逃逸的新鲜不变 brand** `'ctx`(`PhantomData<fn(&'ctx ()) -> &'ctx ()>`,invariant);interop context / module / kernel / buffer / semaphore / frame 全携同一 brand。
- 两个独立 `scope` 的 `'ctx` 不可统一——跨 scope 混用资源 / 句柄逃逸闭包 = 编译期类型/借用错误(不依赖普通 `'ctx` 生命周期或 `Arc` 指针运行期身份,RFC-0001 §4.1)。
- `InteropContext<'ctx>` 为 `!Send + !Sync`(current context 线程绑定)。

**Dynamic Semantics**:

- `scope` 内:`cuDeviceGetLuid` → 同 LUID adapter 上经 shim 建 D3D12 device/swapchain/共享 resource·fence → import external memory/semaphore(临时 NT HANDLE import 后立即 `close`)→ 构造同 brand `InteropContext` + `ReadyFrame` 交闭包;返回后按 §4.4 销毁。
- launch 实参经 `InteropKernelArg<'ctx>` 密封类型化(携值,**无可脱离裸指针**),只能绑同 brand frame 的 `AcquiredFrame::launch`。

**Implementation Requirements**:

- LUID 配对失败 → `InteropError::LuidMismatch`;driver 无 external-resource API → `InteropError::ExternalApiUnavailable`(均运行期 Rust 值,非 RX 码)。

> 锚定测试:`src/rurix-rt/src/interop.rs`(`interop_context_is_thread_bound_not_send`:`InteropContext` !Send brand 根)。跨 scope 混用的编译期拒绝由 conformance compile-fail 样例核对(随后续 PR 补 `conformance/interop_d3d12/reject/**`)。

### RXS-0142 `Ready → Acquired → Presentable` typestate 与共享 fence 偶/奇值 handoff

**Syntax**(消费式状态机):

```
ReadyFrame::wait(self)        -> Result<AcquiredFrame>      // CUDA wait acquire(n)=2n
AcquiredFrame::buffer_mut(&mut self) -> &mut ExternalBuffer  // 仅 Acquired 态暴露可写 backbuffer
AcquiredFrame::launch(&mut self, kernel, grid, block, args) -> Result<()>
AcquiredFrame::signal(self)   -> Result<PresentableFrame>   // CUDA signal cuda_done(n)=2n+1
PresentableFrame::present(self) -> Result<ReadyFrame>       // shim wait 2n+1 → present → signal 2n+2
```

**Legality**:

- 状态机消费式转移(每步 `self` by-value):未 `wait` 无可写 buffer 接口;未 `signal` 无 `present` 方法——跳过 wait/signal 即无对应态句柄,编译期拦截「信号时序违例」(rustc `E0599`/move,无 RX 码)。
- 私有 CUDA stream 被 frame 状态对象捕获;`signal(self)` 不接受 stream 参数,故 wait / launch / signal 必落同一 stream 序。

**Dynamic Semantics**(共享 fence 偶/奇值 handoff,RFC-0001 §4.3):

- 第 `n` 帧(0 起)仅用:`acquire(n)=2n`(CUDA 取写权 wait)/ `cuda_done(n)=2n+1`(CUDA 完成 signal)/ `d3d_done(n)=2n+2`(D3D12 present 后 signal 下一写权)。值 checked `+1/+2`、严格递增、永不 rewind/复用;溢出 → `InteropError::FenceOverflow`(确定停机)。
- API 序固定:CUDA `wait(2n)` → 同私有 stream 写 buffer 的 kernel → CUDA `signal(2n+1)` → (第 4 步 API 成功返回后)shim queue `wait(2n+1)`→present→`Present`→queue `signal(2n+2)` → (queue signal 入队后)下一帧 CUDA `wait(2n+2=acquire(n+1))`。

**Implementation Requirements**:

- 帧计数 `+1` 经 `checked_add`(溢出 → `FenceOverflow`);跨 API 可见性/同步序保证边界见 RFC-0001 §4.3(同 adapter/同进程/整块资源在 fence 边界排他,不扩张 06 §4.2 System 原子语义)。

> 锚定测试:`src/rurix-rt/src/interop.rs`(`fence_handoff_even_odd_protocol`:`2n`/`2n+1`/`2n+2` 值序 + 相邻帧连续;`fence_overflow_deterministic_stop`:溢出 → None 确定停机)。未 wait/未 signal 的编译期拒绝由 conformance compile-fail 样例核对(随后续 PR 补)。

### RXS-0143 D3D12 committed resource import ABI 与 present pass 布局

**Syntax**(版本化扁平 C ABI + descriptor 布局):

```
RxD3D12InteropExport { abi_version; struct_size; memory_handle; allocation_size;
    mapping_size; fence_handle; adapter_luid[8]; node_mask; render_/window_ w/h; channels; reserved[6] }  // 96 字节
```

**Legality**:

- 共享资源采纳 **committed `D3D12_RESOURCE`**(否决 HEAP)+ `CUDA_EXTERNAL_MEMORY_DEDICATED`(RFC-0001 §4.2.2)。
- shim C ABI 版本化(`abi_version == RX_D3D12_ABI_VERSION`、`struct_size == 96`);未知 `flags` 位 → `E_INVALIDARG`;对象固定创建线程(跨线程调用 → `RPC_E_WRONG_THREAD`)。

**Dynamic Semantics**:

- CUDA external-resource descriptor 以头文件 v1 布局 `#[repr(C)]` 复刻,Windows x64 大小:memory handle desc **104** / buffer desc **88** / semaphore handle desc **96** / signal·wait params **144** 字节;`RxD3D12InteropExport` **96** 字节——均由编译期 `const assert!` + 单测核对(RFC-0001 §4.2.3)。
- 共享 buffer 固定行主序紧密 `f32 RGB`、分量 `0…255`(与 RXS-0121 `sr_tonemap` `ViewMut<global, f32>` 输出逐字节同义);shim 私有 fullscreen present pass 读 buffer `/255` 写 `R8G8B8A8_UNORM` backbuffer(非 Rurix shader codegen,不扩张 G2,RFC-0001 §4.2.2)。
- `allocation_size` = `GetResourceAllocationInfo.SizeInBytes`(CUDA import size);`mapping_size` = `render_w·render_h·3·4`(mapped buffer size)。

**Implementation Requirements**:

- ABI 结构不得 `#pragma pack`;C++ `static_assert(sizeof(RxD3D12InteropExport)==96)` 与 Rust `const assert!` + size/offset 单测双向核对。`CreateSharedHandle` 两 NT HANDLE 在 `create` 成功后移交 Rust wrapper,import 后各 `close` 恰好一次(CUDA import 不接管 HANDLE 所有权)。

> 锚定测试:`src/rurix-rt/src/interop.rs`(`external_resource_descriptor_abi_sizes`:104/88/96/144 + export 96 ABI 大小核对);`src/rurix-d3d12`(`interop_export_abi_layout`:export 96 字节 + 关键字段偏移)。带 GPU/窗口的数值/像素对照随步骤 40/41 设备真跑回填。

## 3. 错误码引用汇总（新段位说明 — 脚手架不预造）

> 三类编译期拦截(句柄生命周期 / 跨 context / 信号时序违例)由 **Rust 类型系统原生编译期诊断**拦截(affine move `E0382` / 生成式 brand 不匹配 / typestate 方法不存在 `E0599`),**不新增 RX 段位码**(对齐 M8.3 pipeline.md §3 零新码先例,RFC-0001 §5)。

**运行期诊断**(import 失败 / device LUID mismatch / shim 初始化失败 / fence 溢出等)按 07 §5 在 **7xxx 段位续接**(最高现存 RX7019 @ cublas;从 **RX7020** 起按实现中真实可达、用户可行动的错误类别分配,**脚手架不预留号码、不预造码数**);含义冻结(10 §6,`check_error_codes` 延续),`registry/error_codes.json` 只追加并同时落 [../src/rurixc/src/messages/en.messages](../src/rurixc/src/messages/en.messages) + [../src/rurixc/src/messages/zh.messages](../src/rurixc/src/messages/zh.messages) 双语 message-key(bilingual_coverage 门)。原始 `CUresult` / `HRESULT` 保留为诊断 detail,不直接充当稳定 RX 码(RFC-0001 §5)。本表随实现 PR 落地回填。

## 4. 升档 / 禁区留痕

- **本文件档位 = Full RFC(RFC-0001)**:不同于 M8 互操作/cublas/pipeline 的 Direct(契约 rfc_required:none 授权的初版条款化)——本设计触 **FFI ABI / 内存模型映射(信号时序) / 安全包络**(AGENTS 硬规则 5 / 10 §7.5 禁区),owner 经 AskQuestion 裁决 **Full RFC 前置**,🔒 禁区由 owner 于 2026-06-18 人工落笔/批准(RFC-0001 §4.2/§4.3/§4.4 + Q1~Q5)。**AI 不自判 Direct**,判档争议向上取严。
- **G0 软光栅 kernel 语义面 0-byte**:`src/rurix-rt/kernels/sr_*.rx` 与 RXS-0118~0121 字节不动;present shader 是 shim 私有固定资产(构建期 HLSL→DXBC 嵌入),**不进入 Rurix 语言/API/stable 面**,不扩张 G2(RFC-0001 §4.2.2 / §6)。
- **G2 原生 D3D12+DXIL(D-131)**:着色阶段进语言 + DXIL codegen 第二后端 → G2,不在本文件;触及即停下标注「需人工升档」。
- **多后端 / Python 原生嵌入 / const 泛型值运行期单态化 / device 原子 lowering**:分别为 D-008/SG-003、红线 1/SG-008、RD-007、D-406/RD-008,均不在本文件互操作呈现语义面登记;触及即停下标注「需人工升档」。
- **UB 节禁区**:import 句柄生命周期 / 跨 API 信号时序 / external 资源所有权以 **affine 所有权 + 生成式 brand + typestate + 确定性诊断**定义,**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5;RFC-0001 §4.3 已落笔保守保证边界)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-18 | 新建 spec/interop_d3d12.md(G1.1 CUDA–D3D12 互操作呈现语义面起始文件,PR-1 脚手架):登记编号区间 RXS-0140 起续号预留(RXS-0140 ~ RXS-0143 计划)+ 文件级前言 / 范围(`ExternalBuffer`/`ExternalSemaphore` affine 类型与 import 句柄生命周期 / 生成式 context brand 与跨 context 编译期拦截 / `Ready→Acquired→Presentable` typestate 与共享 fence 偶奇值 handoff / D3D12 committed resource import ABI 与 present pass 布局;薄 C/C++ shim 不进语言、PTX-only、G0 kernel 0-byte、affine+生成式 brand+typestate 不设 UB)/ 依据与授权(RFC-0001 owner 批准 + 06 §8.1/§4.2/§6 + 08 §1/§2 + spec/softraster.md:153 + 01 §6;G1_CONTRACT D-G1-1 / G-G1-1 / G-G1-6 + G1_PLAN §1)/ 计划条款骨架(§2 预留,非裸条款头)/ 错误码新段位说明(§3:编译期 rustc 原生零新码;运行期 7xxx RX7020+ 随实现按需,脚手架不预造)/ 升档·禁区留痕(§4)。**沿 README v1.25 / v1.29 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 G1.1 实现 PR(步骤 40)同落,无体例变更 | **Full RFC**(RFC-0001) |
| v1.1 | 2026-06-18 | 落地带编号条款体 RXS-0140 ~ RXS-0143(G1.1 实现 PR-2,条款体随实现 + 测试锚定同落,去 §2「计划骨架」):RXS-0140 `ExternalBuffer`/`ExternalSemaphore` affine 类型与 import 句柄生命周期(move-only;Drop 销毁序 mapped `cuMemFree`→destroy semaphore→destroy memory→shim destroy;无 public `from_raw_handle`)/ RXS-0141 生成式 context brand 与跨 context 编译期拦截(`for<'ctx>` scope 不变 brand + 密封 `InteropKernelArg`,资源不可逃逸)/ RXS-0142 `Ready→Acquired→Presentable` 消费式 typestate 与共享 fence 偶/奇值 handoff(`2n`/`2n+1`/`2n+2`,checked 溢出确定停机)/ RXS-0143 D3D12 committed resource import ABI(`D3D12_RESOURCE`+`DEDICATED`,否决 HEAP)+ 版本化扁平 C ABI(96 字节 export)+ shim 私有 present pass 布局。每条 ≥1 `src/rurix-rt/src/interop.rs` 测试锚定(trace_matrix 139→143 全锚定)。§1 区间更新为 RXS-0140 ~ RXS-0143 落地;§3 错误码:三类编译期拦截 rustc 原生**零新 RX 码**(对齐 M8.3),运行期 RX7020+ 按需(脚手架不预造)。实现裁决:rurix-rt 新增 external-resource FFI(sys.rs,descriptor `#[repr(C)]` 编译期 size 断言 104/88/96/144)+ interop 模块(feature `d3d12-interop`,unsafe-audit U17/U18);新 `src/rurix-d3d12` 边界 crate(stub 默认绿 / `real-shim` C++ shim,unsafe-audit rurix-d3d12.md);G0 软光栅 kernel RXS-0118~0121 字节 0-byte。无体例变更 | **Full RFC**(RFC-0001) |
