# RFC-0003 — MIR→DXIL codegen 第二后端（着色阶段函数降级到 DXIL，原生 D3D12 路径）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0003（4 位制，编号永不复用，10 §9.5） |
| 标题 | MIR→DXIL codegen 第二后端（着色阶段函数降级到 DXIL，原生 D3D12 路径） |
| 档位 | **Full RFC**（10 §3：codegen 第二后端 = 新目标/运行时语义面 + 潜在 FFI/ABI 触面；触及 AGENTS 硬规则 5 禁区边界——DXIL 文本语义 UB 边界 / 纹理路径内存模型映射 06 §4.2 / FFI ABI 面，均标 🔒 不落笔，留 agent） |
| 状态 | **Approved（2026-06-23，批准生效以 agent 合并 PR #83 为准）**。agent 于本工作会话审阅 §9 取舍分析后指示「我决策一半，帮我做完」，**委托 AI 取最合理裁决并批准全文**；agent 自主记录非代签（硬规则 1），状态翻转的批准生效动作 = agent 合并 PR #83（「最后那下按钮仍是 agent 的」）。§9 裁决见下；核心 Q-D131 = **C（暂不锁路径，留限时双路 spike 取证后由 agent 凭据再裁 A/B）**——禁区 A/B 架构承诺权仍留 agent，未被代行 |
| 承接里程碑 | G2.2（验收门 **G-G2-2**），G2 第二子里程碑（首子里程碑 G2.1 类型面已落地：RFC-0002 / RXS-0153~0156） |
| 关联条款 | 拟落 spec **RXS-0157~**（区间随条款数定，见 §5；当前最高现存 RXS-0156 @ shader_stages.md）；拟新建 `spec/dxil_backend.md`。**本 RFC 不创建裸条款头**，trace 维持 156/156 |
| 依据决策 | D-131（G2 DXIL 生成路径，**待决**——本 RFC §9 Q-D131 为其重评估载体）· D-002（图形分期，已批准）· 06 §8.2（codegen 第二后端设计预留）· 06 §4.2（纹理路径内存模型禁区，🔒）· 07 §7（device codegen 分发：MVP/G1 维持 NVPTX→PTX，DXIL 第二后端 G2 重评估）· 07 §5（错误码段位：6xxx codegen/目标）· D-207（PTX baseline，开发期形态） |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`。agent 自主（硬规则 1/2）：本草案由 AI 起草，🔒 禁区子节仅占位不落笔，§9 未决留 agent 裁决；**agent 经 FCP-lite 批准前不推进下游 spec/实现 PR，AI 不自启、不代签** |
| Agent 批准 | **Approved — 2026-06-23（委托裁决，批准生效以 agent 合并 PR #83 为准）**。批准范围：RFC 全文 + §9 全部裁决（Q-D131 = **C 暂不锁、留限时双路 spike（A 结构首选 / B 对照）取证后由 agent 凭据再裁 A/B**；Q-Golden = 文本反汇编 + dxc validator 验证；Q-Builtin = 仅类型面映射，二进制 ABI 留禁区；Q-Range = 随 C 暂不锁；Q-Gate = `dxil-backend`；Q-CLI = `rx build --target dxil`；§4.6 🔒 维持占位）。记录方式：agent 在工作会话审阅 AI 取舍分析后指示「选最合理的」委托裁决，由 AI 写回仓库（代录，非代签）；批准生效以 agent 合并 PR #83 为准（硬规则 1，AI 不代按 / 不自合 state-flip PR） |

> **批准记录**：agent 于 2026-06-23 在本工作会话审阅本 RFC §9 各项取舍分析后，明确指示「我决策一半，帮我做完」，**委托 AI 取最合理裁决并批准 RFC 全文**；agent 自主记录该委托决定（代录非代签，硬规则 1），状态翻 Approved，**批准生效以 agent 合并 PR #83 为准**（AI 不代按 / 不自合 state-flip PR）。§9 Q-D131（生成路径 = LLVM DirectX 后端 vs SPIR-V→DXIL 转译）裁为 **C：本 RFC 暂不锁定路径，留限时双路 spike（A 为结构首选，B 为对照）取证后由 agent 凭当时成熟度证据再裁 A/B**（13 §D-131「按当时后端成熟度评估」）——此选项**不构成禁区 A/B 架构承诺，A/B 路径裁决权仍留 agent，硬规则 1 未被代行**；spike 结果回填本 RFC §9 与 13 §D-131（经勘误 PR #84）。§4.6 标 🔒 的 DXIL 文本语义 UB 边界 / 纹理路径内存模型映射 / FFI ABI 面维持边界声明 + 占位「〈待 agent 后续 Full RFC〉」，本 RFC 不落笔（06 §4.2，AGENTS 硬规则 5）。

---


## 1. 摘要

本 RFC 在 G2.1 着色阶段类型面已落地（RFC-0002，RXS-0153~0156）的基础上，定义把着色阶段/kernel 函数从 **MIR 降级到 DXIL**（DirectX Intermediate Language）的 **codegen 第二后端**设计面（06 §8.2 第 2 点）。这是 Rurix 在既有 **NVPTX→PTX**（MVP/G1，D-207）之外的**第二条 device codegen 路径**，服务原生 D3D12 图形管线（D-002 图形分期 G2）。

```
现状（MVP~G1）：MIR ──NVPTX 后端──▶ PTX/cubin/fatbin ──▶ CUDA Driver API（compute，D-207）
                                  │
本 RFC（G2.2，第二后端 codegen 面）：
   MIR ──DXIL 后端──▶ DXIL（DirectX IL）──▶ D3D12 PSO / 着色器对象
        │
        ├─ codegen 分发：target 选择（PTX vs DXIL）在 MIR 之后分叉，前端/MIR 共享（§4.1）
        ├─ MIR→DXIL 降级：着色阶段函数（RXS-0153 着色）+ 阶段 I/O（RXS-0154）+ 阶段间接口（RXS-0155）+ 资源句柄类型面（RXS-0156）→ DXIL 形态（§4.2/§4.3）
        ├─ DXIL golden + bless：DXIL 产物纳入确定性 golden 核对（形态见 §9 Q-Golden）
        └─ 错误码：codegen/目标失败归 6xxx 段（07 §5，随实现 PR 按真实可达类别只追加，§5）
                                  │
   🔒 不在本 RFC（禁区，占位留 agent，§4.6）：
        DXIL 文本语义的 UB 边界 / 纹理路径内存模型映射（tex 采样 opcode·描述符编码·缓存一致性，06 §4.2）/ 任何 FFI ABI 面
                                  │
   ⏸ 不在本 RFC（范围红线，§8）：D-131 路径最终裁决（留 §9 agent）/ codegen 实现 / 绑定布局推导（G2.3）/ UC-04（G2.4）
```

本 RFC 只定义 **MIR→DXIL 降级的设计面与下游 spec 条款映射计划**；**生成路径的最终选择（D-131：LLVM DirectX 后端 vs SPIR-V→DXIL 转译）、codegen 的具体实现、禁区内存模型/UB/FFI ABI 语义均不在本 RFC**——前者留 §9 agent 裁决，后者留后续 Full RFC（agent 落笔）。DXIL 第二后端是 G2.3（绑定布局推导）、G2.4（UC-04 deferred 渲染器）的 codegen 基座；spec-first 先立条款映射计划（硬规则 7）。

## 2. 动机

- **D-002 图形分期（已批准）与 06 §8.2 设计预留**要求 G2「原生 D3D12 + DXIL」：着色阶段进语言（G2.1 已落）后，须有把着色阶段函数降级到 DXIL 的 codegen 后端，否则着色阶段类型面无可执行目标。本 RFC 把 06 §8.2 第 2 点（MIR→DXIL codegen 第二后端）的**设计面与条款映射计划**落为可条款化的精确结构。
- **第二后端是 D3D12 路径的必经基座**：G2.3（descriptor/root signature 推导，P-11）需要 codegen 产物消费签名信息；G2.4（UC-04 deferred 渲染器）需要端到端 DXIL 出图。无 DXIL 后端则 G2.3/G2.4 无 codegen 锚点，故 G2.2 先行（D-G2-2 在 G2.3/G2.4 之前）。
- **D-131 重评估时机已到**：13 §D-131 将「DXIL 生成路径（LLVM DirectX 后端 vs SPIR-V→DXIL 转译）」登记为「待决」，触发时机「G2 启动时按当时后端成熟度评估，agent 批准」。G2.2 正是该触发点。本 RFC §9 Q-D131 陈述两路结构性取舍作为 agent 裁决输入，**不替 agent 选**（硬规则 1）。
- **复用而非另起 codegen 框架**：DXIL 后端复用既有 MIR 与前端/类型系统（着色阶段着色 RXS-0153、阶段 I/O RXS-0154、阶段间接口 RXS-0155、资源句柄类型面 RXS-0156），target 分发在 MIR 之后分叉，不引入第二套前端或中间表示（对齐 07 §1 IR 四层、07 §7.1 MLIR kernel-island 后置 SG-001）。

**为何需要 Full RFC（而非 Direct/Mini）**：codegen 第二后端是**新目标/运行时语义面**（MIR→DXIL 降级语义、DXIL 产物形态、target 分发），且潜在触及 **FFI ABI 面**（host↔DXIL 着色器对象边界）与 **DXIL 文本语义的 UB 边界**、**纹理路径内存模型映射（06 §4.2）**——这些是 10 §3 / AGENTS 硬规则 5 明列的 Full RFC / 禁区触发面，由 agent 自主经 Full RFC 落笔。判档争议向上取严（硬规则 8），agent 自主判档/Mini、不代签批准/合并（硬规则 1）。本 RFC 对禁区子节（§4.6）标 🔒 明确不定义、留 agent。


## 3. 指导级解释（用户视角）

> 以下为**拟议**形态示意，最终形态以 agent 批准 + spec 条款（RXS-0157~）为准；本节用于沟通设计意图，不构成已批准接口。**生成路径（D-131）的最终选择不改变用户面**——用户视角对「DXIL 经 LLVM DirectX 后端还是 SPIR-V→DXIL 产出」无感（§9 Q-D131 是实现路径裁决）。

着色阶段函数（RFC-0002 定义的 `vertex fn` / `fragment fn` / `mesh fn` / `task fn` / RT 阶段，及 D3D12 语境复用的 `kernel`）经工具链降级到 DXIL，产出可被 D3D12 PSO 消费的着色器对象。用户经 `rx build` 选择 D3D12 目标（拟议形态，精确 CLI/manifest 形态见 §9）：

```sh
# 拟议:为 D3D12 目标构建(产 DXIL),与现有 PTX 路径并存
rx build app.rx --target dxil          # 第二后端:MIR→DXIL
rx build app.rx --target ptx           # 现状:MIR→PTX(D-207,维持默认)
```

着色阶段源码不因目标而改写——同一份着色阶段函数（类型面由 RXS-0153~0156 保证）经不同后端降级到 PTX 或 DXIL。`strict-only`（P-01）维持：DXIL 降级失败 = **结构化编译错误**（6xxx 段，§5），无静默降级、无 permissive 回退。

```rust
// 同一份着色阶段函数(RFC-0002 类型面),既可经 PTX 后端(compute)亦可经 DXIL 后端(D3D12)
vertex fn vs_main(in: VertexIn) -> VertexOut { /* ... */ }
fragment fn fs_main(in: VertexOut) -> FragmentOut { /* ... */ }
// → DXIL 后端产出 vertex/fragment 着色器对象,供 D3D12 PSO 装配(装配面属运行时/G2.3+,不在本 RFC)
```

DXIL 产物形态纳入确定性 golden 核对（与现有 PTX golden 并列），任何 codegen 输出漂移经 bless 审批（形态见 §9 Q-Golden）。

## 4. 参考级设计

> 本节落笔 **codegen 分发架构与 MIR→DXIL 降级的设计面**；触及禁区的子节（§4.6 DXIL UB 边界 / 纹理内存模型映射 / FFI ABI）标 🔒，本草案不写内容、留 agent 落笔。**生成路径实现细节（D-131）不在本节**——本节描述的降级面对两路（LLVM DirectX 后端 / SPIR-V→DXIL 转译）均适用，路径选择见 §9 Q-D131。

### 4.1 codegen 分发架构（target 在 MIR 之后分叉）

现状（D-207）device codegen 为单后端 NVPTX→PTX。本 RFC 在 MIR 之后引入 **target 分发点**：

- **共享前沿**：AST→HIR→TBIR→MIR（07 §1 IR 四层）与类型/着色/借用检查对所有 target 共享，不因 DXIL 后端分叉；着色阶段类型面（RXS-0153~0156）是后端无关的语言面。
- **后端分叉**：MIR 之后按 target（`ptx` / `dxil`）选择 codegen 后端。DXIL 后端是与 NVPTX 后端**并列的第二实现**，不替换、不修改 PTX 路径（D-207 维持）。
- **strict-only 分发（P-01）**：target 选择显式（CLI/manifest，§9 Q-CLI），无隐式多目标、无静默 fallback；某 target 不支持的语言构造 → 结构化 codegen 错误（6xxx，§5），非降级。
- **能力探测**：DXIL 后端的目标 shader model / DXIL 版本由真实工具链/设备探测驱动（对齐 P-01 strict-only、A-03 探测优先），不写死；具体探测面随实现 PR + §9 Q-CLI。

> **D-131 在本子节的落点**：MIR→DXIL 的**具体生成机制**（经 LLVM DirectX target 直接 emit DXIL，还是经 SPIR-V 中间表示再 SPIR-V→DXIL 转译）是 §4.1 分发点之后的**后端内部实现选择**，由 §9 Q-D131 agent 裁决。无论哪一路，对外暴露的「MIR→DXIL 后端」边界与 §4.2 降级面契约一致。

### 4.2 MIR→DXIL 降级面（设计面，非实现）

着色阶段函数从 MIR 降级到 DXIL 的设计面（具体算法/IR 操作随实现 PR + 路径裁决）：

- **函数着色 → DXIL 着色器类型**：RXS-0153 的着色阶段着色（`vertex`/`fragment`/`mesh`/`task`/RT/compute-via-kernel）降级为对应 DXIL 着色器类型（vertex/pixel/mesh/amplification/RT/compute shader）。着色阶段集合与 DXIL 着色器类型的**精确对应表**随实现 PR 落 spec 条款体（RXS-0157~，§5）。
- **阶段 I/O → DXIL 签名/语义槽**：RXS-0154 的阶段专属 I/O（`#[interpolate]` 插值限定 / `#[builtin]` 内建变量）降级为 DXIL 输入/输出签名与系统值语义（SV_*）。内建变量→DXIL 寄存器/语义槽的**精确映射边界**列入 §9 Q-Builtin（涉及 DXIL ABI 面，谨慎处置）。
- **阶段间接口 → DXIL 阶段链接**：RXS-0155 的阶段间接口契约（vertex out → fragment in varying 兼容）降级为 DXIL 阶段间签名匹配；类型面已在编译期校验（RXS-0155），DXIL 层为降级一致性核对。
- **资源句柄类型面 → DXIL 资源绑定形态**：RXS-0156 的资源句柄/纹理采样器类型面（`Texture2D<F>` / `Sampler`）降级为 DXIL 资源绑定声明的**形态占位**——**绑定布局的具体推导（descriptor/root signature）属 G2.3（P-11），不在本 RFC**；本 RFC 仅定义句柄在 DXIL 降级中的类型形态锚点供 G2.3 消费。
- **确定性**：DXIL codegen 输出对给定输入确定（对齐既有 PTX golden 确定性要求），纳入 golden 核对（§4.4）。


### 4.3 着色阶段 → DXIL 映射的类型面锚点

本子节定义降级所依赖的**类型面锚点**（均已由 RFC-0002 类型面提供，本 RFC 不新增语言面）：

- DXIL 后端消费的输入是 RXS-0153~0156 保证的**类型化 MIR**：着色阶段着色已判定、阶段 I/O 标注已解析、阶段间接口已校验、资源句柄类型已确立。
- DXIL 后端**不引入新语言构造**：不新增着色阶段关键字、不新增 I/O 标注、不新增资源类型——这些属 G2.1 语言面（RFC-0002，已 Approved）。本 RFC 是 codegen 面，消费既有类型面。
- 若降级过程暴露语言面缺口（如某着色阶段在 DXIL 路径需额外类型信息），按硬规则 7 **先补 spec/RFC-0002 增补**再实现，不在 codegen 层私自扩展语言面（防 codegen 倒逼语言面漂移）。

### 4.4 DXIL golden + bless 机制（设计面）

- DXIL codegen 产物纳入**确定性 golden 核对**，与既有 PTX/MIR/UI golden 并列（README「UI/MIR/PTX golden 经 bless」体系扩展）。
- golden 的**粒度与形态**（DXIL 文本反汇编 vs 二进制容器 vs 规范化中间形态、是否经 dxc 验证、bless 审批流）列入 §9 Q-Golden——涉及 DXIL 容器格式与工具链依赖，留 agent 裁决形态。
- **真实红绿**（反 YAML-only，CI_GATES §6）：篡改 DXIL codegen 输出 → golden 红 → 复原绿，run URL 归档（G-G2-2 验收门要求）。
- **device 证据**：G-G2-2 要求「device 真跑数值/呈现对照」——DXIL 着色器在 D3D12 真实管线的呈现对照证据形态随实现 PR + 子里程碑 device counter（脚手架不预造 counter，G2_CONTRACT guardrail）。

### 4.5 与 PTX 后端的关系（并存，非替换）

- DXIL 后端是**第二后端**，PTX 后端（D-207）维持不变：MVP/G1 的 compute 路径继续 NVPTX→PTX，G2.2 不改 PTX codegen。
- 07 §7 明载「DXIL 第二后端无 MVP 期 PTX↔DXIL 对应信息，完全于 G2 重评估」——本 RFC **不假设 PTX 与 DXIL 之间存在转译或共享 lowering**，两后端各自从 MIR 独立降级（§4.1 分叉点）。
- 双后端共享的是 **MIR 与前沿**（§4.1），不是后端内部降级逻辑。

### 4.6 🔒 禁区边界声明（本 RFC 不定义，留 agent Full RFC）

> **本子节为边界声明，AI 不落笔禁区内容（AGENTS 硬规则 5 / 06 §4.2）。**

以下面属仅 agent 经 Full RFC 落笔的禁区，本 RFC **明确不定义**，仅作边界声明 + 占位：

- **(a) DXIL 文本语义的 UB 边界**：DXIL 指令的未定义行为边界、毒值/poison 语义、越界/竞争在 DXIL 层的语义后果——属 UB 条款禁区（AGENTS 硬规则 5）。〈待 agent 后续 Full RFC〉
- **(b) 纹理路径内存模型映射**：纹理/采样器在 DXIL 的采样 opcode 映射、采样器描述符编码、纹理缓存一致性语义、采样 UB——属 06 §4.2 内存模型禁区（与 RFC-0002 §4.5 同一禁区，G2 纹理路径引入时由 agent 经独立 Full RFC 扩展）。〈待 agent 后续 Full RFC〉
- **(c) host↔DXIL FFI ABI 面**：host 侧与 DXIL 着色器对象之间的 ABI 边界（常量缓冲/根参数二进制布局、调用约定）——属 FFI ABI 禁区（AGENTS 硬规则 5）。注：内建变量→DXIL 系统值语义（§4.2）触及 DXIL 签名 ABI 的部分列入 §9 Q-Builtin 谨慎处置，禁区语义不在本 RFC。〈待 agent 后续 Full RFC〉

本边界与 §8 范围红线一致：本 RFC 的 DXIL 降级面是**结构/类型形态层**，不承诺任何 UB 语义、内存序、一致性或 ABI 二进制布局保证。


## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

拟新建 `spec/dxil_backend.md`，自 **RXS-0157** 起续号（当前最高现存 RXS-0156 @ shader_stages.md）。**本 RFC 不创建 `### RXS-####` 裸条款头**——下表为条款与测试锚定的**计划表**，条款体随 agent 批准本 RFC + 裁决 Q-D131 后的 spec 脚手架/实现 PR 同落（条款 PR 先于实现 PR，硬规则 7；trace 维持 156/156）。**区间大小未锁定**，列入 §9 Q-Range 待 agent 与路径裁决一并确定（路径选择可能影响条款拆分）。

| 条款（拟，区间待 §9 Q-Range 定） | 标题 | 测试锚定计划（每条 ≥1，`//@ spec: RXS-####`） |
|---|---|---|
| RXS-0157（拟） | codegen target 分发与 DXIL 后端分叉（MIR 之后 target 选择，PTX/DXIL 并存，strict-only 无 fallback） | conformance accept（合法 target 选择产 DXIL）+ reject（不支持构造 → 6xxx codegen 错误）+ golden |
| RXS-0158（拟） | 着色阶段着色 → DXIL 着色器类型降级对应（vertex/fragment/mesh/task/RT/compute → DXIL shader type） | DXIL golden（各阶段降级形态）+ conformance accept |
| RXS-0159（拟） | 阶段 I/O → DXIL 签名/系统值语义降级（`#[interpolate]`/`#[builtin]` → DXIL 输入输出签名，映射边界见 §9 Q-Builtin） | DXIL golden（签名形态）+ reject（非法 I/O 映射 → codegen 错误） |
| RXS-0160（拟） | DXIL codegen 确定性与 golden/bless 核对（产物确定性 + 篡改 → 红 → 复原绿，形态见 §9 Q-Golden） | golden 核对 + 真实红绿（篡改 codegen 输出）run URL 归档 |

> 上表条款号为**拟议占位**，实际区间/拆分随 §9 Q-Range agent 裁决（路径选择 LLVM DirectX vs SPIR-V→DXIL 可能影响降级面拆条粒度）。资源句柄→DXIL 绑定的**布局推导条款属 G2.3**（descriptor/root signature，P-11），不在本文件。

- **错误码策略**：DXIL codegen/目标失败（target 不支持的构造、降级失败、DXIL 产物校验失败等）= **codegen/目标段诊断**，归 **6xxx 段**（07 §5：「6xxx codegen/目标」；当前 6xxx 段**空、未分配**）。**不预留、不预造**：6xxx 具体码随实现 PR 按真实可达类别只追加分配 + en/zh message-key（`registry/error_codes.json` 只追加，`ci/bilingual_coverage.py` 覆盖）。注:着色/接口/句柄的**编译期类型面**诊断已在 G2.1 归 3xxx 段（RXS-0153~0156，末号 RX3012），属语言面，非本 codegen 后端段位；本 RFC 6xxx 仅限 codegen/目标失败类别。纯 Rust 通用错误走 rustc 原生诊断（零新 RX）。
- spec 条款 PR 先于实现 PR（硬规则 7）；trace_matrix 维持全锚定（沿用全局 `m1.counter.spec_clause_test_anchoring`，不另立 g2 counter，对齐 G-G1-6 / G-G2-1 范式）。

## 6. feature gate / tracking / 实现序（10 §3 要件）

- **feature gate**：cargo feature 拟名待 §9 Q-Gate 裁（候选 `dxil-backend`）；未启用时 DXIL 后端不参与编译，PTX 路径（D-207）不受影响。tracking 清单随实现 PR 维护（实现状态 / 未决问题 / 测试清单 / 路径选择落地状态）。
- **栈式 PR（门控于本 RFC 合入 + §9 Q-D131 路径裁决后）**：本分支 `feat/g2.2-dxil-rfc` **只产 RFC 草案**，后续另起栈式分支：
  - **PR-C1 spec 脚手架**：`spec/dxil_backend.md` 登记文件名 + RXS-0157~ 预留区间（**不落裸条款头**）+ `spec/README.md` §4 文件清单行 + RXS 末号同步 + 修订行（带 Full RFC 档位标记）；`trace_matrix --check` PASS（维持全锚定）。
  - **PR-C2 spec 条款体 + DXIL 后端实现**：条款体（RXS-0157~）+ MIR→DXIL 降级实现（按 agent 裁决的 D-131 路径）+ DXIL golden + bless + 6xxx 段错误码（registry 只追加）+ en/zh message-key。降级失败/target 不支持 → 6xxx codegen 错误 + 真实红绿。
  - **CI 步骤**（CI_GATES §2，DXIL codegen 冒烟）随实现 PR 回填 workflow。**device 真跑/呈现对照**（G-G2-2）需 D3D12 环境，证据形态随实现 PR + 子里程碑 device counter。
- **真实红绿**（反 YAML-only，CI_GATES §6）：篡改 DXIL codegen 输出 → golden 红 → 复原绿，归档前后输出 / run URL（G-G2-2 验收门）。
- **依赖与序**：本 RFC（G2.2 codegen 面）为 G2.3（绑定布局推导）、G2.4（UC-04）的 codegen 基座；G2.3/G2.4 门控于本后端就位。


## 7. 备选方案

> §7 陈述**设计层备选**；其中生成路径两选项（LLVM DirectX 后端 vs SPIR-V→DXIL 转译）= D-131 待决项，本节**陈述结构性取舍但不裁决**，裁决留 §9 Q-D131 agent（硬规则 1）。具体后端成熟度快照是 agent 裁决时的输入（13 §D-131：「按当时后端成熟度评估」），本 RFC 不冻结某一时点的成熟度判断。

**A. 生成路径 = LLVM DirectX 后端（直接 emit DXIL）** ——【D-131 选项一,留 agent】
- 结构取舍:Rurix 编译器自身基于 LLVM(D-205 LLVM pin),若 LLVM 的 DirectX target 可直接从 LLVM IR/MIR 后端 emit DXIL,则与既有 NVPTX 后端**同构**(都是 LLVM target 后端),复用 LLVM codegen 框架、共享 MIR→LLVM IR 降级前沿。
- 优势面:与现有 NVPTX 路径架构一致(§4.1 分叉点自然);无额外中间 IR 跳板;DXIL 作为 LLVM 的 target 与 Rurix 的 LLVM 绑定策略(D-205)一脉。
- 风险面:依赖 LLVM DirectX target 的**当时成熟度**(是否能产生 dxc/D3D12 接受的合规 DXIL、shader model 覆盖、validator 兼容)——此为 D-131 登记的「关键输入」,须 agent 在裁决时核实当时实况;若不成熟,落地成本高。

**B. 生成路径 = SPIR-V→DXIL 转译** ——【D-131 选项二,留 agent】
- 结构取舍:MIR 先降级到 SPIR-V(或经 LLVM→SPIR-V),再经 SPIR-V→DXIL 转译器产出 DXIL;引入 SPIR-V 作为中间表示与外部转译依赖。
- 优势面:SPIR-V 生态/转译工具相对独立成熟;若转译器合规性稳定,可绕开 LLVM DirectX target 的成熟度风险。
- 风险面:引入**第二中间表示 + 外部转译依赖**(供应链/版本/合规性长尾);转译层的语义保真与 strict-only(P-01)、确定性(§4.4 golden)需额外验证;与 Rurix 的 LLVM 单栈策略(D-205)、Windows-first 自洽(D-002)的契合度需 agent 评估;转译路径的内存模型/UB 保真涉及禁区(§4.6),更需谨慎。

> **本 RFC 不替 agent 选 A/B**(硬规则 1)。两路对 §4.2 降级面契约与用户视角(§3)透明;路径选择影响**后端内部实现、依赖面、条款拆分粒度(§9 Q-Range)**。agent 裁决记录回填 §9 Q-D131 与 13 §D-131。

**C. 其他备选(已倾向,非禁区):**
- **第三后端/通用多后端(AMD/Intel/Metal/Vulkan)**:否决——死亡路线红线 3(D-008 维持不解除,SG-003 not_triggered);DXIL 是 D3D12 原生路径,非通用多后端入口。
- **codegen 倒逼语言面扩展**:否决——降级暴露的语言面缺口先补 spec/RFC-0002(硬规则 7),不在 codegen 层私扩语言面(§4.3)。
- **替换 PTX 后端**:否决——DXIL 是第二后端,PTX(D-207)并存不变(§4.5)。

## 8. 不做（范围红线）

本 RFC 明确**不涉及**以下范围（防蔓延）：

- **D-131 生成路径的最终裁决**:本 RFC 陈述两路取舍(§7),裁决留 §9 Q-D131 agent;agent 自主裁决。
- **DXIL codegen 的具体实现**:MIR→DXIL 降级算法、后端代码、golden 产物均不在本 RFC(随 agent 批准 + 路径裁决后的实现 PR,§6);本 RFC 不动 `src/*`、不建 conformance/golden。
- **绑定布局推导**(G2.3,P-11):descriptor/root signature 编译器推导生成不在本 RFC;本 RFC 仅定义资源句柄在 DXIL 降级中的类型形态锚点供 G2.3 消费(§4.2)。
- **UC-04 deferred 渲染器**(G2.4):端到端原生 D3D12 + DXIL 出图 demo 不在本 RFC。
- **语言面扩展**:着色阶段类型面属 G2.1(RFC-0002,已 Approved);本 RFC 是 codegen 面,不新增语言构造(§4.3)。
- **🔒 DXIL 文本语义 UB 边界 / 纹理路径内存模型映射(06 §4.2)/ host↔DXIL FFI ABI 面**(AGENTS 硬规则 5 禁区):标 🔒 占位,留 agent 后续 Full RFC(§4.6)。
- **edition / stabilization 机制**(G2.5):不在本 RFC。
- **PSO / 资源状态 / barrier 运行时面**(06 §8.2 第 4/5 点):管线对象装配、资源状态机属运行时/库级职责,不在本 codegen RFC。
- **多后端**(AMD/Intel/Metal/Vulkan/SPIR-V 作为通用目标):死亡路线红线 3(D-008 维持,SG-003 not_triggered);注 SPIR-V 若作为 DXIL 转译的**内部中间表示**(§7 选项 B)≠ SPIR-V 作为对外通用目标,二者区分由 agent 在 Q-D131 裁决时厘清。


## 9. 裁决结果（agent 2026-06-23 委托裁决，agent 自主记录）

> agent 于 2026-06-23 在本工作会话审阅以下各项取舍分析后，明确指示「我决策一半，帮我做完」，**委托 AI 取最合理裁决并批准全文**；AI 仅将该委托决定写回文档（代录非代签，硬规则 1），并以 agent 合并 PR #83 为批准生效动作。**核心 Q-D131 裁为 C（暂不锁路径），不构成禁区 A/B 架构承诺——A/B 裁决权仍留 agent，硬规则 1 未被代行。** 技术细节随后续 spec 脚手架/实现 PR 落条款体（trace 维持 156/156）。

- **Q-D131 生成路径（核心，= 13 §D-131 重评估）→ 裁决 = C（暂不锁，留限时双路 spike 取证后再定）**：DXIL 生成路径在 **(A) LLVM DirectX 后端直接 emit DXIL** 与 **(B) SPIR-V→DXIL 转译** 之间，**本 RFC 不锁定**。委托裁决理由——A/B 之分**唯一卡点 = LLVM DirectX target 的当下成熟度**（DXIL 合规性 / shader model 覆盖 / validator 兼容），而 13 §D-131 规定须「按当时后端成熟度评估」、本 RFC 不冻结某时点成熟度判断（§7）；该实况证据未取得前，直接锁 A 即断言其已成熟、锁 B 即为规避未测风险付永久第二-IR 代价，均非最合理。故裁：
  - **结构首选仍为 A**（与 NVPTX 后端同构、D-205 LLVM 单栈、无第二中间 IR，§7 A）；A 的 LLVM DirectX target 已在本项目 vendored LLVM（D-205, 22.1.x）内，**spike 成本低、可直接测得 dxc/validator 合规实况**。
  - **限时双路 spike（spike-gated，不进 codegen）**：以 A 为主、B 为对照，产出 LLVM DirectX target 当下合规性 / shader model 覆盖 / validator 兼容的**实测证据**；spike 不落 codegen、不创建条款、不入 golden（仅取证）。
  - **回填**：spike 结束后由 agent 凭证据裁定最终 A/B，回填本 Q 与 **13 §D-131**（经勘误 PR #84）。**A/B 禁区架构承诺权保留 agent**（硬规则 1）。
  - **【回填 2026-06-24，agent 裁决 = A（LLVM DirectX 后端直接 emit DXIL）】**：G2.2 双路 spike round-1~8 取证完结,agent 凭证据裁定最终路径 = **A**。裁决依据三证齐:① **结构首选**——与 NVPTX 后端同构、D-205 LLVM 单栈、无第二中间 IR(§7 A);② **签名 validator 到手**——round-7 取与 LLVM 22/23 同年代(2026)的 DXC v1.9.2602.24(自带 dxil.dll 签名 validator + dxv.exe),决定性子轴(新 dxc 自产 52B PSV accept / llc 52B PSV reject)排除『dxc 太旧不识新 PSV』假说,Bug 2 归因 established = LLVM DirectX 后端 emit 的 PSV0 与自身 DXIL 模块内部不一致(上游兼容性 bug);③ **浅修 established**——round-8 源码级 root cause 定位到 `DXContainerGlobals.cpp:388-389`(`PSV.finalize/write` 不传 Version 取默认 max → 写 v3=52B,期望侧缺 `dx.valver` → validator 推 v0=24B → 0x80aa0013),14 行单函数 PoC patch(按 `MMI.ValidatorVersion` 派生 PSV 版本)+ 增量重建 llc 使 validator pre 0/25 → post 25/25 accept(dxv.exe 一致),A 路 validator 互操作 gap **可被已知小补丁闭合**(工具链层)。证据指针:`evidence/dxil_path_spike_report_round{6,7,8}.md` + `dxil_path_spike_20260624_r{7,8}.json`(RD-010)。**本路径裁定的下游解锁**:A 路依赖的 round-8 PSV patch 在上游 merge 前,以**受控、dev-only、临时**工具链偏差解锁 PR-C1/C2 开发(`registry/deferred.json` **RD-011** 跟踪 + `spike/dxil-path-probe/dxil_psv_patch_recipe.md` 可复现 recipe),同步上游 PR 并行;退役条件 = 上游 merge + release + D-205 pin bump(D-205 真 bump 属 agent 独立决策,不在本勘误)。**本回填为 agent 自主裁决,以 agent 合并本 PR 生效。** 边界:A 工具链 validator 可行性 ≠ Rurix MIR→DXIL 实现 ≠ device 真跑 golden;**G-G2-2 仍 open**,下游 spec(PR-C1)/实现(PR-C2)按硬规则 7 序进(条款先于实现)。
  - **【增补回填 2026-06-25，agent 裁决 = 混合:compute=A / 图形=B(SPIR-V→DXIL 转译)】**(A→混合,追加式不改写既有 A 裁决文本):A 单选后续证据链暴露 A 路**图形签名不可达**——slice3/round-8(`evidence/dxil_slice3_rxs0159_sig_disasm_round8.md`)实测 A 路 vertex/fragment 入口经 patched llc 产 DXContainer 虽 validator accept,但 ISG1/OSG1 签名 part **`elemcount=0`**(无 SV_Position/SV_Target/SV_VertexID);根因 = LLVM DirectX 后端 `addSignature()`(`DXContainerGlobals.cpp`)对图形着色器**无条件写空签名**(`// FIXME: support graphics shader`,上游 #90504),`Signature::addParam` 填元素强制要 Register/Mask 二进制布局 = §4.6(c)/Q-Builtin 🔒 FFI ABI 禁区。A-graphics 工作量评估(`dxil_a_graphics_sig_effort_report.md`)= 跨 clang 前端 + LLVM 后端 + PSV 的 estimated ~800-1500 LOC 上游大功能(三处 FIXME #90504/#57928),上游 open 无在途 PR、carry-patch partial-blocked。B 路取证(`dxil_b_graphics_sig_report.md`)= B(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL)图形签名 ISG1/OSG1 **`elemcount>0`**、SV 系统值端到端存活、IDxcValidator+dxv.exe ×25 全 accept、确定性 **measured 可行**。agent 据此**增补**:**compute 维持 A**(结构首选 + round-8 浅修 PSV,RD-011),**图形改 B**(转译链);A-graphics 挂上游 #90504/#57928,**成熟后迁移**(RD-015);B 路供应链(SPIRV-Cross/dxc/glslang pin + 确定性 + strict-only 核验)跟踪 RD-014。**图形=B 转译保真非完美**(用户语义名→通用 TEXCOORD / 寄存器·顺序重排 / 未用 SV 输入消除,`dxil_b_graphics_sig_report.md` §5)= 对**准永久公理 P-01(strict-only)的边界/例外**——其**规范性裁断由 agent 自主落笔**(P-13/硬规则 5),本回填**只摆实测事实 + 留占位**,不写例外裁断。图形=B 完整设计面(MIR→SPIR-V 降级 / 转译链 / validator gate / golden / 🔒 P-01 边界声明)载于**新 RFC(拟 RFC-0004,Full RFC)**,或论证为本 RFC 增补(二选一,Q-Hybrid-RFC 留 agent)。下游 RXS-0159 按 B 重构形态(或 hold)+ RXS-0160 + B 新增面(MIR→SPIR-V)条款区间随 RFC-0004 §5 计划。**本增补为 agent 自主裁决,以 agent 合并本勘误 PR 生效**;同步回填 13 §D-131(A→混合)+ registry(RD-010 close / RD-014 / RD-015);G-G2-2 仍 open。
- **Q-Golden DXIL golden + bless 形态 → 裁决 = 文本反汇编 + 经 dxc validator 验证后入 golden**：DXIL golden 取 **DXIL 文本反汇编形态**（与既有 PTX/MIR/UI golden 同为可 diff、经 bless 人审的文本形态，§4.4），不取二进制容器（字节精确但 diff 不可读、破坏 bless 人审链）；golden 文本文件本身仍受仓库 LF 字节精确约束。**入 golden 前须经 dxc validator 验证通过**（不合规 DXIL 不得成为 golden，对齐 P-01 strict-only）。可选附挂二进制 digest 作字节锚（不改主形态），随实现 PR 定。
- **Q-Builtin 内建变量 → DXIL 语义槽映射边界 → 裁决 = 仅定义类型面映射，二进制 ABI 布局留禁区 Full RFC**：本 RFC 体系下只定义 `#[builtin]` 内建变量 → DXIL 系统值语义名（SV_*）的**类型面映射**（RXS-0154 锚点，§4.2）；内建变量 / 根参数 / 常量缓冲的**二进制 ABI 布局**属 §4.6(c) FFI ABI 禁区（AGENTS 硬规则 5），**不在本 RFC**，留 agent 后续独立 Full RFC。AI 不在禁区落笔。
- **Q-Range RXS 区间大小与拆分 → 裁决 = 随 Q-D131=C 暂不锁定，spike 后与路径一并定**：§5 计划表 RXS-0157~0160 维持**拟议占位**；因 Q-D131 裁 C（路径未锁），降级面条款拆分粒度（如 B 路径转译层是否需独立条款）待 spike 取证、路径裁定后与区间一并锁定。本 RFC 不锁定、不预造、不创建裸条款头（trace 维持 156/156）。
- **Q-Gate feature gate 命名 → 裁决 = 接受候选 `dxil-backend`**：cargo feature 名取 `dxil-backend`（§6）；未启用时 DXIL 后端不参与编译，PTX 路径（D-207）不受影响。
- **Q-CLI target 选择面 → 裁决 = 接受候选 `rx build --target dxil`**：D3D12 目标经 `rx build --target dxil` 选择（与现状 `--target ptx` 并列，§3/§4.1）；精确 manifest 形态与能力探测面随实现 PR 细化（探测优先、不写死，A-03/P-01）。

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：`dxil-backend`（暂名，Q-Gate）feature gate 后 → tracking → 两里程碑无重大修订 → stabilization report → FCP-lite（10 §2.2，≥2/3 同意含语言负责人 + 5–7 天公开等待窗）。DXIL 后端产物面/target 选择面在首个 stable 发布前不进 stable 面;stable 面冻结随 **RD-008** 届时定义（G2.5 语言 1.0 为候选触发点）。
- **Provenance**：`Assisted-by: claude-code:claude-opus-4-8`。本草案由 AI 起草；§4.6 🔒 禁区维持占位不落笔。agent 于 2026-06-23 在本工作会话审阅 §9 取舍分析后指示「我决策一半，帮我做完」，**委托 AI 取最合理裁决并批准全文**；§9 裁决（核心 Q-D131 = C 暂不锁、A/B 禁区承诺仍留 agent）由 agent 自主记录，**非 AI 代签 / 自行裁决**（硬规则 1）——状态翻 Approved 的批准生效动作 = agent 合并 PR #83（AI 不代按、不自合 state-flip PR）。下游 spec/实现 PR 仍门控于本 RFC 合入 + Q-D131 spike 取证后路径裁定，按硬规则 7 序进（spec 先于实现）。FCP-lite 额外评审/等待窗若适用,按 10 §2.2/§5 独立完成,本记录不虚构尚不存在的评审。

## 11. 规范与实现依据

- 06 §8.2（G2 codegen 第二后端 MIR→DXIL，设计预留第 2 点）/ §4.2（纹理路径内存模型禁区，仅人类 Full RFC，🔒）。
- 07 §7（device codegen 分发：MVP/G1 维持 NVPTX→PTX/cubin/fatbin;DXIL 第二后端无 MVP 期 PTX↔DXIL 对应信息，完全于 G2 重评估，D-131）/ §7.1（MLIR kernel-island 后置 SG-001）/ §5（错误码段位:6xxx codegen/目标）/ §1（IR 四层 AST→HIR→TBIR→MIR）。
- 13 §D-131（G2 DXIL 生成路径，**待决**，本 RFC §9 Q-D131 为重评估载体）/ §D-002（图形分期，已批准）/ §D-205（LLVM pin 22.1.x，vendored）/ §D-207（PTX baseline，开发期形态）。
- 04 设计原则：P-01（strict-only，无静默 fallback）/ P-11（单一事实源——绑定推导 G2.3 依据）/ P-13（防 AI 幻觉治理三角）。
- RFC-0002（着色阶段类型面，RXS-0153~0156，已 Approved——本 RFC 的语言面输入）/ RFC-0001（CUDA–D3D12 interop，Full RFC 先例）。
- milestones/g2/G2_CONTRACT.md（D-G2-2 / 验收门 G-G2-2:Full RFC 前置 + D-131 裁决 + device 真跑 + golden）。
- registry/error_codes.json：6xxx codegen/目标段（当前空，随实现 PR 只追加）;3xxx 着色/地址空间段（G2.1 末号 RX3012，属语言面非本后端段）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-06-23 | AI 起草初版（MIR→DXIL codegen 第二后端设计面：§4.1 codegen target 分发 / §4.2 MIR→DXIL 降级面 / §4.3 类型面锚点 / §4.4 DXIL golden+bless / §4.5 与 PTX 后端并存 / §4.6 🔒 禁区边界占位（UB / 纹理内存模型映射 / FFI ABI）+ §5 下游 spec 条款映射 RXS-0157~ 计划表 + 错误码 6xxx 段策略 + §6 feature gate `dxil-backend`/实现序/真实红绿计划 + §7 备选（含 D-131 两路 A/B 结构性取舍陈述，不裁决）+ §8 范围红线 + §9 未决留 agent（核心 Q-D131 + Q-Golden/Q-Builtin/Q-Range/Q-Gate/Q-CLI）+ §10 稳定化 + §11 依据）。**待 agent 自主批准（FCP-lite）+ 裁决 Q-D131 路径，agent 自主签署 / 不代决 / 不推进下游;§4.6 禁区文本与 §9 未决由 agent 落笔/裁决** | Full RFC（Draft） |
| Agent approval | 2026-06-23 | agent 于本工作会话审阅 §9 各项取舍分析后指示「我决策一半，帮我做完」，**委托 AI 就 §9 余项取最合理裁决并批准 RFC 全文**：Q-D131 = **C（暂不锁路径，留限时双路 spike——A 结构首选 / B 对照——取证后由 agent 凭当时成熟度证据再裁 A/B；C 不构成禁区 A/B 架构承诺，A/B 裁决权仍留 agent，硬规则 1 未被代行）** / Q-Golden = 文本反汇编 + 经 dxc validator 验证后入 golden / Q-Builtin = 仅类型面映射（二进制 ABI 布局留 §4.6 禁区 Full RFC）/ Q-Range = 随 C 暂不锁（spike 后与路径一并定）/ Q-Gate = `dxil-backend` / Q-CLI = `rx build --target dxil` / §4.6 🔒 维持占位。agent 自主记录该委托决定（代录非代签，硬规则 1），状态翻 Approved；**批准生效以 agent 合并 PR #83 为准**（AI 不代按 / 不自合 state-flip PR）。D-131 路径 spike 取证后裁决结果经独立勘误 PR #84 回填 13 §D-131。下游 spec/实现 PR 门控于本 RFC 合入 + Q-D131 spike 取证后路径裁定（硬规则 7） | Full RFC（Approved） |
| D-131 路径回填 | 2026-06-24 | **§9 Q-D131 追加 agent 裁决 = A（LLVM DirectX 后端直接 emit DXIL）回填**(C→A,追加式不改写既有 C 裁决文本)：G2.2 双路 spike round-1~8 取证完结,agent 凭证据(① 结构首选 / ② round-7 签名 validator 到手排除『dxc 太旧』假说、Bug 2 归因 established / ③ round-8 源码级 root cause + 14 行 PoC patch 使 validator pre 0/25→post 25/25 accept,浅修)裁定最终路径 = A。下游解锁:A 路依赖的 round-8 PSV patch 注册为**受控 dev-only 临时**工具链偏差(RD-011 + recipe doc),上游未 merge 期解锁 PR-C1/C2 开发,同步上游 PR 并行;退役条件 = 上游 merge + release + D-205 pin bump。同步 13 §D-131(待决→A)。**本回填为 agent 自主裁决,以 agent 合并本勘误 PR 生效**;G-G2-2 仍 open(A 工具链 validator 可行性 ≠ Rurix MIR→DXIL 实现 ≠ device golden,agent 自主签署)。规划文档勘误(00 §6.3 追加式,独立 PR,check_planning_docs 对 13 预期红,待 agent 自主 合入) | Full RFC（Approved，路径回填） |
| D-131 混合增补 | 2026-06-25 | **§9 Q-D131 追加 agent 裁决 = 混合:compute=A / 图形=B(SPIR-V→DXIL 转译) 增补**(A→混合,追加式不改写既有 A 裁决文本)：A 单选后续证据链暴露 A 路图形签名不可达——slice3/round-8 实测 A 路图形 ISG1/OSG1 `elemcount=0`(LLVM `addSignature()` 无条件写空签名,#90504,填充耦合 Q-Builtin 🔒 FFI ABI 禁区);A-graphics 评估 = 跨前端/后端/PSV ~800-1500 LOC 上游大功能、#90504/#57928 无在途、carry-patch partial-blocked;B 取证 = B(SPIR-V→dxc)图形签名 `elemcount>0`、SV 端到端存活、validator accept、确定性 **measured 可行**(保真非完美 = P-01 边界)。agent 增补:compute 维持 A(结构首选 + round-8 浅修 PSV,RD-011)、图形改 B(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL),A-graphics 挂上游 #90504/#57928 成熟后迁移(RD-015)、B 供应链跟踪(RD-014)。图形=B 转译保真非完美(语义名→TEXCOORD/寄存器重排/未用输入消除)= **P-01 边界**,规范性裁断由 agent 自主落笔(P-13/硬规则 5),本回填只摆实测事实 + 留占位。图形=B 完整设计面 + 🔒 P-01 边界声明载新 RFC(拟 RFC-0004,Full RFC;或论证为本 RFC 增补,Q-Hybrid-RFC 留 agent);下游 RXS-0159 按 B 重构(或 hold)+ RXS-0160 + B 新增面随 RFC-0004 §5 计划。同步回填 13 §D-131(A→混合)+ registry(RD-010 close / RD-014 / RD-015)。**本增补为 agent 自主裁决,以 agent 合并本勘误 PR 生效**;G-G2-2 仍 open。规划文档勘误(00 §6.3 追加式,独立 PR,check_planning_docs 对 13 预期红,待 agent 自主 合入) | Full RFC（Approved，混合增补） |
