# 05 — 语言架构

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 主要输入：r5（所有权/内存模型）、r1（rustc 工程）、r4（运行时对象模型）、r12（标准库类型）
> 关联决策：D-101 ~ D-114（见 [13](13_DECISION_LOG.md)）
> 约定：本文给出语言级设计与裁剪理由；编译器实现见 [07](07_COMPILER_ARCHITECTURE.md)；GPU 语义细节见 [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md)。语法草图为**方向性示意**，最终语法以 `spec/` 为准。

---

## 1. 总体形态：一门语言，两个执行世界

Rurix 是单一语言，但代码运行在两个语义截然不同的世界：

```
┌─────────────────────────────────────────────────────────┐
│  宿主层 (host)                                            │
│  完整系统语言：所有权/借用/trait/堆分配/FFI/std           │
│  编译目标：x86-64 COFF/PE (Windows)                       │
│  职责：资源管理、kernel 调度、IO、与外界互操作              │
├─────────────────────────────────────────────────────────┤
│  kernel 子语言 (device)                                   │
│  受限子集 + 设备扩展：执行层级类型/地址空间/views           │
│  编译目标：PTX (MVP) / DXIL (G2)                          │
│  职责：数据并行计算；G2 起包含光栅/RT/mesh 着色阶段          │
└─────────────────────────────────────────────────────────┘
        共享：类型系统、泛型、模块系统、const eval、诊断
```

**设计要点（D-102）**：两层共享同一类型系统与前端，差异通过**函数着色（function coloring）+ 能力检查**表达，而不是两套语法：

- `fn` —— 宿主函数（默认）。
- `kernel fn` —— GPU 入口函数。只能被 launch API 调用，参数须满足 `DeviceCopy` 或为设备资源句柄，签名携带执行形状类型（§6）。
- `device fn` —— 设备侧可调用函数。可被 kernel 与其他 device fn 调用；MVP 中默认强制内联展开（无设备侧调用栈管理负担，对齐 r2 的 MVP 收缩建议）。
- `const fn` —— 编译期可求值函数，两层皆可调用（§9）。

着色规则由类型检查器静态强制：host fn 不能在 device 上下文被调用，反之亦然；违反产生结构化诊断（如 `RX0301: host function called in device context`）。这避免了 CUDA C++ `__host__ __device__` 双标注的组合爆炸——需要双侧可用的函数写成 `device fn` 且不使用宿主能力，宿主可直接调用 device fn（单向可达：device ⊂ host 可调用集）。

### 不做什么（着色层）

- **不做** Mojo 式"同一函数自动两侧编译"的隐式双目标——违反 P-05（显式优于隐式），且使能力检查边界模糊。
- **不做** CUDA 动态并行（device 侧 launch）——MVP/G1 不支持，登记 spike gating。

## 2. 类型系统

### 2.1 原生类型

| 类别 | 类型 | 说明 |
|---|---|---|
| 整数 | `i8/i16/i32/i64`、`u8/u16/u32/u64`、`usize` | 溢出语义：debug 检查 + release 截断回绕，与 Rust 对齐（D-103） |
| 浮点 | `f16`、`f32`、`f64`、`bf16` | `f16/bf16` 为一等类型（GPU 现实需要）；`f64` 在消费级 GPU 上慢 64 倍，编译器对 device 代码中的隐式 f64 提升发 lint |
| 布尔/字符 | `bool`、`char`（仅 host） | device 侧 `char`/字符串操作不可用 |
| 数学向量/矩阵 | `Vec2/3/4<T>`、`Mat2/3/4<T>` | **语言内建**（非库类型），获得编译器布局/对齐/SIMD 保证与 swizzle 语法；列主序 canonical（D-301，r12）。详见 [09](09_STDLIB_AND_ECOSYSTEM.md) §3 |
| 复合 | `struct`、`enum`（tagged union）、元组、定长数组 `[T; N]` | `enum` 模式匹配穷尽检查；device 侧允许 struct/enum/数组，禁止含 host-only 类型 |
| 引用 | `&T`、`&mut T`（host）；`&[space] T`（device，§5） | device 引用携带地址空间参数 |
| 切片 | `&[T]`、`&mut [T]`（host）；`View<...>`（device，§7） | device 侧不暴露裸切片，统一走 views |

### 2.2 Trait 系统（D-104）

Rust 式 trait + 泛型约束 + 单态化，但 MVP 大幅裁剪：

- **有**：trait 定义/实现、泛型参数约束（`T: DeviceCopy + Add`）、关联类型、运算符重载 trait、一致性规则（orphan rule）。
- **MVP 没有**：trait 对象（`dyn Trait`，动态分发对 device 无意义且 host 侧可延后）、特化（specialization）、HKT、async（语言级不做，GPU 异步走 stream/event 模型）、`impl Trait` 返回位置（延后）。
- **理由**：r1 警告 trait solver 是 rustc 长期成本中心；单态化全静态分发同时满足 host 性能与 device 可编译性。trait 对象在 host 侧的需求（如插件接口）由 C ABI 函数指针过渡，stable 前重评估。

核心内建 trait（编译器已知语义）：

| trait | 语义 |
|---|---|
| `Copy` / `Clone` | 与 Rust 同 |
| `DeviceCopy` | 可按位复制到设备的类型（无 host 指针/引用/句柄字段）；kernel 值参数的必要约束（r12 的 `DeviceCopy` 先例） |
| `Record` | 可自动生成 C ABI 镜像与序列化视图的纯数据类型（P-11 的"状态镜像由编译器生成"载体，替代上一项目五段式手写管道） |
| `Drop` | 析构；affine 资源类型的释放逻辑挂载点（§4） |

### 2.3 不做的类型特性（显式登记）

- 生命周期高阶多态（HRTB）— 延后；MVP 的借用检查场景（NLL + views）不需要。
- 异常/unwind 跨 FFI — 见 §8。
- 反射/RTTI — 永不；`Record` derive 覆盖序列化需求。

## 3. 所有权与借用：双层模型

### 3.1 宿主层：Rust 式 affine 所有权 + NLL 借用检查

与 Rust 语义一致的核心子集（D-105）：

- move 语义默认；`Copy` 类型例外。
- 共享引用 `&T` 可多、可变引用 `&mut T` 独占，借用不超过被借者生命周期。
- 借用检查在 MIR/CFG 层以 NLL 风格数据流实现（**不做 Polonius**——r1 最强警告之一：2026 年 Polonius 仍未 stable 且有已知 soundness issue；见 [07](07_COMPILER_ARCHITECTURE.md) §5）。
- 生命周期标注：MVP 支持函数签名内的显式生命周期参数与省略规则；不支持 HRTB。

### 3.2 设备层：execution resources + views（Descend 路线，D-106）

设备侧的核心问题不是单线程别名，而是**数千线程对同一内存的并发写**。Rust 的 `&mut` 在这里直接失效（Rust-CUDA 的教训：kernel 参数 `&mut [T]` 错误暗示独占，实际多 invocation 共享，r5）。Rurix 采用 Descend 验证过的方案：

1. **执行资源类型**：`Grid<X,Y,Z>`、`Block<X,Y,Z>`、`Warp`、`Thread` 是类型层实体。kernel 的执行形状是签名的一部分；线程索引不是裸整数，而是携带其层级来源的类型化值。
2. **借用收窄（narrowing）**：对一块设备内存的可变访问必须沿执行层级逐级收窄——grid 拥有的缓冲区经 view 分解后，每个 block/thread 只持有自己分片的 `&mut`。borrow checker 在类型层证明分片不相交，无需理解任意索引算术。
3. **views 是收窄的唯一安全语法**（§7）。绕过 views 的任意索引可变写需要 `unsafe`。

### 3.3 安全包络的边界（与 r5 对齐，照搬其分层结论）

| 静态保证（safe 代码） | 必须 `unsafe`（附验证义务，P-03） |
|---|---|
| host/device 资源生命周期（§4） | inline PTX、FFI 到外部同步库 |
| 地址空间不混淆（§5） | 运行时决定的索引共享可变写（图算法 worklist 等） |
| views 规则化分区的无竞争写（§7） | shared memory 手工字节切分/重解释 |
| barrier 可达性（无 divergent barrier，[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §4） | 自定义弱序原子协议（lock-free 队列等） |
| launch 形状与 kernel 签名匹配（§6） | 跨 kernel + host 协同的全局同步协议 |

## 4. 资源生命周期：affine 资源类型

GPU 资源（来自 r4 的对象模型）全部建模为 **affine 类型**（move-only、禁 `Copy`/`Clone`、有 `Drop`）：

```rurix
// 语法草图
let dev: Device = Device::enumerate()?.first()?;        // CUdevice 标识，Copy
let ctx: Context = dev.create_context()?;               // affine 资源根
let stream: Stream<'ctx> = ctx.create_stream()?;        // 借用 context 的 affine 资源
let buf: DeviceBuffer<f32> = ctx.alloc::<f32>(1 << 20)?; // context-bound
```

**关键设计（D-107）：context 归属编码为生命周期参数（brand）**。`Stream<'ctx>`、`DeviceBuffer<T>`（内部携带 `'ctx`）、`Event<'ctx>`、`Module<'ctx>` 的生命周期参数把"资源不得活过其 context、不得跨 context 误用"两条规则变成借用检查的自然结论。这直接消灭 r4 列举的两类核弹：

- `cuCtxDestroy` 在仍有资源/其他线程使用时被调用 → 在 Rurix 中 `Context` 被借用期间无法 drop，编译错误。
- Event record 与 wait 跨 context → 生命周期参数不匹配，编译错误。

其余 r4 陷阱的类型化对策：

| r4 陷阱 | Rurix 对策 |
|---|---|
| current context 是线程局部状态，跨线程转移要 push/pop | `Context: Send + !Sync`（affine 转移合法、共享须显式 `ContextHandle` 租约类型）；运行时在 API 调用点自动管理 current（[08](08_RUNTIME_AND_TOOLING.md) §2） |
| `cuStreamDestroy` 有 pending work 时立即返回、异步回收 | `Stream::drop` 语义文档化为"提交销毁"；需要确定性回收用显式 `stream.synchronize_and_destroy()` |
| 流序分配（`cuMemAllocAsync`）的生命期是流时序区间 | 独立类型 `AsyncBuffer<'stream, T>`，携带分配 stream，跨 stream 使用必须显式 `make_visible_to(event)`（[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §5.4）；MVP 延后此类型，先做经典 `cuMemAlloc` 路径 |
| Graph API 非线程安全 | `Graph: !Send + !Sync`，单线程独占（MVP 不做 Graph，G1 重评估） |
| Driver/Runtime API 混用形成两套资源世界 | Rurix 运行时只用 Driver API；FFI 引入的 Runtime API 库走 primary-context interop 模式，文档化约束（[08](08_RUNTIME_AND_TOOLING.md) §2.5） |

## 5. 地址空间：类型一等公民（D-108）

设备引用与指针类型携带地址空间参数，对齐 NVPTX 的五空间模型（r2：LLVM addrspace 0/1/3/4/5）：

| Rurix 空间 | NVPTX | 暴露形式 | 说明 |
|---|---|---|---|
| `global` | addrspace(1) | `&global T` / `&global mut T`、`DeviceBuffer<T>` | 默认设备内存 |
| `shared` | addrspace(3) | `shared let` 声明 + views 借用 | block 作用域；借用不得逃逸 block（类型层保证，Descend 模式） |
| `constant` | addrspace(4) | `const` kernel 参数自动放置（显式标注，不自动推断——Rust-CUDA 自动放置崩溃教训，r5） | 只读广播 |
| `local`（寄存器/栈） | addrspace(5) | **不暴露指针/引用**（Slang 同款取舍，r5） | 局部变量默认；取地址即编译错误或强制落 unsafe |
| `host` | — | 宿主引用 `&T` | device 代码不可达 |

**显式不做**：OpenCL generic address space / SYCL 式推断进入 safe core（r5 点名弱化 provenance，P-02）。generic 指针转换是 unsafe 操作。

`shared` 声明的语法草图：

```rurix
kernel fn tile_gemm<const TILE: usize>(
    grid: Grid<(N / TILE, M / TILE)>,
    a: View<global, f32, (M, K)>, ...
) {
    shared let tile_a: [[f32; TILE]; TILE];   // block 作用域，编译器计算 shared 用量
    // tile_a 的可变借用必须经 views 按 thread 收窄
}
```

动态 shared memory（launch 时定容）：`shared let buf: SharedSlice<f32>`，容量来自 launch 配置，类型层不可知长度，越界由 debug 模式运行时检查 + release 模式文档化 UB（unsafe 路径）。

## 6. kernel 签名与 launch 的类型契约

launch 错误（形状不匹配、shared 超额、参数布局错位）是 CUDA 的经典运行时炸点。Rurix 把 launch 做成全类型化 API（与 UC-02/UC-03 对齐）：

```rurix
let k = module.kernel::<tile_gemm<32>>()?;        // 单态化实例的句柄
stream.launch(k, GridDim(64, 64), BlockDim(32, 32), (a.view(), b.view(), c.view_mut()))?;
```

- 参数元组类型与 kernel 签名在编译期匹配（经单态化符号系统，[07](07_COMPILER_ARCHITECTURE.md) §7）。
- 执行形状若为 const 泛型参数则编译期检查；运行时形状走运行时校验 + 结构化错误。
- kernel 参数 ABI 按 PTX `.param` 规则布局（上限 32764 字节，r2）；超限是编译错误并建议改用 buffer 间接。

## 7. Views：规则化分区的类型语法（D-109）

views 是设备侧安全可变访问的核心机制，直接采纳 Descend 的算子集（r5）并工程化：

| 算子 | 语义 | 典型用途 |
|---|---|---|
| `split::<N>()` | 等分为 N 段不相交子 view | block 间划分 |
| `group::<K>()` | 按 K 个元素分组 | thread 处理 K 元素 |
| `transpose()` | 行列重映射（不搬数据） | coalesced 访问模式 |
| `reverse()` / `map_idx` 受限族 | 规则重排 | 蝶形等规则模式 |
| `zip()` | 并行迭代多 view | 多数组同步处理 |

类型层规则：`View<space, T, Shape>` 的可变版本只能通过"执行资源拥有的分解路径"获得——`per_block()` / `per_thread()` 把 view 的所有权沿执行层级分发，每个 thread 拿到的 `&mut` 静态不相交。Descend 已证明这套算子覆盖 transpose/reduce/scan/histogram/GEMM 且生成代码与手写 CUDA 同级（r5）。

**包络外**：动态索引（`view[runtime_idx] = x` 其中 idx 无法证明不相交）→ `unsafe { view.write_unchecked(idx, x) }`，配 Compute Sanitizer 动态检测纪律（[14](14_ENGINEERING_DISCIPLINE.md) §6）。

## 8. 错误处理（D-110）

- **宿主层**：`Result<T, E>` + `?` 传播 + `enum` 错误类型。无异常。panic = abort（MVP 不做 unwind——简化 codegen 与 FFI 边界；Windows SEH unwind 交互延后评估）。
- **设备层**：无 panic、无 Result 传播（代价不可控）。三通道：
  1. **编译期**：能静态排除的（形状/地址空间/borrow）全部编译期解决；
  2. **debug 运行时**：越界/断言编译为 device trap，运行时把 `CUDA_ERROR_ASSERT` 映射为结构化错误并**标记 context 为 poisoned**（r4：assert 后 context 不可恢复，须整块重建——affine 类型使"重建 context 子树"成为类型引导的操作）；
  3. **release**：unsafe 路径文档化 UB，safe 路径保持检查或由优化消除。
- **FFI 边界**：C ABI 函数返回错误码（cuBLAS `cublasStatus_t` 模式，r12）；panic 不跨 FFI（abort 兜底）。

## 9. 泛型、const 泛型与编译期求值（D-111）

- **单态化泛型**：全部泛型（含 kernel）单态化，无运行时泛型。kernel 的单态化实例是独立 PTX 符号。
- **const 泛型**：`const TILE: usize` 式值参数是 GPU 编程刚需（tile 尺寸/unroll 因子/形状）。MVP 支持整数/bool const 泛型 + 简单算术表达式求值。
- **const eval**：`const fn` 子集（算术/分支/循环/数组构造），MIR 解释器实现（r1 模式）。用途：tile 布局计算、查找表生成、shared 容量推导。**MVP 不做**：堆分配 const eval、trait 调度 const eval。
- **元编程红线**：**无过程宏**（H06 §5 红线照搬；AI 时代任意编译期代码执行 = 供应链与幻觉双重风险）。内建 `derive(Copy, Clone, DeviceCopy, Record)` 由编译器实现。声明宏：MVP 不做，G1 后按需求重评估（登记 spike gating）。

## 10. 模块与包系统（D-112）

- **模块**：文件即模块（目录 + `mod.rx` 或单文件），`use` 导入，`pub` 可见性分级（`pub`/`pub(package)`/私有）。无头文件。
- **包（package）**：`rurix.toml` manifest + `rurix.lock` lockfile 定义的编译单元，产物为 `lib`（rlib 式静态库 + 元数据）、`bin`（EXE）、`cdylib`（DLL/PYD）。
- host 与 device 代码同包同模块树——kernel 与其调度代码放在一起，这是双层单语言的核心人体工学收益。
- 依赖解析、vendor、供应链见 [09](09_STDLIB_AND_ECOSYSTEM.md) §7。

## 11. FFI 战略（D-113）

**进口（调用 C）**：

```rurix
#[link(name = "cublas64_13")]
extern "C" {
    fn cublasCreate_v2(handle: *mut CublasHandle) -> i32;
}
```

- `extern "C"` + 原始指针 + `#[repr(C)]` struct；调用必然 `unsafe`。
- Windows x64 ABI 唯一（r6）；无 32 位目标（CUDA 12.0 已移除 x86，r6）。
- `-sys` 包惯例 + `links` 元数据防重复符号（r8 的 Cargo 可复制部分）。

**出口（被 C/C++/Python 调用）**：

```rurix
#[export(c)]                       // 进入 DLL 导出表 + 生成头文件条目
pub fn rurix_simulate_step(sim: *mut SimHandle, dt: f32) -> RxStatus { ... }
```

- 编译器内建头文件生成（cbindgen 角色内置化，P-11：单一事实源生成视图）。
- 复杂类型一律不透明句柄 + create/destroy/operate 三元组 + 错误码返回（r12 的 C ABI 结论）。
- Python：不做语言级 Python 绑定；经 C ABI + nanobind 通道（[09](09_STDLIB_AND_ECOSYSTEM.md) §6）。

**显式不做**：C++ ABI 直接互操作（name mangling/异常/虚表——永不）；自动 binding 生成器进 MVP（手写 `-sys` 薄层，r12 的 cuBLAS 结论）。

## 12. 语法风格基调（D-114）

语法承诺在 spec 起草期细化，基调先行固定：

- Rust 系表达式语法（`let`/模式匹配/表达式块/无分号尾表达式），大括号块结构。
- 显式类型标注处用 `:`，返回类型 `->`。
- GPU 扩展关键字集合最小化：`kernel`、`device`、`shared`、地址空间名作为类型位置关键字。
- 标识符/路径风格与 Rust 对齐（snake_case 函数、CamelCase 类型），降低目标用户（大量 Rust/C++ 背景）迁移成本。
- 刻意**不**追求 Python 式语法亲和（Mojo 的差异化选择；Rurix 的用户画像是系统程序员，r10 竞品结论支持此分化）。

## 13. 与后续文档的接口

| 本文档定义 | 展开文档 |
|---|---|
| kernel/launch/同步语义、PTX 内存模型映射、图形阶段模型 | [06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) |
| 类型检查/借用检查/单态化/codegen 的实现策略 | [07](07_COMPILER_ARCHITECTURE.md) |
| Context/Stream/Buffer 运行时实现与 Windows 行为 | [08](08_RUNTIME_AND_TOOLING.md) |
| Vec/Mat、Buffer 家族、views API 的库面 | [09](09_STDLIB_AND_ECOSYSTEM.md) |
| 本文档全部 D-1xx 决策的备选与理由 | [13](13_DECISION_LOG.md) |

## 14. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
