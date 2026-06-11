# 02 — 目标用户与用例

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 依赖：[01 愿景](01_VISION_AND_MISSION.md)、[03 定位](03_POSITIONING_AND_LANDSCAPE.md)

---

## 1. 用户分层总览

Rurix 的用户按采纳顺序分三波。**第一波是 MVP 与 G1 阶段的设计靶心**——所有 MVP 范围裁剪都以第一波用户的旗舰用例为判据；第二、三波只影响远期路线图，不影响 MVP 设计。

| 波次 | 用户群 | 进入时点 | 对应阶段 |
|---|---|---|---|
| 第一波 | Compute kernel 作者、GPU 系统程序员、仿真/视觉计算开发者 | MVP（语言可用即有价值） | M-系列（[11](11_ROADMAP.md) §3） |
| 第二波 | 实时渲染工程师、图形引擎开发者 | G1/G2（图形管线落地后） | G-系列（[11](11_ROADMAP.md) §4） |
| 第三波 | 引擎/工具链构建者、生态库作者 | 开源 + 包管理 + FFI 成熟后 | 3 年期（[11](11_ROADMAP.md) §5） |

## 2. 六类用户画像

### U1 — Compute kernel 作者（第一波，MVP 靶心用户）

- **现状**：用 CUDA C++ 写自定义 kernel（受够指针越界与竞态调试），或用 Triton/Numba（受够 JIT 抖动、弱类型与性能天花板）。深度依赖 Nsight Compute 调优。
- **痛点排序**：① shared memory 分块/同步写错只能运行时炸或静默错果；② host/device 内存拷贝与生命周期管理样板繁重且易错；③ Python DSL 表达不了复杂数据结构与泛型 kernel。
- **Rurix 给的东西**：views 类型化分块（写错分区是编译错误）；affine Buffer + 显式拷贝 API（use-after-free 是编译错误）；与 host 共享的泛型与模块系统；DLPack 零拷贝接入既有 PyTorch 工作流（[09](09_STDLIB_AND_ECOSYSTEM.md) §6）。
- **旗舰用例**：把一个 PyTorch 项目里的瓶颈算子换成 Rurix kernel——`rx build` 产出 PYD，Python 侧 `from_dlpack` 直接调用，性能 ≥ 手写 CUDA 90%，全程没碰过一个裸指针。
- **采纳判据**：单 kernel 迁移成本 < 1 天；Nsight 时间线/源码关联开箱即用；不要求改变既有 Python 工程结构。

### U2 — GPU 系统程序员（第一波）

- **现状**：写运行时、调度器、内存分配器、多 stream 流水线的人。用 CUDA Driver API + C++，被 context 线程局部状态、流序分配时序契约、WDDM 行为差异反复咬伤（r4 列举的全部陷阱都是这群人的日常）。
- **痛点排序**：① context/stream/event 生命周期错误只在特定时序下偶现；② Windows WDDM 与 Linux 行为差异无类型层表达；③ 多 GPU/多 context 代码的正确性靠 code review 维持。
- **Rurix 给的东西**：Context-affine 资源模型（资源持有 context 标记，跨 context 误用编译报错）；Stream/Event 的所有权语义；WDDM/TCC/MCDM 作为运行时一等探测状态（[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §5、[08](08_RUNTIME_AND_TOOLING.md) §2）。
- **旗舰用例**：一个三 stream 重叠（H2D / compute / D2H）的流水线，跨线程提交，所有同步点由 Event 类型连接——编译器静态拒绝"record 与 wait 不在同一 context"“buffer 在归属 stream 完成前被释放"这两类上一代运行时最常见的 bug。
- **采纳判据**：Driver API 全部核心能力族可达（不被语言抽象阉割）；`unsafe` 逃生舱完整；运行时开销相对手写 Driver API 代码 ≤ 3%。

### U3 — 仿真、可视化与 AI 加速图形开发者（第一波）

- **现状**：物理仿真（流体/布料/刚体）、科学可视化、神经渲染研究（NeRF/3DGS）。今天用 Taichi/Warp（表达力天花板）或 CUDA C++（开发效率低）。上一项目 P14 的 8 个特性包就是这类工作在 Python 宿主上撞墙的实录。
- **痛点排序**：① 粒子/网格数据结构在 DSL 里表达受限；② host 侧编排逻辑与 kernel 割裂、调试困难；③ 研究代码到生产代码要换语言重写。
- **Rurix 给的东西**：单语言写完 host 编排 + kernel；泛型 + 编译期求值支撑数据结构抽象（[05](05_LANGUAGE_ARCHITECTURE.md) §8/§9）；G0 软光栅路径提供"compute-only 也能出图"的可视化通道（[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §7）。
- **旗舰用例（MVP 验收 demo 之一）**：一个 SPH 流体仿真 + compute 软光栅渲染的端到端程序——粒子更新 kernel、空间哈希 kernel、光栅化 kernel、host 帧循环全部 Rurix，单可执行文件，输出图像序列。
- **采纳判据**：数学库（Vec/Mat）与几何原语开箱即用；从仿真状态到图像的通路不需要第二门语言。

### U4 — 实时渲染工程师（第二波，G1/G2 靶心用户）

- **现状**：写渲染特性（光照、阴影、后处理、RT）的人。语言现状是 HLSL/Slang + C++ host 的双语言缝合；descriptor/PSO/barrier 管理是 bug 高发区。
- **痛点排序**：① shader 与 host 间的绑定/布局契约靠手工同步；② 资源状态转换（barrier）错误只能靠 validation layer 事后抓；③ shader 语言没有模块/泛型/包管理。
- **Rurix 给的东西**：G2 阶段的单语言管线——绑定布局由编译器从类型推导并双侧检查；pass 间资源依赖显式建模（上一项目 RenderGraph 经验的语言化，H02 §4）；kernel 子语言的模块/泛型直接服务 shader 复用。
- **旗舰用例（G2 验收 demo）**：一个 deferred 渲染器（GBuffer → Lighting → Postprocess → Present），全部 pass 用 Rurix 写成，descriptor 布局零手写，资源 barrier 由 pass 依赖图推导。
- **采纳判据**：性能与 HLSL 手写管线相当；可与现存 D3D12 引擎逐 pass 渐进混用（interop 优先于全量替换）。

### U5 — 图形引擎开发者（第二波）

- **现状**：维护引擎的 RHI/render graph/资源系统的人。上一项目 backend/ 的全部痛点持有者。
- **痛点**：引擎核心层（资源生命周期、并行录制、管线缓存）的不变量全靠纪律维持；C++ 的 UB 与 shader 的不可类型化使核心层重构风险极高。
- **Rurix 给的东西**：把上一项目用 90+ 计数器和字节级 guardrails 人工维持的不变量（1 submit / 0 bridge、缓存命中率、RID 生命周期）变成类型系统与所有权检查的自然结果。
- **旗舰用例**：用 Rurix 实现一个最小 RHI + render graph 核心（G2 后期），对照上一项目的 Python 实现做"同一组不变量，类型系统拦截 vs 计数器事后观测"的对比报告。
- **采纳判据**：C ABI FFI 成熟（引擎是渐进采纳，必须能嵌入 C++ 工程）；编译时间可控（增量 check < 5s，[07](07_COMPILER_ARCHITECTURE.md) §6）。

### U6 — 引擎/工具链构建者与生态库作者（第三波）

- **现状**：会为新语言写库、写绑定、写工具的早期社区贡献者。
- **Rurix 给的东西**：可测试规范 + conformance 套件（贡献语义有明确边界）；无任意构建脚本的包管理（供应链可信）；cbindgen 式 C ABI 导出与 nanobind Python 通道（库可以服务语言外用户）。
- **旗舰用例**：第三方维护的 cuDNN 完整绑定包、几何处理库、ECS 框架。
- **采纳判据**：RFC 流程透明可参与；stable 面承诺可信（[10](10_GOVERNANCE.md) §6）。

## 3. 反用户（明确不服务的群体）

为防止范围蔓延，以下群体的需求**不进入** MVP 与 G1/G2 的设计考量（对应 [03](03_POSITIONING_AND_LANDSCAPE.md) §4 死亡路线）：

| 群体 | 不服务的原因 | 给他们的答案 |
|---|---|---|
| 需要跨厂商/跨平台部署的团队 | 与 CUDA-first 冲突；过早跨平台是已验证的资源黑洞（H06 §5、r10） | 留在 SYCL/WGSL/wgpu；Rurix 远期再评估可移植层 |
| 期望自动并行/自动调优的用户 | Bend/HVM 路线已验证"易用但厂商与专家都不买账"（r10） | 留在 Triton/PyTorch；Rurix 提供显式控制 |
| 想要 Python 级交互式工作流的研究者 | 嵌入式 DSL 是上一项目证伪的路线 | 经 DLPack 互操作使用 Rurix kernel，宿主仍是 Python |
| 通用应用/后端开发者 | Rurix 不做通用语言的全域生态（web/异步 IO 等） | Rust/Go；Rurix 经 C ABI 被它们调用 |

## 4. 用例 → 语言能力映射表

旗舰用例反推出的能力需求，作为后续文档的设计输入（编号供引用）：

| 用例编号 | 用例 | 关键语言/工具能力 | 落点文档 |
|---|---|---|---|
| UC-01 | PyTorch 瓶颈算子替换（U1） | kernel 子语言、DLPack/`__cuda_array_interface__`、PYD 产出、性能 ≥ 90% 手写 CUDA | 05/06/09 |
| UC-02 | 三 stream 重叠流水线（U2） | affine Context/Stream/Event/Buffer、跨线程所有权转移、流序分配类型化 | 05/06/08 |
| UC-03 | SPH 仿真 + 软光栅端到端（U3，MVP 验收 demo） | 泛型 kernel、Vec/Mat 数学库、图像输出、单可执行文件分发 | 05/09/11 |
| UC-04 | Deferred 渲染器（U4，G2 验收 demo） | D3D12 管线建模、绑定布局推导、pass 依赖图 | 06/11 |
| UC-05 | 最小 RHI + render graph（U5） | C ABI FFI、增量编译、资源不变量类型化 | 05/07/08 |
| UC-06 | 第三方生态库（U6） | 包管理、conformance、稳定性承诺 | 09/10 |

## 5. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
