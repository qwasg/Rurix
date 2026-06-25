# RFC-0004 — SPIR-V→DXIL 图形后端 / 混合 codegen（compute=A / 图形=B）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0004（4 位制，编号永不复用，10 §9.5） |
| 标题 | SPIR-V→DXIL 图形后端 / 混合 codegen（compute=A / 图形=B） |
| 档位 | **Full RFC**（10 §3：新 codegen 路径 + 第二中间表示(SPIR-V) + 外部转译依赖；触 AGENTS 硬规则 5 禁区边界——DXIL 文本语义 UB / 纹理路径内存模型映射(06 §4.2) / 签名·FFI ABI 二进制布局，均标 🔒 不落笔留 owner；且触**准永久公理 P-01(strict-only)**——§4.4 strict-only 达标要求的核心规范句为 AI 代录 owner 已给裁断(P-01 不开例外,owner 合并 #100 生效),细化处留〈待 owner〉占位(P-13)） |
| 状态 | **Draft（2026-06-25，AI 起草骨架）**。**owner 经 FCP-lite 批准前不推进下游 spec/实现 PR（硬规则 1，AI 不代签 / 不代决 / 不自合）**;§4.4 strict-only 达标要求核心规范句 = 代录 owner 裁断(owner 合并 #100 生效)+ 细化处〈待 owner〉占位;🔒 §4.6 禁区子节仅占位,留 owner 落笔 |
| 承接里程碑 | G2.2（验收门 **G-G2-2**），承 RFC-0003 混合 codegen 分发(图形分支) |
| 关联条款 | 拟重构 spec **RXS-0159**(按 B 路径)+ **RXS-0160** + B 新增面(MIR→SPIR-V)预留区间(见 §5)；落 `spec/dxil_backend.md`(承 RFC-0003)。**本 RFC 不创建裸条款头**，trace 维持现状 |
| 依据决策 | D-131（G2 DXIL 生成路径,v1.4 增补 = **混合 compute=A/图形=B**）· D-002（图形分期,已批准）· D-205（LLVM pin,vendored）· RFC-0003（MIR→DXIL 第二后端,Owner Approved;本 RFC 为其图形分支细化）· 06 §4.2(纹理内存模型禁区,🔒)· 04 P-01(strict-only,准永久公理)/ P-13(防 AI 幻觉治理) |
| Provenance | `Assisted-by: kiro:claude-opus-4-8`。Human-in-the-loop（硬规则 1/2）：本草案由 AI 起草骨架,§4.4 strict-only 达标要求核心规范句为 AI 代录 owner 裁断(owner 合并 #100 生效)、细化处〈待 owner〉占位,🔒 §4.6 禁区子节仅占位不落笔,§9 未决留 owner 裁决;**owner FCP-lite 批准前不推进下游实现,AI 不自启、不代签** |
| Owner 批准 | 〈待 owner FCP-lite 批准；批准范围含 §4.4 strict-only 达标要求(代录裁断之确认 + 细化处〈待 owner〉落笔) + §4.6 🔒 禁区子节 + §9 全部裁决项；记录方式 owner 落笔〉 |

---

## 1. 摘要

本 RFC 在 RFC-0003（MIR→DXIL 第二后端,Owner Approved）的基础上,细化 **D-131 v1.4 混合裁决的图形分支**:compute kernel 经 **A 路**(LLVM DirectX 后端直接 emit DXIL,RFC-0003 既有)降级,图形着色阶段(vertex/fragment/mesh/task/RT)经 **B 路**(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL 转译)降级。

混合动因(证据,非本 RFC 裁决):A 路图形签名经 slice3/round-8 实测**不可达**(ISG1/OSG1 `elemcount=0`,上游 #90504 未实现 + 填充耦合 FFI ABI 禁区);B 路图形签名经取证**实测可行**(`elemcount>0`、SV 端到端存活、validator accept、确定性);A-graphics 评估 = ~800-1500 LOC 上游大功能、无在途 PR。证据指针见 §11。

```
compute kernel  ──A:LLVM DirectX 后端─────────────▶ DXIL(compute)   ← RFC-0003 既有
图形着色阶段    ──B:MIR→SPIR-V→SPIRV-Cross→HLSL→dxc─▶ DXIL(graphics) ← 本 RFC 图形分支
```

本 RFC 只定义**图形=B 的设计面 + 混合分发判据 + 下游条款计划**;**🔒 P-01 边界(转译保真非完美的例外裁断)、🔒 禁区(签名/FFI ABI 二进制布局、纹理内存模型、DXIL UB)、codegen 实现均不在本 RFC**——前者留 owner 落笔(P-13),后者留实现 PR(硬规则 7)。

## 2. 动机

- **D-131 v1.4 混合裁决落地需图形分支设计载体**：RFC-0003 §9 Q-D131 已增补 = 混合(compute=A/图形=B),但 RFC-0003 §4 的降级面以 A 路单后端为主述;图形=B 的转译链、第二中间表示(SPIR-V)、外部转译依赖(SPIRV-Cross/dxc)、保真边界需独立 Full RFC 精确化。
- **图形=B 是 G2.3/G2.4 的图形 codegen 基座**：G2.3(绑定布局推导)、G2.4(UC-04 deferred 渲染器)需图形着色阶段产带真实 SV 签名的 DXIL;A 路图形签名不可达(slice3),B 路是当前唯一 measured 可行的图形 codegen 路径。
- **A-graphics 上游成熟后迁移**：本 RFC 不放弃 A-graphics——挂上游 #90504(后端签名 part 生成)/#57928(签名元数据构造),成熟后图形分支可由 B 迁回 A(迁移条件见 §4.5,跟踪 RD-015)。

**为何需要 Full RFC（而非 Direct/Mini）**：本 RFC 引入**第二中间表示(SPIR-V)+ 外部转译依赖链(SPIRV-Cross/dxc/glslang)**,且触及 **签名/FFI ABI 二进制布局**(§4.6)、**DXIL 文本语义 UB 边界**、**纹理路径内存模型映射(06 §4.2)**——10 §3 / 硬规则 5 明列的 Full RFC / 禁区触发面;更触及**准永久公理 P-01(strict-only)的边界声明**(转译链保真非完美,§4.4),P-01 例外/边界仅 owner 经 Full RFC 落笔(P-13)。判档争议向上取严(硬规则 8);AI 不自判 Direct/Mini、不代签批准/合并(硬规则 1)。

## 3. 指导级解释（用户视角）

> 以下为**拟议**形态示意,最终以 owner 批准 + spec 条款为准;**混合分发对用户透明**——用户对某着色函数经 A 还是 B 产 DXIL 无感(分发由阶段类别在 MIR 后自动判定,§4.1)。

用户经 `rx build --target dxil`(RFC-0003 §9 Q-CLI)为 D3D12 目标构建;同一份源码内 compute kernel 与图形着色阶段函数各走 A/B 分支,产出可被 D3D12 PSO 消费的着色器对象:

```rust
// compute kernel → A 路(LLVM DirectX 后端)→ DXIL compute shader
kernel fn cs_main(/* ... */) { /* ... */ }
// 图形着色阶段 → B 路(MIR→SPIR-V→转译)→ DXIL vertex/pixel shader(带真实 SV 签名)
vertex fn vs_main(in: VertexIn) -> VertexOut { /* ... */ }
fragment fn fs_main(in: VertexOut) -> FragmentOut { /* ... */ }
```

`strict-only`(P-01)维持:任一分支降级失败 = **结构化编译错误**(6xxx 段,RFC-0003 §5),无静默降级、无 permissive 回退。**注（代录 owner 裁断，owner 合并 #100 生效）**:B 路严禁任何对用户声明/可观察签名元素的静默降级或丢弃——留不住即显式 6xxx 编译错(P-01 不开例外、不设边界,§4.4 是 B 的达标条件而非例外);用户语义名经 by-construction 保真 + 强制译后签名一致性校验门兜底(§4.4/§4.2)。varying 名/寄存器布局/未用输入的契约线归类细化处留〈待 owner〉(§4.4)。

## 4. 参考级设计

> 本节落笔**混合分发架构与 B 路图形降级的设计面**;§4.4 为 **strict-only 达标要求**(代录 owner 裁断,核心规范句 owner 合并 #100 生效,细化处留〈待 owner〉占位);触及禁区的子节(§4.6)标 🔒,本草案不写规范性内容、仅占位 + 摆实测事实,留 owner 落笔。

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
- **确定性**：B 全链对给定 MIR 输入确定,纳入 golden 核对(形态待 §9 Q-Golden-B)。
- **强制签名一致性校验门（设计面，承 §4.4 strict-only 达标要求）**：B 链产 DXIL 后,codegen **强制**比对 DXIL ISG1/OSG1 签名 part 与 MIR 意图签名(用户语义名 / 系统值 / 被使用元素);任何用户声明或可观察元素未保真(含声明但未用的输入)→ **6xxx 显式编译错**,无静默通过、无静默降级/丢弃。该校验门是图形=B codegen 的**不可裁剪组成**(不存在「跳过校验直接产物」的配置)。设计级可行性 measured:签名 part 可程序化解析(`evidence/dxil_b_strict_only_report.md`),6xxx 段已存在(RFC-0003 §5);校验器位置/检测粒度/6xxx 类别随实现 PR(§5/§4.4),本 RFC 仅落设计面、不落 codegen。
- **能力探测**：目标 shader model / DXIL 版本由真实工具链探测驱动(A-03/P-01),不写死。

### 4.3 供应链（SPIRV-Cross/dxc/glslang 版本 pin + 确定性 + strict-only 核验）

B 路引入外部转译依赖,供应链纪律类比 D-205(LLVM pin):

- **版本 pin**：SPIRV-Cross / dxc / glslang(若作 SPIR-V producer 备选)各 pin 明确版本 + SHA256(取证实测版本见 §11);pin 形态(vendored / 显式 env / lockfile `[[toolchain]]`)随实现 PR 与 §9 Q-Supply 裁定。
- **确定性核验**：同输入 ×N 容器 SHA256 一致为 CI 门(取证已 measured deterministic,§11)。
- **strict-only 核验(P-01)**：转译链任一段失败(SPIR-V 不合规 / spirv-cross 失败 / dxc validator reject)→ 结构化 6xxx codegen 错误,无静默降级;入 golden 前 DXIL 须经 dxc validator 验证通过(对齐 RFC-0003 §9 Q-Golden)。**并叠加 §4.2 强制签名一致性校验门**(译后 ISG1/OSG1 vs MIR 意图签名比对):validator accept **不等于**用户签名意图保真——校验门补足「accept 但用户声明/可观察元素未保真」的缺口,留不住即 6xxx 显式错(§4.4 达标要求)。
- **再分发合规**：SPIRV-Cross/dxc/glslang 再分发许可审计(类比 D-313 NVIDIA 白名单 / D-205 vendored),随实现 PR + 供应链跟踪(RD-014)。

### 4.4 strict-only 达标要求（代录 owner 裁断 + 强制签名一致性校验门）

> **本子节为图形=B 对准永久公理 P-01（strict-only，04 P-01）的达标要求,非例外/边界声明。核心规范句为 AI 代录 owner 已给裁断(owner 合并 #100 生效);细化处标〈待 owner〉占位,AI 不替补(P-13/硬规则 1)。**

**规范句（代录 owner 裁断，owner 合并 #100 生效）**:图形=B 路**严禁任何对用户声明或可观察签名元素的静默降级或丢弃**——凡用户在源码中声明的、或外部可观察的签名元素,转译链(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL)若留不住,**必须显式 6xxx 编译错误**(承 RFC-0003 §5),**绝不静默丢/改**。**P-01(strict-only)不开例外、不设边界**:§4.4 是图形=B 的**达标条件**,而非对 P-01 的例外/边界声明——B 要被接受,须证语言层零静默降级(不靠任何 P-01 例外)。

**达标机制（measured 事实，来自 `evidence/dxil_b_strict_only_report.md`，命令真实输出；该取证为本栈 PR #101，随 #100 一并待 owner 批）**:

1. **用户语义名 by-construction 保真**:Rurix MIR→SPIR-V 自有降级握有用户 I/O 全部语义信息(RXS-0154 `#[builtin]`/`#[interpolate]` + 字段名),可对所有用户命名 I/O **by-construction** emit SPIR-V `UserSemantic` 装饰并在 SPIR-V→HLSL 段驱动保名。measured 证此机制对顶点输入语义名有效:`POSITION`/`NORMAL` 默认经 SPIR-V 往返降级为通用 `TEXCOORD#`,经 `dxc -spirv -fspv-reflect`(携 `UserSemantic`)+ `spirv-cross --set-hlsl-named-vertex-input-semantic`(经 SPIR-V 反射自动导出,非硬编码)端到端**保真存活**(签名 part dump 实证:`vs_sig` ISG1 默认 `[TEXCOORD0,TEXCOORD1,TEXCOORD2]` → 保名 `[POSITION0,NORMAL0,TEXCOORD0]`)。
2. **强制译后签名一致性校验门**:B 链产 DXIL 后,codegen **强制**解析 DXIL ISG1/OSG1 签名 part 与 MIR 意图签名(用户语义名集合 / 系统值 / 被使用元素)做结构化对照;**任何用户声明或可观察元素未保真**——含**声明但未用的输入元素**(按 owner「严禁静默」裁:不许静默删,须保留或显式诊断)——→ 发 **6xxx 显式编译错误**。measured 支撑:签名 part 可程序化解析(×N 稳定解出 elemcount + 名 + 系统值 + register),译后校验门有可靠输入;6xxx 段已存在(RFC-0003 §5)→ 错误码载体就位 → 该校验门**设计级可行**,不依赖 P-01 例外。
3. **校验门不可裁剪**:上述强制签名一致性校验门是图形=B codegen 的**不可裁剪组成**——不存在「跳过校验直接产物」的配置;校验门失败即 6xxx,无静默通过。

**契约线归类（摆事实 + 引据；非「静默丢声明物」）**:转译链 measured 的余项保真损耗——① varying 语义名(vs-out→ps-in 经 spirv-cross 硬绑 `TEXCOORD#`)、② 寄存器/顺序重排——在**用户声明面以下**:RXS-0155 阶段间接口契约 = 类型级字段/类型/插值匹配,**非 HLSL 语义串**;寄存器/二进制布局属 §4.6(a) 签名 ABI 禁区(dxc/D3D12 conformance 既定算法,用户从未声明名/寄存器)。故其工具层重写**非「静默丢用户声明物」**。**但**校验门仍须确认 varying location 双侧对齐、不破坏阶段间链接(vs-out↔ps-in);链接异常即**显式报**(不静默接受错链)。

**细化处留占位（〈待 owner〉，AI 不替补，P-13）**:

- 〈待 owner〉「用户可观察」的**精确边界**——哪些签名元素属「用户声明/外部可观察」(顶点输入名已 measured 属此类)、哪些落契约线下(varying 名/寄存器布局拟判属此),由 owner 落笔精确定义。
- 〈待 owner〉**声明但未用的输入元素**处置——保留(强制存活)抑或显式诊断(编译期告知用户未用),二者均不许静默删;具体形态由 owner 裁。
- 〈待 owner〉语义级**运行期等价**(strict-only 运行期行为等价,非签名结构)须 device 真跑 golden 验证,超出取证 spike 范围;验证形态由 owner + 实现 PR 定。

### 4.5 与 A-graphics 的关系（上游 #90504/#57928 跟踪 + 迁移条件）

- **图形=B 为当前路径,A-graphics 为迁移目标**：A 路图形签名当前不可达(slice3:LLVM `addSignature()` 写空签名 #90504、签名元数据 `nullptr` #57928、前端 packing 占位);A-graphics 评估 estimated ~800-1500 LOC 跨前后端、上游 open 无在途 PR(`dxil_a_graphics_sig_effort_report.md`)。
- **迁移条件(跟踪 RD-015)**：上游 #90504 + #57928 落地(后端从模块元数据 emit ISG1/OSG1 + 签名元数据构造)+ release + D-205 pin 覆盖该版本后,图形分支可由 B 迁回 A(分发判据 §4.1 不变,对 spec 透明);迁移触发 + 形态由 owner 届时裁决(D-205 pin bump 属 owner 独立决策)。
- **packing 属 conformance 非自由 ABI**(事实陈述,`dxil_a_graphics_sig_effort_report.md` §4):register/mask packing = 复刻 dxc/D3D12 既定算法(MS HLSL packing rules),非 Rurix 自由 ABI 设计——此事实供 owner 判 §4.6 签名布局禁区是否可降级为 conformance 说明(由 owner 落笔,AI 不替裁)。

### 4.6 🔒 禁区边界声明（本 RFC 不定义，留 owner Full RFC）

> **本子节为边界声明,AI 不落笔禁区内容（硬规则 5 / 06 §4.2）。**

- **(a) 签名/内建变量·FFI ABI 二进制布局**：签名元素的寄存器打包 / 字节偏移 / component mask / 根参数·常量缓冲二进制布局——属 FFI ABI 禁区(承 RFC-0003 §4.6(c)/§9 Q-Builtin)。B 路由 dxc emit 签名布局,Rurix 不定义/不冻结/不作保证。〈待 owner 后续 Full RFC〉
- **(b) 纹理路径内存模型映射**：纹理/采样器在 SPIR-V/DXIL 的采样 opcode 映射、描述符编码、缓存一致性、采样 UB——属 06 §4.2 内存模型禁区。〈待 owner 后续 Full RFC〉
- **(c) DXIL/SPIR-V 文本语义 UB 边界**：转译链中间表示(SPIR-V)与 DXIL 的未定义行为边界、poison 语义、越界/竞争语义后果——属 UB 条款禁区(硬规则 5)。〈待 owner 后续 Full RFC〉

本边界与 §8 范围红线一致:本 RFC 的 B 路降级面是**结构/类型形态层 + 转译链工程面**,不承诺任何 UB 语义、内存序、一致性或 ABI 二进制布局保证。

## 5. 下游 spec 条款计划表（spec diff，10 §3 要件；不落条款体）

落 `spec/dxil_backend.md`(承 RFC-0003)。**本 RFC 不创建 `### RXS-####` 裸条款头**——下表为条款重构/新增的**计划表**,条款体随 owner 批准本 RFC 后的实现 PR 同落(条款 PR 先于实现 PR,硬规则 7;trace 维持全锚定)。**区间大小未锁定**(随 §9 Q-Range-B 与实现拆分一并定)。

| 条款（拟，区间待 §9 Q-Range-B 定） | 标题 | 处置 | 测试锚定计划（每条 ≥1，`//@ spec`） |
|---|---|---|---|
| RXS-0159（已落,按 A 类型面） | 阶段 I/O → DXIL 签名/系统值语义降级 | **按 B 重构**(类型面 SV 映射经 SPIR-V `BuiltIn`/`Location` decoration → dxc 产真实 ISG1/OSG1;或 hold 至 owner 批准本 RFC) | dxil-sig accept(SV 真达,`elemcount>0`)+ reject(不可映射 → RX6009)+ DXIL golden |
| RXS-0160（拟,RFC-0003 §5 计划项） | 阶段间接口 → DXIL/SPIR-V 阶段链接一致性核对 | 新落(按 B 路径,vertex out↔fragment in varying 经 SPIR-V location 匹配) | dxil-sig accept + reject + golden |
| RXS-016x（拟,B 新增面） | 图形着色阶段 MIR→SPIR-V 降级面(execution model / I/O decoration / 资源句柄) | 新增(B 路 §4.2(a)) | SPIR-V/DXIL golden + conformance accept/reject |
| RXS-016x（拟,B 供应链） | B 转译链确定性 + validator gate + golden/bless 形态 | 新增(§4.3) | 确定性核对 + 真实红绿(篡改转译输出 → 红 → 复原绿) |

> RXS-0157(target 分发)/RXS-0158(阶段→着色器类型,compute/vertex/fragment 已落)维持 RFC-0003 既有;本 RFC 重构的是图形 I/O 签名降级(RXS-0159 由 A 类型面 stub 改 B 真达)及新增 B 转译面。**🔒 签名二进制 ABI 布局不进任何条款**(§4.6(a))。

- **错误码策略**：B 路 codegen/转译失败归 **6xxx 段**(承 RFC-0003 RX6007~6009,只追加;新可达类别随实现 PR 按真实分配 + en/zh message-key,registry/error_codes.json 只追加,ci/bilingual_coverage.py 覆盖)。不预留、不预造。
- spec 条款 PR 先于实现 PR(硬规则 7);trace_matrix 维持全锚定(沿用全局 counter)。

## 6. feature gate / tracking / 实现序（10 §3 要件）

- **feature gate**：复用 RFC-0003 `dxil-backend`(图形=B 为其图形分支),或细分 `dxil-graphics-b`(待 §9 Q-Gate-B);未启用时图形分支不参与编译,compute A 路 + PTX 路径不受影响。
- **栈式 PR（门控于本 RFC 批准 + §9 裁定后）**：
  - **PR-D1 spec 脚手架**：`spec/dxil_backend.md` 登记 B 新增面预留区间 + RXS-0159 重构说明(**不落裸条款头**)+ README §4 同步;`trace_matrix --check` PASS。
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

- **codegen 实现**：MIR→SPIR-V 降级、转译链接线、golden 产物均不在本 RFC(随 owner 批准后实现 PR,§6);不动 `src/*`、不建 golden。
- **🔒 签名/FFI ABI 二进制布局 / 纹理路径内存模型映射(06 §4.2)/ DXIL·SPIR-V UB 边界**(硬规则 5 禁区):§4.6 占位,留 owner 后续 Full RFC。
- **🔒 P-01 边界/例外的规范性裁断**(§4.4):owner 已裁 P-01 不开例外——§4.4 改为 **strict-only 达标要求**(代录 owner 裁断,核心规范句 owner 合并 #100 生效);本 RFC 不自创超裁断的规范内容,细化处(「用户可观察」精确边界 / 未用输入保留 vs 诊断)留〈待 owner〉占位(P-13)。
- **绑定布局推导**(G2.3,P-11)/ **UC-04 渲染器**(G2.4)/ **PSO·资源状态·barrier 运行时面**:不在本 codegen RFC。
- **D-205 pin bump / A-graphics 迁移触发**：属 owner 独立决策(§4.5),不在本 RFC。
- **语言面扩展**：着色阶段类型面属 G2.1(RFC-0002);本 RFC 是 codegen 面,不新增语言构造。

## 9. 未决问题 / 关键裁决（留 owner）

| Q | 待裁项 | AI 倾向（供参,不代决） | 裁决 |
|---|---|---|---|
| Q-Hybrid-RFC | 图形=B 设计面**新建 RFC-0004** vs **作 RFC-0003 增补** | 新建 RFC-0004(B 引入第二 IR + 外部依赖 + P-01 边界,体量与禁区面足以独立;RFC-0003 维持 A 主述 + §9 指针) | 〈待 owner〉 |
| Q-P01-Boundary | §4.4 转译链保真:strict-only 达标要求 vs P-01 例外/边界 | 〈代录 owner 裁断,见裁决列〉 | **代录 owner 裁断(owner 合并 #100 生效)**:P-01 不开例外、不设边界;§4.4 改为 **strict-only 达标要求**——B 严禁对用户声明/可观察签名元素静默降级/丢弃,留不住即 6xxx 显式错,经 by-construction 保名 + **强制译后签名一致性校验门**达标。细化处(「用户可观察」精确边界 / 未用输入保留 vs 诊断 / 运行期等价验证形态)留〈待 owner〉,见 §4.4 |
| Q-Range-B | RXS-0159 重构 + RXS-0160 + B 新增面区间大小/拆分 | 随实现拆分,暂不锁(类比 RFC-0003 Q-Range) | 〈待 owner〉 |
| Q-Supply | SPIRV-Cross/dxc/glslang pin 形态(vendored/env/lockfile) + 再分发审计 | lockfile `[[toolchain]]` + SHA256 pin + 再分发白名单(类比 D-205/D-313) | 〈待 owner〉 |
| Q-Gate-B | 复用 `dxil-backend` vs 细分 `dxil-graphics-b` feature | 复用 `dxil-backend`(图形=B 为其分支) | 〈待 owner〉 |
| Q-Golden-B | B 转译产物 golden 形态(SPIR-V 中间 + DXIL 反汇编 / 仅 DXIL 反汇编) | 仅 DXIL 文本反汇编(对齐 RFC-0003 §9 Q-Golden)+ 可选 SPIR-V 中间 digest | 〈待 owner〉 |

## 10. 稳定化与 provenance

- **稳定化**(10 §5)：图形=B 经 feature gate → tracking → 两里程碑无重大修订 → stabilization report → FCP-lite(10 §2.2);B 转译产物面/供应链 pin 在首个 stable 前不进 stable 面(随 RD-008)。
- **Provenance**：`Assisted-by: kiro:claude-opus-4-8`。本草案由 AI 起草骨架;§4.4 strict-only 达标要求核心规范句为代录 owner 裁断(owner 合并 #100 生效)、细化处〈待 owner〉占位,§4.6 🔒 禁区维持占位不落笔,§9 未决留 owner。**owner FCP-lite 批准前不推进下游 spec/实现 PR,AI 不代签 / 不代决 / 不自合**(硬规则 1)。FCP-lite 评审/等待窗按 10 §2.2 独立完成,本记录不虚构尚不存在的评审。

## 11. 规范与实现依据

- **证据(measured,命令真实输出)**:`evidence/dxil_slice3_rxs0159_sig_disasm_round8.md`(A 路图形签名 ISG1/OSG1 elemcount=0 + 根因 #90504 + Signature::addParam FFI ABI 耦合)/ `evidence/dxil_b_graphics_sig_report.md`(B 路图形签名 elemcount>0、SV 端到端存活、IDxcValidator+dxv.exe ×25 accept、×25 容器 SHA256 deterministic、§5 保真子轴)/ `evidence/dxil_b_strict_only_report.md`(B strict-only 达标取证:顶点输入语义名 by-construction 保名 measured 可消除损耗①;译后签名一致性校验门设计级可行——签名 part 可程序化解析 + 6xxx 段就位;本栈 PR #101,随 #100 待 owner 批)/ `evidence/dxil_a_graphics_sig_effort_report.md`(A-graphics estimated ~800-1500 LOC、#90504/#57928 open 无在途、carry-patch partial-blocked、packing=conformance)。
- **工具链版本(取证实测,隔离不入库)**:dxc -spirv 1.8.0.4739 / spirv-val v2024.4 / spirv-cross vulkan-sdk-1.3.290 / dxc 1.9.2602.24(round-7 套件,含 dxil.dll 签名 validator + dxv.exe);glslang 15.0.0(producer 备选)。SHA256 见 `dxil_b_graphics_sig_20260625.json`。
- **决策/上游**:13 §D-131(v1.4 混合)· RFC-0003(MIR→DXIL 第二后端)· D-002/D-205 · 06 §4.2(纹理禁区)· 04 P-01(strict-only)/P-13 · 上游 [#90504](https://github.com/llvm/llvm-project/issues/90504)/[#57928](https://github.com/llvm/llvm-project/issues/57928)(A-graphics 迁移前置,RD-015)。
- **registry**:RD-010(A/B 裁决,close)· RD-011(A compute PSV patch)· RD-014(B 供应链跟踪)· RD-015(A-graphics 上游迁移跟踪)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-06-25 | AI 起草骨架(§1 摘要混合通路图 / §2 动机 + 为何 Full RFC / §3 用户视角混合透明 / §4.1 混合分发判据 / §4.2 B 转译链设计面 / §4.3 供应链 pin + 确定性 + strict-only 核验 / §4.4 🔒 P-01 边界声明占位 + 实测事实 / §4.5 A-graphics 迁移条件 + #90504/#57928 / §4.6 🔒 禁区占位(签名 ABI / 纹理内存模型 / UB)/ §5 下游条款计划表(RXS-0159 按 B 重构 + RXS-0160 + B 新增面,不落条款体)/ §6 feature gate + 栈式 PR + 真实红绿 / §7 备选 / §8 范围红线 / §9 未决留 owner(Q-Hybrid-RFC/Q-P01-Boundary/Q-Range-B/Q-Supply/Q-Gate-B/Q-Golden-B)/ §10 稳定化 / §11 依据)。**待 owner FCP-lite 批准 + 裁决 §9;§4.4 P-01 边界 + §4.6 禁区由 owner 落笔。AI 不代签 / 不代决 / 不推进下游** | Full RFC（Draft） |
| Draft v0.2 | 2026-06-25 | 代录 owner 对 §4.4 的裁断(P-01 不开例外;B 严禁对用户声明/可观察签名元素静默降级/丢弃,留不住即显式 6xxx)+ 强制签名一致性校验门入设计面:§4.4 由「🔒 P-01 边界/例外声明占位」改写为 **strict-only 达标要求**(代录 owner 裁断核心规范句 owner 合并 #100 生效 + 达标机制 by-construction 保名/强制译后签名一致性校验门/校验门不可裁剪 + 契约线归类 + 细化处〈待 owner〉占位)/ §4.2 落「强制签名一致性校验门(MIR 意图 vs DXIL 签名 → 6xxx,不可裁剪)」设计面 / §4.3 strict-only 核验叠加校验门 / §3 用户视角注代录裁断 / §9 Q-P01-Boundary 更新为代录裁断(其余 §9 项维持〈待 owner〉)/ §4 intro·§8·§10·front-matter·§11 证据指针(+ `dxil_b_strict_only_report.md`)同步。**§4.6 🔒 禁区维持占位不落笔;不落 codegen/条款体;AI 代录非代决,owner 合并 #100 生效,细化处〈待 owner〉** | Full RFC（Draft） |
