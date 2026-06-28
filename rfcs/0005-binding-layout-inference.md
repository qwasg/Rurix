# RFC-0005 — 绑定布局推导（descriptor / root signature 编译器推导）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0005（4 位制，编号永不复用，10 §9.5） |
| 标题 | 绑定布局推导（从 shader 资源使用推导 D3D12 descriptor / root signature，P-11 单一事实源） |
| 档位 | **Full RFC**（10 §3：新 codegen 推导面 + 触 AGENTS 硬规则 5 禁区边界——签名/绑定二进制 ABI 布局（RFC-0004 §4.6(a) 同级）/ 纹理路径内存模型映射（06 §4.2）/ DXIL·SPIR-V UB 边界；本 RFC 只引边界声明，不落禁区语义本体） |
| 状态 | **Accepted / Owner Approved（2026-06-28）**。owner（Language Lead）已在本工作会话同意 RFC-0005 全文 + §9 全部裁决，并批准 3bcc486 作为 RFC-0005 FCP-lite/owner 批准记录的技术载体；记录由 Codex 代录，非 AI 代签。下游 PR 仍按 §6 栈式序进（PR-E1 spec 脚手架先于 PR-E2 实现） |
| 承接里程碑 | G2.3（验收门 **G-G2-3**，D-G2-3），承 G2.1 着色阶段类型面（RFC-0002）+ G2.2 图形=B codegen（RFC-0004）；可与 G2.2 部分并行 |
| 关联条款 | 拟落 spec **RXS-0163 ~ RXS-0166**（§9 Q-Range 已裁，见 §5）；落点 = 新建 `spec/binding_layout.md`（§9 Q-File 已裁）。**本 RFC 不创建裸条款头**，trace 维持现状 |
| 依据决策 | D-002（图形分期，已批准）· 06 §8.2（descriptor / root signature 编译器推导 = G2 设计预留，P-11）· 06 §4.2（纹理路径内存模型禁区，🔒）· 04 P-11（单一事实源——host 结构体 ↔ shader 布局）· 04 P-01（strict-only）· RFC-0002（着色阶段类型面，资源句柄类型 RXS-0156）· RFC-0004（图形=B codegen + §4.6 禁区边界） |
| Provenance | `Assisted-by: kiro:claude-opus-4-8`（Draft）+ `Assisted-by: codex:gpt-5`（owner 裁决落文档）。Human-in-the-loop（硬规则 1/2）：本草案由 AI 起草，§4 measured 锚定来自隔离 spike 真实命令输出，禁区子节仅作边界声明；§9 路径抉择由 owner 于 2026-06-28 裁决，Codex 代录 |
| Owner 批准 | **Approved — owner（Language Lead）2026-06-28**。批准范围：RFC-0005 全文；§4.5 🔒 禁区边界声明（不落禁区语义本体）；§9 全部裁决（Q-Space=B / Q-RootShape=B / Q-Sampler=B / Q-Bindless=A→RD-018 / Q-Gate=A / Q-Err=6xxx 续号策略 / Q-File=B / Q-Range=RXS-0163~0166 / Q-Inference-vs-Explicit=C）；3bcc486 给 FCP-lite/owner 批准。记录方式：Codex 按 owner 本会话明确同意代录，非 AI 代签；本批准不声称 device 真跑、golden bless、稳定化或禁区语义本体已完成 |

> **批准记录**：本 RFC 触及 🔒 签名/绑定二进制 ABI 布局（RFC-0004 §4.6(a) 同级:register/space/mask/packing/descriptor table 偏移/root parameter DWORD 物理布局）、纹理路径内存模型映射（06 §4.2）、DXIL·SPIR-V UB 边界——这些只能由人类经 Full RFC 落笔（硬规则 5）。owner 于 2026-06-28 以 Language Lead 身份批准 RFC-0005 全文并裁决 §9 全部路径项；Codex 仅代录该人工决定。§4/§8 仍仅作**边界声明**，不落禁区语义本体；本批准不把 register/space 具体值、descriptor table 偏移或 root parameter DWORD 物理布局冻结为 stable 语言保证。

---

## 1. 摘要

本 RFC 在**不实现绑定推导 codegen**、**不落条款体**、**不接线 codegen** 的前提下，定义把 **D3D12 descriptor / root signature 由编译器从 shader 资源使用推导生成**（06 §8.2，P-11 单一事实源）的**设计面 + 推导判据 + 下游条款计划**：

```
RXS-0156 资源使用（着色阶段签名中的资源句柄类型面，RFC-0002）
   Texture2D<F> / Sampler 句柄 + constant buffer / structured buffer
                          │
本 RFC（G2.3，仅设计面 + spike 留痕 + 脚手架预备）：
   ├─ 推导面锚点：资源使用 → SPIR-V DescriptorSet/Binding 装饰 → register/space → root signature 形态
   ├─ 推导判据：register/space 分配确定性导出（按声明序）+ root signature 形态推导（Rurix 编译器侧职责）
   ├─ strict-only 兜底：不可推导 / 超上限 / register 冲突 → 编译期 6xxx 诊断（无运行期 fallback，P-01）
   └─ 单一事实源（P-11）：host 绑定结构 ↔ shader 布局由编译器推导对齐，不手维护两份
                          │
   🔒 不在本 RFC：register/space/mask/packing 物理布局 ABI（RFC-0004 §4.6(a) 同级）/ 纹理内存模型映射（06 §4.2）/ 推导 codegen 实现 / device 真跑出图（G2.4）
```

本 RFC 只定义绑定布局推导的**设计面、推导判据、错误类别与下游条款计划**；**推导实现、codegen 接线、golden、device 真跑均不在本 RFC**（随 owner 批准后实现 PR，条款先于实现，硬规则 7）。§4 的可行性以**隔离分支 `spike/g2.3-binding-layout` 的 measured 实测**为锚（§4.2/§11），严格区分「measured 已验证」与「assumed 待 owner 裁 / 实现侧待建」（对齐 RFC-0004 strict-only spike 诚实纪律）。

## 2. 动机

- **06 §8.2 + P-11 要求 G2 把绑定布局做成编译器推导**：descriptor / root signature 由编译器从 shader 资源使用单一事实源推导生成（P-11：host 结构体 ↔ shader 布局不手维护两份、不漂移）。G2.3 是该设计预留的落地子里程碑（D-G2-3 / G-G2-3）。
- **G2.4 UC-04 deferred 渲染器的前置**：多 pass deferred 管线需 PSO 消费带正确 root signature 的着色器对象；着色阶段类型面（G2.1，RXS-0156 资源句柄）与图形=B codegen（G2.2，RFC-0004）已就位，但**资源句柄 → 绑定布局**这一段是空白——无推导则 UC-04 无法端到端出图。
- **承接 G2.1/G2.2 的真实缺口**：RXS-0156 把 `Texture2D<F>`/`Sampler` 定为类型面句柄、明确「绑定布局推导属 G2.3」；RFC-0004 §4.2 的 B 链当前 `io_sig`/`MirIoType` 仅表达标量/向量 I/O，**结构上无资源绑定表达**（measured，§4.2 / §11）——绑定推导是 G2.3 的新建面，需独立 Full RFC 精确化。

**为何需要 Full RFC（而非 Direct/Mini）**：本 RFC 引入**新 codegen 推导面**（从资源使用推导 root signature 形态并序列化进 DXIL 容器），且触及 **签名/绑定二进制 ABI 布局**（RFC-0004 §4.6(a) 同级:register/space/mask/packing/descriptor table 物理布局）、**纹理路径内存模型映射**（06 §4.2）、**DXIL·SPIR-V UB 边界**——10 §3 / 硬规则 5 明列的 Full RFC / 禁区触发面。判档争议向上取严（硬规则 8）；AI 不自判 Direct/Mini、不代签批准/合并、不代 owner 裁 §9（硬规则 1）。

## 3. 指导级解释（用户视角）

> 以下为**拟议**形态示意，最终以 owner 批准 + spec 条款为准；**绑定布局对用户尽量透明**——用户在着色阶段签名里声明资源句柄（RXS-0156），descriptor / root signature 由编译器推导，无需手写 root signature 或手维护 host/shader 两份布局（P-11）。

用户在着色阶段签名中声明资源句柄（RFC-0002 RXS-0156 类型面），编译器推导出 D3D12 绑定布局：

```rust
// 着色阶段签名声明资源句柄（RXS-0156 类型面：Texture2D<F> / Sampler）
fragment fn fs_textured(
    in: VertexOut,
    tex: Texture2D<f32>,   // → 编译器推导为 SRV 绑定
    samp: Sampler,         // → 编译器推导为 sampler 绑定（static vs dynamic = §9 Q-Sampler）
) -> FragmentOut { /* 采样调用的内存模型映射不在本 RFC，留禁区 Full RFC */ }
```

编译器从资源使用**确定性推导**出 descriptor / root signature 形态（register/space 分配 + root parameter 形态），用户不手写 root signature；推导结果是 host 侧绑定与 shader 侧布局的**单一事实源**（P-11），二者由编译器对齐、不漂移。

`strict-only`（P-01）维持：资源使用无法推导为合规布局（超 root signature 上限 / 资源种类不可映射 / register 冲突）→ **结构化编译错误**（6xxx 段），无静默降级、无运行期 fallback。

> **拟议中的开放问题（不在本节定型，全部留 §9 owner 裁决）**：register/space 分配策略（按声明序打包 / 按种类分 space / 是否开放 `#[binding(...)]` 显式标注）、root signature 形态（全 descriptor table / CBV root descriptor / root constant / 混合）、sampler 归类（static vs dynamic）、bindless 是否进本期——本 RFC **不替用户/owner 预定**，§3 示例仅示意类型面声明，不示意已定的布局策略。

## 4. 参考级设计

> 本节落笔**绑定布局推导的设计面与可推导路径锚点**；可行性以隔离分支 `spike/g2.3-binding-layout` 的 **measured 实测**为锚（§4.2/§11），严格区分 measured / assumed。**具体 register 值、space 分配轴、descriptor table 偏移、root parameter DWORD 物理布局不由本 RFC 发明**（🔒 §4.5 边界声明）。推导算法/codegen 实现留实现 PR。

### 4.1 推导面在 B 链中的位置（分叉点与上下游）

承 RFC-0004 图形=B 链（MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL），绑定布局推导是 **资源句柄 → 绑定布局** 这一段的编译器推导面：

```
RXS-0156 资源句柄类型（着色阶段签名）
   │  (a) Rurix MIR→SPIR-V：资源句柄 → SPIR-V opaque 资源类型 + DescriptorSet/Binding 装饰
   ▼                          [本 RFC 新增推导面；当前结构上不可达，assumed §4.3]
 SPIR-V (set,binding) 装饰
   │  (b) spirv-cross：(set,binding) → HLSL register(x#, spaceN)（确定性，measured §4.2）
   ▼
 HLSL register(x#, spaceN)
   │  (c) Rurix 推导 root signature 形态（descriptor table / root descriptor / root constant；
   │      static vs dynamic sampler）→ 经 dxc [RootSignature]/序列化 API 注入 RTS0
   ▼                          [推导=Rurix 职责，measured：工具链不自动合成 §4.2]
 dxc → DXIL 容器（RTS0 root signature + PSV0 资源绑定反射）
```

- **共享前沿**：着色阶段类型面（RFC-0002）+ B 链 codegen（RFC-0004）对绑定推导共享，不分叉。
- **推导面锚点**：(a) 资源句柄 → SPIR-V 资源绑定装饰（按声明/io_sig 顺序确定性分配，分配轴策略 = §9 Q-Space）；(c) 从资源使用推导 root signature 形态并序列化为容器 `RTS0` part（形态策略 = §9 Q-RootShape）。
- **strict-only（P-01）**：不可推导构造 / 超上限 / register 冲突 → 结构化 6xxx 诊断（§9 Q-Err），无运行期 fallback。

### 4.2 Measured 可行性（隔离 spike，命令真实输出，§11）

`spike/g2.3-binding-layout`（round-1，`evidence/g2.3-binding-layout/binding_layout_spike_20260627.json`，`status: MEASURED`，4/4 语料 chain 全通）实测：

- **(measured-1) register/space 映射确定性可预测**：spirv-cross 默认从 SPIR-V 装饰派生 HLSL 寄存器——`DescriptorSet N → spaceN`；`Binding K → 寄存器索引 K`；register class 由资源种类定（CBV→`b`、SRV(`Texture2D`/`StructuredBuffer`)→`t`、Sampler→`s`、UAV(`RWStructuredBuffer`)→`u`）。混合语料 `ps_mixed`（binding 全局连号 0..4）→ `b0,t1,t2,s3,s4`（measured）——即**索引轴有两种自洽策略**：(A) 单一全局 binding 轴（spirv-cross 默认，t/s 跨种类连号）；(B) 按种类各自从 0 计数（`ps_rootsig` 手写 register 实测 `b0,t0,t1,s0,s1`）。两者都确定性、都产合规 DXIL → **register/space 可由编译器按声明序确定性导出**（measured），但「索引轴策略」是路径抉择（§9 Q-Space），**不自填默认**。
- **(measured-2) root signature 不由工具链自动合成**：默认编译（`ps_textured`/`ps_mixed`/`cs_structured`，3/3）DXIL 容器含 `PSV0` 资源绑定反射、**无 `RTS0`**；仅显式 `[RootSignature(...)]`（`ps_rootsig`）时 dxc 把 root signature 序列化为 `RTS0` part（measured）。→ **「绑定布局推导（资源使用 → root signature 形态）是 Rurix 编译器侧职责」**；工具链提供 (a) `PSV0` 反射作交叉校验输入、(b) `[RootSignature]`/序列化作 `RTS0` 承接，**不替代推导**。
- **(measured-3) 确定性**：4/4 语料同输入二次编译，DXIL 容器 SHA256 全等（`deterministic: true`）→ 可纳入 golden（对齐 RXS-0162 Property 3）。

### 4.3 Assumed / 实现侧待建（严格分栏，不冒充 measured）

- **(assumed-1) Rurix MIR→SPIR-V emit 资源绑定 = 当前结构上不可达**：`dxil_spirv::emit_spirv` 仅 emit `Location`/`BuiltIn`/`UserSemantic`；`mir::IoSigKind`（Builtin/Interpolate/Varying）与 `MirIoType`（Scalar/Vector）**无资源种类**，无法表达资源句柄（measured 代码现状，§11）。spike 用 Vulkan dxc producer + `[[vk::binding]]` **模拟** Rurix 应 emit 的 SPIR-V，**不等于** Rurix 已能产此 SPIR-V。「按 io_sig 顺序确定性 emit 资源绑定装饰」属实现 PR 待建面（条款先于实现）。
- **(assumed-2) RXS-0156 句柄 → SPIR-V opaque 资源类型 + 绑定装饰降级面**：RXS-0161 当前仅承诺 opaque 类型形态、纹理访问语义结构上不可达 → RX6013 升档（RFC-0004 §4.6(b)）。资源绑定降级面是本 RFC 待建，**未实测**。
- **(assumed-3) 推导出的 root signature 语义正确性 / device 真跑**：spike 只证「工具链能序列化给定 root signature」「不自动合成」；**Rurix 推导出的 root signature 是否与资源使用语义一致、能否被 D3D12 PSO 接受并 device 真跑出图，未验证**（属 G-G2-3 + UC-04/G2.4）。

### 4.4 strict-only 推导失败处置（设计面，承 P-01）

绑定布局推导失败 = **结构化编译期 6xxx 诊断**（无运行期 fallback、无静默降级，P-01）。推导失败类别（错误码段位 = §9 Q-Err 已裁：6xxx codegen 段，按实际最高空号续）：

- **超 root signature 上限**：D3D12 root signature 有 64 DWORD 上限（root constant 1 DWORD/个、root descriptor 2 DWORD/个、descriptor table 1 DWORD/个）——推导结果超限 → 6xxx。
- **资源种类不可映射**：着色阶段签名出现无法映射为 D3D12 绑定的资源构造 → 6xxx（承 RXS-0161 RX6013「不可映射构造」语义类，是否复用 = §9 Q-Err）。
- **register 冲突**：推导出的 register/space 分配冲突（同 class 同 space 同 index 重复）→ 6xxx。

校验输入 measured 可得：`PSV0` 资源绑定反射可程序化解析（§4.2 measured-2），译后可比对推导意图与实际容器绑定（类比 RFC-0004 §4.4 签名一致性校验门）。校验门形态/粒度随实现 PR。

### 4.5 🔒 禁区边界声明（owner 落笔，本 RFC 不落语义本体）

> **本子节为边界声明，AI 不落禁区内容。** 触及即标「需人工升档」。

- **(a) 签名/绑定二进制 ABI 布局（RFC-0004 §4.6(a) 同级，🔒）**：register/space 编号、component mask、packing、descriptor table 字节偏移、root parameter DWORD 物理布局、descriptor heap 编码——属 FFI ABI 二进制布局禁区。本 RFC 仅承诺**源码层资源句柄的存在性 / 种类 / 绑定关系的可推导性**，**不承诺**任何具体物理布局值，亦不把某工具版本的具体布局冻结为语言 stable 保证；不合规或不可验证 → 6xxx。
- **(b) 纹理路径内存模型映射（06 §4.2，🔒）**：descriptor 编码、采样/load/store opcode、缓存一致性、LOD/导数、越界采样后果、memory-order——未建模即拒绝（承 RFC-0004 §4.6(b)、RXS-0161 🔒）；本 RFC 仅触及资源句柄的**绑定布局**，不触及访问语义。
- **(c) DXIL/SPIR-V UB 边界（🔒）**：不建立独立于 Rurix 源码语义的 DXIL/SPIR-V UB/poison/undef 契约（承 RFC-0004 §4.6(c)）。

以上边界使本 RFC 可定义推导**设计面 + 推导判据 + 错误类别 + 条款计划**；register/space 物理布局、纹理访问语义、新 UB 空间均不进入本 RFC，须经 owner（独立 Full RFC / §9 裁决）落笔。

## 5. 下游 spec 条款计划表（spec diff，10 §3 要件；不落条款体）

落点 = 新建 **`spec/binding_layout.md`**（§9 Q-File 已裁）。**本 RFC 不创建 `### RXS-####` 裸条款头**——下表为条款新增的**计划表**，条款体随 PR-E2 实现 PR 同落（条款 PR 先于实现 PR，硬规则 7；trace 维持全锚定）。**区间大小/拆分 = 4 条 RXS-0163 ~ RXS-0166**（§9 Q-Range 已裁；当前最高现存 RXS-0162 @ `spec/dxil_backend.md`）。

| 条款（计划，Q-Range 已锁定） | 标题 | 测试锚定计划（每条 ≥1，`//@ spec`） |
|---|---|---|
| RXS-0163 | 资源句柄 → SPIR-V 资源绑定降级面（RXS-0156 句柄 → opaque 资源类型 + DescriptorSet/Binding 装饰，按 io_sig 顺序确定性分配） | conformance accept（合法资源句柄 → 确定性绑定装饰）+ reject（不可映射 → 6xxx）+ SPIR-V/golden |
| RXS-0164 | register/space 分配推导（SPIR-V (set,binding) → register(x#, spaceN)，分配轴策略按 Q-Space 裁定） | accept（确定性分配）+ reject（register 冲突 → 6xxx）+ golden |
| RXS-0165 | root signature 形态推导 + 容器 `RTS0` 序列化（descriptor table / root descriptor / root constant；static vs dynamic sampler，按 Q-RootShape/Q-Sampler 裁定） | accept（推导 → RTS0 + dxc validator）+ reject（超 64 DWORD 上限 → 6xxx）+ golden |
| RXS-0166 | 绑定布局推导一致性校验门 + strict-only 推导失败（PSV0 反射 vs 推导意图比对；不可推导/冲突/超限 → 6xxx，无 fallback） | accept（一致）+ reject（篡改推导 → 6xxx 真实红绿）+ 确定性核对 |

> 上表为 PR-E1 计划锚点，**PR-E1 仅登记预留区间，不落裸条款头**；条款体与每条 ≥1 测试锚定随 PR-E2 同落。

- **错误码策略（§9 Q-Err 已裁）**：推导失败归 **6xxx codegen 段**（只追加）。**不预留、不预造**：新可达类别随实现 PR 按**当时实际最高号续**分配 + en/zh message-key（`registry/error_codes.json` 只追加，`ci/bilingual_coverage.py` 覆盖）。当前 6xxx 段最高 = **RX6013**；RX6008/RX6009 已被 RD-012/RD-013 预引占用，不复用；**RX6014 在 `spec/dxil_backend.md` RXS-0160 IR3 作错链候选码被提及但 registry 未分配**，本 RFC 不预占 RX6014，避免与 RXS-0160 错链码归类争号。「资源种类不可映射」复用 RX6013；root signature 超 64 DWORD、register/layout 冲突、PSV0 反射与推导意图不一致等新真实可达类别新开码。
- spec 条款 PR 先于实现 PR（硬规则 7）；trace_matrix 维持全锚定（沿用全局 counter）。

## 6. feature gate / tracking / 实现序（10 §3 要件）

- **feature gate（§9 Q-Gate 已裁）**：复用 `dxil-backend`；绑定布局推导为 DXIL 图形分支的推导面，不新增 `binding-layout` 子 gate，避免暴露新的用户面组合维度。
- **栈式 PR（门控于本 RFC 批准 + §9 裁定后）**：
  - **PR-E1 spec 脚手架**：spec 落点（`spec/binding_layout.md`，Q-File 已裁）登记 RXS-0163~0166 预留区间（**不落裸条款头**）+ README §4/§5 同步；`trace_matrix --check` PASS（维持全锚定、零新 RXS、零裸条款头、零悬空锚点）。
  - **PR-E2 spec 条款体 + 推导实现**：资源句柄 → SPIR-V 资源绑定降级 + register/space 推导 + root signature 形态推导 + RTS0 序列化 + 一致性校验门 + golden + bless + 6xxx 错误码 + device 真跑（G-G2-3 / UC-04 G2.4）。
  - **CI 步骤**（推导冒烟 + 确定性 + validator gate + 推导一致性校验）随实现 PR 回填；device 真跑出图需 D3D12 环境。
- **真实红绿**（反 YAML-only）：篡改推导出的绑定布局 → golden/validator 红 → 复原绿，run URL 归档。
- **依赖与序**：本 RFC（绑定推导面）依赖 G2.1 类型面（RXS-0156 已就位）+ G2.2 B 链 codegen（RFC-0004 已就位），为 G2.4 UC-04 deferred 渲染器的绑定布局基座；可与 G2.2 剩余面部分并行。

## 7. 备选方案

- **编译器推导（本 RFC）vs 用户全手写 root signature**：采纳**编译器推导**为主路（P-11 单一事实源，06 §8.2 已锁方向）；用户全手写 root signature 否决为**默认**——违 P-11（host/shader 两份手维护、易漂移）。是否允许用户**标注覆盖**推导（`#[binding(...)]`）= §9 Q-Inference-vs-Explicit / Q-Space，不在本节定型。
- **推导 root signature（Rurix 序列化 RTS0）vs 依赖运行期反射 + 应用侧构造**：采纳**编译期推导 + RTS0 序列化**（measured：工具链不自动合成，§4.2 measured-2；编译期推导兑现 P-11 单一事实源）；纯运行期反射（应用从 PSV0 自行构造 root signature）否决——把单一事实源责任推给应用、违 P-11。
- **register/space 分配轴：单一全局 binding 轴 vs 按种类分轴**：两者 measured 都确定性、都产合规 DXIL（§4.2 measured-1）；§9 Q-Space 已裁为 **按资源种类分轴**（首期默认 `space0`，不开放显式标注）。
- **SPIR-V 作绑定中间表示 vs 自写资源绑定 IR**：复用 B 链既有 SPIR-V 装饰通道（RFC-0004），不另起资源绑定 IR；SPIR-V 在此仅作 B 路内部中间表示（≠ 对外通用目标，D-008/SG-003 维持）。
- **bindless / descriptor heap 直索引**：本期 **defer**，登记 RD-018（§9 Q-Bindless 已裁）；否决「本期即做完整 bindless」——范围蔓延风险（复杂度黑洞 SG-005）。

## 8. 不做（范围红线）

本 RFC 明确**不涉及**以下范围（防蔓延）：

- **推导 codegen 实现**：资源句柄 → SPIR-V 绑定降级、register/space 推导、root signature 形态推导、RTS0 序列化、一致性校验门、golden 产物均不在本 RFC（随 owner 批准后实现 PR，§6）；不动 `src/*`、不建 golden、不接线 codegen。
- **🔒 签名/绑定二进制 ABI 布局**（RFC-0004 §4.6(a) 同级）：register/space/mask/packing/descriptor table 偏移/root parameter DWORD 物理布局/descriptor heap 编码不在本 RFC（§4.5(a) 边界声明）；越出须 owner 落笔。
- **🔒 纹理路径内存模型映射**（06 §4.2 禁区）：descriptor 编码 / 采样 opcode / 缓存一致性 / LOD / UB 留独立 Full RFC（§4.5(b)）；本 RFC 仅触绑定布局、不触访问语义。
- **🔒 DXIL/SPIR-V UB 边界**：不建立独立后端 UB/poison/undef 契约（§4.5(c)）。
- **UC-04 deferred 渲染器**（G2.4）/ **PSO·资源状态·barrier 运行时面**：不在本 codegen 推导 RFC；本 RFC 是绑定推导面，device 真跑出图属 G-G2-3 + G2.4。
- **bindless / unbounded descriptor array 完整支持**：本期 **defer 至 RD-018**；不登记 SG-010 gating，不永久/条件裁剪该方向。
- **语言面扩展**：着色阶段类型面属 G2.1（RFC-0002 RXS-0156）；本 RFC 是 codegen 推导面，不新增语言构造（`#[binding(...)]` 显式标注是否引入 = §9 Q-Inference-vs-Explicit，若引入触新语法面、可能需回 RFC-0002 增补或本 RFC 升档，向上取严）。
- **多后端 / Python 原生嵌入 / 高级 GPU intrinsics / registry / VMM·多 GPU**：分别为 D-008/SG-003、红线 1/SG-008、SG-001/SG-002、D-312/SG-007、A-06，均 out_of_scope 不动。

## 9. §9 owner 裁决清单（Accepted / Owner Approved 2026-06-28）

> 以下为本 RFC 的**路径性抉择**。owner（Language Lead）于 2026-06-28 在本工作会话明确同意下表裁决，并批准 3bcc486 作为 RFC-0005 的 FCP-lite/owner 批准技术载体；Codex 代录，非 AI 代签。候选与 AI 倾向保留为审计上下文，裁决列为后续 PR-E1/PR-E2 的约束。所有裁决均不把 register/space 具体值、descriptor table 偏移、root parameter DWORD 物理布局冻结为 stable 语言保证（🔒 §4.5）。

| Q | 待裁项 | 候选（2–3） | AI 倾向（供参，不代决；取严默认） | 裁决 |
|---|---|---|---|---|
| **Q-Space** | register/space 分配策略 | (A) 按声明序单一全局 binding 轴打包（spirv-cross 默认，t/s/b/u 索引跨种类连号，measured `b0,t1,t2,s3,s4`）；(B) 按资源种类分轴（每类 t/s/u/b 各自从 0 计数，可选各占独立 space）；(C) 纯推导 + 开放 `#[binding(space=, register=)]` 显式标注覆盖 | 倾向 **(B) 按种类分轴**（更贴 D3D12 惯例与 root signature descriptor range 组织，host 侧 binding 直观）；**取严默认**：先不开放 (C) 显式标注（显式标注触新语法面 + 与 Q-Inference-vs-Explicit 耦合，向上取严留后期）。两轴 measured 都确定性，**不自填**，留 owner | **裁决 = B**：按资源种类分轴；首期默认单 set/`space0`，CBV/SRV/UAV/Sampler 分别映射到 `b/t/u/s` 轴并按声明序各自从 0 递增。多 space 策略与 `#[binding(...)]` 显式覆盖不进本期 |
| **Q-RootShape** | root signature 形态推导规则 | (A) 全 descriptor table（最简、最省 root DWORD，但每次绑定走 heap）；(B) CBV 走 root descriptor + SRV/UAV/Sampler 走 descriptor table（混合）；(C) 小常量走 root constant + 其余混合 | 倾向 **(B) 混合：CBV root descriptor + 其余 descriptor table**（兼顾常量缓冲低延迟与上限友好）；**取严默认**：root constant (C) 本期不自动推（需常量大小阈值策略，易过度工程），先不做、留后期。**不自填**，留 owner | **裁决 = B**：CBV 走 root descriptor；SRV/UAV 走 descriptor table；Sampler 走 sampler descriptor table。root constant 本期不自动推，后期按独立判档接入 |
| **Q-Sampler** | RXS-0156 `Sampler` 默认归 static 还是 dynamic sampler | (A) 默认 static sampler（编译期固定采样状态，省 descriptor，但采样参数不可运行期改）；(B) 默认 dynamic sampler（descriptor table/heap 绑定，运行期可改）；(C) 由句柄类型/标注区分 | 倾向 **(B) dynamic sampler 为默认**（语义最不意外、运行期可配置；static sampler 需采样状态在编译期已知，属优化而非默认）；**取严默认**：static sampler 作后期可选优化，不在本期默认推。**不自填**，留 owner | **裁决 = B**：`Sampler` 默认 dynamic sampler；static sampler 留后期优化/显式面，不作为本期默认推导 |
| **Q-Bindless** | bindless / unbounded descriptor array / descriptor heap 直索引 | (A) 本期 defer（登 RD-018，留后续里程碑）；(B) out_of_scope（登 SG-010 永久/条件 gating）；(C) 本期纳入 | 倾向 **(A) defer 登 RD-018**（bindless 是 SM6.6+ / resource heap 大面，本期绑定推导先收敛有界 descriptor；不永久裁剪以免堵死 UC-04 后续）；**取严默认**：不在本期纳入 (C)。**裁剪/ defer 抉择留 owner**，AI 草拟 RD-018 文案见本节末，不落 registry | **裁决 = A**：本期 defer，登记 **RD-018**；不登记 SG-010 gating，不永久/条件裁剪 bindless 方向；不纳入本期实现 |
| **Q-Gate** | feature gate | (A) 复用 `dxil-backend`（绑定推导为其图形分支推导面）；(B) 新开 `binding-layout` 子 gate | 倾向 **(A) 复用 `dxil-backend`**（避免暴露新用户面组合维度，对齐 RFC-0004 Q-Gate-B 复用先例）；**不自填**，留 owner | **裁决 = A**：复用 `dxil-backend`；不新增 `binding-layout` 子 gate |
| **Q-Err** | 推导失败错误码段位 + 类别 | 归 6xxx codegen 段，**按当时 registry 实际最高号续**（当前最高 RX6013；RX6008/RX6009 已被 RD-012/RD-013 预引，**RX6014 在 RXS-0160 IR3 作错链候选码被提及但 registry 未分配**——本 RFC 不预占）；类别 = 超 64 DWORD 上限 / 资源种类不可映射（复用 RX6013 vs 新开）/ register 冲突 | 倾向 **落码时按当时实际最高空号续、不预占 RX6014**（与 RXS-0160 错链码归类解耦，避免抢号）；「资源种类不可映射」**倾向复用 RX6013**（同语义类），超上限 / register 冲突 **倾向新开**；**不自填**，留 owner（与 RXS-0160 RX6011 复用/RX6014 新开裁决一并看更佳） | **裁决**：推导失败归 6xxx codegen 段；实现落码时按 registry 实际最高空号续，不预占 RX6014。「资源种类不可映射」复用 RX6013；root signature 超 64 DWORD、register/layout 冲突、PSV0 反射与推导意图不一致等新真实可达类别新开码 |
| **Q-File** | spec 落点 | (A) 延伸 `spec/dxil_backend.md`（绑定推导承 DXIL 后端语义面，RXS 续号同文件）；(B) 新建 `spec/binding_layout.md`（绑定布局独立语义面文件） | 倾向 **(B) 新建 `spec/binding_layout.md`**（绑定布局是独立语义面、可与 G2.4 PSO/资源状态承接，避免 dxil_backend.md 过载；对齐 shader_stages.md 独立成文先例）；**不自填**，留 owner | **裁决 = B**：新建 `spec/binding_layout.md`；`spec/dxil_backend.md` 仅交叉引用，不继续承载绑定布局条款 |
| **Q-Range** | RXS 区间大小 / 拆分 | §5 计划表 AI 建议 4 条 RXS-0163~0166（资源绑定降级面 / register-space 推导 / root signature 形态+RTS0 / 一致性校验门+strict-only）；可并条或增条 | 倾向 **4 条 RXS-0163~0166**（与推导面四阶段对齐）；区间**不预占**、不预造，条款体落地时按当时最高号续；**不自填**，留 owner | **裁决**：锁 4 条计划映射 **RXS-0163 ~ RXS-0166**；PR-E1 只登记预留区间，不落裸条款头；条款体与锚定测试随 PR-E2 同落 |
| **Q-Inference-vs-Explicit** | 纯推导 vs 允许用户标注覆盖 | (A) 纯推导（P-11 推导优先，本期不开放覆盖）；(B) 推导优先 + 允许 `#[binding(...)]` 显式覆盖（本期纳入）；(C) 推导优先 + 覆盖留后期 | 倾向 **(C) 推导优先、覆盖留后期**（P-11 推导优先；显式覆盖触新语法面 + 与 Q-Space (C) 耦合，向上取严不在本期定型）；**取严默认**：本期纯推导，覆盖作后期独立判档。**不自填**，留 owner | **裁决 = C**：推导优先，显式覆盖留后期独立判档；本期执行效果等同纯推导，不开放 `#[binding(...)]` |

**registry 处置（owner 2026-06-28 裁决）**：

- **RD-018**：Q-Bindless 裁 defer 后登记于 `registry/deferred.json`，title「bindless / unbounded descriptor array / descriptor heap 直索引绑定推导」；status `open`；owner_milestone `G2`。
- **SG-010**：本裁决不登记。bindless 不是永久/条件裁剪方向，本期仅延期。

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：绑定布局推导经 `dxil-backend` feature gate（§9 Q-Gate）→ tracking → 两里程碑无重大修订 → stabilization report → FCP-lite（10 §2.2）；推导产物面/root signature 形态在首个 stable 前不进 stable 面（随 RD-008）。register/space/packing 物理布局不进 stable 语言保证（§4.5(a) 🔒）。
- **Provenance**：`Assisted-by: kiro:claude-opus-4-8`（Draft）+ `Assisted-by: codex:gpt-5`（owner 裁决落文档）。本草案由 AI 起草；§4 measured 锚定来自隔离分支 `spike/g2.3-binding-layout` 真实命令输出（§11）；§4.5 🔒 禁区维持边界声明不落语义本体；§9 路径抉择由 owner（Language Lead）于 2026-06-28 裁决，Codex 代录，非 AI 代签/代决。下游 spec/实现 PR 按硬规则 7 序进（spec 先于实现）。

## 11. 规范与实现依据

- **证据（measured，命令真实输出）**：`evidence/g2.3-binding-layout/binding_layout_spike_20260627.json`（schema `g2.3-binding-layout-spike/v1`，`status: MEASURED`，4/4 语料 chain 全通：SPIR-V (set,binding) → spirv-cross register 映射确定性可预测、dxc 不自动合成 root signature（默认无 RTS0 / 仅显式 `[RootSignature]` 产 RTS0）、PSV0 恒产资源绑定反射、确定性 SHA256 全等）+ `evidence/g2.3-binding-layout/binding_layout_spike_report.md`（round-1 报告，measured/assumed 严格分栏）。spike 探针 `spike/g2.3-binding-layout/probe_binding_layout.py` + `corpus/*.hlsl`（隔离分支，不入 src/）。
- **工具链版本（取证实测，隔离不入库）**：dxc signed pin 1.9.2602.24（d355aa836，`H:\dxc-round7\extracted\bin\x64`）/ spirv-cross vulkan-sdk-1.3.290.0-44-g65d73934 / dxc -spirv producer 1.8.0.4739 / spirv-dis（Vulkan SDK 1.3.296.0）。
- **代码现状（measured，assumed 锚点）**：`src/rurixc/src/dxil_spirv.rs`（`emit_spirv` 仅 emit Location/BuiltIn/UserSemantic，无资源绑定装饰）/ `src/rurixc/src/mir.rs`（`IoSigKind`/`MirIoType` 无资源种类）→ Rurix MIR→SPIR-V emit 资源绑定当前结构上不可达（§4.3 assumed-1）。
- **决策/上游**：06 §8.2（descriptor / root signature 编译器推导,P-11 设计预留）· 06 §4.2（纹理禁区,🔒）· 04 P-11（单一事实源）/ P-01（strict-only）/ P-13 · D-002（图形分期）· RFC-0002（着色阶段类型面,RXS-0156 资源句柄）· RFC-0004（图形=B codegen + §4.6 禁区边界:Q-ABI-B/Q-Texture-B/Q-UB-B）· `spec/dxil_backend.md`（RXS-0157~0162,B 链）· `spec/shader_stages.md`（RXS-0153~0156,类型面）。
- **registry**：6xxx 段当前最高 RX6013（RX6008/RX6009 RD-012/RD-013 预引；RX6014 RXS-0160 错链候选未分配,本 RFC 不预占,§5/§9 Q-Err）· RD-018 已按 Q-Bindless 裁决登记（bindless defer）；SG-010 不登记。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-06-27 | AI 起草骨架（§1 摘要绑定推导通路图 / §2 动机 + 为何 Full RFC / §3 用户视角资源句柄声明透明 + 拟议开放问题不定型 / §4.1 推导面在 B 链位置 / §4.2 measured 可行性（隔离 spike：register/space 映射确定性、dxc 不自动合成 root signature、确定性全等）/ §4.3 assumed 严格分栏（Rurix MIR→SPIR-V emit 资源绑定当前结构上不可达）/ §4.4 strict-only 推导失败处置 / §4.5 🔒 禁区边界声明（签名·绑定二进制 ABI 布局 / 纹理内存模型 / UB）/ §5 下游条款计划表 RXS-0163~ 不落条款体不预占 / §6 feature gate + 栈式 PR + 真实红绿 / §7 备选 / §8 范围红线 / §9 Q-Range owner 待裁清单（Q-Space/Q-RootShape/Q-Sampler/Q-Bindless/Q-Gate/Q-Err/Q-File/Q-Range/Q-Inference-vs-Explicit 各候选+AI 倾向+取严默认，全部〈待 owner〉；RD-018/SG-010 草拟不落 registry）/ §10 稳定化 / §11 依据 + measured 证据指针）。**状态 Draft / Awaiting Owner；§9 全部抉择留 owner FCP-lite；AI 不代签 / 不代决 / 不自填默认 / 不自判 Direct / 不推进下游；🔒 禁区只引边界声明不落语义本体；spike 隔离不入 src/、不落 codegen、不落条款体、不动 registry** | Full RFC（Draft） |
| Owner approval | 2026-06-28 | owner（Language Lead）在本工作会话同意 RFC-0005 全文并裁决 §9 全部路径项：Q-Space=B（按资源种类分轴，首期默认 `space0`，不开放显式标注）/ Q-RootShape=B（CBV root descriptor + SRV/UAV/Sampler descriptor table，root constant 后期）/ Q-Sampler=B（dynamic sampler 默认）/ Q-Bindless=A（defer，登记 RD-018，不开 SG-010）/ Q-Gate=A（复用 `dxil-backend`）/ Q-Err=6xxx codegen 段按实际最高空号续、不预占 RX6014、不可映射复用 RX6013、超限/冲突/PSV0 mismatch 新开 / Q-File=B（新建 `spec/binding_layout.md`）/ Q-Range=4 条 RXS-0163~0166 / Q-Inference-vs-Explicit=C（推导优先，覆盖后期独立判档）。owner 同意 3bcc486 给 FCP-lite/owner 批准；Codex 代录，非 AI 代签。状态翻 Accepted / Owner Approved；下游 PR-E1 spec 脚手架解锁（仍不落条款体、不接线实现、不动禁区语义本体） | Full RFC（Owner Approved） |
