# 03 — 语言定位与竞品全景

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 主要输入：r10（竞品全景）、r5（安全模型谱系）、H04（上一项目教训）
> 关联决策：D-002（图形分期）、D-101（语言形态）

---

## 1. Rurix 的形态定位

**Rurix 是一门独立的、静态编译的系统编程语言，采用双层模型**：

- **宿主层**：完整的系统语言（Rust 式所有权/借用、模块、泛型、C ABI），编译为原生 Windows x64 代码（COFF/PE）；
- **kernel 子语言**：同一语言的受限子集（以 `kernel fn` 标注），附加执行层级与地址空间类型约束，编译为 PTX（MVP）/ DXIL（G2）。

它**不是**：嵌入宿主语言的 DSL（Triton/Taichi 路线）、纯着色器语言（Slang/HLSL 路线）、通用语言加 GPU 库（Julia/Rust+cust 路线）。形态选择的完整论证见 §3 与 [13](13_DECISION_LOG.md) D-101。

一句话定位：**"GPU 系统编程的 Rust"——在 CUDA 生态之上提供安全的编译前端与运行时，而不是替代 CUDA 生态。**

## 2. 与现存系统的关系矩阵

每行回答三个问题：它是什么、Rurix 向它学什么、Rurix 与它如何相处。（基于 r10 全景 + r1/r3/r5 专项）

### 2.1 平台与生态（共存，不竞争）

| 系统 | 它是什么 | 学什么 | 关系 |
|---|---|---|---|
| **CUDA C++ / NVCC** | NVIDIA 事实标准，HPC/DL 的性能与生态天花板 | scoped atomics 与线程层级语义；库生态（cuBLAS/cuDNN）工程模式；Nsight 工具链整合深度 | **共生**：Rurix 是 CUDA Driver API 之上的另一个编译前端，复用全部驱动/库/工具生态；绝不正面争夺 CUDA C++ 的存量用户，争夺的是"想要安全与现代语言体验"的增量 |
| **Rust** | 系统编程安全标准与治理范本 | 所有权/借用/NLL；feature gate + RFC + edition 治理；诊断体系；Cargo 工程体验 | **师承**：host 层语义大量继承 Rust；治理是 Rust 模式的小团队裁剪版（r7）；不试图兼容 Rust 语法或 ABI，经 C ABI 互操作 |
| **LLVM** | 后端基础设施 | NVPTX 后端、COFF 产出、libdevice 链接流程 | **依赖**：pin 22.1.x（D-205）；接受"深度绑定 LLVM 版本与 ptxas 行为"的工程现实（r2，Triton 同款代价） |

### 2.2 直接竞品（差异化生存）

| 系统 | 它是什么 | 学什么 | 差异化 |
|---|---|---|---|
| **Mojo** | Python 语法 + 静态编译 + 所有权的系统语言，最接近的竞品 | CPU+GPU 单语言野心；`SIMD[DType, Width]` 宽度入类型参数；MLIR Core 自建方言（不用 linalg/affine）的架构判断 | Mojo 三个缺口即 Rurix 三个支点：**Windows 原生**（Mojo 仅 WSL）、**开放治理**（Mojo 编译器闭源至 2026，计划开源但未兑现）、**图形方向**（Mojo 是 AI-first，无图形管线野心）。风险：Mojo 开源 + Windows 支持若提前落地会压缩窗口期（[12](12_RISKS.md) R-501） |
| **Slang** | HLSL 超集、模块化着色器语言，Khronos 托管开源 | 模块/泛型进着色器；**capability system**（`[require(...)]` 式平台能力门控，被 r5 认定为 effects 层的正确参照）；多目标 codegen 工程 | Slang 是着色器语言，无 host 层、无所有权、无资源生命周期模型；Rurix G2 阶段与 Slang 在"现代图形语言"上正面相遇，差异化是**单语言全栈 + 安全类型系统**。在 G0/G1 阶段二者无交集 |
| **Triton** | Python 嵌入式 kernel DSL，PyTorch Inductor 后端 | 块级编程抽象的人体工学；"性能可以做到"的存在性证明；pin LLVM + 自建方言的工程模式 | Triton 锁定 Python 宿主与 DL 工作负载；Rurix 是独立语言、通用 GPU 系统编程。对 U1 用户（kernel 作者）有重叠，Rurix 以静态类型、可分发产物（EXE/DLL/PYD）、非 DL 工作负载差异化 |
| **Descend** | PLDI 2024 学术原型：Rust 式所有权 + execution resources + views 的安全 GPU 语言 | **设备侧安全模型的直接蓝本**（r5 核心结论）；同时学它的边界——运行时索引/弱序协议必须落 unsafe | Descend 证明了方向可行（benchmark 达手写 CUDA 同级）但无工具链、无生态、无 Windows 工程。Rurix 是"Descend 的类型学成果 × rustc 的工程方法 × Windows/CUDA 的产品化"三者的工程合成，这正是市场空白的形状 |

### 2.3 警示与边界（学教训，不走它的路）

| 系统 | 教训 | Rurix 的对应红线 |
|---|---|---|
| **Taichi**（上一项目宿主） | 垂直领域强，但 Python 宿主决定了表达力与系统编程天花板（16 个阶段实证） | 红线 1：永不做嵌入式 DSL |
| **Numba / CUDA.jl** | 低门槛但性能上限与静态保证弱；Numba 0.61 拆分 CUDA 出主仓的生态信号 | 同红线 1 |
| **Bend / HVM** | 自动并行化"易用但难调优"，专家与厂商都不买账 | 红线 2：永不做自动并行黑盒；显式控制是卖点不是负担 |
| **WGSL / wgpu、SYCL、HIP** | 跨平台优先 → 性能、能力暴露、生态深度全部让位；SYCL 地址空间推断弱化 provenance（r5 点名不适合 safe core） | 红线 3：MVP/G 阶段不做跨厂商可移植层；地址空间显式不推断 |
| **Halide / CUTLASS-CuTe** | 领域 DSL 性能极致但难成通用平台；CuTe Python 层专有 EULA 限制采纳 | 红线 4：不做窄域 DSL；红线 5：不走专有许可（D-003） |
| **C++ AMP / OpenCL** | 与官方栈正面竞争且无独特价值主张 → 被吞并消亡 | 红线 6：不替代 CUDA，站在它上面 |
| **rust-gpu / Rust-CUDA** | "Rust 语法 + GPU"不等于安全：设备侧全 unsafe、constant memory 自动放置引发崩溃（r5） | 设备安全模型必须是为 GPU 设计的（Descend 路线），不是 host 语义的直接搬运 |

## 3. 形态决策：为什么是独立语言（D-101）

四个候选形态的比较（这是本项目最根本的技术形态决策，论证保留全文）：

| 形态 | 代表 | 优势 | 否决理由 |
|---|---|---|---|
| A. 嵌入 Python 的 DSL | Triton/Taichi | 生态引力、上手成本低 | **上一项目用 16 个阶段实证否决**：表达力天花板（地址空间/所有权/设备泛型做不成语言能力）、样板税、Windows 二等公民。这是整个项目存在的前提判断 |
| B. Rust 的 GPU 方言/扩展 | rust-gpu、Rust-CUDA | 复用 rustc 与 crates 生态 | 受制于 rustc 演进节奏与 Rust 语义承诺；设备侧需要的执行层级类型、views、地址空间无法以库或 fork 形式自然表达（Rust-CUDA "全 kernel unsafe" 即此路线的终态，r5）；fork rustc 的维护成本对单人团队不可承受 |
| C. 纯着色器/纯 kernel 语言 | Slang/WGSL | 范围小、见效快 | 放弃 host 层 = 放弃资源生命周期安全这个核心价值主张；用户仍要在 C++ 里管理 context/stream/buffer，问题清单（[01](01_VISION_AND_MISSION.md) §3）的 #2/#5 无解 |
| **D. 独立双层语言（选定）** | Mojo（形态近似）、Descend（语义近似） | 类型系统覆盖 host+device 全链路；语义自主权；治理自主权 | 代价：自建编译器/工具链/生态，是四个选项中成本最高的。接受此代价的理由：(a) r1/r2 证明 4–6 人规模的最小可行链路存在且有 Julia/Numba/Mojo 先例；(b) 调研已给出全部关键组件的去风险路线；(c) 市场空白（§2.2）只有此形态能占据 |

## 4. 死亡路线红线（正式登记）

以下六条写入 spike gating 永久候选清单（[14](14_ENGINEERING_DISCIPLINE.md) §7），任何里程碑提议触碰必须走 Full RFC + agent 批准：

1. **不做 Python 宿主 DSL**——任何"为了易用性内嵌到 Python"的提案。互操作只走 DLPack/C ABI 通道。
2. **不做自动并行/自动调优黑盒**——调度永远显式。允许的上限是显式标注的调度提示（远期）。
3. **不过早跨平台**——AMD/Intel/Metal/Vulkan compute 后端、跨平台抽象层在 G2 完成前一律 not_triggered。
4. **不做窄域 DSL 化**——拒绝把语言核心特化为某单一领域（图像处理/GEMM/NN）的提案；领域能力进库不进语言。
5. **不走专有许可**——MVP 后开源承诺（D-003）不可撤回；核心组件不引入专有依赖导致的传染限制（NVIDIA 再分发白名单除外，见 [09](09_STDLIB_AND_ECOSYSTEM.md) §8）。
6. **不替代 CUDA 生态**——不自研 BLAS/DNN 全量替代品、不自研驱动层；始终站在 Driver API 与厂商库之上。

## 5. 价值主张总结（电梯陈述）

> 给 GPU 与图形开发者的 Rurix，相当于给系统程序员的 Rust：
> - 对 **CUDA C++ 用户**：同样的控制力与性能，编译期拦截你最痛的内存与生命周期 bug；
> - 对 **Triton/Taichi 用户**：不再被 Python 宿主限制表达力，产物是真正的原生程序；
> - 对 **Slang/HLSL 用户**（G2 起）：host 与 shader 终于是同一门语言、同一套类型；
> - 对 **Rust 用户**：你熟悉的所有权模型，加上为 GPU 真正设计的设备侧类型系统；
> - 对 **Windows 开发者**：第一门把你的平台当一等公民的 GPU 语言。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
