# Graph API 评估 — Spike Report（G1.2，D-G1-2 子项，G-G1-2）

| 字段 | 值 |
|---|---|
| 类型 | **Spike report**（评估产出物;**非立项、非实现**）。立项与否由 **owner 人工裁决留痕**;触发新扩张方向才登记 `registry/spike_gating.json` SG-###（**AI 不自行立项**，AGENTS 硬规则 8 / 10 §3）。 |
| 承接 | G1.2 流序分配 AsyncBuffer 与 **Graph API 评估**（11 §4 G1 定义 / 08 §2.2 / D-232） |
| 范围 | CUDA Graph API × 流序分配 `AsyncBuffer<'stream,T>` 交互 / CUB-Thrust 实现对标 / 立项决策树 + 推荐 |
| 状态 | **owner 已裁决不立项**（2026-06-19，§7）。defer 至 G1.3 出现实测 launch-overhead 瓶颈时或 G2 重评估；不改语言核心、不建管线、不登记 SG-010。 |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8` |

---

## 1. 评估范围与背景

11 §4 把 G1 期定义含「流序分配 AsyncBuffer **+ Graph API 评估**」。08 §2.2 / D-232 锁定 G1 运行时内存策略为 stream-ordered allocator（`cuMemAllocAsync` + `CUmemoryPool`，G1.2 已落地，RXS-0144~0148），并把 VMM / 更激进的执行模型留 G2。本 spike 评估 **CUDA Graph API 是否应在 Rurix 立项**:它与刚落地的流序分配如何交互、相对 CUB/Thrust 生态能带来什么、以及立项的成本与触发条件。

**结论先行（详见 §6）**:建议 **G1.2 不立项 Graph API**,留 G1.3（引擎集成 per-frame compute pass 可能暴露 launch 开销瓶颈)或 G2 重评估。立项与否 owner 裁决。

## 2. CUDA Graph API 技术面

CUDA Graph 把一串操作（kernel launch / memcpy / memset / host callback / event record·wait / **memory alloc·free node** / child graph）固化为 DAG,实例化后可低开销重复 launch:

- **构造路径**:① **stream capture**（`cuStreamBeginCapture`/`cuStreamEndCapture` 把一段 stream 操作序列录成图,既有 stream 代码近乎零改动）;② **显式 API**（`cuGraphAddKernelNode` 等逐节点构造）。
- **执行**:`cuGraphInstantiate` → 可执行图;`cuGraphLaunch` 重复发射,**摊薄 CPU 端 per-launch 开销**(对「多个小 kernel 重复发射」的工作负载,如迭代求解器 / 逐帧 compute pass,收益显著;对少量大 kernel 收益微小)。
- **memory node**（CUDA 11.4+,`cuGraphAddMemAllocNode` / `cuGraphAddMemFreeNode`):把流序分配/释放**录入图**,得「**graph-ordered allocation**」——分配生命周期绑定到图的一次执行,图可重复 launch 则 alloc/free 节点随之重复执行。
- **价值定位**:Graph 是 **launch 开销优化**,正交于「算什么」;不改变 kernel 语义 / 数值结果。

## 3. 与流序分配 `AsyncBuffer<'stream,T>` 的交互（核心评估点）

G1.2 `AsyncBuffer` 的 typestate 假设 **eager 流序执行**:`alloc_async` 即时发 `cuMemAllocAsync`、`share_with` 即时发 `cuEventRecord`+`cuStreamWaitEvent`、`Drop` 即时发 `cuMemFreeAsync`。Graph 引入两处张力:

1. **capture mode 下副作用延迟**:在 `cuStreamBeginCapture` 与 `EndCapture` 之间,流序 API（含 `cuMemAllocAsync` / `cuStreamWaitEvent` / `cuMemFreeAsync`)**不即时执行**,而被驱动录入图,真正执行在 `cuGraphLaunch`。`AsyncBuffer` 的 Rust 调用点与 GPU 副作用**解耦**——既有 eager typestate 的「调用即排队」心智模型在 capture 下不再成立。驱动 *会* 自动 capture 这些调用,故 API 表面「可能能跑」,但**语义需重新论证**。
2. **graph-scoped 生命周期 vs affine 所有权生命周期**:图可重复 launch → alloc/free memory node 重复执行;而 `AsyncBuffer` 的 Rust 单一所有权 + `Drop = cuMemFreeAsync` 假设**一次分配一次释放**。二者生命周期模型**冲突**:graph 内的分配由图管理(图销毁时回收),Rust 侧的 affine Drop 会与之重复/错配。要安全支持,需要么 (a) 独立 `GraphBuffer` 类型(其操作构图节点而非即时发射),要么 (b) 让 `AsyncBuffer` capture-aware(typestate 区分 eager / capture 两态)。

**判断**:Graph × 流序分配的安全交互**触及执行模型**（capture 态 typestate + graph-scoped 分配生命周期映射),按 10 §3 属 **类型系统变更 / 运行时语义** → **Full RFC 档**(非 Mini,非 Direct);触 AGENTS 硬规则 5 边缘(执行模型/生命周期映射),须人工经 Full RFC 落笔。

## 4. CUB / Thrust 实现对标

- **CUB**（device-wide 并行原语:reduce/scan/sort/select…):**stream-based**,API 不依赖 Graph;用户可经 stream capture 把 CUB 调用录入图。
- **Thrust**（高层算法 + 执行策略 `thrust::cuda::par.on(stream)`):同样 stream-based、可被 capture。
- **对标结论**:CUB/Thrust **均不要求 Graph API**——都是 stream 模型 + 可选 capture。Rurix 要在**功能/算法面**对标 CUB/Thrust,**无需** Graph;Graph 仅是二者之上的 **launch 开销优化**,且对 Rurix 与对 CUB/Thrust 同等适用(谁都能被 capture)。Rurix 的差异化价值(affine 类型安全资源 + 编译期生命周期拦截)**正交于** Graph;Graph 不增强也不削弱该价值。
- **真分发面**:CUB/Thrust 是 header-only / 库层;若未来 Rurix 生态包(09 §5,geometry 后)需要 device-wide 原语,那是**库层**话题(对标算法集),与「语言核心是否立项 Graph」是两件事。

## 5. 立项决策树

```
Q1  G1 期是否存在被 kernel **launch 开销**(非 compute)瓶颈的具体工作负载?
    ├─ 否 → 不立项(无收益锚点)。  ← G1.2 当前状态(AsyncBuffer/interop 非 launch-bound)
    └─ 是(如 G1.3 引擎逐帧 compute pass 重复小 kernel)→ Q2
Q2  Graph 支持是否需要执行模型 / 类型系统变更?
    ├─ 是(capture 态 typestate + graph-scoped 分配生命周期,§3)→ **Full RFC 前置**(人工) → Q3
    └─ 否 → 不适用(本评估判定为「是」)
Q3  成本(新类型面 + 执行模型 + capture 语义 + golden/Sanitizer 回归)是否被 G1 优先级证成?
    ├─ 是 → owner 立项:登记 SG-###(下一可用 SG-010)+ Full RFC + feature gate + tracking issue
    └─ 否 → defer(G1.3 暴露具体需求时 / G2 重评估)
```

判据量化建议(供 owner 裁决):若某真实工作负载经 `rx bench` 实测 **launch 开销占端到端 ≥ 20%** 且 kernel 数 ≥ 数十/迭代,则 Q1=是,值得进入 Q2/Q3。

## 6. 推荐（AI 分析建议;owner 裁决）

**建议 G1.2 不立项 Graph API**,理由:

1. **无收益锚点**:G1.2（流序分配)与 G1.1（interop 呈现)均非 launch-overhead-bound;当前无被 launch 开销瓶颈的实测工作负载(决策树 Q1=否)。
2. **成本高且触执行模型**:Graph × 流序分配的安全交互需 capture 态 typestate + graph-scoped 分配生命周期映射,属 **Full RFC** 档(§3),非本子里程碑(Mini-RFC/MR-0001)范围。
3. **对标不要求**:CUB/Thrust 均 stream-based、不依赖 Graph(§4);Rurix 功能对标无需 Graph。
4. **自然评估时机后移**:**G1.3 引擎集成**(逐帧 compute pass,UC-05 前奏)是最可能暴露 launch 开销瓶颈的场景——届时以 `rx bench` 实测 launch 占比再过决策树 Q1,比 G1.2 凭空立项更有依据;否则留 G2(与 VMM / 多 GPU 一同重评估,08 §2.2)。

**若 owner 裁定立项**(决策树 Q3=是):由 owner 登记 `registry/spike_gating.json` **SG-010**(Graph API,triggered)+ 起 **Full RFC**(执行模型:capture 态 + graph-scoped 分配生命周期;feature gate + tracking issue + spec diff + conformance + stabilization report,10 §3)。**AI 不自行登记 SG、不起 RFC、不建管线**。

## 7. 裁决留痕（owner 人工裁决）

**裁决：G1.2 不立项 Graph API。** defer 至 G1.3 引擎集成出现真实工作负载后，按 §5 的决策树以 `rx bench` 实测 launch 开销占比复评；若 G1.3 仍无收益锚点，则留 G2 与 VMM / 多 GPU 一并重评估。当前不登记 SG-010、不起 Full RFC、不建 feature gate / tracking issue。

依据：§6 四点成立——G1.2 无 launch-overhead 收益锚点；Graph × `AsyncBuffer` 会引入 capture 态 typestate 与 graph-scoped 生命周期，必须 Full RFC；CUB/Thrust 对标不依赖 Graph；G1.3 是更自然的实测窗口。

留痕：owner 于 2026-06-19 本工作会话明确授权完成 G1.2 人工收尾并采纳 spike 的 defer 推荐；由 Codex 代录机器可核对文字，**非 AI 自行立项 / 代签署名承诺**。因裁决为“不立项”，未触发新扩张方向，故 `registry/spike_gating.json` 保持 0-byte、不登记 SG-010。

> 本裁决满足 G-G1-2 的「Graph API 评估产出 + 立项与否裁决留痕」要件（契约 §4 第 2 条）。
