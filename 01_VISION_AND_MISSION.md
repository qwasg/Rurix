# 01 — 愿景与使命

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 关联决策：D-001 ~ D-004（见 [13](13_DECISION_LOG.md)）

---

## 1. 一句话使命

**让 GPU 系统编程拥有自己的 Rust**：一门把"资源所有权、地址空间、并行执行层级"做成类型系统一等公民的独立静态编译语言，使图形与 GPU 计算程序在不牺牲 CUDA 级底层控制的前提下，获得可静态证明的安全性、可预测的性能与可长期治理的生态。

## 2. 为什么 Rurix 应该存在

### 2.1 直接动机：上一项目的结构性天花板（实证，非推测）

Rurix 不是凭空立项。它的前身——Taichi Engine 优化计划（P0–P15，约 60 个里程碑，2026-05-24 封板）——用 16 个大阶段的实际工程证明了一个判断（H01/H04）：

> 在一个不为此设计的宿主（嵌入 Python 的 Taichi DSL）之上叠加图形引擎、原生 RHI、编译期吞吐优化，工程纪律可以做到极致（90+ 原子计数器、6 阶段预算门禁、三层 CI、约 60 个里程碑零验收漂移），但**宿主语言与上游架构的天花板无法靠下游工程纪律突破**。

天花板的具体形态（H04 §2）：

1. **表达力天花板**。地址空间（global/shared/register）、资源所有权（Buffer/Stream/Event 的 affine 语义）、设备端泛型——这些 GPU 系统编程的核心概念在 Python 宿主上只能用"计数器 + 状态字典 + 环境变量开关"模拟，无法成为可检查的语言能力。P14 的 8 个前沿特性包（Hybrid RT / ReSTIR / Nanite-class / 3DGS / 可微渲染）最终绝大部分停留在 stub + bookkeeping 计数器，不是执行力问题，而是宿主表达不出真实实现所需的语义。
2. **平台现实天花板**。Vulkan compute + Python 在 Windows 上是二等公民：`vkQueueSubmit` 0xC0000409 崩溃导致 21 个测试永久失败且本地无解；WDDM 调度探测只能靠环境变量 mock。而 CUDA 在 Windows 上是 NVIDIA 一等支持的栈（r4/r6）。
3. **样板税天花板**。每个新观测点要手写五段式镜像管道（C++ atomic → Pybind → dict → normalize → dataclass），到 P15 时单个里程碑的"管道 + 文档税"已接近实际功能工作量（H04 §2.4）。这是"语言没有自动生成绑定/状态镜像能力"的直接代价。

终止决策的完整逻辑链（H04 §3）：下游能做的优化已做尽 → 剩余瓶颈全部在上游语言定位层 → 继续投入的边际收益低于新开一门独立语言的预期收益。**Rurix 就是这个"更换上游"决策的产物。**

### 2.2 行业层面：现有语言留下的真空（r10）

把 2026 年的 GPU 编程语言版图按四个轴打分——**独立语言 / 静态编译 / Rust 级安全类型 / CUDA 优先 + Windows 原生**——没有任何现存系统同时满足四项：

| 系统 | 独立语言 | 静态编译 | 安全类型系统 | CUDA 优先 + Windows 原生 | 缺口 |
|---|---|---|---|---|---|
| CUDA C++ | 半（C++ 扩展） | 是 | **否**（指针越界/竞态自负） | 是 | 安全性、可治理性 |
| Triton / Taichi / Numba | **否**（Python 宿主 DSL） | 否（JIT） | 弱 | 部分 | 通用性、表达力天花板（上一项目实证） |
| Slang | 是 | 是 | 部分（无所有权） | 否（着色器专用） | 非通用计算、无资源所有权 |
| Mojo | 是 | 是 | 是 | **否**（Windows 仅 WSL，编译器闭源至 2026） | Windows 原生、开放治理 |
| Rust + rust-cuda / cuda-oxide | 是 | 是 | host 侧是 | 部分（"所有 GPU fn 都是 unsafe"） | 设备侧安全模型缺失 |
| Descend | 是（学术原型） | 是 | **是**（所有权 + views） | 部分（无生态/工具链） | 工程化、生态、Windows 工具链 |
| WGSL / SYCL / HIP | 各异 | 各异 | 部分 | **否**（跨平台优先，性能/生态让位） | 与 CUDA-first 定位冲突 |

这个真空不是偶然：占据它需要同时具备（a）编译器工程能力（rustc 级前端 + NVPTX 后端）、（b）GPU 运行时与 Windows 工具链的深度整合、（c）从 Descend 等学术成果到工程产品的转化能力、（d）抗 AI 幻觉的治理纪律。单项都有先例，四项合一没有人做——这正是 Rurix 的立足点，也是它的护城河设计（详见 [03](03_POSITIONING_AND_LANDSCAPE.md)）。

### 2.3 时代层面：AI 时代的语言基础设施需求

两个正在发生的趋势使"现在"成为正确的立项时点：

1. **GPU 计算从专家领域变为大众领域**。AI 加速图形（神经渲染、3DGS、神经降噪、可微渲染）、实时仿真、GPU-driven 渲染管线正在把"写 GPU 代码"从少数引擎团队的工作扩散到整个图形/视觉计算行业。而现有工具要么不安全（CUDA C++），要么不通用（Slang/Triton），要么表达力不足（Python DSL）。
2. **AI 重度参与开发成为常态，语言与治理必须为此设计**。上一项目的实际经验（H06 §4）：AI 参与下最常见的漂移是验收标准被悄悄放宽、占位实现被描述成完成、文档承诺超出代码事实。Linux kernel 与 LLVM 已在 2025 年发布正式 AI 贡献政策（r7）。Rurix 从第一天就把"可测试规范 + conformance 唯一验收边界 + provenance 强制"内建为治理骨架（[10](10_GOVERNANCE.md)），这在现存 GPU 语言中没有先例——AI 时代的新语言，治理纪律本身就是产品力。

## 3. Rurix 解决什么问题（现有系统解决不好的）

| # | 问题 | 现状 | Rurix 的回答 |
|---|---|---|---|
| 1 | **GPU 代码的内存与并发安全** | CUDA C++ 完全靠人；Rust-CUDA 设备侧全 unsafe；Descend 证明了可行性但停留在学术原型 | 宿主层 Rust 式所有权 + 设备层 execution resources / views / 地址空间类型（[05](05_LANGUAGE_ARCHITECTURE.md)）；结构化并行模式静态证明无竞争，弱序协议显式落 `unsafe` 并配验证义务 |
| 2 | **host/device 资源生命周期** | `cuCtxDestroy` 跨线程核弹、流序分配 use-after-free、context-bound 资源跨 context 误用——全部运行时炸（r4） | Context/Stream/Buffer 做成 affine 类型，生命周期错误变成编译错误（[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §5） |
| 3 | **静默降级与不可诊断的工具链** | 模板 fallback、permissive 编译、配置说支持运行时炸（上一项目全程在对抗这些） | strict-only：lowering 失败 = 结构化编译错误；能力位由真实设备探测驱动（[04](04_DESIGN_PRINCIPLES.md) P-01/P-04） |
| 4 | **Windows 上 GPU 开发的二等公民待遇** | Mojo 仅 WSL；Vulkan compute 驱动黑洞；Linux-first 工具链在 Windows 上处处摩擦 | COFF/PE/PDB/Authenticode 原生工具链 + CUDA Driver API 一等运行时 + WDDM/TDR 作为一等环境条件（[08](08_RUNTIME_AND_TOOLING.md)） |
| 5 | **kernel 与宿主代码的语言割裂** | C++ host + HLSL/GLSL shader + CUDA kernel 三套语言三套类型系统 | 单语言双层模型：宿主与 kernel 共享类型系统、泛型与模块系统，编译器静态检查 launch 边界（[05](05_LANGUAGE_ARCHITECTURE.md) §2） |
| 6 | **性能不可预测、不可观测** | JIT 抖动、隐式同步、自动并行黑盒（Bend 路线的失败） | 静态编译 + 显式内存/同步 + 编译器与运行时内建 telemetry（`-Z self-profile` 式 + CUPTI Activity，[07](07_COMPILER_ARCHITECTURE.md)/[08](08_RUNTIME_AND_TOOLING.md)） |
| 7 | **生态的混乱生长与 AI 幻觉 API** | npm 式供应链事故；AI 生成不存在的 API 并进入文档 | 规范条款编号 ↔ conformance 测试 ↔ PR 强制引用三角；包管理无任意构建脚本（[09](09_STDLIB_AND_ECOSYSTEM.md) §7、[10](10_GOVERNANCE.md)） |

## 4. Rurix 想创造什么样的未来

**五年图景**（与 [11](11_ROADMAP.md) §6 对齐）：

1. **图形引擎的下一代地基**。一个用 Rurix 写成的渲染器：场景遍历、剔除、光照、后处理 kernel 与 host 调度代码在同一语言里，资源生命周期与 pass 间依赖由类型系统保证，整条管线没有一行不可诊断的胶水代码。上一项目用 16 个阶段没能在 Python 宿主上做到的事（真实 GPU-driven 渲染、RT、神经渲染的生产实现），在为此设计的语言上成为自然表达。
2. **GPU 系统编程的安全标准**。"结构化 kernel 默认安全、弱序协议显式 unsafe + 验证义务"成为行业认知，就像 Rust 改变了系统编程对内存安全的预期。
3. **Windows + NVIDIA 上最好的 GPU 开发体验**。从 `rurixup install` 到第一个 kernel 跑出 Nsight 时间线少于十分钟；VS Code / Visual Studio 里的借用错误诊断质量对标 rustc。
4. **可信的生态**。规范可测试、API 可追溯、供应链可审计——一个 AI 大规模参与贡献但语义从不漂移的语言社区。

**克制声明**：Rurix 不试图取代 CUDA 生态（它在 CUDA 之上提供安全的编译前端与运行时）、不试图首发跨平台（NVIDIA 单栈做深，可移植性是远期议题）、不试图成为 ML 框架（与 PyTorch 经 DLPack 零拷贝互操作，不是替代它）。每一条克制都对应一条已验证的死亡路线（[03](03_POSITIONING_AND_LANDSCAPE.md) §4）。

## 5. 为什么这件事对图形编程的兴起重要

图形编程正处在三十年来最大的范式迁移中：固定管线 → 可编程着色器 → **GPU-driven 一切**（GPU 剔除、间接绘制、mesh shader、work graph、神经渲染）。迁移的瓶颈已经不是硬件能力，而是**编程模型**：今天写一条现代管线需要在 C++、HLSL、CUDA、Python 工具脚本之间缝合，每条边界都是类型系统的断裂带，每个资源都靠程序员心智模型管理生命周期。

历史上每次图形编程的跃迁都伴随语言层的更新（汇编 shader → Cg/HLSL → compute 通用化）。下一次跃迁——GPU 成为自治的计算主体、AI 与渲染融合——需要一门把 GPU 语义做进类型系统的语言。Rurix 的使命是成为这门语言：**不是更方便的脚本，而是更正确的地基。**

## 6. 成功判据（什么叫"Rurix 成了"）

分三层，避免空泛（具体里程碑见 [11](11_ROADMAP.md)）：

- **MVP 成功（12–18 个月）**：在 RTX 4070 Ti 上，用 Rurix 写的 SAXPY/Reduction/GEMM kernel 达到手写 CUDA C++ ≥ 90% 性能；compute 软光栅演示渲染出真实图像；借用检查器拦截全部预设的资源生命周期错误类别；编译器自身的诊断/性能基线全部 `measured_local`。
- **生态成功（3 年）**：开源后 12 个月内出现 ≥ 3 个非作者维护的真实项目；G2 原生图形管线跑通首个引擎集成 demo；语言规范 conformance 测试覆盖全部 stable 特性。
- **使命成功（5 年+）**：至少一个生产级渲染器/仿真系统选择 Rurix 作为主语言；"GPU 代码的安全类型系统"成为新语言/新框架的对标项。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
