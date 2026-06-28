# RFC-0006 — UC-04 deferred 渲染器 / 原生 D3D12 运行时出图路径

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0006（4 位制，编号永不复用，10 §9.5） |
| 标题 | UC-04 deferred 渲染器 demo / 原生 D3D12 运行时出图路径（多 pass：G-buffer + lighting + present/readback） |
| 档位 | **Full RFC**（10 §3：首次落 **D3D12 运行时面**——PSO 装配 / 资源状态机 / barrier 语义 / swapchain 呈现（06 §8.2 第 4/5 点，RFC-0003 §8 / RFC-0004 §8 显式 defer 到本面）；并触 AGENTS 硬规则 5 禁区边界——**纹理路径内存模型映射（06 §4.2 🔒）** / **D3D12 运行时 stable ABI** / **FFI ABI（host↔D3D12/DXIL 运行时边界）** / **barrier·资源状态并发语义**；本 RFC 只作设计面 + 边界声明 + owner 裁决清单（§9 已裁 2026-06-28），不落运行时语义本体、不实现 renderer） |
| 状态 | **Accepted / Owner Approved（2026-06-28）**。owner（Language Lead）已在本工作会话同意 RFC-0006 全文 + §9 全部裁决（Q-Present=offscreen-first / Q-DemoCrate=独立 demo crate（`src/uc04-demo`）/ Q-RuntimeShape=safe wrapper / Q-DeferredPass=G-buffer(albedo+normal+depth)→单光源→offscreen readback / Q-Barrier=首期手动 barrier 编排 / Q-Texture=不落纹理内存模型本体 / Q-Range=RXS-0167~0170 / Q-Err=6xxx 续号（自 RX6018）/ Q-Gate=新增 `d3d12-runtime`/`uc04-demo` 专属 gate / Q-RD=RD-019/020/021 按实际 scope append-only / Q-CIStep=step 48 offscreen REQUIRE_REAL）；记录由 AI 代录，非 AI 代签。下游 PR 仍按 §6 栈式序进（PR-F1 spec 脚手架先于 PR-F2 实现），🔒 禁区语义本体（纹理内存模型 / barrier 并发语义 / 运行时 stable ABI）仍须 owner 后续 Full RFC 落笔 |
| 承接里程碑 | G2.4（验收门 **G-G2-4**，D-G2-4），承 G2.1 着色阶段类型面（RFC-0002）+ G2.2 DXIL B 链 codegen（RFC-0003 / RFC-0004）+ G2.3 绑定布局推导（RFC-0005，owner Approved 2026-06-28）就位 |
| 关联条款 | 拟落 spec **RXS-0167 ~ RXS-0170**（§9 Q-Range 已裁，见 §5）；落点（新建 `spec/d3d12_runtime.md` vs 延伸既有文件）随 **PR-F1** 按实际 scope 定（本轮 §9 未单列 Q-File，比照 Q-RD 处置）。**本 RFC 不创建裸条款头**，trace 维持现状（当前最高现存 RXS-0166 @ [binding_layout.md](../spec/binding_layout.md)） |
| 依据决策 | D-002（图形分期，已批准）· 06 §8.2 第 4/5 点（PSO / 资源状态 / barrier 运行时面 = G2 设计预留）· 06 §4.2（纹理路径内存模型禁区，🔒）· 04 P-01（strict-only）· 04 P-13（防 AI 幻觉治理）· RFC-0002（着色阶段类型面）· RFC-0003 §8 / RFC-0004 §8（PSO/资源状态/barrier 运行时面 defer 到 G2.4）· RFC-0005（绑定布局推导 + RTS0）· RFC-0001（CUDA–D3D12 interop，D3D12 device/queue/swapchain 运行时先例） |
| Provenance | `Assisted-by: kiro:claude-opus-4-8`（Draft + owner 裁决落文档）。Human-in-the-loop（硬规则 1/2）：本草案由 AI 起草，§9 全部路径抉择由 owner（Language Lead）于 2026-06-28 裁决，AI 代录、非代签 / 不代决；禁区子节仅作边界声明，不落语义本体 |
| Owner 批准 | **Approved — owner（Language Lead）2026-06-28**。批准范围：RFC-0006 全文；§4.5 🔒 禁区边界声明（不落禁区语义本体）；§9 全部裁决（Q-Present=offscreen-first / Q-DemoCrate=独立 demo crate `src/uc04-demo` / Q-RuntimeShape=safe wrapper / Q-DeferredPass=G-buffer(albedo+normal+depth)→单光源→offscreen readback / Q-Barrier=首期手动 barrier 编排 / Q-Texture=不落纹理内存模型本体 / Q-Range=RXS-0167~0170 / Q-Err=6xxx 续号自 RX6018 / Q-Gate=`d3d12-runtime`/`uc04-demo` 专属 gate / Q-RD=RD-019/020/021 append-only / Q-CIStep=step 48 offscreen REQUIRE_REAL）。记录方式：AI 按 owner 本会话明确裁决代录，非 AI 代签；本批准不声称 device 真跑、golden bless、稳定化或禁区语义本体已完成 |

> **批准记录**：本 RFC 是 G2 期**首个触及 D3D12 运行时执行面**的 RFC——RFC-0002/0003/0004/0005 均把 PSO 装配 / 资源状态机 / barrier 语义 / swapchain 呈现显式 defer 到 G2.4（见 RFC-0003 §8「PSO / 资源状态 / barrier 运行时面…不在本 codegen RFC」、RFC-0004 §8）。这些面触及 🔒 纹理路径内存模型映射（06 §4.2）、D3D12 运行时 stable ABI、host↔运行时 FFI ABI、barrier/资源状态并发语义——只能由人类经 Full RFC 落笔（硬规则 5）。owner（Language Lead）于 2026-06-28 以人工裁决批准 RFC-0006 全文并裁决 §9 全部路径项（Q-Present / Q-DemoCrate / Q-RuntimeShape / Q-DeferredPass / Q-Barrier / Q-Texture / Q-Range / Q-Err / Q-Gate / Q-RD / Q-CIStep，见 §9）；AI 仅代录该人工决定，非代签 / 不代决。§4/§8 仍仅作**边界声明**，不落运行时语义本体；本批准不把任何禁区语义本体（纹理采样内存模型 / barrier 并发语义 / 运行时 stable ABI / FFI ABI 物理布局）冻结或落地，触及即另起 owner Full RFC。

---

## 1. 摘要

本 RFC 在**不实现 deferred renderer**、**不落条款体**、**不接线 D3D12 运行时 codegen/库**、**不改 CI workflow**、**不动 registry** 的前提下，定义 **UC-04 deferred 渲染器 demo** 端到端原生 D3D12 + DXIL 出图所需的**设计面 + 依赖闭合判据 + 红线边界 + 下游条款计划 + 真实红绿/device 见证计划**：

```
G2.1 着色阶段类型面（RFC-0002，vertex/fragment fn + 资源句柄 RXS-0156）
G2.2 DXIL B 链 codegen（RFC-0003/0004，图形=B：MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL）
G2.3 绑定布局推导 + RTS0（RFC-0005，descriptor/root signature 编译器推导，G-G2-3 已闭环）
                          │
本 RFC（G2.4，仅设计面 + 边界声明 + owner 裁决清单，§9 已裁）：
   ├─ deferred 管线最小形态：几何 pass（G-buffer：albedo/normal/depth 等 MRT）
   │                         → lighting pass（采样 G-buffer → 着色）
   │                         → present/readback（窗口呈现 或 offscreen 像素回读对照）
   ├─ D3D12 运行时面锚点：device/queue/PSO 装配 / 资源状态 + barrier / RTV·DSV·SRV 视图 / 命令录制
   ├─ 单一事实源消费：RTS0（RFC-0005）+ DXIL 着色器对象（RFC-0004）由运行时装配进 PSO
   ├─ strict-only 兜底：管线装配/资源状态/barrier 不合规 → 显式诊断（无静默降级，P-01）
   └─ device 见证 + run-url：原生 D3D12 hardware 多 pass draw + 像素对照（G-G2-4 验收）
                          │
   🔒 不在本 RFC：纹理采样内存模型映射（06 §4.2）/ barrier 并发语义本体 / D3D12 运行时 stable ABI /
                  FFI ABI 二进制布局 / 实现 codegen / golden bless / device 真跑（随 owner 批准后实现 PR）
```

本 RFC 只定义 UC-04 deferred 渲染器的**设计面、依赖闭合判据、运行时面锚点、错误类别与下游条款计划**；**运行时实现、PSO/资源状态/barrier 语义本体、纹理内存模型映射、golden、device 真跑均不在本 RFC**（随 owner 批准后实现 PR，条款先于实现，硬规则 7）。可行性以 G2.1~G2.3 已闭环事实（RFC-0002/0003/0004/0005 + G-G2-1~G-G2-3 device witness）为锚，严格区分「已 measured/已闭环」与「assumed 待 owner 裁 / 实现侧待建」（对齐 RFC-0004/0005 strict-only 诚实纪律）。

## 2. 动机

- **11 §5 / 06 §8.2 要求 G2 兑现 UC-04 deferred 渲染器**：G2 期成功判据之一 = UC-04 deferred 渲染器 demo 端到端原生 D3D12 + DXIL 出图（多 pass deferred 管线）。G2.4（D-G2-4 / G-G2-4）是该判据的落地子里程碑，且是 G2.1~G2.3 语言面 + codegen 面 + 推导面的**首个端到端集成验证点**。
- **运行时面是 G2.1~G2.3 共同 defer 的最后一段**：RFC-0002 §8（「UC-04…不在本 RFC」）、RFC-0003 §8（「PSO / 资源状态 / barrier 运行时面…属运行时/库级职责，不在本 codegen RFC」）、RFC-0004 §8、RFC-0005 §1（「PSO / resource state / barrier 运行时面与 UC-04 deferred renderer…device 真跑出图归 G2.4 后续证据」）均把**运行时执行面**显式 defer 到 G2.4。本 RFC 是这些 defer 的承接点——首次需要定义 D3D12 运行时如何装配 DXIL 着色器对象 + RTS0 root signature 成可执行的多 pass 管线。
- **承接 G2.3 已闭环的真实产物**：G-G2-3 已 device 见证 RTS0 root signature 经 `CreateRootSignature` accept + textured PSO + `Texture2D<f32>` 经 `Sampler` 绑定 + offscreen draw 采样像素对照（run 28319166995 / run 28319066260）。但该见证是**单 pass textured draw 冒烟**，非 UC-04 要求的**多 pass deferred 管线**（G-buffer MRT → lighting → present）。deferred 管线需要 MRT 渲染目标、G-buffer 资源状态转换（render target ↔ shader resource）、多 PSO 编排——这些是 G2.3 冒烟未覆盖的新面，需独立 Full RFC 精确化。

**为何需要 Full RFC（而非 Direct/Mini）**：本 RFC 首次引入 **D3D12 运行时执行面**（PSO 装配 / 资源状态机 / barrier 语义 / swapchain 呈现），且触及 **纹理路径内存模型映射**（06 §4.2 🔒：deferred 的 G-buffer 写入/采样、render target ↔ shader resource 状态转换涉及纹理访问语义）、**D3D12 运行时 stable ABI**、**host↔运行时 FFI ABI 二进制布局**、**barrier/资源状态并发语义**——10 §3 / 硬规则 5 明列的 Full RFC / 禁区触发面。判档争议向上取严（硬规则 8）；AI 不自判 Direct/Mini、不代签批准/合并、不代 owner 裁 §9（硬规则 1）。

## 3. 指导级解释（用户视角）

> 以下为**拟议**形态示意，最终以 owner 批准 + spec 条款为准；**deferred 管线对用户尽量声明式**——用户写着色阶段函数（RFC-0002）+ 声明资源句柄（RXS-0156，绑定布局由 RFC-0005 推导），运行时把 DXIL 着色器对象 + RTS0 装配成多 pass 管线，用户不手写 D3D12 样板（device/queue/PSO/barrier 由 demo 运行时层承担）。

UC-04 deferred 渲染器把场景几何先渲染到一组 G-buffer 渲染目标（几何 pass），再用 lighting pass 采样 G-buffer 做延迟着色，最后呈现/回读：

```rust
// 几何 pass 着色阶段（RFC-0002 类型面 + RFC-0005 绑定推导）
vertex fn gbuffer_vs(in: VertexIn) -> GBufferVaryings { /* ... */ }
fragment fn gbuffer_fs(in: GBufferVaryings) -> GBufferTargets { /* 写 albedo/normal/depth MRT */ }

// lighting pass 着色阶段（采样上一 pass 的 G-buffer 作为 shader resource）
fragment fn lighting_fs(
    in: FullscreenVaryings,
    albedo: Texture2D<f32>,   // ← 上一 pass 的 G-buffer，运行时做 RT→SRV 状态转换（拟议）
    normal: Texture2D<f32>,
    samp: Sampler,
) -> FragmentOut { /* 延迟着色，采样语义的内存模型映射不在本 RFC，留禁区 Full RFC */ }
```

运行时（demo 层，形态见 §9 Q-RuntimeShape）负责：创建 D3D12 device/queue/swapchain（或 offscreen 目标）、用 RFC-0004 产的 DXIL + RFC-0005 推导的 RTS0 装配每个 pass 的 PSO、在 pass 间插入资源状态 barrier（G-buffer：`RENDER_TARGET` → `PIXEL_SHADER_RESOURCE`）、录制命令、提交、呈现或回读像素。

`strict-only`（P-01）维持：管线装配失败（PSO 创建失败 / 资源状态非法 / barrier 缺失或冲突 / RTS0 与 PSO 不匹配）→ **结构化错误**，无静默降级、无运行期 fallback。

> **拟议中的开放问题（不在本节定型，全部留 §9 owner 裁决）**：呈现策略（窗口 swapchain present / offscreen-first 像素回读 / 两阶段先 offscreen 后窗口）、demo crate 位置与边界、D3D12 运行时封装层形态（薄 FFI 绑定 / safe wrapper crate / demo 内联）、deferred 最小 pass 集（G-buffer 通道数 / lighting 模型 / 是否含 present）、资源状态与 barrier 最小模型（手动 barrier API / 状态跟踪推导）、纹理内存模型是否触及 06 §4.2——本 RFC **不替用户/owner 预定**，§3 示例仅示意声明式意图，不示意已定的运行时策略。

## 4. 参考级设计

> 本节落笔 **deferred 管线与 D3D12 运行时面的设计锚点 + 禁区边界声明**；具体运行时 API 形态、barrier 语义本体、纹理内存模型映射、PSO 装配 codegen 均**不**由本 RFC 发明，留 §9 owner 裁决 + 后续实现 PR / 禁区 Full RFC。

### 4.1 deferred 管线最小形态（设计锚点）

UC-04 deferred 渲染器的最小可验收形态（§9 Q-DeferredPass 已裁：G-buffer albedo+normal+depth → 单光源 lighting → offscreen readback）：

- **几何 pass（G-buffer 生成）**：场景几何经 `gbuffer_vs`/`gbuffer_fs`（RFC-0002 着色阶段）渲染到多渲染目标（MRT：至少 albedo + normal + depth；通道数/格式留 §9）。绑定布局由 RFC-0005 推导，DXIL 由 RFC-0004 B 链产出。
- **lighting pass（延迟着色）**：全屏 pass 采样几何 pass 写出的 G-buffer（作为 shader resource）做延迟光照计算，输出到中间色彩目标或直接到呈现目标。
- **offscreen readback**：offscreen 渲染后回读像素做数值对照（§9 Q-Present 已裁 = offscreen-first；窗口 swapchain present 作后续可选阶段，不阻塞 G-G2-4，defer 登 RD-019）。
- **strict-only 装配**：每 pass 的 DXIL 着色器对象 + RTS0 root signature 经运行时装配进 PSO；装配不一致（RTS0 与着色器绑定不匹配 / RT 格式与 PSO 不匹配）→ 显式错误，无静默降级（P-01）。

### 4.2 D3D12 运行时面锚点（设计锚点，本体留 §9 / 实现 PR）

deferred 管线需要的 D3D12 运行时执行面（首次在本 RFC 承接，承 RFC-0001 device/queue/swapchain interop 先例）：

- **device / command queue / swapchain（或 offscreen 目标）**：复用/扩展 RFC-0001 的 D3D12 device 互操作基座；窗口呈现 vs offscreen = §9 Q-Present。
- **PSO 装配**：把 RFC-0004 产的 DXIL 着色器对象（VS/PS）+ RFC-0005 推导的 RTS0 root signature + 渲染目标格式/混合/深度状态组装成 graphics PSO。RTS0 与 PSO 一致性核验（承 G-G2-3 RTS0 `CreateRootSignature` accept 见证）。
- **资源视图**：G-buffer 的 RTV（render target view）/ DSV（depth stencil view）/ SRV（shader resource view，lighting pass 采样用）创建与绑定。
- **命令录制 + 提交 + 同步**：command list 录制多 pass draw + barrier + present/copy，queue 提交，fence 同步。
- **资源状态 + barrier**：pass 间 G-buffer 资源状态转换（`RENDER_TARGET` → `PIXEL_SHADER_RESOURCE` → 回 `RENDER_TARGET`）的 barrier 编排。**barrier 语义本体（并发/可见性语义）= §9 Q-Barrier + 🔒 边界（§4.5）**。

### 4.3 单一事实源消费（承 G2.1~G2.3，不新增语言构造）

本 RFC 是运行时集成面，不新增语言构造：着色阶段类型面（RFC-0002）、DXIL codegen（RFC-0004）、绑定布局推导 + RTS0（RFC-0005）均已就位。本 RFC 定义运行时**如何消费**这些产物装配可执行管线，运行时装配以编译期推导的 RTS0 + DXIL 为单一事实源（P-11），不在运行时手维护第二份绑定布局。

### 4.4 strict-only 装配核验（设计面，承 P-01）

deferred 管线装配的 strict-only 核验面（具体诊断粒度/错误类别 = §9 Q-Err + 实现 PR）：

- PSO 创建失败（DXIL 着色器对象非法 / RT 格式不匹配 / RTS0 与着色器绑定不一致）→ 显式错误。
- 资源状态非法转换 / barrier 缺失导致的状态不一致 → 显式错误（运行时校验形态留 §9 Q-Barrier）。
- 无静默降级、无运行期 fallback（P-01）；device 真跑出图正确性由 G-G2-4 像素对照兜底。

### 4.5 🔒 禁区边界声明（owner 落笔，不在本 RFC 落语义本体）

> 以下子节标 🔒，**只作边界声明**，语义本体须由人类经 Full RFC 落笔（硬规则 5）。本 RFC 不发明任何采样 opcode、内存序、barrier 并发语义、运行时 ABI 二进制布局。

- **(a) 纹理路径内存模型映射（06 §4.2 🔒）**：deferred 的 G-buffer 写入（MRT render target）与 lighting pass 采样（SRV）涉及纹理访问语义 / 采样 opcode / 描述符编码 / 缓存一致性 / LOD/导数 / 越界采样后果。本 RFC **不**落这些语义本体；凡需要纹理访问内存模型映射的语义，标「需人工升档」，留独立 Full RFC（与 RFC-0004 §4.6(b) 同级边界）。资源句柄维持 opaque 形态（RFC-0002 RXS-0156 / RFC-0004 §4.6(b)）。
- **(b) barrier / 资源状态并发语义本体**：pass 间资源状态转换 barrier 的**并发/可见性/内存序语义**触及内存模型面。本 RFC 仅定义 barrier 在 deferred 管线中的**编排锚点（哪里需要状态转换）**，不定义 barrier 的并发语义本体；后者标「需人工升档」（§9 Q-Barrier 裁边界，语义本体留禁区 Full RFC）。
- **(c) D3D12 运行时 stable ABI**：运行时封装层（device/queue/PSO/command list 的 host↔D3D12 边界）的二进制 ABI / 接口布局**不冻结为 stable 语言/运行时 ABI**（与 RFC-0004 §4.6(a) / RFC-0005 RXS-0165 🔒 同级：实现确定、gate 后、非 stable）。runtime 接口形态 = §9 Q-RuntimeShape，stable 面定义随 RD-008（G2.5 候选触发点），不在本 RFC 冻结。
- **(d) host↔运行时 / host↔DXIL FFI ABI 二进制布局**：运行时调用 D3D12（经 FFI）的 ABI 布局属硬规则 5 FFI ABI 面，承 RFC-0001 interop ABI 边界纪律，不在本 RFC 发明新 ABI；凡触及 stable ABI 冻结，标「需人工升档」。
- **(e) DXIL/SPIR-V UB 边界**：不建立独立于源码语义的 DXIL/SPIR-V UB 契约（承 RFC-0004 §4.6(c)）；依赖未建模行为的运行时 lowering 须显式拒绝。

## 5. 下游 spec 条款计划表（spec diff，10 §3 要件；不落条款体）

落点（新建 `spec/d3d12_runtime.md` vs 延伸既有文件）随 **PR-F1** 按实际 scope 定（本轮 §9 未单列 Q-File，比照 Q-RD 处置）。**本 RFC 不创建 `### RXS-####` 裸条款头**——下表为条款的**计划表**，条款体随 owner 批准本 RFC 后的实现 PR 同落（条款 PR 先于实现 PR，硬规则 7；trace 维持全锚定）。**区间 §9 Q-Range 已裁定锁 RXS-0167 ~ RXS-0170**（4 条，对齐 Q-Present=offscreen-first / Q-DeferredPass 最小集），下表条款号即裁定区间（当前最高现存 RXS-0166 @ binding_layout.md，自 RXS-0167 起续号）。

| 条款（§9 Q-Range 已裁锁定） | 标题 | 测试锚定计划（每条 ≥1，`//@ spec`） |
|---|---|---|
| RXS-0167 | DXIL + RTS0 → graphics PSO 装配一致性 | PSO 装配 accept（RTS0/着色器/RT 格式一致）+ reject（不一致 → strict-only 显式错）+ host 侧装配核验 |
| RXS-0168 | deferred 多 pass 编排（几何 pass MRT → lighting pass 采样 G-buffer → offscreen readback） | 多 pass 编排 accept + reject（pass 顺序/目标缺失）+ device 像素对照 |
| RXS-0169 | 资源状态 + barrier 编排锚点（pass 间 RT → SRV → RT/Copy/Readback 状态转换；首期手动编排；🔒 并发语义本体不在本条） | 状态转换编排 accept + reject（缺 barrier/非法转换 → strict-only）；🔒 并发语义本体「需人工升档」 |
| RXS-0170 | offscreen readback + 像素对照（Q-Present=offscreen-first；窗口 present 不进必要条款，登 RD-019） | offscreen readback accept + device 像素对照（REQUIRE_REAL，Q-CIStep）；窗口 present 路径作 RD 后续 |

> 上表条款号、条数、拆分由 **§9 Q-Range owner 裁定锁定 RXS-0167 ~ RXS-0170**（PR-F1 只登记预留区间不落裸条款头，条款体与锚定测试随 PR-F2 同落）。🔒 纹理访问内存模型映射（06 §4.2）/ barrier 并发语义本体 / 运行时 stable ABI / FFI ABI 二进制布局**不进任何条款**（§4.5），触及即停手标「需人工升档」，另起 owner Full RFC。

- **错误码策略（§9 Q-Err 已裁）**：编译期/装配期可预测错误**续用 6xxx codegen/装配段，自 RX6018 起**按真实可达类别追加（当前 6xxx 段最高现存 RX6017）。**本 RFC 不预留、不预造、不落码、不改 `registry/error_codes.json`**——具体码随实现 PR（PR-F2）按真实可达类别只追加分配 + en/zh message-key（`ci/bilingual_coverage.py` 覆盖）。**D3D12 API 返回的纯运行期/环境失败不滥发语言 RX**，作为 smoke/evidence runtime failure 报告；若后续需稳定运行时诊断段，再由 owner 单独裁新段位。纯 Rust 通用错误走 rustc 原生诊断（零新 RX）。

## 6. feature gate / tracking / 实现序 + 真实红绿 + device 见证（10 §3 要件）

- **feature gate（§9 Q-Gate 已裁）**：新增**运行时/demo 专属 gate**（推荐 `d3d12-runtime` 或 `uc04-demo`，终名随 PR-F1），**不把 D3D12 runtime 面塞进 `dxil-backend`**——`dxil-backend` 只作为 codegen 前置依赖。新 crate（Q-DemoCrate=独立 demo crate `src/uc04-demo`）默认 `unsafe_code=deny`，D3D12 运行时边界若必须 unsafe 须集中到最小 runtime module、按硬规则 9 每 `unsafe` 块 `// SAFETY:` + unsafe-audit 注册（U23 续号）。
- **栈式 PR（本 RFC 已 owner Approved + §9 已裁，闸口开启）**：
  - **PR-F1 spec 脚手架**：新建/延伸 spec 文件登记 **RXS-0167 ~ RXS-0170** 预留区间与计划映射（**不落裸条款头**）+ spec/README §4 同步 + RD-019/020/021 按实际 scope append-only 落 `registry/deferred.json`（Q-RD）+ 落点（`spec/d3d12_runtime.md` vs 延伸）定调；`trace_matrix --check` PASS。
  - **PR-F2 spec 条款体 + 运行时实现**：RXS-0167~0170 条款体 + `src/uc04-demo` 独立 demo crate（`unsafe_code=deny`）+ safe wrapper D3D12 device/queue/PSO/command list/resource/barrier 封装 + 首期手动 barrier 编排 + offscreen readback + 错误码自 RX6018 落码 + golden（若有）+ bless + device 真跑。
  - **CI 步骤 48**（CI_GATES §2 计划项，UC-04 deferred offscreen 冒烟）随实现 PR 回填 workflow；策略（§9 Q-CIStep 已裁）= offscreen readback REQUIRE_REAL（`RURIX_REQUIRE_REAL=1` 缺 D3D12/MSVC/signed DXC pin/validator/GPU 即红，窗口 present 路径若存在可 SKIP 但不替代 offscreen 真跑）。
- **防降级验收线（G-G2-4 硬门）**：绿色证据必须证明 **Rurix source → rurixc 图形=B DXIL 路径 → RFC-0005 RTS0 / 绑定布局 → D3D12 graphics PSO → hardware 多 pass deferred draw → offscreen readback** 的闭环；至少一个 G-buffer pass 着色器与一个 lighting pass 着色器须来自 Rurix 源码经编译链产物（手写 HLSL/DXIL 只能作对照、探针或 red/tamper 输入，不能作为 G-G2-4 green 主证据）。若 RD-013（DXIL 着色阶段入口 body 数据流降级）或其他前置缺口导致 Rurix 产物不可达，必须停为 `blocked-on-RD-013`/对应 RD，**不得**用手写 HLSL、CPU 预填纹理、单 pass textured draw、fullscreen copy、固定像素注入、host-only 模拟或窗口 present 截图替代验收。
- **真实红绿（反 YAML-only，CI_GATES §6 第 4 项）**：构造 deferred 管线 pass 结果篡改 / 像素篡改 → 红 → 复原绿，归档前后输出 / run URL（G-G2-4 验收门）。host 段可达面（PSO 装配一致性 / 资源状态编排核验）与 device 段（原生 D3D12 hardware 多 pass draw + 像素对照）分轴，对齐 G-G2-2/G-G2-3 host+device 双段先例。
- **device 见证 + run-url 要求（G-G2-4 验收硬要件，反 YAML-only）**：
  - device 见证须为**原生 D3D12 hardware 多 pass deferred draw**（几何 pass MRT → lighting pass 采样 G-buffer → offscreen readback，Q-Present=offscreen-first），非 G2.3 的单 pass textured draw 冒烟；输出含 adapter 名 + 多 pass 状态 + 采样/offscreen 像素对照（对齐 G-G2-3 `DXIL_BIND: ok adapter=... draw=ok` 日志范式）。
  - step 48 的 REQUIRE_REAL green 不得由 SKIP、窗口 present 截图、外部 HLSL-only 管线或 host 侧 digest/golden 代替；若运行环境缺失，CI 可按策略红/skip，但 **G-G2-4 owner 签字必须等待真实 device run URL**。
  - run-url 须为**真实 GitHub Actions device 见证入口**（AI 不伪造，硬规则 1），device witness 回填 `evidence/g2.4-uc04-deferred/`（evidence 只增不删不改，M0.3 起）。
  - signed pin 纪律延续 G-G2-2/G-G2-3（`RURIX_DXC_DIR` dxc+dxv+dxil.dll 三件齐备方认定签名 pin；`RURIX_REQUIRE_REAL=1` 缺 validator/D3D12/MSVC 即红）。
  - **G-G2-4 签字归 owner**（AI 代录非代签，硬规则 1）：device run URL、golden bless（若有）、子里程碑签字、CI step 48 落地由 owner 兑现。
- **依赖与序**：本 RFC（G2.4 运行时集成面）门控于 G2.1（RFC-0002）+ G2.2（RFC-0003/0004）+ G2.3（RFC-0005，G-G2-3 已闭环）就位；UC-04 是 G2.1~G2.3 的首个端到端集成验证点。

## 7. 备选方案

- **窗口呈现优先（swapchain present 直接出图）**：可视化直观，但 CI runner 无显示环境 → device 见证须降级 SKIP，削弱 G-G2-4 的 CI 红绿可验证性。留 §9 Q-Present 由 owner 权衡（窗口 / offscreen-first / 两阶段）。
- **offscreen-first 像素回读对照**：无需显示环境，CI 可 device 真跑 + 像素对照（对齐 G-G2-2/G-G2-3 offscreen readback 先例），可验证性强；代价 = 不直接演示窗口呈现。亦留 §9 Q-Present。
- **forward 渲染替代 deferred**：否决——UC-04 明确要求 deferred 渲染器（11 §5），forward 不满足多 pass G-buffer 验收面。
- **运行时走通用多后端抽象（wgpu 式 / Vulkan/Metal 后端）**：否决——死亡路线红线 3（D-008 维持，SG-003 not_triggered）；本 RFC 是 D3D12 原生路径纵深，非通用多后端。
- **把 PSO/资源状态/barrier 语义本体在本 RFC 落笔**：否决——触 🔒 纹理内存模型映射 / barrier 并发语义 / 运行时 ABI（硬规则 5），须 owner 经 Full RFC 落笔；本 RFC 仅作边界声明 + 编排锚点。

## 8. 不做（范围红线）

- **运行时/库实现**：D3D12 运行时封装层、PSO 装配、资源状态/barrier 编排、deferred demo crate、command list 录制均不在本 RFC（随 owner 批准后实现 PR，§6）；不动 `src/*`、不建 demo crate、不建 golden、不改 CI workflow。
- **🔒 纹理路径内存模型映射（06 §4.2）**：G-buffer 写入/采样的纹理访问语义 / 采样 opcode / 描述符编码 / 缓存一致性 / LOD/导数 / 越界后果 = 禁区，留 owner 后续 Full RFC（§4.5(a)）。
- **🔒 barrier / 资源状态并发语义本体**：barrier 的并发/可见性/内存序语义本体不在本 RFC（§4.5(b)）；本 RFC 仅定义编排锚点（哪里需状态转换），语义本体「需人工升档」。
- **🔒 D3D12 运行时 stable ABI / host↔运行时·host↔DXIL FFI ABI 二进制布局**：不冻结为 stable，承 RFC-0004 §4.6(a)/RFC-0005 RXS-0165 同级边界（§4.5(c)(d)）。
- **🔒 DXIL/SPIR-V UB 边界**：不建立独立 UB 契约（§4.5(e)，承 RFC-0004 §4.6(c)）。
- **语言面扩展 / codegen 面**：着色阶段类型面属 G2.1（RFC-0002）、DXIL codegen 属 G2.2（RFC-0003/0004）、绑定推导属 G2.3（RFC-0005）；本 RFC 是运行时集成面，不新增语言构造、不改 codegen、不改绑定推导。
- **edition / stabilization（G2.5）/ 语言 1.0**：不在本 RFC。
- **registry 触发**：包 registry（D-312，SG-007）维持 not_triggered，本 RFC 不触发。
- **多后端 / Python 嵌入 / 高级 GPU intrinsics / VMM·多 GPU**：死亡路线红线 1/3 + 永久 gating（SG-001/002/003/004/005/008）+ A-06 单机单 GPU 边界，均不触碰。
- **CI step 48 落地 / G-G2-4 签字 / RD·SG 状态翻转**：归 owner / 实现 PR，本 RFC 草案阶段不动。

## 9. §9 owner 裁决清单（Accepted / Owner Approved 2026-06-28）

> 以下为本 RFC 的**路径性抉择**。owner（Language Lead）于 2026-06-28 在本工作会话明确裁决下表全部项并批准 RFC-0006 全文；AI 代录，非 AI 代签 / 不代决。候选与 AI 倾向保留为审计上下文，裁决列为后续 PR-F1/PR-F2 的约束。触 🔒 禁区的语义本体（Q-Barrier 并发语义 / Q-Texture 采样内存模型）**仍须 owner 后续 Full RFC 落笔**，本批准不落任何禁区语义本体（硬规则 5）。

| Q | 待裁项 | AI 倾向（供参，不代决） | 裁决（owner 2026-06-28） |
|---|---|---|---|
| Q-Present | 呈现策略：窗口 swapchain present / offscreen-first 像素回读 / 两阶段（先 offscreen 后窗口） | offscreen-first（CI device 可真跑 + 像素对照，对齐 G-G2-2/G-G2-3 readback 先例；窗口呈现作可选第二阶段） | **裁决 = offscreen-first**。理由：CI/self-hosted GPU 可稳定真跑 + 像素对照；窗口 swapchain present 作为后续可选阶段，不阻塞 G-G2-4（窗口 present defer，登 RD-019） |
| Q-DemoCrate | UC-04 demo crate 位置与边界（`examples/` / 独立 crate / `src/` 下子 crate；与既有引擎集成 crate 的关系） | 独立 demo crate（默认 `unsafe_code=deny`，运行时边界最小开 unsafe + U23 注册），不污染语言核心 | **裁决 = 独立 demo crate**。位置 `src/uc04-demo`（owner 推荐，over `examples/uc04-deferred`，避免 examples 承担运行时 crate 职责）；默认 `unsafe_code=deny`，D3D12 边界若必须 unsafe 集中到最小 runtime module 并登记 unsafe-audit（U23 续号） |
| Q-RuntimeShape | D3D12 运行时封装层形态：薄 FFI 绑定 / safe wrapper crate / demo 内联（与 RFC-0001 interop 基座的复用关系） | safe wrapper（最小 D3D12 device/queue/PSO/command list 封装，复用 RFC-0001 device 基座），运行时 ABI 非 stable（§4.5(c)） | **裁决 = safe wrapper crate/module**。做最小 D3D12 device/queue/PSO/command list/resource/barrier 封装；运行时 ABI 明确 non-stable，不进入语言 stable 面（§4.5(c)） |
| Q-DeferredPass | deferred 最小 pass 形态：G-buffer 通道数/格式 + lighting 模型 + 是否含 present | 最小集 = G-buffer（albedo+normal+depth）→ 单光源 lighting → offscreen readback；present 作可选 | **裁决 = 最小 deferred**：G-buffer(albedo + normal + depth) → 单光源 lighting → offscreen readback。窗口 present 不作为 G-G2-4 必要条件 |
| Q-Barrier | 资源状态/barrier 最小模型：手动 barrier API / 编译器状态跟踪推导；**并发语义本体边界** | 首期手动 barrier 编排锚点（运行时显式插入），状态跟踪推导后期；🔒 **barrier 并发语义本体须 owner Full RFC 落笔**，本期仅编排锚点 | **裁决 = 首期手动 barrier 编排**。实现层显式插入 RT → SRV → RT/Copy/Readback 状态转换；不做编译器自动状态跟踪（自动状态推导 defer，登 RD-020）。🔒 barrier 并发/可见性语义本体不在本期定义，触及即升档（owner Full RFC） |
| Q-Texture | texture memory model 是否触及 06 §4.2：G-buffer 写入/采样若需纹理访问语义映射，**须 Full RFC owner 落笔** | 首期维持 opaque 句柄 + RT/SRV 视图绑定形态（不落采样内存模型本体）；凡触及采样 opcode/内存序即停手「需人工升档」 → owner Full RFC | **裁决 = 不落纹理内存模型本体**。首期只消费 opaque `Texture2D`/`Sampler` 句柄 + D3D12 RT/SRV 视图绑定；🔒 不定义采样 opcode、LOD/导数、越界、缓存一致性等 06 §4.2 语义。触及这些语义则停手，另起 owner Full RFC（defer 登 RD-021） |
| Q-Range | 下游 RXS 区间/条数/拆分（拟自 RXS-0167 起，§5 拟 4 条） | RXS-0167~0170（PSO 装配 / 多 pass 编排 / 资源状态 barrier 编排 / 呈现回读），随 Q-Present/Q-DeferredPass 调整 | **裁决 = RXS-0167~0170**：RXS-0167（DXIL + RTS0 → graphics PSO 装配一致性）/ RXS-0168（deferred 多 pass 编排）/ RXS-0169（资源状态/barrier 编排锚点）/ RXS-0170（offscreen readback + 像素对照）。窗口 present 不进必要条款，可作 RD 后续 |
| Q-Err | 错误码段位：复用 6xxx codegen/目标段 / 新开运行时诊断段位（当前 6xxx 最高 RX6017） | 运行时装配失败若属 codegen/目标可达类别复用 6xxx 续号；纯运行期 D3D12 失败若需新段位由 owner 按 07 §5 分配；不预留、不预造 | **裁决 = 编译期/装配期可预测错误续用 6xxx**，从 **RX6018** 起按真实可达类别追加；D3D12 API 返回的纯运行期/环境失败不滥发语言 RX，作为 smoke/evidence runtime failure 报告。若后续需稳定运行时诊断段，再由 owner 单独裁新段位 |
| Q-Gate | feature gate：复用 `dxil-backend` / 新增 `d3d12-runtime` 或 demo crate feature | demo/运行时专属 gate（隔离运行时面，不把 D3D12 运行时暴露成 dxil-backend 用户面组合维度） | **裁决 = 新增运行时/demo 专属 gate**（推荐 `d3d12-runtime` 或 `uc04-demo`，终名随 PR-F1）；不把 D3D12 runtime 面塞进 `dxil-backend`，`dxil-backend` 只作为 codegen 前置依赖 |
| Q-RD | 是否需新 RD：deferred 范围内做不完的项（如窗口呈现 / 高级 lighting / 状态跟踪推导 / 纹理内存模型）登记 RD（下一个未用 = RD-019，RD-016 已跳号永不复用） | 若 Q-Present 选 offscreen-first，则窗口呈现登记 RD（RD-019+）；纹理内存模型若 defer 亦登记 RD；均 owner 裁 | **裁决 = 需要，按实际 defer append-only 登记**：RD-019（窗口 swapchain present deferred）/ RD-020（自动资源状态跟踪推导 deferred）/ RD-021（纹理内存模型映射 deferred，需 owner Full RFC）。不在 RFC 草案阶段预造 registry；在本批准记录或 PR-F1 按实际 scope 登记 |
| Q-CIStep | CI step 48（UC-04 deferred 冒烟）SKIP/REQUIRE_REAL 策略 | 对齐步骤 46/47：默认 `RURIX_DXC_DIR` pin，`RURIX_REQUIRE_REAL=1` 缺 D3D12/MSVC/validator 即红，无显示环境 present 路径降级 SKIP（offscreen readback 路径维持 REQUIRE_REAL） | **裁决 = step 48 offscreen readback 为 REQUIRE_REAL**。对齐步骤 46/47：`RURIX_REQUIRE_REAL=1` 下缺 D3D12/MSVC/signed DXC pin/validator/GPU 即红；窗口 present 路径若存在可 SKIP，但不得替代 offscreen 真跑 |

**registry 处置（owner 2026-06-28 裁决，append-only 按实际 scope）**：

- **RD-019 / RD-020 / RD-021**：分别登记窗口 swapchain present defer（Q-Present）/ 自动资源状态跟踪推导 defer（Q-Barrier）/ 纹理内存模型映射 defer（Q-Texture，须 owner Full RFC）。**本批准阶段不预造 registry 条目**——按 owner 裁决在 **PR-F1** 按实际 scope append-only 落 `registry/deferred.json`（下一个未用 RD = RD-019，RD-016 已跳号永不复用，10 §9.5）。
- **错误码（Q-Err）**：6xxx codegen/装配段自 **RX6018** 起按真实可达类别追加，落码随 PR-F2，本批准不预占、不落码、不改 `registry/error_codes.json`（当前最高现存 RX6017）；纯运行期/环境 D3D12 失败作 smoke/evidence runtime failure，不发语言 RX。
- **unsafe-audit（U23）**：demo crate D3D12 边界若必须 unsafe，集中到最小 runtime module，随实现 PR 每 `unsafe` 块 `// SAFETY:` + U23 续号注册（硬规则 9）。
- **SG / stable 面**：包 registry（D-312，SG-007）维持 not_triggered；运行时 ABI 非 stable（§4.5(c)），stable 面随 RD-008（G2.5 候选触发点），本 RFC 不冻结。
- **spec 落点（Q-File，本轮 §9 未单列）**：新建 `spec/d3d12_runtime.md` vs 延伸既有文件随 **PR-F1** 按实际 scope 定（比照 Q-RD「批准记录 / PR-F1 登记」处置，owner 后续可定调）。

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：UC-04 运行时面经 feature gate → tracking → 两里程碑无重大修订 → stabilization report → FCP-lite（10 §2.2，≥2/3 同意含语言负责人 + 5–7 天公开等待窗）。D3D12 运行时接口 / 绑定布局物理布局在首个 stable 前不进 stable 面（随 RD-008，G2.5 候选触发点）。
- **Provenance**：`Assisted-by: kiro:claude-opus-4-8`（Draft + owner 裁决落文档）。owner（Language Lead）于 2026-06-28 裁决 §9 全部路径项并批准 RFC-0006 全文；批准记录由 AI 代录，非 AI 代签 / 自行裁决（硬规则 1）。

## 11. 规范与实现依据

- **决策/规范**：13 §D-002（图形分期，已批准）· 06 §8.2 第 4/5 点（PSO / 资源状态 / barrier 运行时面 = G2 设计预留）· 06 §4.2（纹理路径内存模型禁区，🔒）· 04 P-01（strict-only）/ P-11（单一事实源）/ P-13（防 AI 幻觉治理）· 11 §5（G2 = UC-04 deferred 渲染器 demo）。
- **前置 RFC（依赖闭合）**：[RFC-0002](0002-shader-stages.md)（着色阶段类型面，Owner Approved）· [RFC-0003](0003-dxil-backend.md)（MIR→DXIL 第二后端，Owner Approved）· [RFC-0004](0004-spirv-dxil-graphics-backend.md)（图形=B codegen + §4.6 禁区边界，Owner Approved）· [RFC-0005](0005-binding-layout-inference.md)（绑定布局推导 + RTS0，Owner Approved 2026-06-28）· [RFC-0001](0001-cuda-d3d12-interop.md)（CUDA–D3D12 interop，D3D12 device/queue/swapchain 运行时先例）。
- **G2.3 已闭环 device 见证（UC-04 前置事实）**：G-G2-3 RTS0 `CreateRootSignature` accept + textured PSO + offscreen draw 像素对照（[G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md) §8.3 / [CI_GATES.md](../milestones/g2/CI_GATES.md) §7 v1.4 step 47）。
- **registry**：RD-008（stable API 快照冻结，open，G2.5 候选触发点）· RD-018（bindless defer，RFC-0005）· **§9 Q-RD 已裁登记 RD-019（窗口 swapchain present defer）/ RD-020（自动资源状态跟踪推导 defer）/ RD-021（纹理内存模型映射 defer，须 owner Full RFC）**——按 owner 裁决在 PR-F1 按实际 scope append-only 落 `registry/deferred.json`，本批准阶段不预造（下一个未用 RD = RD-019，RD-016 已跳号永不复用，10 §9.5）· 6xxx codegen 段最高现存 RX6017，§9 Q-Err 已裁自 **RX6018** 续号（落码随 PR-F2，不预占）· 下一个未用 RFC = RFC-0007（README §5 编号台账）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-06-28 | AI 起草骨架（§1 摘要 deferred 通路图 + 边界 / §2 动机 + 为何 Full RFC（首次承接 RFC-0002/0003/0004/0005 共同 defer 的运行时面）/ §3 用户视角声明式 deferred 示意 + 开放问题留 §9 / §4.1 deferred 管线最小形态 / §4.2 D3D12 运行时面锚点 / §4.3 单一事实源消费 / §4.4 strict-only 装配核验 / §4.5 🔒 禁区边界声明（纹理内存模型 / barrier 并发语义 / 运行时 stable ABI / FFI ABI / DXIL·SPIR-V UB）/ §5 下游条款计划表（RXS-0167~0170 占位，不落条款体）/ §6 feature gate + 栈式 PR + 真实红绿 + device 见证/run-url 要件 / §7 备选 / §8 范围红线 / §9 未决留 owner（Q-Present/Q-DemoCrate/Q-RuntimeShape/Q-DeferredPass/Q-Barrier/Q-Texture/Q-Range/Q-Err/Q-Gate/Q-RD/Q-CIStep）/ §10 稳定化 / §11 依据）。**Draft / Awaiting Owner——待 owner FCP-lite 批准 + 裁决 §9；🔒 禁区语义本体由 owner 落笔；不落 codegen/条款体/运行时实现/CI/registry；AI 不代签 / 不代决 / 不推进下游** | Full RFC（Draft） |
| Owner approval | 2026-06-28 | owner（Language Lead）在本工作会话同意 RFC-0006 全文并裁决 §9 全部路径项：Q-Present=offscreen-first（窗口 present defer→RD-019）/ Q-DemoCrate=独立 demo crate `src/uc04-demo`（`unsafe_code=deny`，D3D12 边界 unsafe 集中最小 runtime module + U23）/ Q-RuntimeShape=safe wrapper（最小 device/queue/PSO/command list/resource/barrier 封装，运行时 ABI non-stable）/ Q-DeferredPass=G-buffer(albedo+normal+depth)→单光源 lighting→offscreen readback / Q-Barrier=首期手动 barrier 编排（RT→SRV→RT/Copy/Readback；自动状态跟踪 defer→RD-020；🔒 并发语义本体触即升档）/ Q-Texture=不落纹理内存模型本体（opaque Texture2D/Sampler + RT/SRV 视图绑定；触采样语义停手→RD-021/owner Full RFC）/ Q-Range=RXS-0167~0170 / Q-Err=6xxx 续号自 RX6018（纯运行期 D3D12 失败不发语言 RX，作 runtime failure 报告）/ Q-Gate=新增 `d3d12-runtime`/`uc04-demo` 专属 gate（`dxil-backend` 仅 codegen 前置）/ Q-RD=RD-019/020/021 按实际 scope append-only（PR-F1 落 registry）/ Q-CIStep=step 48 offscreen readback REQUIRE_REAL（对齐步 46/47，窗口 present 路径可 SKIP 不替代）。AI 代录，非 AI 代签。状态翻 Accepted / Owner Approved；下游 PR-F1 spec 脚手架解锁（仍不落条款体、不接线实现、不动禁区语义本体；G-G2-4 签字 / device run URL / CI step 48 落地归 owner 兑现） | Full RFC（Owner Approved） |
| Anti-downgrade note | 2026-06-28 | 追加 G-G2-4 防降级验收线：green 必须证明 Rurix source 经 `rurixc` 图形=B DXIL 路径与 RFC-0005 RTS0/绑定布局进入真实 D3D12 PSO，并在 hardware 上完成多 pass deferred draw + offscreen readback；手写 HLSL/DXIL、CPU 预填纹理、单 pass textured draw、fullscreen copy、固定像素注入、host-only 模拟、窗口 present 截图或 SKIP 均不得替代 G-G2-4 green。若 RD-013 或其他前置缺口阻断 Rurix 产物，必须标 blocked，不得降级签署。 | Full RFC（Owner Approved / guardrail clarification） |
