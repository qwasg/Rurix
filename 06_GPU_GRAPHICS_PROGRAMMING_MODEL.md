# 06 — GPU 与图形编程模型

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 主要输入：r4（Driver API 运行时）、r5（PTX 内存模型）、r2（NVPTX 链路）、r11（基准）、H02（上一项目图形架构经验）
> 关联决策：D-002（图形分期）、D-120 ~ D-131（见 [13](13_DECISION_LOG.md)）

---

## 1. 执行模型总览

Rurix 的 GPU 执行模型直接对齐 CUDA/PTX 的 SIMT 现实，不做抽象稀释（P-02/P-05）：

- **执行层级**：`Grid → Block (→ Warp) → Thread`，作为类型层实体（[05](05_LANGUAGE_ARCHITECTURE.md) §3.2）。MVP 不暴露 cluster（Hopper+ 能力，登记为能力位扩展点）。
- **提交模型**：host 经 `Stream` 提交 kernel launch 与内存操作；stream 内 FIFO 有序，stream 间并发须经 `Event` 显式连接。
- **目标基线**：`compute_89`（Ada / RTX 4070 Ti，本机），PTX 经 `cuModuleLoadDataEx` JIT 装载（r2 的 MVP 链路）。

```
host (Rurix)                          device (Rurix kernel 子语言)
──────────────                        ─────────────────────────────
Context ── Module ── Kernel<F>        kernel fn f(grid, views...)
   │          │                          │ block/thread 索引（类型化）
 Stream ── launch(k, dims, args) ───▶    │ shared let / views / barrier
   │                                     │ scoped atomics
 Event ◀── record/wait ──────────────    └ device fn 调用（内联）
```

## 2. Kernel 抽象

### 2.1 形态（D-120）

kernel 是普通的 Rurix 函数加 `kernel` 着色，与宿主代码同模块、同类型系统、同泛型机制：

```rurix
kernel fn saxpy(t: ThreadCtx<1>, a: f32,
                x: View<global, f32>, mut y: View<global, mut f32>) {
    let i = t.global_id();              // 类型化索引，非裸 u32
    if i < y.len() {
        let yi = y.per_thread(i);       // 收窄到本线程分片
        *yi = a * x[i] + *yi;
    }
}
```

- `ThreadCtx<DIM>` 打包 grid/block/thread 索引与维度查询，下译为 `llvm.nvvm.read.ptx.sreg.*`（r2）。
- 单态化的每个 kernel 实例产出独立 PTX 符号，调用约定 `ptx_kernel`，launch bounds 经属性下译为 `nvvm.maxntid` 等（r2）。
- **MVP 范围限定**（与 r2 的第一阶段范围对齐）：1D/2D/3D grid/block；POD 标量 + view 参数；无动态并行、无 cooperative groups、无 cluster、无 Tensor Core intrinsic（全部登记 spike gating，触发条件见 [14](14_ENGINEERING_DISCIPLINE.md) §7）。

### 2.2 device fn 与控制流

- `device fn` MVP 默认强制内联（无设备调用栈管理）；递归禁止（编译错误）。
- 循环/分支无限制，但编译器对明显的 warp divergence 模式提供 lint（非错误）。
- barrier 可达性规则：`block.sync()` 调用点必须对 block 内全部线程**一致可达**（uniform control flow 检查，MVP 做保守版本：禁止在依赖 thread id 的分支内调用 barrier，违例须 unsafe）。这是 r5 列入 MVP 静态保证的"防 divergence deadlock"。

## 3. 内存空间与显式数据移动（D-121）

默认路径照搬 r4 的"最可控"结论：**显式 H2D/D2H + pinned staging，UM/零拷贝 opt-in**。

| 操作 | API 草图 | 底层 | 说明 |
|---|---|---|---|
| 设备分配 | `ctx.alloc::<T>(n)` → `DeviceBuffer<T>` | `cuMemAlloc` | affine，context-brand |
| 锁页主机内存 | `ctx.alloc_pinned::<T>(n)` → `PinnedBuffer<T>` | `cuMemAllocHost` | 异步拷贝的前提；文档警示过量 page-lock 损害系统（r4） |
| 拷贝 | `stream.copy(src, dst)` | `cuMemcpyAsync` | 方向由类型决定（Pinned→Device = H2D），无方向枚举可写错 |
| 映射主机内存 | `MappedBuffer<T>`（opt-in feature） | `cuMemHostRegister` | 零拷贝场景 |
| 托管内存 | `ManagedBuffer<T>`（opt-in feature + Windows 语义警示） | `cuMemAllocManaged` | r4：Windows native 无 full managed support，文档强制标注性能/语义差异 |
| 流序分配 | `AsyncBuffer<'stream, T>`（**G1 阶段引入**） | `cuMemAllocAsync` | 生命期 = 流时序区间，类型携带分配 stream（见 §5.4） |

MVP 只交付前四行 + ManagedBuffer 的 opt-in 形态；流序分配类型化是 G1 任务（D-122：先把经典路径做对，流序语义复杂度高、CUDA.jl #780 的混用事故为证，r4）。

## 4. 同步模型与 PTX 内存模型映射（D-123）

这是 Rurix 设备语义最深的设计点。r5 的结论：**源语言不能把"Rust Send/Sync + C++ atomics"直接当设备语义**，必须显式设计到 PTX scope/order 的映射层。

### 4.1 三层同步原语

| 层 | 原语 | PTX 下译 |
|---|---|---|
| 结构化（safe） | `block.sync()`（block barrier）；kernel 边界（grid 级同步 = 分 kernel） | `bar.sync` / launch 边界 |
| scoped 原子（safe，受限） | `Atomic<T, Scope>`：`fetch_add/cas/...`，`Scope ∈ {Block, Gpu, System}`，order ∈ {Relaxed, Acquire, Release, AcqRel} | `atom.{order}.{scope}` |
| 弱序协议（unsafe） | `fence(scope, order)`、volatile/mmio 访问、自定义自旋/队列协议 | `fence.sc/.acq_rel` 等 |

### 4.2 映射层规范（写入 spec 的核心条款）

- 源语言的 `Atomic<T, Scope>` 操作保证编译为满足 **morally strong** 条件的 PTX 指令对（同 proxy、scope 双向包含、完全重叠——r5/PTX ISA 的公理前提），从而获得原子性与 SC-per-location。
- safe 代码中不可能构造 mixed-size 冲突访问（类型系统保证同一位置的访问类型一致）——避开 PTX 公理未覆盖区。
- proxy（tex/generic）差异 MVP 不暴露：safe 层全部走 generic proxy；纹理路径（G2）引入时再扩展映射条款。
- `System` scope 原子在 WDDM 下的语义按官方文档保守声明，跨设备/跨进程一致性不做承诺（登记为 G1+ 验证项）。
- 数据竞争定义：safe 代码经 views + barrier + scoped atomics 构造的程序**无数据竞争**（这是 borrow checker + barrier 检查的可靠性命题，conformance 测试锚定）；unsafe 代码竞争语义遵循 PTX 公理（uniform-size race 有界约束，mixed-size 无约束），照搬入 spec 并标注 UB 边界。

### 4.3 host-device 同步

- `Event<'ctx>`：`stream.record(event)` / `stream.wait(event)` / `event.synchronize()`；record 与 wait 的 context 一致性由类型保证（[05](05_LANGUAGE_ARCHITECTURE.md) §4）。
- `stream.synchronize()`：阻塞 host；基准纪律要求计时区前后强制刷队列（WDDM batch 扭曲计时，r11 → [14](14_ENGINEERING_DISCIPLINE.md) §5 的基准协议）。
- 计时统一 CUDA Event（r11）；`Stream::timed_scope` API 内置正确计时模式。

## 5. 资源模型与运行时对象（语言面）

完整运行时实现见 [08](08_RUNTIME_AND_TOOLING.md)；语言面承诺：

1. **Device**（`Copy` 标识）→ **Context**（affine 根）→ Stream/Module/Buffer/Event（context-brand affine 资源）的层级与 [05](05_LANGUAGE_ARCHITECTURE.md) §4 一致。
2. **Module/Kernel 装载**：MVP 形态——编译器把 PTX 嵌入可执行文件 data 段，运行时 `ctx.load_module(embedded::MODULE)?` 显式装载；`module.kernel::<f>()` 取强类型 kernel 句柄。lazy/eager 装载策略跟随 `CUDA_MODULE_LOADING` 并可显式覆盖（r4）。
3. **错误**：全部 Driver API 错误映射为结构化 `enum CudaError` + 上下文链；`CUDA_ERROR_ASSERT`/`CONTEXT_IS_DESTROYED` 触发 context poisoned 状态，poisoned context 上的后续操作返回确定性错误而不是 UB 级联（r4 的"整块重建"语义类型化）。
4. **环境画像**：`ctx.environment()` 返回驱动版本/WDDM-TCC-MCDM/HAGS/TDR 配置快照（P-04/P-14），供程序与基准 harness 消费。

### 5.4 流序分配的类型契约（G1 设计预留）

`AsyncBuffer<'stream, T>` 三规则（r4 的时序契约类型化）：分配操作完成前的访问被 stream 顺序天然排除；释放后访问 = 编译期生命周期错误；跨 stream 使用必须 `buf.share_with(other_stream, event)` 显式建立时序边。设计在 G1 实现并以 Compute Sanitizer 回归锁定（CUDA.jl #780 事故类别的永久回归项）。

## 6. 图形路线总览（D-002 展开）

```
G0 (MVP 内)          G1 (MVP+6mo 量级)        G2 (3年期)
compute 软光栅   →   CUDA–D3D12 interop   →   原生 D3D12 + DXIL
出图通道(离屏)        实时呈现(窗口)             完整图形管线
零图形 API 依赖       D3D12 仅作 present        raster/RT/mesh 语言建模
```

分期理由（D-002 已批准）：图形 API + shader codegen 超出 MVP 红线（H06 §5）；但图形是项目愿景，必须保证每一阶段都有"出图"能力反哺动力与演示价值，且语言设计预留图形语义扩展点（本节 §8）。

## 7. G0：compute 软光栅（MVP 内）

**定位**：纯 CUDA kernel 实现的离屏渲染演示——不是产品级渲染器，是三件事的载体：(a) MVP 验收 demo（UC-03：SPH 仿真 + 软光栅出图）；(b) views/shared/原子的真实压力测试场；(c) 性能验收 L3 级 mini-app（r11 分层）。

**技术形态**：

- 顶点变换 kernel → 三角形 binning kernel（tile 化，scoped atomics 压力点）→ per-tile 光栅化 + 深度测试 kernel（shared memory tile，views 压力点）→ tone map kernel → `D2H` 拷出 → PNG/BMP 写盘。
- 全部 safe 代码可达为目标；做不到的点位是安全包络的真实反馈，记入 deferred 注册表并反推 views 算子集是否要扩。
- 不做：纹理采样硬件路径（无 tex proxy）、MSAA、任何窗口系统交互。

## 8. G1/G2：图形管线的语言建模（设计预留，非 MVP 承诺）

### 8.1 G1：CUDA–D3D12 interop 呈现（D-130）

- 通路：D3D12 创建 swapchain + 共享堆 → `cuImportExternalMemory` / `cuImportExternalSemaphore` 把 backbuffer 等价纹理映射为 CUDA 资源 → Rurix kernel 写入 → 信号量同步 present。
- 语言面新增：`ExternalBuffer` / `ExternalSemaphore` affine 类型（import 句柄生命周期 + 信号时序的类型化）；D3D12 侧以薄 C FFI 驱动，不进语言。
- 价值：在不做 shader codegen 的前提下获得实时窗口呈现，软光栅 demo 升级为交互式；验证 Windows 图形互操作的全部驱动现实。

### 8.2 G2：原生 D3D12 + DXIL（D-131，3 年期）

语言扩展的设计预留（现在定方向、不定细节，避免 G2 时推翻 MVP 决策）：

1. **着色阶段 = kernel 着色的扩展**：`vertex fn` / `fragment fn` / `compute fn`（D3D12 语境）/ `mesh fn` / `task fn` / RT 阶段（`raygen/closesthit/anyhit/miss`）复用 kernel 子语言的类型系统与 views，各自附加阶段专属的输入/输出语义类型（插值限定、内建变量类型化）。
2. **codegen 第二后端**：MIR → DXIL（经 LLVM DirectX target 或 SPIR-V→DXIL 路径，G2 启动时重评估——LLVM DirectX 后端成熟度是当时的关键输入；此为 13 号文档登记的"未来agent决策" D-131）。
3. **绑定模型**：descriptor/root signature 由编译器从 kernel 签名推导生成，双侧（host 结构体 ↔ shader 布局）单一事实源（P-11；直接消灭 U4 痛点 #1）。
4. **资源状态/barrier**：pass 间依赖显式建模（上一项目 RenderGraph 的 hazard 推断经验语言化，H02 §4）；barrier 自动推导作为库级 render graph 的职责，语言提供资源状态类型。
5. **PSO/管线对象**：affine 资源 + 编译期已知的状态描述 const 化；PSO cache 持久化是运行时职责（上一项目已验证模式）。
6. **纹理/采样器**：`Texture2D<F>` 等格式参数化类型 + sampler 类型；tex proxy 的内存模型条款扩展（§4.2 预留点）。

### 8.3 引擎级工作流（U5 服务承诺）

语言不内置 render graph/ECS——它们是库。语言的职责是让这些库可以被安全地写出来：affine 资源 + 生命周期 brand + `Record` derive（状态镜像生成）+ C ABI（嵌入现存引擎）。G2 后期的"最小 RHI + render graph"对照实验（UC-05）是此承诺的验收形式。

## 9. NVIDIA/Windows-first 假设清单（显式登记）

以下假设允许全设计链路引用（变更须走 Full RFC）：

| 编号 | 假设 | 来源 |
|---|---|---|
| A-01 | 目标 GPU 为 NVIDIA，SM ≥ 8.9 为 MVP 基线，PTX 前向兼容承担向新硬件迁移 | r2 |
| A-02 | 目标 OS 为 Windows 11 x64；驱动模型默认 WDDM（GeForce 无 TCC） | r4/r6 |
| A-03 | CUDA Toolkit 13.x、驱动单独安装（13.1+ 现实）；探测优先 NVML | r6 |
| A-04 | 开发机基准硬件：RTX 4070 Ti（上一项目本机延续，能力快照已有：`VP_VULKANINFO_*.json` 对照基线） | H05 |
| A-05 | 图形 API 路线 D3D12（非 Vulkan）——Windows-first 自洽 + 上一项目 Vulkan-on-Windows 驱动黑洞实证 | D-002/H04 |
| A-06 | 单机单 GPU 是 MVP 语义边界；多 GPU/NVLink/MIG 全部 G1+ | 范围红线 |

## 10. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
