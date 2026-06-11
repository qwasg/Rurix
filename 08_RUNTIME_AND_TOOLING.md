# 08 — 运行时与工具链

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 主要输入：r4（Driver API 运行时）、r6（Windows 工具链与分发）、r11（基准/Nsight/CUPTI）、r9（IDE 体验）
> 关联决策：D-230 ~ D-241（见 [13](13_DECISION_LOG.md)）

---

## 1. 运行时职责边界（D-230）

Rurix 运行时（`rurix-rt`，随工具链分发的静态库）是**薄层**：它管理 Driver API 对象生命周期、装载协商、错误映射与 telemetry，**不做**调度器、不做自动内存管理、不做隐式数据搬运（P-05）。

| 职责 | 不是职责 |
|---|---|
| Context/Stream/Module/Buffer/Event 的 RAII 封装与 affine 语义支撑 | GC / 引用计数自动回收设备内存 |
| 启动协商：驱动/Toolkit/PTX 版本、设备能力、WDDM 画像 | 跨设备自动负载均衡 |
| PTX 装载（`cuModuleLoadDataEx`）与 JIT 选项管理 | 运行时代码特化/重编译 |
| 错误码 → 结构化错误 + context poisoned 状态机 | 异常恢复语义（poisoned 即重建） |
| 内建 telemetry（计数器/NVTX/CUPTI Activity） | 完整 APM 系统 |

## 2. Driver API 对象模型（r4 工程化）

### 2.1 对象层级与线程语义（D-231）

```
GpuDevice (CUdevice, Copy 标识)
  └─ OwnedContext (cuCtxCreate, affine 根, Send + !Sync)
       ├─ Stream<'ctx>   (affine, Send)      — FIFO 队列
       ├─ Event<'ctx>    (affine, Send)      — 时序连接点
       ├─ Module<'ctx>   (affine)            — PTX 装载单元
       │    └─ Kernel<'m, F> (强类型句柄, Copy)
       ├─ DeviceBuffer<T>/PinnedBuffer<T>/... ('ctx brand)
       └─ MemoryPool / AsyncBuffer (G1)
```

- **current context 管理**：Driver API 的 current context 是线程局部状态（r4 核心陷阱）。运行时策略：每次 API 调用以 `cuCtxPushCurrent/PopCurrent` 包裹（guard 模式），不依赖调用方维持 current；热点路径（launch/copy）用"已是 current 则跳过"的快速判定。MVP 单 context 现实下此开销可忽略，计数器观测留作证据。
- **Primary context**：与外部 Runtime API 库（cuBLAS 等经 FFI 进入的世界）互操作时统一走 `cuDevicePrimaryCtxRetain` 租约模式（`PrimaryContextLease` 类型）；Rurix 自身 `OwnedContext` 与 primary context 不混用（r4：两套资源世界硬性禁止，文档 + debug 断言双保险）。
- **销毁纪律**：`OwnedContext::drop` 前置条件由借用检查保证（无存活子资源借用）；drop 实现先同步全部 stream 再 `cuCtxDestroy`，杜绝"销毁期间 API 在用"的 UB（r4）。

### 2.2 内存分配策略（D-232）

- MVP：经典三件套——`cuMemAlloc`（device）/ `cuMemAllocHost`（pinned）/ `cuMemHostRegister`（映射，opt-in）。无运行时内部池化：分配成本显式暴露给用户（P-05），池化是库层选择（[09](09_STDLIB_AND_ECOSYSTEM.md) §4 的 `BufferPool`）。
- G1：stream-ordered allocator（`cuMemAllocAsync` + `CUmemoryPool`）按 [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §5.4 的类型契约引入。
- VMM（`cuMemAddressReserve` 族）：大 arena/外部共享场景，G2 评估；Windows 共享句柄的 `LPSECURITYATTRIBUTE` 复杂度默认不暴露（r4）。

### 2.3 WDDM/TDR：一等环境条件（D-233，P-14）

启动时探测并构建**环境画像**（程序可查询、telemetry 自动附带、基准 harness 强制记录）：

| 项 | 来源 | 用途 |
|---|---|---|
| 驱动模型 WDDM/TCC/MCDM | NVML / `nvmlDeviceGetDriverModel` | 计时与提交行为预期；GeForce 默认 WDDM（r4） |
| HAGS 开关状态 | 注册表/DXGI | 性能 A/B 维度，不假设更快（r4） |
| TDR 配置（TdrDelay/TdrLevel） | 注册表 | 长 kernel 风险评估；运行时对单 kernel 预计 >100ms 的 launch 提供 lint 级警告通道（r11 拆分建议） |
| 驱动/Toolkit/NVML 版本 | NVML 优先，`nvidia-smi` 仅人工后备（输出格式不保证兼容，r6） | 装载协商与诊断 |

### 2.4 装载协商与 PTX 兼容（D-234）

装载 PTX 前的协商序列（消灭 Numba 式 `CUDA_ERROR_UNSUPPORTED_PTX_VERSION` 整类事故，r2）：

1. 解析嵌入 PTX 的 `.version` / `.target`；
2. 查询驱动支持的 PTX JIT 能力上限；
3. 不匹配 → 结构化错误 `RX7xxx`，给出"驱动需 ≥ X / 或用 --ptx-floor 重编"的可执行指引；
4. 匹配 → `cuModuleLoadDataEx`，JIT 日志缓冲常开（失败时日志进诊断）。
5. **明确边界**：Windows 不支持 CUDA Minor Version Compatibility（r6），协商逻辑不照搬 Linux 假设。

### 2.5 错误模型

- 全部 CUresult 映射为 `enum CudaError`（非穷尽枚举 + 原始码保留）。
- 异步错误现实（r4：可能在后续无关 API 上观察到）→ 错误附带"检测点 ≠ 起因点"的标注与最近 N 次操作的 telemetry 环形缓冲摘要。
- `CUDA_ERROR_ASSERT` / `CONTEXT_IS_DESTROYED` → context 进入 poisoned；poisoned 后全部操作返回 `Err(Poisoned)`，重建路径由 affine 类型引导（[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §5）。

## 3. 内建 Telemetry（D-235，P-07）

- **计数器**：运行时每类操作（alloc/free/copy/launch/sync/module-load）原子计数 + 计时桶；编译器侧对应 [07](07_COMPILER_ARCHITECTURE.md) §6。上一项目方法论 + H02 §5 的硬化规则：**每个计数器合入后 2 个里程碑内必须有非零真实证据，否则降级或删除**。
- **NVTX**：运行时关键路径自动 NVTX range（`kernel.launch` / `memory.copy_h2d` / `module.load`），用户 API `nvtx_scope!()`；Nsight Systems 时间线开箱即用（r11）。
- **CUPTI Activity API**：opt-in profiling 模式（开销 1–5%，r11），异步 ring buffer → Chrome Trace JSON 导出。**互斥纪律**：CUPTI Profiling API 与 Nsight Compute 不能同时用（r11）——运行时探测并拒绝冲突配置。
- 关闭时零成本：telemetry 点编译为可消除的分支（feature flag + 运行时开关双层）。

## 4. 基准与性能验收基础设施（D-236）

r11 的方法论作为工具链组件交付（`rx bench` + harness 库），不是文档建议：

- **L0 环境验证**：锁频（`nvidia-smi -pm 1; -lgc/-lmc`，Boost Clock 验收）、温度稳定窗、进程隔离检查；未锁频运行标记 `evidence=unlocked`（差异可达 50%+，r11）。
- **微基准协议**：warmup ≥10 + 稳态判定（连续 5 次 CV<5%）→ 50 次 × 3 trials → trial 内中位数 → 跨 trial trimmed mean（去头尾 20%）→ IQR 异常剔除 → bootstrap 95% CI；timed 迭代前清 256MB L2；计时统一 CUDA Event + 测量区前后 `stream.synchronize()` 刷 WDDM batch（r11 全套数字照搬）。
- **回归判定**：基线/候选各 30 样本，Mann-Whitney U（p<0.05）+ 效应量门；GPU 阈值 1% Warning / 5% Critical（r11）。
- **分层**：L0 环境 → L1 微基准（SAXPY/Reduction/Transpose/GEMM，`ncu` SOL 指标）→ L2 模式（Scan/Sort）→ L3 mini-app（软光栅/SPH，`nsys`+NVTX）→ L4 端到端。MVP 建到 L1，L3 随 G0 demo。
- 上一项目直接复用资产（H05）：`triple_run.py`/`trimmed_mean.py`/`threshold_drift.py`/`capability_probe.py` 思路与 `regression.py` 框架（改造为 CUDA 探测与 Rurix 预算 schema，[14](14_ENGINEERING_DISCIPLINE.md) §5）。

## 5. 调试与剖析工具（D-237）

| 工具 | MVP | 后续 |
|---|---|---|
| host 调试 | PDB + VS/WinDbg 断点/栈/行号；标准库 Natvis（Buffer/View/Vec/Mat 可视化） | VS expression evaluator 深度集成（Phase 2，r6 第三层） |
| device 调试 | line-tables（Nsight 源码关联）；debug 模式 device assert + 越界检查 | cuda-gdb 风格交互调试不做承诺（Windows 现实），以 printf-kernel + sanitizer 路线替代 |
| 剖析 | `ncu`/`nsys` CLI 开箱可用（lineinfo + NVTX 已内建）；`rx bench` 集成 `ncu --csv` 解析 | occupancy/寄存器数据回灌 IDE inlay hints（r9 远期） |
| 正确性 | Compute Sanitizer（racecheck/memcheck）作为 unsafe 代码的官方开发流程（r5：HiRace 指出 racecheck 不查 global races——文档明示边界） | 自研静态竞争检查扩展（远期研究项） |

## 6. 热重载与迭代体验（D-238）

- **kernel 级热重载**：PTX 模块独立装载的天然能力——`rx watch` 监视源码，重编译变更 kernel 的 PTX，运行时 API `ModuleRegistry::reload()` 原子换新（旧 Module 经 affine drop 排空后卸载）。host 代码不热重载（静态语言现实，不做承诺）。
- 目标：单 kernel 修改 → 重载完成 < 2s（MVP 后期预算项）。
- 这是图形迭代（G0 软光栅调参、G2 shader 迭代）的核心体验投资，对标 shader 热重载工作流。

## 7. 开发者工具集（D-239）

| 工具 | 形态 | 时点 |
|---|---|---|
| `rx`（CLI 总入口） | build/run/check/test/bench/fmt/doc/fix/watch/vendor | MVP（核心子命令） |
| `rurixup` | 工具链版本管理器（rustup/juliaup 模式，r6）；MSI + winget 分发 | MVP 后期 |
| formatter（`rx fmt`） | 无配置项的规范格式器（gofmt 哲学——杜绝风格争论与 AI 风格漂移） | MVP |
| linter | 编译器内建 lint 集（warn/deny 分级），无独立 clippy；lint 也走错误码注册表 | MVP 起步集 |
| 测试框架（`rx test`） | 内建 `#[test]`；GPU 测试自动子进程隔离选项（H03 §6 纪律工具化） | MVP |
| 文档生成（`rx doc`） | 从源码注释生成；spec/错误码索引同一管线（P-11） | MVP 后期 |
| 包管理 | `rx add/vendor/lock`（[09](09_STDLIB_AND_ECOSYSTEM.md) §7） | MVP 最小集 |

## 8. IDE 集成（D-240）

- **VS Code 优先**：官方扩展 = LSP 客户端 + 语法高亮（TextMate 起步）+ 调试适配（cppvsdbg 复用，PDB 路线天然兼容）。
- **Visual Studio**：LSP + VSIX（VS 15.8+ 内置支持，r9）；与 VS Code 共用 language server；不做 Roslyn 式原生服务（r9 明确否决）。
- LSP 设计见 [07](07_COMPILER_ARCHITECTURE.md) §9（单一前端原则）。

## 9. 分发与签名（D-241）

r6 全套结论照搬：

- **工具链**：`rurixup` 引导 + MSI + winget；编译器/运行时/标准库按版本原子分发；语言本体与 NVIDIA 再分发组件分离打包。
- **NVIDIA 组件**：仅 Attachment A 白名单最小集（MVP 实际只需 `libdevice.10.bc`；cuBLAS 绑定包按需附带 runtime DLL）；完整 Toolkit/驱动/Nsight 永不捆绑（许可红线，r6）；"装了 Toolkit ≠ 有驱动"（13.1+）进安装诊断。
- **签名**：全部 EXE/DLL/MSI Authenticode + 时间戳；OV 证书或 Azure Artifact Signing（EV 不再豁免 SmartScreen，r6）；信誉积累期在 [12](12_RISKS.md) 登记为已知摩擦。
- **SBOM**：构建生成 SPDX（发布附 CycloneDX 视图）；CI 强制许可白名单审计。
- 用户产物（EXE/DLL/PYD）的分发指引文档化（含 PYD 的 DLL 搜索顺序陷阱，r12 → [09](09_STDLIB_AND_ECOSYSTEM.md) §6）。

## 10. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
