# RD-013 Unblock — Owner 决策包（spec 语义问题清单 + 边界提案草案，2026-06-28）

> 体例参考 `rfcs/mini-0002-engine-integration.md`（单页提案表头 + 分节）。**本文件不是已编号 RFC、不占用 Mini-RFC 序列号、不裁档**——它是为解锁 RD-013 而起草的 **owner 决策包**：把实现 RD-013 最小 body lowering 切片**前置必须由 owner 裁定**的语义问题整理成清单，给出**带取舍的候选方案**，但**不替 owner 选**。owner 裁定后方可据此推进条款 PR（先于实现 PR，AGENTS 硬规则 7）。

| 字段 | 值 |
|---|---|
| 文档类型 | **Owner 决策包草案**（spec 语义问题清单 + 边界提案）；非条款、非裁决、非 RFC 编号占位 |
| 关联 deferred | **RD-013**（DXIL 着色阶段 I/O 入口 body 数据流降级，open，owner_milestone G2.2） |
| 上游输入 | `evidence/rd013_body_lowering_preflight_20260628.md`（blocked-honest 可行性预检，本决策包不重复其机器事实，仅引用结论） |
| 触及条款面 | `spec/dxil_backend.md` RXS-0159（I/O 签名类型面 + IR4 body deferred）/ RXS-0161（MIR→SPIR-V 降级面）/ RXS-0162（validator gate + golden）；`spec/shader_stages.md` RXS-0154（阶段 I/O 语义类型，body 访问语义上游）/ RXS-0155（阶段间接口契约） |
| 触及 RFC 面 | RFC-0003 §4.6 🔒（FFI ABI 二进制布局禁区，留 owner Full RFC）/ RFC-0004 §4.6(a) 🔒（签名/FFI ABI 二进制布局裁为外部 conformance，不进任何条款）/ RFC-0004 §4.4（strict-only 达标要求） |
| 只读事实依据 | `src/rurixc/src/{dxil_spirv.rs,dxil_codegen.rs,mir.rs,mir_build.rs}`（本任务只读，不改） |
| 待裁问题 | **Q1**（源码层 body I/O 访问语义）· **Q2**（MIR place ↔ io_sig In/Out 绑定规则）· **Q3**（绑定与 §4.6 ABI 禁区划界）· **Q4**（最小 rvalue 子集 ↔ RX6001/RX6003 device codegen 子集边界） |
| 档位归属 | **未定，属 owner 裁断**（判档争议向上取严，硬规则 8）。Q3 显式触及 RFC-0003 §4.6 / RFC-0004 §4.6(a) Full RFC 禁区面 → 决策包**提示**该面或须 Full RFC 处置，但**不替 owner 定档** |
| Provenance | `Assisted-by: codex:gpt-5`。AI 仅起草问题与候选方案；语义裁定、status 翻转、条款落笔均 owner 范围（硬规则 1/7） |

---

## 0. 本决策包的边界（先说不做什么）

- **不裁定任何 Q1–Q4 候选**：每问给 ≥2 个带取舍候选，owner 选项写在「待 owner」处。
- **不落条款体、不落裸 `### RXS-####` 头、不造错误码、不入 golden、不碰 `src/*`**。
- **不签 / 不翻 G-G2-4，不实现 PR-F2，不替代 CI step 48**，不动 RD-021（纹理内存模型）/ §4.6 ABI 禁区本体。
- **RD-013 维持 open**：本 PR 仅新增本决策包 + registry append-only 留痕（记「owner 决策包已起草、待裁」）。
- 预检（上游输入）已确证：最小切片须**同时**(a) 定义源码层着色 body I/O 数据流语义、(b) 定义 MIR↔签名绑定，二者越出 graphics=B「类型面 only」owner 裁边界（RXS-0159 IR4），故须 owner 先裁。本决策包把「须裁什么」收敛成四个聚焦问题。

## 1. 背景与最小切片定义（不重复预检，仅锚定范围）

预检结论（`rd013_body_lowering_preflight_20260628.md`）：当前 graphics=B 链 `emit_dxil_b(stage, &body.io_sig, &body.resources)` 只消费签名类型面，`body.blocks`/`locals` 结构上从不进入 B 链，`emit_spirv` 按设计 emit 平凡 passthrough `main`；`io_sig`（`mir.rs` `IoSigElem { field_name, kind, ty, dir }`）由 `mir_build.rs::dxil_io::io_sig_for` 从 AST 形参/返回 I/O 结构体字段标注提取，与 body 的 `LocalIdx`/`Place`/`ProjElem::Field` 投影**无任何绑定**。

「RD-013 最小 body lowering 切片」在本决策包中**定义为**（owner 可在裁定时收窄/放宽）：

1. 读取**已声明输入**签名元素（vertex 阶段输入 / fragment 阶段输入 varying 或 builtin）；
2. 仅 `Use` / `Const` / 标量·向量 `BinaryOp`（`mir.rs::Rvalue`/`BinOp` 已建模子集）的纯算术；
3. 写出**已声明输出**签名元素（vertex 输出 / fragment 输出）；
4. 不触资源句柄 / 采样 / 纹理（`MirIoType` 仅标量/向量，绕开 RD-021 / 06 §4.2 禁区）。

要把该切片实现成诚实 PR，**必须先有** owner 对下列四问的裁定（条款先于实现，硬规则 7）。

---

## 2. 待裁问题清单（每问：问题 → 候选 → 取舍 → 待 owner）

### Q1 — Rurix 着色 body 如何读取已声明输入 / 写出已声明输出（源码层访问语义）

**问题**：RXS-0154 只定义了阶段 I/O 字段的**类型面**（`#[builtin(..)]` / `#[interpolate(..)]` 标注合法性），未定义 body **怎么在源码层读到输入值、怎么把计算结果写进输出**。RXS-0159 IR4 明确把「真实读写 I/O 的语句级 codegen」划归 RD-013。要 lower body，必须先有一条 owner 裁定的源码层访问语义：着色函数体内，输入从何而来、输出向何处去。

**候选（带取舍，不选）**：

- **C1-a｜形参/返回值即 I/O（值语义，最贴现有 MIR）**：着色入口的输入聚合结构体是普通形参（`arg_count` 覆盖的 locals），输出是返回值；body 经普通 `Place`（形参 local + `ProjElem::Field` 投影读字段；构造返回聚合后 `return`）访问。
  - 取舍：与现有 host/device codegen 的形参/返回 lowering 同构，复用面最大、最少新语义；但要求「形参结构体字段 ↔ 输入签名元素」「返回结构体字段 ↔ 输出签名元素」的绑定（见 Q2）严格成立，且 SPIR-V 侧需把 `OpFunctionParameter`/返回值改写为 Input/Output 变量的 `OpLoad`/`OpStore`（emit 层非平凡）。
- **C1-b｜内建访问 intrinsic / 属性入口（贴 HLSL/SPIR-V 习惯）**：输入/输出经专门的 builtin 访问形态（如属性化入口参数 + 内建 load/store），body 不把 I/O 当普通值。
  - 取舍：更贴 DXIL/SPIR-V 的 input/output 变量模型，未来扩 builtin（`SV_*`）更自然；但引入新源码层构造 / 新语言面，明显超出「最小切片」，且与 RXS-0154「属性式标注、否决 type-level 包裹」既有裁决的关系需 owner 厘清。
- **C1-c｜显式 I/O intrinsic 读写（`load_input(field)` / `store_output(field, v)`）**：body 经编译器内建函数显式读写命名 I/O 元素。
  - 取舍：绑定关系最显式、最易做 strict-only 校验（命名直达）；但属新增语言/库面，最重，且与 RXS-0154 标注式 I/O 模型并存会产生两套 I/O 心智模型。

**对 strict-only（P-01 / RFC-0004 §4.4）的共同约束**：无论选哪条，body 对**已声明但未被读/写**的 I/O 元素的处置须可被 RXS-0159 IR2 强制签名一致性校验门（`dxil_sig_gate::check`）覆盖——未用输入元素的消除须落「显式诊断 vs 标准 DCE」既有判断（参见 RD-014 strict-only 取证里 owner 待裁的死 I/O 消除分类），不得静默漂移。

**待 owner**：⟨选 C1-a / C1-b / C1-c / 其他；并指明是否限定本期仅 C1-a 值语义最小面⟩

### Q2 — MIR place（形参字段序 / 返回字段序）↔ io_sig In/Out 元素的绑定规则

**问题**：`io_sig: Vec<IoSigElem>`（`field_name`/`kind`/`ty`/`dir`）由 AST 字段标注提取，与 body 的 `Place{ local, proj }`/`ProjElem::Field(idx)` 投影**无任何映射**。要 lower「读输入元素 → 算 → 写输出元素」，必须有一条规则把 body 触达的 MIR place 绑定到 `io_sig` 的具体 In/Out 元素。这是一条 owner 须裁的语言语义（哪个 place 对应哪个签名元素），而非机械转写。

**候选（带取舍，不选）**：

- **C2-a｜字段序绑定（positional）**：输入聚合形参的字段定义序 `ProjElem::Field(i)` ↔ `io_sig` 中 `dir==In` 元素的第 i 个；返回聚合字段序 ↔ `dir==Out` 元素第 i 个。绑定键 = 结构体字段声明序。
  - 取舍：实现最直接，`io_sig_for` 本就按字段序提取，天然对齐；但「字段序」成为隐式契约，重排字段即改绑定，且要求 `io_sig` 的 In/Out 子序与形参/返回字段序严格同序（须在 `io_sig_for` 侧固化此不变量并加断言/测试）。
- **C2-b｜字段名绑定（nominal，按 `field_name`）**：以 `IoSigElem.field_name` 与 MIR place 投影回指的源字段名做键绑定。
  - 取舍：与 RXS-0159/RXS-0160 既有「语义名等价为链接键、ABI 中立」口径一致（`semantic_name_matches`），对字段重排稳健、利于 strict-only 诊断；但 MIR 投影到源字段名的回指信息须在 `io_sig_for` 建模时一并保留（当前 `IoSigElem` 已存 `field_name`，但 `Place` 侧无名，需补 field-index→name 映射或在 lowering 时携带）。
- **C2-c｜lowering 时显式建表（intrinsic 直绑，配合 C1-c）**：若 Q1 选显式 I/O intrinsic，则绑定在 intrinsic 调用点直接携带目标 `io_sig` 元素标识，无需从 place 反推。
  - 取舍：绑定最无歧义；但依赖 Q1=C1-c，且把绑定责任前移到前端/HIR，范围更大。

**与校验门的关系**：无论选哪条，绑定规则须使 `dxil_sig_gate::check`（比对译后 ISG1/OSG1 与 MIR intent，比较域 = 语义名/系统值/被用输入元素，**不取**寄存器/mask/顺序）仍成立——即绑定**只**决定「哪个 place 对应哪个命名/系统值元素」，**不**决定该元素的寄存器/布局（那属 Q3 / §4.6 禁区）。

**待 owner**：⟨选 C2-a / C2-b / C2-c / 其他；并明确绑定键（字段序 or 字段名）与 In/Out 子序不变量⟩

---

### Q3 — 该绑定如何与 RFC-0003 §4.6 / RFC-0004 §4.6(a) 🔒 签名二进制 ABI 布局禁区划清界线

**问题**：RD-013 reason 自身把「完整 body 数据流降级」与「签名 ABI 布局（寄存器/偏移，RFC-0003 §4.6 🔒 FFI ABI 禁区）」登记为**耦合**。但最小切片只需「命名/系统值元素的读写数据流」，不需冻结布局值。需 owner 确认：Q2 的绑定是否可**只**承诺源码层元素存在性 + 语义名 + 系统值 + 方向（§4.6(a) 已裁的 conformance 面），而把寄存器编号/component mask/packing/字节偏移**留给 SPIR-V `Location`/`BuiltIn` 装饰 + 外部 pin 工具链 + D3D12 conformance** 决定——即 SPIR-V 层 `OpLoad`/`OpStore` 是否确属 **ABI 中立**。

**事实依据（只读，供裁断；不替 owner 落笔禁区）**：

- RFC-0004 §4.6(a) 已裁：Rurix **仅承诺**签名元素的存在性、语义名、系统值、插值与阶段链接契约；**不承诺**寄存器编号、顺序、component mask、packing、字节偏移；布局不合规即发 6xxx。
- RXS-0159 IR2 / RXS-0160 IR1 校验门比较域**显式排除**寄存器/mask/顺序（标注「ABI 中立，§4.6(a)」）。
- RXS-0161 IR1：`emit_spirv` 以 `Location`/`BuiltIn`/`UserSemantic` 装饰 emit Input/Output 变量；`Location` 是 SPIR-V 层逻辑槽位，**非** DXIL 容器内的 register/mask 物理布局（后者由 SPIR-V→HLSL→dxc 转译链按 conformance 派生）。

**候选（带取舍，不选）**：

- **C3-a｜判定「ABI 中立」成立 → 最小切片不触禁区**：裁定 body lowering 产 `OpLoad`(Input var)/算术/`OpStore`(Output var) 只读写**逻辑 I/O 变量**，物理 register/mask 由转译链 conformance 决定，故最小切片不进 §4.6 禁区，可经条款 PR（Mini 或随实现）推进，无需新 Full RFC。
  - 取舍：解锁路径最短、与 §4.6(a) 既有裁决自洽；但须 owner 明确「`OpLoad`/`OpStore` 的 `Location` 选择本身不构成布局承诺」这一论断成立，并在条款里写死「不冻结 location 数值为 stable 保证」。
- **C3-b｜判定耦合不可拆 → 须 owner 独立 Full RFC**：维持 RD-013 reason 的耦合登记，认为任何 body↔签名绑定都隐含布局承诺，须 §4.6 Full RFC 先行。
  - 取舍：最保守、零禁区风险；但 RD-013 解锁被 Full RFC 阻塞，周期最长，且可能与 §4.6(a)「布局属外部 conformance、不进条款」的既有裁决重复。
- **C3-c｜分层裁定**：最小切片（标量/向量、命名/系统值、无资源）按 C3-a 推进；任何需要显式 register/space/offset 的扩展（含资源绑定、root signature）停手归 §4.6 Full RFC（对齐 RD-018 bindless / RD-021 纹理边界）。
  - 取舍：兼顾解锁与守禁区，边界面最清晰；但须 owner 明确「最小切片」与「需布局」之间的精确触发线（如出现 `ProjElem` 越出标量/向量、或出现资源类 `IoSigKind` 即停手）。

**待 owner**：⟨裁定 SPIR-V `OpLoad`/`OpStore` 是否 ABI 中立（C3-a / C3-b / C3-c）；并指明触发停手升 §4.6 Full RFC 的精确边界⟩

### Q4 — 最小 rvalue 子集（Use/Const/标量·向量 BinaryOp）与 RX6001/RX6003 device codegen 子集边界的关系

**问题**：最小切片只 lower `Rvalue::Use` / `Rvalue::BinaryOp(BinOp, ..)` + `Operand::Const`（`mir.rs` 已建模）。device codegen 既有以 RX6001/RX6003 标识「host/device codegen 子集边界」（子集外构造拒绝）。需 owner 裁定：图形=B 着色 body 的可降级 rvalue 子集，是**复用** device codegen 子集边界（同一套 RX6001/RX6003 语义 + 错误码），还是 graphics=B **另立**子集判定（落 6xxx 段新可达类别）。这决定「子集外构造」在着色 body 里发什么码、归谁管。

**候选（带取舍，不选）**：

- **C4-a｜复用 device codegen 子集边界（RX6001/RX6003）**：图形 body 的 rvalue/语句子集判定与 device codegen 同源，子集外即沿既有 RX6001/RX6003 通道拒绝。
  - 取舍：零新码、与 RD-013 reason「需 device codegen 语句降级扩展，对齐 RX6001/RX6003 子集边界」的登记直接呼应，复用面最大；但要求 graphics=B 与 device(NVPTX) 后端共享子集定义——而 RFC-0003 §4.1 明确两后端**不共享后端内部 lowering**，故须 owner 确认「子集边界」是前沿共享语义（可复用）还是后端内部逻辑（不可复用）。
- **C4-b｜graphics=B 另立子集 + 新 6xxx 可达类别**：B 路按自身可降级子集判定，子集外构造落 6xxx 段新码（按真实可达类别只追加 + 双语 message-key，不预造）。
  - 取舍：与 §4.1「后端各自从 MIR 独立降级」自洽，B 路子集可独立演进；但新增错误码 + message-key，且与 device codegen 子集出现「同一 MIR 构造两后端判定不一」时的用户心智须 owner 厘清。
- **C4-c｜本期仅 `Use`/`Const`/标量·向量 `BinaryOp` 白名单，其余一律 6xxx 拒绝（不区分 host/device 来源）**：不引用 RX6001/RX6003 语义，只在 B 路 emit 阶段对白名单外构造发既有 B 路 codegen 失败码（RXS-0159/0161 段，如 RX6013「不可映射」或 RX6007）。
  - 取舍：最小、最不牵动 device 子集语义；但「白名单」需在条款里精确枚举，且与未来扩子集时的码归属须预留说明。

**待 owner**：⟨裁定子集边界归属（C4-a 复用 RX6001/RX6003 / C4-b 另立新码 / C4-c 白名单+既有 B 路码）；并确认「子集」属前沿共享语义还是后端内部逻辑（关系 RFC-0003 §4.1）⟩

---

## 3. 裁定后的下游路径（DoD 提示，供 owner 参考；本 PR 不执行）

owner 裁定 Q1–Q4 后，一个真正解锁 RD-013 的实现链（仍须 owner 终审）大致为：

1. **条款 PR（先于实现，硬规则 7）**：据裁定在 `spec/shader_stages.md` / `spec/dxil_backend.md` 落 body I/O 访问语义 + MIR↔签名绑定条款体（保号或续号由 Q-Range 类裁决定），**不**冻结 §4.6 布局；每条 ≥1 `//@ spec` 锚定。
2. **实现 PR**：把 `body.blocks`+`locals` threading 进 B 路（`emit_dxil_b`→`run_b_chain`→`emit_spirv`），lower 最小 rvalue 子集到 SPIR-V `OpLoad`/`OpConstant`/算术/`OpStore`，保持 spirv-val 干净 + RXS-0159 IR2 校验门通过。
3. **golden + 红绿**：validator-accepted 端到端 golden，须 owner pin 签名 DXC 环境 bless（本机仅能标 NOT BLESSED / pending-human-review，不能闭合 RD-013）。

以上仅为路径提示；**Q1–Q4 未裁前不得起草上述条款体或代码**（避免发明语义，硬规则 1）。

## 4. 与硬门 / 红线的关系（防降级声明）

- **不签 G-G2-4**：本决策包不触 G-G2-4 任何验收项（不禁手写 HLSL/DXIL 之外另起、不固定像素、不 host-only、不 fullscreen copy、不窗口截图、不以 SKIP 充 green）；G-G2-4 签字、device run URL、CI step 48 均归 owner。
- **不实现 PR-F2、不替代 CI step 48**。
- **不裁档**：Q1–Q4 及其整体档位（Mini vs Full RFC）属 owner（硬规则 8 向上取严）；Q3 已标注或须 Full RFC 处置。
- **不碰 RD-021 / §4.6 ABI 禁区本体 / 纹理内存模型**；最小切片仅作用标量/向量 I/O，不触资源采样。
- **AI 代录非代签**：本决策包以 owner 裁定 Q1–Q4 + 合并本 PR 生效；RD-013 维持 open。

## 5. 引用

- 上游输入：`evidence/rd013_body_lowering_preflight_20260628.md`
- registry：`registry/deferred.json` RD-013（reason / backfill_condition / history）；本 PR 追加 history 留痕 + revision_log v1.36
- 条款：`spec/dxil_backend.md` RXS-0159 / RXS-0160 / RXS-0161 / RXS-0162；`spec/shader_stages.md` RXS-0154 / RXS-0155
- RFC：RFC-0003 §4.1 / §4.6；RFC-0004 §4.4 / §4.6(a)
- 体例参考：`rfcs/mini-0002-engine-integration.md`
- 只读事实依据：`src/rurixc/src/{dxil_spirv.rs,dxil_codegen.rs,mir.rs,mir_build.rs}`
