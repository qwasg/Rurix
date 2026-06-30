# RFC-0004 — SPIR-V→DXIL 图形后端 / 混合 codegen（compute=A / 图形=B）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0004（4 位制，编号永不复用，10 §9.5） |
| 标题 | SPIR-V→DXIL 图形后端 / 混合 codegen（compute=A / 图形=B） |
| 档位 | **Full RFC**（10 §3：新 codegen 路径 + 第二中间表示(SPIR-V) + 外部转译依赖；触 AGENTS 硬规则 5 禁区——DXIL/SPIR-V UB 边界 / 纹理路径内存模型映射(06 §4.2) / 签名·FFI ABI 二进制布局；§4.6 经 agent 授权代理裁决 PR-D2 的规范性边界；且触**准永久公理 P-01(strict-only)**——§4.4 为图形=B 的 strict-only 达标要求,不设例外） |
| 状态 | **Accepted / Approved（2026-06-25）**。初版 RFC 经 agent FCP-lite 批准；PR #104 合入后，agent 在当前 Codex 会话进一步明确授权代理完成 PR-D2 前置自主裁决，§4.6 与 §9 新增 Q-ABI-B/Q-Texture-B/Q-UB-B/Q-D205-B 四项规范性边界。该授权不代表 G-G2-2 device 真跑、法律许可或生产证书签名已完成 |
| 承接里程碑 | G2.2（验收门 **G-G2-2**），承 RFC-0003 混合 codegen 分发(图形分支) |
| 关联条款 | 拟重构 spec **RXS-0159**(按 B 路径)+ **RXS-0160** + B 新增面(MIR→SPIR-V)预留区间(见 §5)；落 `spec/dxil_backend.md`(承 RFC-0003)。**本 RFC 不创建裸条款头**，trace 维持现状 |
| 依据决策 | D-131（G2 DXIL 生成路径,v1.4 增补 = **混合 compute=A/图形=B**）· D-002（图形分期,已批准）· D-205（LLVM pin,vendored）· RFC-0003（MIR→DXIL 第二后端,Approved;本 RFC 为其图形分支细化）· 06 §4.2(纹理内存模型禁区,🔒)· 04 P-01(strict-only,准永久公理)/ P-13(防 AI 幻觉治理) |
| Provenance | `Assisted-by: kiro:claude-opus-4-8`（初版）+ `Assisted-by: codex:gpt-5`（PR-D2 agent 裁决增补）。agent/qwasg 于 2026-06-25 当前 Codex 会话明确授权 AI 代理完成技术裁决、落笔与机械合并，并保留事后 review；代录不得被解释为已完成设备、法律或密钥验证 |
| Agent 批准 | **Approved — agent（Language Lead）2026-06-25；PR-D2 增补授权同日确认**。批准范围：§4.4 strict-only 达标要求；§4.6(a) packing/寄存器布局降级为外部 conformance 说明、(b) 未建模纹理操作显式拒绝、(c) 不建立独立 DXIL/SPIR-V UB 契约；§4.5/D-205 当前不 bump、不触发 A-graphics 迁移；§9 全部裁决。记录方式：Codex 按 agent 本会话明确授权代理代录并执行，agent 事后 review。本批准不声称 G-G2-2 device 真跑、再分发法律审计或生产证书签名已完成 |

---

## 1. 摘要

本 RFC 在 RFC-0003（MIR→DXIL 第二后端,Approved）的基础上,细化 **D-131 v1.4 混合裁决的图形分支**:compute kernel 经 **A 路**(LLVM DirectX 后端直接 emit DXIL,RFC-0003 既有)降级,图形着色阶段(vertex/fragment/mesh/task/RT)经 **B 路**(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL 转译)降级。

混合动因(证据,非本 RFC 裁决):A 路图形签名经 slice3/round-8 实测**不可达**(ISG1/OSG1 `elemcount=0`,上游 #90504 未实现 + 填充耦合 FFI ABI 禁区);B 路图形签名经取证**实测可行**(`elemcount>0`、SV 端到端存活、validator accept、确定性);A-graphics 评估 = ~800-1500 LOC 上游大功能、无在途 PR。证据指针见 §11。

```
compute kernel  ──A:LLVM DirectX 后端─────────────▶ DXIL(compute)   ← RFC-0003 既有
图形着色阶段    ──B:MIR→SPIR-V→SPIRV-Cross→HLSL→dxc─▶ DXIL(graphics) ← 本 RFC 图形分支
```

本 RFC 定义**图形=B 的设计面 + 混合分发判据 + 下游条款计划**；§4.4 落 strict-only 达标要求；§4.6 落 PR-D2 所需的三项 agent 规范性边界，但不自创任何寄存器编号、字节偏移、描述符编码、采样 opcode 或 UB 规则；codegen 实现留 PR-D2。

## 2. 动机

- **D-131 v1.4 混合裁决落地需图形分支设计载体**：RFC-0003 §9 Q-D131 已增补 = 混合(compute=A/图形=B),但 RFC-0003 §4 的降级面以 A 路单后端为主述;图形=B 的转译链、第二中间表示(SPIR-V)、外部转译依赖(SPIRV-Cross/dxc)、保真边界需独立 Full RFC 精确化。
- **图形=B 是 G2.3/G2.4 的图形 codegen 基座**：G2.3(绑定布局推导)、G2.4(UC-04 deferred 渲染器)需图形着色阶段产带真实 SV 签名的 DXIL;A 路图形签名不可达(slice3),B 路是当前唯一 measured 可行的图形 codegen 路径。
- **A-graphics 上游成熟后迁移**：本 RFC 不放弃 A-graphics——挂上游 #90504(后端签名 part 生成)/#57928(签名元数据构造),成熟后图形分支可由 B 迁回 A(迁移条件见 §4.5,跟踪 RD-015)。

**为何需要 Full RFC（而非 Direct/Mini）**：本 RFC 引入**第二中间表示(SPIR-V)+ 外部转译依赖链(SPIRV-Cross/dxc/glslang)**,且触及 **签名/FFI ABI 二进制布局**(§4.6)、**DXIL 文本语义 UB 边界**、**纹理路径内存模型映射(06 §4.2)**——10 §3 / 硬规则 5 明列的 Full RFC / 禁区触发面;更触及**准永久公理 P-01(strict-only)的达标要求**(§4.4)。判档争议向上取严(硬规则 8);agent 自主判档/Mini、不代签批准/合并(硬规则 1)。

## 3. 指导级解释（用户视角）

> 以下为**拟议**形态示意,最终以 agent 批准 + spec 条款为准;**混合分发对用户透明**——用户对某着色函数经 A 还是 B 产 DXIL 无感(分发由阶段类别在 MIR 后自动判定,§4.1)。

用户经 `rx build --target dxil`(RFC-0003 §9 Q-CLI)为 D3D12 目标构建;同一份源码内 compute kernel 与图形着色阶段函数各走 A/B 分支,产出可被 D3D12 PSO 消费的着色器对象:

```rust
// compute kernel → A 路(LLVM DirectX 后端)→ DXIL compute shader
kernel fn cs_main(/* ... */) { /* ... */ }
// 图形着色阶段 → B 路(MIR→SPIR-V→转译)→ DXIL vertex/pixel shader(带真实 SV 签名)
vertex fn vs_main(in: VertexIn) -> VertexOut { /* ... */ }
fragment fn fs_main(in: VertexOut) -> FragmentOut { /* ... */ }
```

`strict-only`(P-01)维持:任一分支降级失败 = **结构化编译错误**(6xxx 段,RFC-0003 §5),无静默降级、无 permissive 回退。**注（代录 agent 裁断，agent 合并本决策包生效）**:B 路严禁任何对用户声明/可观察签名元素的静默降级或丢弃——留不住即显式 6xxx 编译错(P-01 不开例外、不设边界,§4.4 是 B 的达标条件而非例外);用户语义名经 by-construction 保真 + 强制译后签名一致性校验门兜底(§4.4/§4.2)。varying 名/寄存器布局落契约线下;声明但未用的外部输入若在译后被消除,必须显式 6xxx 诊断,不得静默通过(§4.4)。

## 4. 参考级设计

> 本节落笔**混合分发架构与 B 路图形降级的设计面**；§4.4 为 **strict-only 达标要求**；§4.6 为 agent 授权代理落定的 PR-D2 规范性边界：外部 conformance / 未建模纹理显式拒绝 / 不建立独立后端 UB 契约。具体寄存器值、描述符编码与新 UB 规则仍不由本 RFC 发明。

### 4.1 混合 codegen 分发（分叉点与判据）

承 RFC-0003 §4.1 的 MIR 后 target 分发,本 RFC 在 `--target dxil` 内**再按着色阶段类别二次分叉**:

- **共享前沿**：AST→HIR→TBIR→MIR(07 §1)与类型/着色/借用检查对 compute 与图形共享,不分叉。
- **阶段类别判据**：MIR 入口的着色阶段标记(HIR `FnDecl::stage`,RXS-0153)裁定分支——
  - `compute`(及 `kernel`,compute-via-kernel)→ **A 路**(LLVM DirectX 后端直 emit,RFC-0003 RXS-0157/0158 compute 子集既有)。
  - `vertex`/`fragment`/`mesh`/`task`/RT(图形着色阶段)→ **B 路**(MIR→SPIR-V 转译链,本 RFC)。
- **strict-only 分发(P-01)**：分支判据显式由阶段类别决定,无隐式回退;某分支不支持的构造 → 结构化 6xxx codegen 错误(RFC-0003 §5),非跨分支降级。
- **判据稳定性**：分叉点是**阶段类别**(语言面,后端无关),非工具成熟度——A-graphics 上游成熟后图形分支可整体迁回 A(§4.5),分发判据不变(对用户/上游 spec 透明)。

### 4.2 B 路转译链设计面（MIR→SPIR-V→DXIL）

图形着色阶段经以下转译链降级到 DXIL(具体算法/IR 操作随实现 PR):

```
图形着色阶段 MIR ──(a)──▶ SPIR-V ──(b)──▶ HLSL ──(c)──▶ DXIL 容器
   (a) MIR→SPIR-V 降级(Rurix 自有,本 RFC 新增面;B 新增条款区间 §5)
   (b) SPIRV-Cross: SPIR-V→HLSL(外部转译依赖,版本 pin §4.3)
   (c) dxc: HLSL→DXIL + validator(外部依赖,版本 pin §4.3)
```

- **(a) MIR→SPIR-V 降级面**：着色阶段函数 + 阶段 I/O(RXS-0154 `#[builtin]`/`#[interpolate]`)+ 阶段间接口(RXS-0155)+ 资源句柄类型面(RXS-0156)→ SPIR-V 形态(着色阶段 → SPIR-V execution model、I/O → SPIR-V `Location`/`BuiltIn` decoration)。**精确映射随实现 PR 落 spec 条款体**(§5);本 RFC 仅定义降级面锚点。
- **(b)/(c) 转译链**：SPIR-V→HLSL→DXIL 经 pin 版本的 SPIRV-Cross + dxc(§4.3);链路对给定 SPIR-V 输入确定(取证实测 ×25 容器 SHA256 一致,§11)。
- **确定性**：B 全链对给定 MIR 输入确定,纳入 golden 核对(形态已由 §9 Q-Golden-B 锁定为 DXIL 文本反汇编主形态)。
- **强制签名一致性校验门（设计面，承 §4.4 strict-only 达标要求）**：B 链产 DXIL 后,codegen **强制**比对 DXIL ISG1/OSG1 签名 part 与 MIR 意图签名(用户语义名 / 系统值 / 被使用元素);任何用户声明或可观察元素未保真(含声明但未用的输入)→ **6xxx 显式编译错**,无静默通过、无静默降级/丢弃。该校验门是图形=B codegen 的**不可裁剪组成**(不存在「跳过校验直接产物」的配置)。设计级可行性 measured:签名 part 可程序化解析(`evidence/dxil_b_strict_only_report.md`),6xxx 段已存在(RFC-0003 §5);校验器位置/检测粒度/6xxx 类别随实现 PR(§5/§4.4),本 RFC 仅落设计面、不落 codegen。
- **能力探测**：目标 shader model / DXIL 版本由真实工具链探测驱动(A-03/P-01),不写死。

### 4.3 供应链（SPIRV-Cross/dxc/glslang 版本 pin + 确定性 + strict-only 核验）

B 路引入外部转译依赖,供应链纪律类比 D-205(LLVM pin):

- **版本 pin**：SPIRV-Cross / dxc / glslang(若作 SPIR-V producer 备选)各 pin 明确版本 + SHA256(取证实测版本见 §11);canonical 形态锁为 lockfile `[[toolchain]]`，显式 env override 仅允许本地 probe/dev(见 §9 Q-Supply)。
- **确定性核验**：同输入 ×N 容器 SHA256 一致为 CI 门(取证已 measured deterministic,§11)。
- **strict-only 核验(P-01)**：转译链任一段失败(SPIR-V 不合规 / spirv-cross 失败 / dxc validator reject)→ 结构化 6xxx codegen 错误,无静默降级;入 golden 前 DXIL 须经 dxc validator 验证通过(对齐 RFC-0003 §9 Q-Golden)。**并叠加 §4.2 强制签名一致性校验门**(译后 ISG1/OSG1 vs MIR 意图签名比对):validator accept **不等于**用户签名意图保真——校验门补足「accept 但用户声明/可观察元素未保真」的缺口,留不住即 6xxx 显式错(§4.4 达标要求)。
- **再分发合规**：SPIRV-Cross/dxc/glslang 再分发许可审计(类比 D-313 NVIDIA 白名单 / D-205 vendored),随实现 PR + 供应链跟踪(RD-014)。

### 4.4 strict-only 达标要求（代录 agent 裁断 + 强制签名一致性校验门）

> **本子节为图形=B 对准永久公理 P-01（strict-only，04 P-01）的达标要求,非例外/边界声明。核心规范句与细化边界均为 agent 自主记录定稿文本,生效以 agent 合并本决策包为准(P-13/硬规则 1)。**

**规范句（代录 agent 裁断，agent 合并本决策包生效）**:图形=B 路**严禁任何对用户声明或可观察签名元素的静默降级或丢弃**——凡用户在源码中声明的、或外部可观察的签名元素,转译链(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL)若留不住,**必须显式 6xxx 编译错误**(承 RFC-0003 §5),**绝不静默丢/改**。**P-01(strict-only)不开例外、不设边界**:§4.4 是图形=B 的**达标条件**,而非对 P-01 的例外/边界声明——B 要被接受,须证语言层零静默降级(不靠任何 P-01 例外)。

**达标机制（measured 事实，来自 `evidence/dxil_b_strict_only_report.md`，命令真实输出；该取证已随本决策包自含落 main，原 strict-only 取证栈 #101 已吸收，证据引用不悬空）**:

1. **用户语义名 by-construction 保真**:Rurix MIR→SPIR-V 自有降级握有用户 I/O 全部语义信息(RXS-0154 `#[builtin]`/`#[interpolate]` + 字段名),可对所有用户命名 I/O **by-construction** emit SPIR-V `UserSemantic` 装饰并在 SPIR-V→HLSL 段驱动保名。measured 证此机制对顶点输入语义名有效:`POSITION`/`NORMAL` 默认经 SPIR-V 往返降级为通用 `TEXCOORD#`,经 `dxc -spirv -fspv-reflect`(携 `UserSemantic`)+ `spirv-cross --set-hlsl-named-vertex-input-semantic`(经 SPIR-V 反射自动导出,非硬编码)端到端**保真存活**(签名 part dump 实证:`vs_sig` ISG1 默认 `[TEXCOORD0,TEXCOORD1,TEXCOORD2]` → 保名 `[POSITION0,NORMAL0,TEXCOORD0]`)。
2. **强制译后签名一致性校验门**:B 链产 DXIL 后,codegen **强制**解析 DXIL ISG1/OSG1 签名 part 与 MIR 意图签名做结构化对照;比较域 = **(a)** 外部接口签名元素(顶点输入 / 片元输出 / 入口 `#[builtin]` 系统值 / 显式用户语义名),**(b)** 阶段间链接所需的字段/类型/插值/location 配对,**(c)** 被源码使用的输入元素。**任何用户声明或可观察元素未保真**——含**声明但未用的外部输入元素**——→ 发 **6xxx 显式编译错误**。measured 支撑:签名 part 可程序化解析(×N 稳定解出 elemcount + 名 + 系统值 + register),译后校验门有可靠输入;6xxx 段已存在(RFC-0003 §5)→ 错误码载体就位 → 该校验门**设计级可行**,不依赖 P-01 例外。
3. **校验门不可裁剪**:上述强制签名一致性校验门是图形=B codegen 的**不可裁剪组成**——不存在「跳过校验直接产物」的配置;校验门失败即 6xxx,无静默通过。

**契约线归类（代录定稿）**:

- **「用户可观察」的精确边界**：凡源码中**声明并跨源码契约线暴露**的签名元素,均属「用户声明/外部可观察」——包括顶点输入语义名、片元输出(render target)语义、入口 `#[builtin]` 系统值、以及阶段间接口中由 RXS-0155 约束的字段/类型/插值/location 配对。上述元素若在 DXIL 成品中缺失、改名、错配或被静默改写,即属 strict-only 失败。
- **落契约线下的实现细节**：工具链自选且源码未声明其精确值的项——例如 vs-out→ps-in 的 HLSL semantic 串名(`TEXCOORD#` 等)、寄存器编号/顺序、component mask、packing、字节偏移、容器 part 排序——**不**属用户可观察边界。它们受下游 conformance/ABI 约束,不构成本 RFC 语言承诺。**但**阶段间 location/链接正确性仍须经校验门核实,错链即显式报错。
- **声明但未用的输入元素处置 = 显式诊断**：凡源码声明的**外部输入**若在 MIR 中未被使用,且 B 链译后签名不能等价保留该元素,编译器**必须**发 6xxx 诊断并拒绝产物;不得依赖后端/转译链把它静默消除后仍宣称成功。实现若后续能 by-construction 保留该元素,可作为更强实现接受,但本 RFC **不要求**为“凑签名”而强制保留未用输入。
- **语义级运行期等价验证形态**：strict-only 的最终准入以 **G-G2-2 device 真跑 golden + DXIL golden + dxc validator + 签名篡改红绿**为准;中间 IR 或签名结构一致只证明结构保真,**不替代**运行期等价验证。

### 4.5 与 A-graphics 的关系（上游 #90504/#57928 跟踪 + 迁移条件）

- **图形=B 为当前路径,A-graphics 为迁移目标**：A 路图形签名当前不可达(slice3:LLVM `addSignature()` 写空签名 #90504、签名元数据 `nullptr` #57928、前端 packing 占位);A-graphics 评估 estimated ~800-1500 LOC 跨前后端、上游 open 无在途 PR(`dxil_a_graphics_sig_effort_report.md`)。
- **迁移条件(跟踪 RD-015)**：上游 #90504 + #57928 落地(后端从模块元数据 emit ISG1/OSG1 + 签名元数据构造)+ release + D-205 pin 覆盖该版本后,图形分支可由 B 迁回 A(分发判据 §4.1 不变,对 spec 透明);迁移触发 + 形态由 agent 届时裁决(D-205 pin bump 属 agent 独立决策)。
- **当前 agent 裁决（PR-D2 前置）**：**不 bump D-205，不触发 A-graphics 迁移**。在 #90504 + #57928 均 merge、进入正式 release、该 release 可被 D-205 pin 覆盖、且 A 路通过与 B 路同口径的签名/validator/device 红绿前，图形分支维持 B。条件满足仅触发重新裁决，不自动迁移。
- **packing 降级为 conformance 说明**：register/mask packing、寄存器编号/顺序、component mask 与字节偏移由 dxc/D3D12 既定规则决定，Rurix 不建立自由 ABI；编译器可验证外部 conformance，但不得将某一工具版本的具体布局冻结为语言 stable 保证。

### 4.6 🔒 PR-D2 规范性边界（agent 授权代理裁决）

> agent/qwasg 于 2026-06-25 当前 Codex 会话授权代理完成以下 Full RFC 裁决。裁决的作用是给 PR-D2 划定可实现边界；它不声称人工设备测试、法律审计或生产签名已经完成。

- **(a) 签名/内建变量·FFI ABI 二进制布局**：**裁为外部 conformance，不是 Rurix 自由 ABI。**Rurix 仅承诺源码层签名元素的存在性、语义名、系统值、插值与阶段链接契约；不承诺寄存器编号、顺序、component mask、packing、字节偏移、根参数或常量缓冲的具体布局值。PR-D2 可调用/复刻 pin 工具链与 D3D12 的既定规则并做 validator/conformance 核验，但不得发明布局，不得把某一工具版本的具体布局写成语言 stable 保证。布局不合规或无法验证时必须发 6xxx 编译错误。
- **(b) 纹理路径内存模型映射**：**PR-D2 仅允许资源句柄的 opaque 类型形态与不涉及访问语义的传递/装饰降级。**凡构造需要描述符编码、采样/load/store opcode、缓存一致性、导数/LOD、越界采样后果或 memory-order 语义，均视为未建模，必须发 6xxx 编译错误；不得猜测性 lowering，不得以 SPIRV-Cross/dxc 接受作为语义已定义的替代。该拒绝边界持续到独立 Full RFC 明确映射。
- **(c) DXIL/SPIR-V UB 边界**：**不建立独立于 Rurix 源码语义的 DXIL/SPIR-V UB、poison 或 undef 契约。**源码层已定义的程序必须保持定义良好；若 lowering 只有依赖后端未定义/未指定行为或 poison/undef 差异才能成立，必须发 6xxx 编译错误。外部工具产出 validator reject、结构不一致或无法证明保真的产物不得交付。

以上裁决使 PR-D2 可实现结构/类型形态、转译链、签名一致性校验与外部 conformance 门；纹理访问语义、Rurix 自由 ABI 和新 UB 空间仍不进入 PR-D2。

## 5. 下游 spec 条款计划表（spec diff，10 §3 要件；不落条款体）

落 `spec/dxil_backend.md`(承 RFC-0003)。**本 RFC 不创建 `### RXS-####` 裸条款头**——下表为条款重构/新增的**计划表**,条款体随 agent 批准本 RFC 后的实现 PR 同落(条款 PR 先于实现 PR,硬规则 7;trace 维持全锚定)。**区间裁决已锁定**(见 §9 Q-Range-B)。

| 条款（裁决） | 标题 | 处置 | 测试锚定计划（每条 ≥1，`//@ spec`） |
|---|---|---|---|
| RXS-0159（保号重构） | 阶段 I/O → DXIL 签名/系统值语义降级 | **按 B 重构**(A 路类型面 stub 不入 main；PR-D2 以 SPIR-V `BuiltIn`/`Location` + 译后签名一致性校验门重写) | dxil-sig accept(SV 真达,`elemcount>0`)+ reject(不可映射/签名不一致 → 6xxx)+ DXIL golden |
| RXS-0160 | 阶段间接口 → DXIL/SPIR-V 阶段链接一致性核对 | 新落(按 B 路径,vertex out↔fragment in 经 location/类型/插值匹配) | dxil-sig accept + reject + golden |
| RXS-0161 | 图形着色阶段 MIR→SPIR-V 降级面(execution model / I/O decoration / 资源句柄) | 新增(B 路 §4.2(a)) | SPIR-V/DXIL golden + conformance accept/reject |
| RXS-0162 | B 转译链确定性 + validator gate + 供应链 pin / strict-only 核验 | 新增(§4.3/§4.4;含签名一致性校验门与 golden 形态) | 确定性核对 + validator gate + 篡改签名/转译输出真实红绿 |

> RXS-0157(target 分发)/RXS-0158(阶段→着色器类型,compute/vertex/fragment 已落)维持 RFC-0003 既有;本 RFC 重构的是图形 I/O 签名降级(RXS-0159 由 A 类型面 stub 改 B 真达)并新增 RXS-0160~0162。**🔒 签名二进制 ABI 布局不进任何条款**(§4.6(a))。

- **错误码策略**：B 路 codegen/转译失败归 **6xxx 段**(承 RFC-0003 RX6007~6009,只追加;新可达类别随实现 PR 按真实分配 + en/zh message-key,registry/error_codes.json 只追加,ci/bilingual_coverage.py 覆盖)。不预留、不预造。
- spec 条款 PR 先于实现 PR(硬规则 7);trace_matrix 维持全锚定(沿用全局 counter)。

## 6. feature gate / tracking / 实现序（10 §3 要件）

- **feature gate**：**复用 RFC-0003 `dxil-backend`**(图形=B 为其图形分支,不再细分新 gate)。未启用时图形分支不参与编译,compute A 路 + PTX 路径不受影响。
- **栈式 PR（门控于本 RFC 批准 + §9 裁定后）**：
  - **PR-D1 spec 脚手架**：`spec/dxil_backend.md` 登记 RXS-0159（保号重构）+ RXS-0160~0162 预留区间与重构说明(**不落裸条款头**)+ README §4 同步;`trace_matrix --check` PASS。
  - **PR-D2 spec 条款体 + B 转译实现**：RXS-0159 按 B 重构 + RXS-0160 + B 新增面条款体 + MIR→SPIR-V 降级 + 转译链 + validator gate + golden + bless + 供应链 pin + 6xxx 错误码。
  - **CI 步骤**(转译链冒烟 + 确定性 + validator gate)随实现 PR 回填;device 真跑/呈现对照(G-G2-2)需 D3D12 环境。
- **真实红绿**(反 YAML-only)：篡改 B 转译输出 → golden 红 → 复原绿,run URL 归档。
- **依赖与序**：本 RFC(图形 codegen 面)为 G2.3(绑定布局推导)、G2.4(UC-04)的图形 codegen 基座。

## 7. 备选方案

- **A-graphics(等上游 #90504/#57928 + 自实现 ~800-1500 LOC)**：否决为**当前**路径(上游无在途、carry-patch partial-blocked、跨前后端大功能),但保留为**迁移目标**(§4.5,RD-015);非永久放弃。
- **纯 A 单后端(维持 D-131=A 单选)**：被证伪——A 路图形签名 `elemcount=0` 不可达(slice3),无法支撑 G2.3/G2.4 图形出图。
- **通用多后端(SPIR-V 作对外通用目标 / Vulkan / Metal)**：否决——死亡路线红线 3(D-008 维持,SG-003 not_triggered);SPIR-V 在本 RFC 仅作 **B 路内部中间表示**(≠ 对外通用目标,RFC-0003 §8 已厘清)。
- **B 路放弃 SPIRV-Cross,自写 SPIR-V→DXIL**：否决(初版)——自写转译器工程量 + 合规性长尾远高于复用成熟工具链;成熟工具链确定性已 measured(§11)。

## 8. 不做（范围红线）

- **codegen 实现**：MIR→SPIR-V 降级、转译链接线、golden 产物均不在本 RFC(随 agent 批准后实现 PR,§6);不动 `src/*`、不建 golden。
- **🔒 禁区实现扩张**：§4.6 已裁定 PR-D2 边界，但不授权自创签名布局、纹理访问语义或后端 UB 契约；越出所列 conformance/显式拒绝边界须另走 Full RFC。
- **🔒 P-01 边界/例外的规范性裁断**(§4.4):agent 已裁 P-01 不开例外——§4.4 仅定义 **strict-only 达标要求** 及其细化边界(「用户可观察」精确边界 / 未用输入 = 显式诊断 / 运行期等价验证形态),**不**写任何“例外豁免”。
- **绑定布局推导**(G2.3,P-11)/ **UC-04 渲染器**(G2.4)/ **PSO·资源状态·barrier 运行时面**:不在本 codegen RFC。
- **D-205 pin bump / A-graphics 迁移触发**：本次 agent 裁为“不 bump / 不迁移”；未来满足 §4.5 全部条件后另行裁决。
- **语言面扩展**：着色阶段类型面属 G2.1(RFC-0002);本 RFC 是 codegen 面,不新增语言构造。

## 9. 关键裁决（agent 授权代理代录；随本增补合入生效）

| Q | 待裁项 | AI 倾向（供参,不代决） | 裁决 |
|---|---|---|---|
| Q-Hybrid-RFC | 图形=B 设计面**新建 RFC-0004** vs **作 RFC-0003 增补** | 新建 RFC-0004(B 引入第二 IR + 外部依赖 + strict-only 达标要求 + 三个 🔒 禁区边界,应与 RFC-0003 的 compute=A 主述解耦) | **新建 RFC-0004**。RFC-0003 保留 compute=A 主述与 D-131 指针;图形=B 的设计/供应链/strict-only 细化统一收口在 RFC-0004,避免把第二 IR 与图形转译细节回灌进 RFC-0003 主体。 |
| Q-P01-Boundary | §4.4 转译链保真:strict-only 达标要求 vs P-01 例外/边界 | 〈代录 agent 裁断,见裁决列〉 | **代录 agent 裁断(agent 合并本决策包生效)**:P-01 不开例外、不设边界;§4.4 改为 **strict-only 达标要求**。用户可观察边界 = 外部接口签名元素 + `#[builtin]` 系统值 + 顶点输入/片元输出语义 + 阶段间字段/类型/插值/location 配对;varying semantic 串名/寄存器/packing 落契约线下。**声明但未用的外部输入 = 显式 6xxx 诊断**,不要求为过关而强制保留。运行期等价以 G-G2-2 device 真跑 golden + validator + DXIL golden + 签名篡改红绿验收。 |
| Q-Range-B | RXS-0159 重构 + RXS-0160 + B 新增面区间大小/拆分 | 锁 `RXS-0159` 保号重构 + `RXS-0160~0162` | **锁定**:`RXS-0159` 保号按 B 重构;新增 `RXS-0160`(阶段链接一致性)、`RXS-0161`(MIR→SPIR-V 图形降级面)、`RXS-0162`(B 转译链确定性/validator gate/供应链 pin/strict-only 核验)。#97 A 路 `RXS-0159` 不入 main,由 PR-D2 统一重写。 |
| Q-Supply | SPIRV-Cross/dxc/glslang pin 形态(vendored/env/lockfile) + 再分发审计 | lockfile `[[toolchain]]` + SHA256 pin + 再分发白名单(类比 D-205/D-313) | **锁 `[[toolchain]]` + SHA256 pin** 为 canonical 形态;显式 env override 仅允许本地 probe/dev,不构成 CI/stable path。SPIRV-Cross/dxc/glslang 再分发审计纳入白名单/许可门,作为 PR-D2/G-G2-2 前置。 |
| Q-Gate-B | 复用 `dxil-backend` vs 细分 `dxil-graphics-b` feature | 复用 `dxil-backend`(图形=B 为其分支) | **复用 `dxil-backend`**。不再引入 `dxil-graphics-b` 子 gate,避免把混合分支暴露成新的用户面组合维度。 |
| Q-Golden-B | B 转译产物 golden 形态(SPIR-V 中间 + DXIL 反汇编 / 仅 DXIL 反汇编) | 仅 DXIL 文本反汇编(对齐 RFC-0003 §9 Q-Golden)+ 可选 SPIR-V 中间 digest | **仅 DXIL 文本反汇编入 golden**(对齐 RFC-0003 Q-Golden);SPIR-V/HLSL 中间产物只留 digest/证据,不进入 bless 主 golden 面。签名 part dump/篡改签名红绿作为 strict-only 证据,不另立第二套 golden 主形态。 |
| Q-ABI-B | 签名寄存器/顺序/mask/packing/字节偏移是否成为 Rurix ABI | 降级为外部 conformance | **裁为外部 conformance，不是 Rurix stable ABI。**PR-D2 可核验 dxc/D3D12 既定布局，不得发明或冻结具体布局值；不可验证即 6xxx。 |
| Q-Texture-B | RXS-0161 资源句柄触及时的纹理内存模型边界 | opaque handle 可过，访问语义拒绝 | **仅 opaque 类型形态与无访问语义传递可降级。**描述符/采样或 load/store opcode/缓存/LOD/导数/越界语义一律 6xxx，等待独立 Full RFC。 |
| Q-UB-B | DXIL/SPIR-V 是否建立独立 UB/poison/undef 契约 | 不建立 | **不建立。**源码已定义语义不得借后端 UB 空间漂移；依赖未建模差异的 lowering 一律 6xxx。 |
| Q-D205-B | 现在是否 bump D-205 并触发 A-graphics 迁移 | 暂不 | **不 bump、不迁移。**§4.5 全部上游、release、同口径验证条件满足后仅触发重新裁决，不自动切换。 |

## 10. 稳定化与 provenance

- **稳定化**(10 §5)：图形=B 经 feature gate → tracking → 两里程碑无重大修订 → stabilization report → FCP-lite(10 §2.2);B 转译产物面/供应链 pin 在首个 stable 前不进 stable 面(随 RD-008)。
- **Provenance**：初版 `Assisted-by: kiro:claude-opus-4-8`；PR-D2 agent 裁决增补 `Assisted-by: codex:gpt-5`。agent/qwasg 于 2026-06-25 当前 Codex 会话明确授权代理完成技术裁决、落笔和机械合并，并将事后 review。该授权记录不替代 G-G2-2 device run URL、再分发法律签署或生产证书/CI secret 签名。

## 11. 规范与实现依据

- **证据(measured,命令真实输出)**:`evidence/dxil_slice3_rxs0159_sig_disasm_round8.md`(A 路图形签名 ISG1/OSG1 elemcount=0 + 根因 #90504 + Signature::addParam FFI ABI 耦合)/ `evidence/dxil_b_graphics_sig_report.md`(B 路图形签名 elemcount>0、SV 端到端存活、IDxcValidator+dxv.exe ×25 accept、×25 容器 SHA256 deterministic、§5 保真子轴)/ `evidence/dxil_b_strict_only_report.md`(B strict-only 达标取证:顶点输入语义名 by-construction 保名 measured 可消除损耗①;译后签名一致性校验门设计级可行——签名 part 可程序化解析 + 6xxx 段就位;已随本决策包自含落 main，原 strict-only 取证栈 #101 已吸收)/ `evidence/dxil_a_graphics_sig_effort_report.md`(A-graphics estimated ~800-1500 LOC、#90504/#57928 open 无在途、carry-patch partial-blocked、packing=conformance)。
- **工具链版本(取证实测,隔离不入库)**:dxc -spirv 1.8.0.4739 / spirv-val v2024.4 / spirv-cross vulkan-sdk-1.3.290 / dxc 1.9.2602.24(round-7 套件,含 dxil.dll 签名 validator + dxv.exe);glslang 15.0.0(producer 备选)。SHA256 见 `dxil_b_graphics_sig_20260625.json`。
- **决策/上游**:13 §D-131(v1.4 混合)· RFC-0003(MIR→DXIL 第二后端)· D-002/D-205 · 06 §4.2(纹理禁区)· 04 P-01(strict-only)/P-13 · 上游 [#90504](https://github.com/llvm/llvm-project/issues/90504)/[#57928](https://github.com/llvm/llvm-project/issues/57928)(A-graphics 迁移前置,RD-015)。
- **registry**:RD-010(A/B 裁决,close)· RD-011(A compute PSV patch)· RD-014(B 供应链跟踪)· RD-015(A-graphics 上游迁移跟踪)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-06-25 | AI 起草骨架(§1 摘要混合通路图 / §2 动机 + 为何 Full RFC / §3 用户视角混合透明 / §4.1 混合分发判据 / §4.2 B 转译链设计面 / §4.3 供应链 pin + 确定性 + strict-only 核验 / §4.4 🔒 P-01 边界声明占位 + 实测事实 / §4.5 A-graphics 迁移条件 + #90504/#57928 / §4.6 🔒 禁区占位(签名 ABI / 纹理内存模型 / UB)/ §5 下游条款计划表(RXS-0159 按 B 重构 + RXS-0160 + B 新增面,不落条款体)/ §6 feature gate + 栈式 PR + 真实红绿 / §7 备选 / §8 范围红线 / §9 未决留 agent(Q-Hybrid-RFC/Q-P01-Boundary/Q-Range-B/Q-Supply/Q-Gate-B/Q-Golden-B)/ §10 稳定化 / §11 依据)。**待 agent FCP-lite 批准 + 裁决 §9;§4.4 P-01 边界 + §4.6 禁区由 agent 落笔。agent 自主签署 / 不代决 / 不推进下游** | Full RFC（Draft） |
| Draft v0.2 | 2026-06-25 | 代录 agent 对 §4.4 的裁断(P-01 不开例外;B 严禁对用户声明/可观察签名元素静默降级/丢弃,留不住即显式 6xxx)+ 强制签名一致性校验门入设计面:§4.4 由「🔒 P-01 边界/例外声明占位」改写为 **strict-only 达标要求**(代录 agent 裁断核心规范句 agent 合并本决策包生效 + 达标机制 by-construction 保名/强制译后签名一致性校验门/校验门不可裁剪 + 契约线归类 + 细化处〈待 agent〉占位)/ §4.2 落「强制签名一致性校验门(MIR 意图 vs DXIL 签名 → 6xxx,不可裁剪)」设计面 / §4.3 strict-only 核验叠加校验门 / §3 用户视角注代录裁断 / §9 Q-P01-Boundary 更新为代录裁断(其余 §9 项维持〈待 agent〉)/ §4 intro·§8·§10·front-matter·§11 证据指针(+ `dxil_b_strict_only_report.md`)同步。**§4.6 🔒 禁区维持占位不落笔;不落 codegen/条款体;AI agent 自主裁决,agent 合并本决策包生效,细化处〈待 agent〉** | Full RFC（Draft） |
| Draft v0.3 | 2026-06-25 | 代录定稿三处留白:§4.4 细化边界**落定**(「用户可观察」精确边界 = 外部接口签名元素 + `#[builtin]` 系统值 + 顶点输入/片元输出语义 + 阶段间字段/类型/插值/location 配对;varying semantic 串名/寄存器/packing 落契约线下;**声明但未用的外部输入 = 显式 6xxx 诊断**,不要求强制保留;运行期等价验证形态 = G-G2-2 device 真跑 golden + validator + DXIL golden + 签名篡改红绿);§4.6 🔒 禁区由“占位”改为**边界声明**(签名/FFI ABI 只承诺源码层存在性与语义名,不承诺寄存器/packing/字节布局;纹理路径内存模型未建模即显式拒绝;SPIR-V/DXIL 不建立独立 UB 契约,不得借后端 UB 空间静默漂移);§9 全部裁决落定(Q-Hybrid-RFC=新建 RFC-0004 / Q-Range-B=`RXS-0159` 保号重构 + `RXS-0160~0162` / Q-Supply=`[[toolchain]]`+SHA256 pin / Q-Gate-B=复用 `dxil-backend` / Q-Golden-B=仅 DXIL 文本反汇编入 golden)。并同步 §5 计划表 / §6 gate / §8 范围红线 / §10 provenance / §11 strict-only 证据(原 #101)自含约束。**仍待 agent FCP-lite 批准;strict-only 证据(原 #101)已随本决策包自含;agent 自主签署 / 不自合** | Full RFC（Draft） |
| Draft v0.4 | 2026-06-25 | 干净自含化勘误(无语义改动):本决策包重构为干净自含 PR(去 #96/#97 代码祖先、strict-only 证据原 #101 已吸收入本包)后,将全文「生效以 agent 合并 #100 为准」「#101 须先于/随 #100 落地」等旧 stacked 栈绑定改为 PR 号无关的通用生效条件「agent 合并本决策包生效」+「strict-only 证据已自含、引用不悬空」(front-matter 状态/Provenance/Agent 批准、§3、§4 intro、§4.4、§9 header/Q-P01-Boundary、§10、§11、修订记录 v0.2/v0.3)。**仅清理 PR 号引用,agent 裁断实质(P-01 不开例外 / §4.4 strict-only 达标要求 / §9 全部裁决 / §4.6 禁区边界)逐字不变;仍待 agent FCP-lite 批准;agent 自主签署 / 不自合** | Full RFC（Draft） |
| Agent addendum v1.0 | 2026-06-25 | PR #104 合入后，agent/qwasg 在当前 Codex 会话明确授权代理完成 PR-D2 前置自主裁决：Q-ABI-B=packing/寄存器布局降级为外部 conformance；Q-Texture-B=仅 opaque handle 形态可过、访问语义未建模即 6xxx；Q-UB-B=不建立独立 DXIL/SPIR-V UB；Q-D205-B=当前不 bump、不迁移。同步 §4.5/§4.6/§8/§9/front matter/provenance。授权不代表 device、法律或生产签名已完成 | Full RFC（Agent-authorized proxy addendum） |
