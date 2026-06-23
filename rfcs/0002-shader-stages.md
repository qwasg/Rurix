# RFC-0002 — 着色阶段进语言的类型面（vertex/fragment/compute/mesh/task/RT 作为 kernel 着色扩展）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0002（4 位制，编号永不复用，10 §9.5） |
| 标题 | 着色阶段进语言的类型面（vertex/fragment/compute/mesh/task/RT 作为 kernel 着色扩展） |
| 档位 | **Full RFC**（10 §3：新语法 + 类型系统扩张；触及 AGENTS 硬规则 5 禁区边界——纹理路径内存模型映射 06 §4.2 标 🔒 不落笔） |
| 状态 | **Owner Approved（2026-06-23）** — owner 已在本工作会话明确裁决 §9 Q1~Q6 与 §4.5 🔒 边界处置；批准记录由 AI 代录，**不是 AI 代签或自行裁决**（硬规则 1）。下游 spec/实现 PR 仍须按硬规则 7 序进（spec 先于实现）；FCP-lite 额外评审/等待窗若适用，仍按 10 §2.2/§5 独立完成 |
| 承接里程碑 | G2.1（验收门 **G-G2-1**），G2 首子里程碑 |
| 关联条款 | 拟落 spec **RXS-0153~RXS-0156**（4 条，已锁定，见 §5/§9 Q5；当前最高现存 RXS-0152 @ release.md）；拟新建 `spec/shader_stages.md` |
| 依据决策 | D-002（图形分期，已批准）· 06 §8.2（着色阶段 = kernel 着色扩展，设计预留）· 06 §4.2（纹理路径内存模型禁区，🔒）· 05 §1（device⊂host 单向可达）· 05 §2.2（trait 单态化子集 D-104） |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`。Human-in-the-loop（硬规则 1/2）：本草案由 AI 起草，禁区子节仅占位不落笔；owner 于 **2026-06-23** 在本工作会话明确裁决 §9 Q1~Q6 与 §4.5 边界处置，AI 仅代录该 owner 决定，不以 AI 身份署名或代签 |
| Owner 批准 | **Approved — 2026-06-23**。批准范围：RFC 全文，特别包括 §9 Q1~Q6 裁决与 §4.5 🔒 禁区边界处置（维持占位，纹理内存模型映射留后续独立 Full RFC）。记录方式：owner 在工作会话中直接裁决，由 AI 写回仓库（代录，非代签） |

> **批准记录**：本 RFC §4.5 标 🔒 的纹理/采样器内存模型映射边界属仅 owner 经 Full RFC 落笔的禁区（06 §4.2）。owner 已于 2026-06-23 批准本 RFC 并裁决 §4.5 **维持边界声明 + 占位「〈待 owner 后续 Full RFC〉」**——纹理内存模型映射条款留后续独立 Full RFC，不在本 RFC 落笔。本 RFC 仅定义类型面（§4.4），状态翻为 Owner Approved；下游 spec/实现 PR 仍按硬规则 7 序进，FCP-lite 额外评审/等待窗若适用按治理规则独立完成。

---

## 1. 摘要

本 RFC 在**不做 DXIL codegen**（G2.2）、**不做绑定布局推导实现**（G2.3）、**不做 UC-04**（G2.4）的前提下，定义把图形/计算/网格/光线追踪着色阶段作为 **kernel 着色扩展**进语言的**类型面与语法面**条款（06 §8.2 line 142）：

```
现状（G0~G1）：kernel fn / device fn / host fn 三类函数着色（RXS-0066）+ View<space,T> 地址空间（RXS-0067）+ launch 类型契约（RXS-0074）
                                  │
本 RFC（G2.1，仅类型面/语法面）：
   ├─ 着色阶段函数着色扩展：vertex / fragment / compute / mesh / task fn + RT raygen/closesthit/anyhit/miss fn
   │    （新 coloring，复用 kernel 子语言类型系统 + views，遵守 device⊂host 单向可达 + trait 单态化子集）
   ├─ 阶段专属 I/O 语义类型：插值限定（type-level）+ 内建变量类型化（position/vertex-id 等）
   ├─ 阶段间接口类型契约：vertex out → fragment in 类型兼容编译期校验
   └─ 资源句柄 / 纹理采样器参数化类型的【类型面】：Texture2D<F> / sampler 类型形态（平行于 View<space,T>）
                                  │
   🔒 不在本 RFC：纹理/采样器内存模型映射（tex proxy / PTX tex opcode / 采样器描述符编码 / 缓存一致性 / UB）= 06 §4.2 禁区，留后续 Full RFC（owner 落笔）
```

本 RFC 只定义着色阶段在语言中的**类型形态、着色规则、接口契约与编译期拦截类别**；codegen（G2.2）、绑定推导实现（G2.3）、纹理内存模型映射（禁区）均**不在本 RFC**。着色阶段语言面是 G2.2/G2.3/G2.4 的共同依赖基座，故 spec-first 先立条款（规范先行硬规则 7）。

## 2. 动机

- **D-002 图形分期（已批准）** 与 **06 §8.2 设计预留** 要求 G2 把着色阶段进语言，且「现在定方向、不定细节，避免 G2 时推翻 MVP 决策」。本 RFC 把 06 §8.2 第 1 点（着色阶段 = kernel 着色扩展）的**语言类型面**落为可条款化的精确设计。
- **共同基座**：DXIL codegen（G2.2，把着色阶段函数降级到 DXIL）、绑定布局推导（G2.3，从着色阶段签名推导 descriptor/root signature，P-11）、UC-04 deferred 渲染器（G2.4，多 pass 管线由着色阶段组合）均依赖着色阶段在语言中的类型表达。无类型面则下游无锚点，故先行（硬规则 7）。
- **复用而非另起**：着色阶段复用既有 kernel 子语言类型系统（RXS-0066 函数着色）+ views（RXS-0067 地址空间 / RXS-0078 views 算子集），不引入第二套设备子语言，降低复杂度黑洞风险（Taichi/Slang 前科，SG-005）。

**为何需要 Full RFC（而非 Direct/Mini）**：本 RFC 触及 **新语法 + 类型系统扩张**（着色阶段函数着色、阶段 I/O type-level 标注、阶段间接口契约、纹理采样器参数化类型）——这是 10 §3 / AGENTS 硬规则 5 明列的 Full RFC 触发面，只能由人类经 Full RFC 落笔。判档争议向上取严（硬规则 8），AI 不自判 Direct/Mini、不代签批准/合并（硬规则 1）。此外纹理路径内存模型映射（06 §4.2）属禁区，本 RFC 标 🔒 明确不定义、留 owner。

## 3. 指导级解释（用户视角）

> 以下为**拟议**语法形态的示意，最终语法以 owner 批准 + spec 条款（RXS-0153~）为准；本节示例用于沟通设计意图，不构成已批准语法。

着色阶段以函数着色关键字声明，与现状 `kernel fn` 平行：

```rust
// 顶点着色阶段：输入顶点属性，输出裁剪空间位置 + 传递给 fragment 的插值数据
vertex fn vs_main(
    in: VertexIn,                 // 阶段输入：顶点属性（type-level 绑定语义）
) -> VertexOut { /* ... */ }

// 片元着色阶段：消费 vertex 阶段输出的插值数据，输出颜色
fragment fn fs_main(
    in: VertexOut,                // 阶段间接口：类型须与 vertex 输出兼容（编译期校验）
) -> FragmentOut { /* ... */ }

// 网格/任务着色阶段
mesh fn ms_main(/* ... */) -> MeshOut { /* ... */ }
task fn ts_main(/* ... */) { /* ... */ }

// 光线追踪阶段（RT）
raygen fn rg_main(/* ... */) { /* ... */ }
closesthit fn ch_main(/* ... */) { /* ... */ }
anyhit fn ah_main(/* ... */) { /* ... */ }
miss fn miss_main(/* ... */) { /* ... */ }
```

阶段 I/O 用 type-level 标注表达插值限定与内建变量（拟议形态）：

```rust
struct VertexOut {
    #[builtin(position)] clip_pos: Vec4<f32>,   // 内建变量类型化（位置输出）
    #[interpolate(perspective)] uv: Vec2<f32>,  // 插值限定（type-level）
    #[interpolate(flat)] mat_id: u32,
}
```

资源句柄与纹理采样器作为**参数化类型**使用，与 `View<space, T>` 平行（仅类型形态，不含本 RFC 范围外的内存模型语义）：

```rust
fragment fn fs_textured(
    in: VertexOut,
    tex: Texture2D<f32>,          // 格式参数化纹理类型（类型面）
    samp: Sampler,                // 采样器类型形态
) -> FragmentOut { /* 采样调用的内存模型映射不在本 RFC，留后续 Full RFC */ }
```

类型系统保证（编译期拦截，详见 §4）：着色阶段误用（如对非着色函数施加阶段调用、阶段关键字错配）、阶段间接口不匹配（vertex out 与 fragment in 类型不兼容）、资源句柄违例均在编译期 100% 拦截，无运行期回退（P-01 strict-only）。

## 4. 参考级设计

> 本节落笔**类型面/语法面**设计；触及禁区的子节（§4.5 纹理/采样器内存模型映射）标 🔒，本草案不写内容、留 owner 落笔。

### 4.1 着色阶段函数着色（扩展 RXS-0066 function coloring）

现状函数着色（RXS-0066）划分 `host` / `device` / `kernel`（含 `const`）着色，并以跨着色调用合法性（device 上下文不可调 host-only 函数 / 不可直接调用 kernel fn → RX3001）约束。本 RFC 把着色阶段作为**新增 coloring 类别**接入同一着色格：

- **图形/计算阶段着色**：`vertex` / `fragment` / `compute`（D3D12 语境）/ `mesh` / `task`。
- **光线追踪阶段着色**：`raygen` / `closesthit` / `anyhit` / `miss`。
- 各着色阶段函数体复用 **kernel 子语言类型系统**（设备受限子集，05 §1）与 **views**（`View<space,T>` RXS-0067 / views 算子集 RXS-0078）；不引入第二套设备子语言。
- **device⊂host 单向可达（05 §1）**：着色阶段函数属设备侧着色，遵守 kernel 着色的单向可达——着色阶段可调用 `device fn`，host 侧不可直接进入着色阶段体（阶段进入点经管线/launch 类比，运行期分发面不在本 RFC）。跨着色非法调用复用既有 RX3001 类别 + 着色阶段专属新类别（§5）。
- **trait 单态化子集（D-104，05 §2.2）**：着色阶段选择与分发为**编译期静态**，无 `dyn` 派发、无特化、无 HKT/async；阶段着色在 HIR 层静态可判（对齐 07 §3 着色检查无数据流）。

### 4.2 阶段专属 I/O 语义类型

着色阶段的输入/输出携带阶段专属语义，以 **type-level 标注**表达（拟议形态，精确语法见 §9 未决问题）：

- **插值限定（interpolation qualifier）**：片元阶段输入的插值方式（perspective / linear / flat / centroid 等）作为 type-level 标注，参与阶段间接口类型校验（§4.3）。
- **内建变量类型化（builtin variable typing）**：阶段内建输入/输出（如顶点阶段的 `position` 输出、`vertex-id` / `instance-id` 输入，片元阶段的 `frag-coord`，计算阶段的 `thread-id` 等）作为 **type-level 输入/输出契约**，编译期校验其类型与所属阶段匹配（错配 → 着色阶段误用新类别，§5）。
- I/O 聚合类型（如 `VertexOut`）的字段语义由标注决定，普通字段缺标注的处置规则列入 §9 未决问题。

本子节只定义类型面；内建变量在 codegen 后端的具体寄存器/语义槽映射属 G2.2（DXIL codegen），不在本 RFC。

### 4.3 阶段间接口类型契约

相邻着色阶段经类型契约连接，编译期校验类型兼容性：

- **vertex out → fragment in**：片元阶段输入类型须与上游顶点阶段输出类型**兼容**（字段、类型、插值限定一致）；不兼容 → 阶段间接口不匹配新类别（§5）。
- 网格管线（`task` → `mesh` → `fragment`）与 RT 管线（`raygen` ↔ `closesthit`/`anyhit`/`miss` 经 payload/attribute 类型）的阶段间接口契约形态列入 §9 未决问题（payload/attribute 类型化与 vertex→fragment 是否完全同构待裁）。
- 接口契约为**编译期静态校验**（HIR/typeck 层），无运行期协商；契约不匹配为编译错误（P-01 strict-only）。

### 4.4 资源句柄 / 纹理采样器参数化类型的类型面

资源句柄与纹理采样器作为**参数化类型**进入着色阶段签名，平行于 `View<space, T>`（RXS-0067）：

- **纹理类型**：`Texture2D<F>` 等格式参数化类型（`F` = 像素格式类型参数），以及其它维度变体（`Texture1D`/`Texture3D`/`TextureCube` 等，精确集合列入 §9）。本 RFC 只定义其**类型形态**（参数化、着色阶段签名中的可出现位置、与现有类型系统的关系）。
- **采样器类型**：`Sampler` 类型形态（作为着色阶段参数的类型面）。
- **资源句柄**：descriptor 可绑定资源（纹理/采样器/缓冲）作为着色阶段签名中的类型化句柄；其**绑定布局推导**（host 结构体 ↔ shader 布局单一事实源，P-11）属 G2.3，不在本 RFC——本 RFC 仅定义句柄在签名中的类型表达，供 G2.3 推导消费。
- 资源句柄违例（如句柄类型与着色阶段不相容、句柄出现在非法位置）作为编译期拦截新类别（§5）。

### 4.5 🔒 纹理 / 采样器内存模型映射边界（禁区，本 RFC 不定义）

> **本子节为边界声明，AI 不落笔禁区内容。**

纹理/采样器的**内存模型映射**——包括但不限于：tex proxy 选择、PTX/DXIL 纹理采样 opcode 映射、采样器描述符编码、纹理缓存一致性语义、采样的未定义行为（UB）边界——属 **06 §4.2 内存模型禁区**（纹理路径 G2 引入时再扩展映射条款，仅人类经 Full RFC 落笔，AGENTS 硬规则 5）。

本 RFC **明确不定义**上述任何内存模型语义，仅在 §4.4 定义纹理/采样器的**类型形态（type-level shape）**。内存模型映射条款留后续 Full RFC：

〈待 owner 后续 Full RFC〉 — owner 于 2026-06-23 批准本 RFC 时裁决 §4.5 **维持占位**（§9 Q6）：纹理内存模型映射属独立未来 Full RFC，不在本 RFC 落笔。

本边界与 §8 范围红线一致：本 RFC 的 `Texture2D<F>` / `Sampler` 是**类型面参数化形态**，不承诺任何采样语义、内存序或一致性保证。

## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

拟新建 `spec/shader_stages.md`，自 **RXS-0153** 起续号（当前最高现存 RXS-0152 @ release.md）。**区间已锁定 4 条 `RXS-0153~RXS-0156`**（owner 2026-06-23 裁决，§9 Q5）。下表为条款与测试锚定计划，**本会话不创建 `### RXS-0153` 等裸条款头**（条款体随 owner 批准后的 spec 脚手架/实现 PR 同落，trace 维持 152/152）：

| 条款（拟） | 标题 | 测试锚定计划（每条 ≥1，`//@ spec: RXS-####`） |
|---|---|---|
| RXS-0153 | 着色阶段函数着色规则（vertex/fragment/compute/mesh/task + RT raygen/closesthit/anyhit/miss 作为新 coloring，扩展 RXS-0066） | conformance accept（合法着色阶段声明 0 诊断）+ reject（着色阶段误用 / 跨着色非法调用）+ UI golden |
| RXS-0154 | 阶段专属 I/O 语义类型（插值限定 type-level + 内建变量类型化） | conformance accept（合法 I/O 标注）+ reject（内建变量类型/阶段错配）+ UI golden |
| RXS-0155 | 阶段间接口类型契约（vertex out → fragment in 兼容性编译期校验） | conformance reject（接口类型不匹配）+ accept（兼容接口）+ UI golden |
| RXS-0156 | 资源句柄 / 纹理采样器参数化类型的类型面（`Texture2D<F>` / `Sampler` 类型形态，平行 `View<space,T>`） | conformance accept（合法句柄签名）+ reject（句柄违例 / 非法位置）+ UI golden |

> 区间已锁定为 4 条 `RXS-0153~RXS-0156`（§9 Q5，owner 2026-06-23）：网格/RT 阶段间接口（payload/attribute）并入 RXS-0155、纹理类型集合并入 RXS-0156，本里程碑不拆条。

- **错误码策略**：着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例 = **Rurix 语义诊断**（编译期可检的着色/接口/句柄合法性，对齐 RXS-0066 着色诊断先例），归 **3xxx 着色/地址空间段位续号**（07 §5 语义分配；当前该段末号 **RX3010**，下一可用 **RX3011+**——**非全局 RX7020**，7xxx 为运行期/互操作段）。纯 Rust 通用错误（类型不符等）走 rustc 原生诊断（零新 RX）。**不预留、不预造**：RX3011+ 随实现 PR 按真实可达类别只追加分配 + en/zh message-key（`registry/error_codes.json` 只追加，`ci/bilingual_coverage.py` 覆盖）。
- spec 条款 PR 先于实现 PR（硬规则 7）；trace_matrix 维持全锚定（沿用全局 `m1.counter.spec_clause_test_anchoring`，不另立 g2 counter，对齐 G-G1-6）。

## 6. feature gate / tracking / 实现序（10 §3 要件）

- **feature gate**：cargo feature 拟名 `shader-stages`；未启用时着色阶段语法/类型面不参与编译。tracking 清单随实现 PR 维护（实现状态 / 未决问题 / 测试清单）。
- **栈式 PR**（门控于本 RFC 合入后；本分支 `feat/g2.1-shader-stages-rfc` 只产 RFC 草案，后续另起栈式分支）：
  - **PR-B1 spec 脚手架**：`spec/shader_stages.md` 登记文件名 + RXS-0153~ 预留区间（**不落裸条款头**）+ `spec/README.md` §4 文件清单行 + RXS 末号同步 + 修订行（带 Full RFC 档位标记，spec/README §3）；`trace_matrix --check` PASS（维持全锚定）。
  - **PR-B2 spec 条款体 + rurixc 着色阶段前端**：条款体（RXS-0153~）+ 着色阶段解析 + 着色规则 + 类型检查；着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例 100% 编译期拦截 → conformance reject 类别（`tests/ui` 等）+ UI golden（`.stderr`，经 `bless_log` 审批）；新段位错误码 RX3011+（07 §5 段续号）+ en/zh message-key（registry 只追加）。边界若落 unsafe → U23+ 注册 + `// SAFETY:`（纯前端 typeck 预期零新 unsafe）。
  - **CI 步骤 45**（CI_GATES §2，着色阶段条款/拦截冒烟，**纯 host/CPU-only 可跑**）随实现 PR 回填 workflow。
- **真实红绿**（反 YAML-only，CI_GATES §6）：放行着色阶段类型违例 / 阶段接口不匹配 → check 红 → 复原绿，归档前后输出 / run URL。**纯 host/CPU-only 可完整演示**（着色阶段类型面为编译期，无需 device）。
- **device 证据**：本里程碑着色阶段条款先行为编译期面，**预期沿用全局 spec 锚定断言，无新 device counter**（`g2.counter.*` 仅当后续 codegen 子里程碑需 device 证据时新增）。

## 7. 备选方案

- **独立着色子语言 vs kernel 着色扩展**：采纳**着色阶段 = kernel 着色扩展**（复用 RXS-0066 函数着色 + RXS-0067/0078 views，06 §8.2 已锁方向）；否决另起独立着色子语言（引入第二套设备类型系统，复杂度黑洞，Taichi/Slang 前科 SG-005，且违 06 §8.2 设计预留）。
- **插值限定作属性（attribute）vs type-level 标注**：本 RFC 倾向 type-level 标注（参与阶段间接口类型校验，编译期可判），但精确形态（属性式 `#[interpolate(..)]` vs 类型包裹）留 §9 Q2 待裁；否决纯运行期/约定式插值（违 P-01 strict-only，无法编译期校验接口契约）。
- **阶段进入/分发 API**：扩展既有 `launch`/RXS-0074 vs 新 `dispatch`/管线对象入口——留 §9 Q3 待裁（本 RFC 仅定义类型面，分发面运行期语义不在范围）。
- **纹理/采样器类型是否完全平行 View**：倾向平行 `View<space,T>` 形态（§4.4），但是否完全同构（地址空间参数、子 view 算子是否适用）留 §9 Q4；内存模型语义一律不在本 RFC（§4.5 🔒）。

## 8. 不做（范围红线）

本 RFC 明确**不涉及**以下范围（防蔓延）：

- **DXIL codegen**（G2.2，D-131）：MIR→DXIL 后端、内建变量寄存器/语义槽映射、DXIL 文本 golden 均不在本 RFC。
- **绑定布局推导实现**（G2.3，P-11）：descriptor / root signature 编译器推导生成不在本 RFC；本 RFC 仅定义资源句柄在签名中的类型表达供 G2.3 消费。
- **UC-04 deferred 渲染器**（G2.4）：端到端出图 demo 不在本 RFC。
- **edition**（G2.5）：edition / stabilization 机制不在本 RFC。
- **🔒 纹理路径内存模型映射**（06 §4.2 禁区）：tex proxy / 采样 opcode / 采样器描述符编码 / 缓存一致性 / UB 留后续 Full RFC（owner 落笔，§4.5）。
- **FFI ABI / 安全包络 / UB 条款**（AGENTS 硬规则 5 禁区）：本 RFC 不定义。
- **多后端**（AMD/Intel/Metal/Vulkan/SPIR-V）：死亡路线红线 3（D-008 维持不解除，SG-003 not_triggered）；DXIL 是 D3D12 原生路径，非通用多后端。
- **多 GPU / VMM / NVLink / MIG**（A-06 单机单 GPU 语义边界）：不在本 RFC。

## 9. Q1~Q6 裁决结果（owner 2026-06-23 批准，AI 代录）

> owner 于 2026-06-23 在本工作会话明确裁决以下 Q1~Q6；AI 仅将该人工决定写回文档（代录），不自行裁决或代签（硬规则 1）。技术细节随 owner 批准后的 spec 脚手架/实现 PR 落条款体（trace 维持 152/152）。

- **Q1 着色阶段集合与关键字形态**：采用**前缀式 `<stage> fn`**——`vertex` / `fragment` / `compute` / `mesh` / `task` fn + RT `raygen` / `closesthit` / `anyhit` / `miss` fn，与现状 `kernel fn` 平行（对齐 §3 示例）；否决属性式 `#[stage(..)] fn`。**compute 阶段（D3D12 语境）复用既有 `kernel` 着色**，不另立独立 coloring。
- **Q2 阶段 I/O 标注形态**：插值限定与内建变量用**属性式标注** `#[interpolate(..)]` / `#[builtin(..)]`（对齐 §3/§4.2 示例）；否决 type-level 包裹类型。普通**无标注字段 = 编译期拒绝**（P-01 strict-only，不默认插值、不静默放行）。
- **Q3 阶段调用 / 分发 API**：本 RFC **仅定义类型面**；阶段进入/分发的运行期面（扩展 `launch`/RXS-0074 vs 新 `dispatch`/管线对象入口）**明确不在本 RFC**，作 tracking 项留后续 spec/实现 PR 裁决（§6 tracking 清单维护）。
- **Q4 纹理采样器类型与 View 的关系**：`Texture2D<F>` / `Sampler` 与 `View<space,T>` **平行但不强制完全同构**——地址空间参数化、子 view 算子是否适用按需，不预先承诺。**首批仅 `Texture2D<F>` + `Sampler`**；其余纹理维度（`Texture1D` / `Texture3D` / `TextureCube` / `Array`）**defer** 后续。内存模型语义一律不在本 RFC（§4.5 🔒）。
- **Q5 RXS 区间大小**：**锁定 4 条 `RXS-0153~RXS-0156`**（§5 表为准）。网格（`task`→`mesh`→`fragment`）与 RT（payload/attribute）阶段间接口**并入 RXS-0155**，纹理类型集合**并入 RXS-0156**，本里程碑**不拆条**。区间不预留、不预造；条款体随 owner 批准后的 spec 脚手架/实现 PR 同落（trace 维持 152/152）。
- **Q6 禁区 §4.5 文本**：**维持占位「〈待 owner 后续 Full RFC〉」**。纹理/采样器内存模型映射（tex proxy / 采样 opcode / 描述符编码 / 缓存一致性 / UB）= 06 §4.2 禁区，属**独立未来 Full RFC**（纹理路径 G2 引入时再扩展），本 RFC 不落笔，仅在 §4.5 作边界声明。

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：`shader-stages` feature gate 后 → tracking → 两里程碑无重大修订 → stabilization report → FCP-lite（10 §2.2，≥2/3 同意含语言负责人 + 5–7 天公开等待窗）。stable 面冻结随 **RD-008** 届时定义（G2.5 语言 1.0 为候选触发点）；本 RFC 引入的着色阶段类型面在首个 stable 发布前不进 stable 面。
- **Provenance**：`Assisted-by: claude-code:claude-opus-4-8`。本草案由 AI 起草；§4.5 🔒 禁区维持占位不落笔。owner 于 **2026-06-23** 在本工作会话明确裁决 §9 Q1~Q6 并授权记录批准；**该决定由 AI 代录，非 AI 代签 / 自行裁决**（硬规则 1）。技术问题已收敛为 §9 裁决；FCP-lite 额外评审/等待窗若适用，仍按 10 §2.2/§5 独立完成，本记录不虚构尚不存在的第二位评审。下游 spec/实现 PR 按硬规则 7 序进（spec 先于实现）。

## 11. 规范与实现依据

- 06 §8.2（G2 着色阶段 = kernel 着色扩展，vertex/fragment/compute/mesh/task/RT，设计预留）/ §4.2（纹理路径内存模型禁区，仅人类 Full RFC）。
- 05 §1（device⊂host 单向可达，kernel 子语言受限子集）/ §2.2（trait 单态化子集 D-104：无 dyn/特化/HKT/async）。
- 04 设计原则：P-01（strict-only，无静默 fallback）/ P-11（单一事实源——绑定推导 G2.3 依据）/ P-13（防 AI 幻觉治理三角）。
- spec/device.md：RXS-0066（函数着色与跨着色调用合法性）/ RXS-0067（地址空间类型与一致性 `View<space,T>`）/ RXS-0074（launch 类型契约）/ RXS-0078（views 算子集）。
- registry/error_codes.json：3xxx 着色/地址空间段位（当前末号 RX3010）。
- RFC-0001（CUDA–D3D12 interop，Full RFC 先例）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-06-23 | AI 起草初版（着色阶段语言类型面：§4.1 函数着色扩展 / §4.2 阶段 I/O 语义类型 / §4.3 阶段间接口契约 / §4.4 资源句柄·纹理采样器参数化类型面 / §4.5 🔒 纹理内存模型禁区边界占位 + §5 下游 spec 条款映射 RXS-0153~ + 错误码 RX3011+ 策略 + §6 feature gate `shader-stages`/实现序/真实红绿计划 + §7 备选 + §8 范围红线 + §9 Q1~Q6 未决留 owner + §10 稳定化 + §11 依据）。**待 owner 人工批准（FCP-lite），AI 不代签 / 不推进下游；§4.5 禁区文本与 §9 未决由 owner 落笔/裁决** | Full RFC（Draft） |
| Owner approval | 2026-06-23 | owner 在本工作会话明确裁决 §9 Q1~Q6（Q1 前缀式 `<stage> fn` + compute 复用 kernel / Q2 属性式标注 + 无标注字段编译期拒绝 / Q3 仅类型面·分发面留 spec/impl PR / Q4 纹理平行 View 不强制同构·首批 `Texture2D<F>`+`Sampler` / **Q5 锁定 4 条 RXS-0153~0156** / Q6 §4.5 维持占位）并授权记录批准；AI 仅将该人工决定写回文档（代录），不代签。状态翻 Owner Approved。FCP-lite 额外评审/等待窗若适用按 10 §2.2/§5 独立完成 | Full RFC（Owner Approved） |
