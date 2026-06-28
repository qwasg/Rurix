# Rurix 语言规范 — 绑定布局推导语义面（descriptor / root signature；G2.3 起）

> 条款:**RXS-0163 ~ RXS-0166 计划区间**(G2.3 绑定布局推导语义面:资源句柄 → SPIR-V 资源绑定降级 / register-space 分配推导 / root signature 形态推导 + RTS0 序列化 / 绑定布局一致性校验门 + strict-only 推导失败)。体例见 [README.md](README.md)。
> 依据:**[RFC-0005](../rfcs/0005-binding-layout-inference.md)**(绑定布局推导,owner Approved 2026-06-28);06 §8.2(descriptor / root signature 编译器推导,P-11 单一事实源);04 P-01(strict-only);04 P-11(host 绑定结构 ↔ shader 布局单一事实源);[RFC-0002](../rfcs/0002-shader-stages.md) RXS-0156(资源句柄类型面);[RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md)(图形=B codegen 与禁区边界);[dxil_backend.md](dxil_backend.md) RXS-0157~0162(DXIL B 链)。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)(D-G2-3,G-G2-3)+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.3 子里程碑。
> 档位:**Full RFC**(RFC-0005;10 §3:本设计触新 codegen 推导面,并触及签名/绑定二进制 ABI 布局、纹理路径内存模型映射、DXIL/SPIR-V UB 边界等硬规则 5 禁区边界)。RFC-0005 已由 owner(Language Lead)于 2026-06-28 批准并裁决 §9 全部路径项。**AI 无权自判 Direct**,判档以 RFC-0005 与 G2_CONTRACT 授权为据,判档争议向上取严。任何触及 **register/space/mask/packing/descriptor table 偏移/root parameter DWORD 物理布局** / **纹理路径内存模型映射(06 §4.2)** / **DXIL-SPIR-V UB 边界** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):不可推导 / 超上限 / register-layout 冲突 / PSV0 mismatch 以编译期 6xxx codegen 诊断(P-01 strict-only,无运行期 fallback)定义。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 >=1 测试锚定(`//@ spec: RXS-####`)。**本轮 PR-E2a 已落带编号条款体 RXS-0163~0166 + host 侧 safe 推导逻辑 + 每条 ≥1 测试锚定**(FLS 体例,见 §2)——承 PR-E1 脚手架(commit ca43fc2)。本轮**不接线生产 codegen emit、不改 `registry/error_codes.json`(错误码占位「6xxx」)、不落 golden、不 device 真跑**——生产接线 + 错误码落码 + golden/bless + device 真跑/run URL 归 **PR-E2b**(owner 闸门,G-G2-3)。

---

## 1. 范围与编号区间

本文件承载 **绑定布局推导** 的语义条款(G2.3+,D-G2-3)。绑定布局推导把 RXS-0156 资源句柄类型面与 RFC-0004 图形=B codegen 链连接起来,由编译器从 shader 资源使用推导 D3D12 descriptor / root signature,兑现 P-11 单一事实源:host 绑定结构与 shader 布局不手维护两份、不静默漂移。

覆盖语义面(RFC-0005 §4 / §9):

- **资源句柄 → SPIR-V 资源绑定降级面**:RXS-0156 的 `Texture2D<F>` / `Sampler` / constant buffer / structured buffer 等资源使用降级为 SPIR-V opaque 资源类型与 `DescriptorSet`/`Binding` 装饰。当前 Rurix MIR→SPIR-V 资源绑定结构仍为待建面,不得冒充已实测。
- **register/space 分配推导**:§9 Q-Space 裁决为按资源种类分轴;首期默认单 set/`space0`,CBV/SRV/UAV/Sampler 分别走 `b/t/u/s` 轴并按声明序各自从 0 递增。多 space 与 `#[binding(...)]` 显式覆盖不进本期。
- **root signature 形态推导 + RTS0 序列化**:§9 Q-RootShape 裁决为 CBV root descriptor + SRV/UAV descriptor table + Sampler descriptor table;§9 Q-Sampler 裁决为 `Sampler` 默认 dynamic sampler。root constant 与 static sampler 后期独立判档。
- **一致性校验门 + strict-only 推导失败**:使用 PSV0 资源绑定反射与推导意图交叉校验;不可推导、超 root signature 64 DWORD 上限、register/layout 冲突、PSV0 mismatch → 6xxx codegen 诊断,无运行期 fallback。

明确不在本文件落语义本体的范围:

- **绑定二进制 ABI 布局禁区**:register/space/mask/packing、descriptor table 字节偏移、root parameter DWORD 物理布局、descriptor heap 编码均不冻结为 stable 语言保证。
- **纹理路径内存模型映射**:采样/load/store opcode、缓存一致性、LOD/导数、越界采样后果、memory-order 留独立 Full RFC。
- **bindless / unbounded descriptor array / descriptor heap 直索引**:本期 defer 至 RD-018;不登记 SG-010 gating,不永久/条件裁剪该方向。
- **PSO / resource state / barrier 运行时面与 UC-04 deferred renderer**:本文件仅覆盖绑定布局推导 spec 面,device 真跑出图归 G-G2-3 / G2.4 后续证据。

**编号区间**:本文件条款为 **RXS-0163 ~ RXS-0166**(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;当前最高现存 RXS-0162 @ [dxil_backend.md](dxil_backend.md))。本轮 **PR-E2a 已落带编号条款体**(下文 §2,FLS 体例),每条 ≥1 `//@ spec` 测试锚定(host 侧 safe 推导单测,`src/rurixc/src/binding_layout.rs`)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 本节落带编号条款体(PR-E2a)。各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(10 §7.5:不可推导 / 超上限 / register-layout 冲突 / PSV0 mismatch 以编译期 6xxx codegen 诊断定义,P-01 strict-only,无运行期 fallback)。本轮 host 侧 safe 推导落 [`src/rurixc/src/binding_layout.rs`](../src/rurixc/src/binding_layout.rs)(纯 host/safe,零新 unsafe,照 RXS-0160 `check_stage_link` 范本),**不接线生产 codegen emit、不改 `registry/error_codes.json`(错误码占位「6xxx」)、不落 golden、不 device**(均归 PR-E2b / owner,G-G2-3)。**本片不碰** 🔒 register/space/mask/packing 数值物理布局 / descriptor table 字节偏移 / root parameter DWORD 物理布局 / 纹理路径内存模型映射(06 §4.2)/ DXIL·SPIR-V UB 边界——只引边界声明,触及物理布局冻结为 stable / 触 UB 即停手标「需人工升档」,不在本文件自落笔。

### RXS-0163 资源句柄 → SPIR-V 资源绑定降级面

RXS-0156 资源句柄(`Texture2D<F>` / `Sampler` / constant buffer / structured buffer)降级为 SPIR-V **opaque** 资源类型 + `DescriptorSet`/`Binding` 装饰,按 io_sig 声明序确定性导出。本条只承诺**绑定装饰的存在性 / 资源种类归类 / 声明序确定性**;🔒 具体 `DescriptorSet`/`Binding` 数值物理布局**不属本条承诺**(实现确定、gate 后、非 stable,不冻结为 ABI)。

#### Syntax

资源绑定降级为 codegen 面,非语言文法面:资源句柄由着色阶段签名形参(RXS-0156)给定,不因降级改写。

#### Legality

- L1(可降级子集):`Texture2D<F>`(F = 已建模标量分量类型)/ `Sampler` / constant buffer / structured buffer(只读 → SRV / 可写 → UAV)的单 descriptor([`ResourceCount::One`])与有界数组([`ResourceCount::Bounded(n)`])可降级。
- L2(不可映射,strict-only):unbounded / bindless descriptor array([`ResourceCount::Unbounded`])→ **RD-018** defer → 占位「6xxx」(计划复用 RX6013 `codegen.dxil_unmappable`,落码归 PR-E2b);非建模资源种类同理。无 fallback,不发明 descriptor heap 编码。
- 🔒(布局边界):承诺具体 `DescriptorSet`/`Binding` 数值越出本条 = RFC-0004 §4.6(a) 同级 ABI 禁区,**需人工升档**;本条仅作存在性 / 种类 / 确定性边界声明。

#### Dynamic Semantics

资源绑定降级为编译期确定性变换,本条无运行期语言语义(着色器在 D3D12 管线的资源访问属运行时 / G2.4)。给定资源使用,SPIR-V `DescriptorSet`/`Binding` 装饰对相同输入确定(两次推导一致)。

#### Implementation Requirements

- IR1(确定性导出):host 侧 [`binding_layout::infer_spirv_bindings`](../src/rurixc/src/binding_layout.rs)`(&[ResourceBinding])` 按声明序导出每资源 `SpirvBinding { set: 0, binding }`——首期单 set,`binding` 自 0 起按声明序递增(有界数组占 `count` 个连续 binding);纯 host/safe,零新 unsafe。
- IR2(strict-only):unbounded / bindless → `BindingInferError::Unmappable`(RD-018);无运行期 fallback。
- IR3(不接线生产 emit):本条 host 推导**不**接 [`dxil_spirv::emit_spirv`](../src/rurixc/src/dxil_spirv.rs) 资源绑定装饰生产路径(归 PR-E2b);错误码占位「6xxx」不落 `registry/error_codes.json`。
- IR4(测试锚定):≥1 `//@ spec: RXS-0163`——`infer_spirv_bindings` 单测(accept 确定性 set/binding + 有界数组跨多 binding + reject unbounded → `Unmappable`)。

### RXS-0164 register/space 分配推导

资源使用经 **§9 Q-Space=B 按资源种类分轴**推导 D3D12 register/space:CBV/SRV/UAV/Sampler 分别走 `b`/`t`/`u`/`s` 轴,各轴按声明序自 0 递增,首期单 `space0`。本条只承诺**分轴归类 / 声明序确定性 / register-layout 冲突核验**;🔒 具体 register/space 数值物理布局**不属本条承诺**(实现确定、gate 后、非 stable)。

#### Syntax

register/space 分配为 codegen 面,非语言文法面。

#### Legality

- L1(分轴):CBV→`b` / SRV(纹理 + 只读 structured buffer)→`t` / UAV(可写 structured buffer)→`u` / Sampler→`s`,各轴自 0 递增,首期单 `space0`;多 space 与 `#[binding(...)]` 显式覆盖不进本期(后期独立判档)。
- L2(register/layout 冲突,strict-only):同轴 + 同 space 内 register 区间重叠 → 占位「6xxx」(register/layout 冲突,新真实可达类别,落码归 PR-E2b)。无 fallback。
- L3(不可映射):unbounded / bindless → 占位「6xxx」(承 RXS-0163 L2,RD-018)。
- 🔒(布局边界):承诺具体 register/space 数值越出本条 = ABI 禁区,**需人工升档**;本条仅作分轴 / 确定性 / 冲突核验边界声明。

#### Dynamic Semantics

register/space 分配为编译期确定性变换,本条无运行期语言语义。给定资源使用,register/space 分配对相同输入确定。

#### Implementation Requirements

- IR1(分轴递增):[`binding_layout::infer_register_assignments`](../src/rurixc/src/binding_layout.rs)`(&[ResourceBinding])` 按声明序、四轴(`b`/`t`/`u`/`s`)各自从 0 递增分配 register 基号,space 恒 0,有界数组占 `count` 个连续号;纯 host/safe。
- IR2(冲突门,strict-only):[`binding_layout::detect_register_conflict`](../src/rurixc/src/binding_layout.rs)`(&[RegisterAssignment])` 核实同轴 + 同 space 内 `[register, register + span)` 半开区间不重叠;重叠 → `BindingInferError::RegisterConflict`(无 fallback)。不同轴同号不构成冲突(`b0` 与 `t0` 互不干扰,ABI 中立)。
- IR3(不接线生产 emit):host 推导**不**接 register/space 生产 codegen 路径(归 PR-E2b);错误码占位「6xxx」不落 registry。
- IR4(测试锚定):≥1 `//@ spec: RXS-0164`——accept(按种类分轴确定性分配 + 有界数组消费 span + 跨轴不冲突)+ reject(同轴区间重叠 → `RegisterConflict` / unbounded → `Unmappable`)。

### RXS-0165 root signature 形态推导 + RTS0 序列化

资源使用经 **§9 Q-RootShape=B** 推导 root signature 形态:每个 CBV → CBV root descriptor;全部 SRV + UAV → 单一 descriptor table(SRV range 先于 UAV range);全部 Sampler → 独立 descriptor table(**§9 Q-Sampler=B dynamic sampler**;root constant 与 static sampler 后期独立判档)。形态经 RTS0 容器按 D3D12 既定格式机械序列化。本条只承诺**形态结构 / 参数序确定性 / 64 DWORD 上限核验 / RTS0 容器机械序列化**;🔒 具体 descriptor table 字节偏移 / root parameter DWORD 物理布局**不属本条承诺**(实现确定、gate 后、非 stable)。

#### Syntax

root signature 推导与 RTS0 序列化为 codegen / 容器面,非语言文法面。

#### Legality

- L1(形态):每个 CBV → CBV root descriptor;全部 SRV+UAV → 单一 descriptor table(SRV range 先于 UAV range);全部 Sampler → 独立 descriptor table(D3D12 sampler 必须独表)。参数序确定:CBV root descriptors → SRV/UAV 表 → Sampler 表。
- L2(超上限,strict-only):root signature 推导 DWORD 成本(CBV root descriptor = 2 DWORD / descriptor table = 1 DWORD)> 64 → 占位「6xxx」(超 root signature 上限,新真实可达类别,落码归 PR-E2b)。无 fallback。
- L3(不可映射):unbounded / bindless → 占位「6xxx」(承 RXS-0163 L2,RD-018)。
- 🔒(布局边界):承诺具体 descriptor table 字节偏移 / root parameter DWORD 物理布局越出本条 = ABI 禁区,**需人工升档**。**RTS0 序列化按 D3D12 既定容器格式机械落字节**(类比 [dxil_backend.md](dxil_backend.md) RXS-0162 DXIL 容器),其布局为**实现确定、gate 后、非 stable**,不自创 ABI;真链 validator / device 核验归 PR-E2b。

#### Dynamic Semantics

root signature 推导与 RTS0 序列化为编译期确定性变换,本条无运行期语言语义。给定资源使用,root signature 形态与 RTS0 字节对相同输入确定(两次序列化字节全等)。

#### Implementation Requirements

- IR1(形态推导):[`binding_layout::infer_root_signature`](../src/rurixc/src/binding_layout.rs)`(&[ResourceBinding])` 按 Q-RootShape=B 导出 `RootSignature { parameters, flags: 0 }`;纯 host/safe。
- IR2(成本核验):[`binding_layout::root_signature_cost_dwords`](../src/rurixc/src/binding_layout.rs) 计 DWORD 成本,> 64(`ROOT_SIGNATURE_DWORD_LIMIT`)→ `BindingInferError::RootSignatureTooLarge`(strict-only)。
- IR3(RTS0 序列化):[`binding_layout::serialize_rts0`](../src/rurixc/src/binding_layout.rs)`(&RootSignature)` 按 D3D12 既定容器格式机械落字节(外层 DXBC 容器 + 单一 `RTS0` part,载荷为 versioned root signature v1.0 序列化形态);DXBC 16 字节摘要为零占位(真实摘要归 PR-E2b),descriptor range offset 取 D3D12 `APPEND` 哨兵不冻结物理偏移;纯 host/safe。
- IR4(不接线生产 emit):host 推导与 RTS0 序列化**不**接 root signature 生产 emit / RTS0 入容器生产路径(归 PR-E2b);不落 golden、不 device、错误码占位「6xxx」不落 registry。
- IR5(测试锚定):≥1 `//@ spec: RXS-0165`——accept(Q-RootShape=B 形态 + RTS0 确定性 + DXBC/`RTS0` 容器结构可解码回参数计数)+ reject(超 64 DWORD → `RootSignatureTooLarge`)。

### RXS-0166 绑定布局推导一致性校验门 + strict-only 推导失败

绑定布局推导意图(RXS-0164 register/space 分配)与产物侧 **PSV0** 资源绑定反射经一致性校验门交叉比对:推导意图须在 PSV0 反射中等价兑现(无静默漂移 / 缺失 / 多出),否则 strict-only 失败。本条只承诺**「推导意图 ↔ 产物反射」内部一致性核验**;🔒 具体 register/space 数值为实现确定、gate 后、非 stable,本门**不**把数值冻结为 stable 语言 ABI。

#### Syntax

一致性校验门为 codegen 面,非语言文法面。

#### Legality

- L1(可核验):RXS-0164 推导意图(种类轴 / register / space / span)与 PSV0 反射的资源绑定(class / register / space / count)。
- L2(PSV0 mismatch,strict-only):PSV0 反射缺失 / 多出 / 与意图失配(资源数不等 / register·space·count 不等价)→ 占位「6xxx」(PSV0 mismatch,新真实可达类别,落码归 PR-E2b)。
- L3(不可推导 / 超上限 / 冲突):承 RXS-0163~0165——不可映射 / 超 64 DWORD / register-layout 冲突在推导阶段即占位「6xxx」strict-only 失败,无 fallback。
- 🔒(布局边界):本门比对推导意图与反射的内部一致性,**不**把 register/space 数值冻结为 stable ABI;触及物理布局冻结为 stable → **需人工升档**。

#### Dynamic Semantics

一致性校验门为编译期确定性变换,本条无运行期语言语义(着色器在 D3D12 管线的绑定消费属运行时 / G2.4;运行期等价由 G-G2-3 device 真跑兑现,归 PR-E2b)。给定推导意图与 PSV0 反射,校验结论对相同输入确定。

#### Implementation Requirements

- IR1(校验门,不可裁剪):[`binding_layout::check_binding_consistency`](../src/rurixc/src/binding_layout.rs)`(intent: &[RegisterAssignment], reflected: &Psv0Reflection)` 比对推导意图与 PSV0 反射:资源数须相等,每个意图须在反射中以等价 `(class, register, space, span)` 出现(顺序无关);失配 → `BindingInferError::Psv0Mismatch`(strict-only,绝不静默通过)。
- IR2(strict-only 推导失败):不可推导 / 超上限 / 冲突 / PSV0 mismatch 均 strict-only,无运行期 fallback(P-01);本条 host 校验门纯 host/safe,零新 unsafe。
- IR3(不接线生产 emit):本门**不**接 PSV0 校验门生产 emit(归 PR-E2b);错误码占位「6xxx」不落 registry、不落 golden、不 device。
- IR4(测试锚定):≥1 `//@ spec: RXS-0166`——accept(PSV0 反射与推导意图一致,顺序无关)+ reject(register 失配 / 资源数失配 → `Psv0Mismatch` 真实红绿)。

## 3. 裁决摘要与实现门控

- **Feature gate**:复用 `dxil-backend`;不新增 `binding-layout` 子 gate。
- **错误码策略**:绑定布局推导失败归 6xxx codegen 段,按实现时 registry 实际最高空号续;**PR-E2a 不预留、不预造、不落码**(条款体占位「6xxx」)。不可映射资源计划复用 RX6013;超 64 DWORD、register/layout 冲突、PSV0 mismatch 等新真实可达类别新开码——落码归 **PR-E2b**(避开 RX6014 与 RXS-0160 争号)。
- **Bindless**:defer 至 RD-018;本期遇到 bindless / unbounded descriptor array / descriptor heap 直索引保持 deferred/out-of-scope,以占位「6xxx」诊断显式拒绝(host 推导 `BindingInferError::Unmappable`)或保持结构上不可达。
- **显式标注覆盖**:`#[binding(...)]` 不进本期。推导优先;覆盖能力后期独立判档。
- **PR 序**:PR-E1(commit ca43fc2)= spec 脚手架(文件名 + 计划区间 + registry RD-018/RFC 记录);**PR-E2a(本轮)= 带编号条款体 RXS-0163~0166 + host 侧 safe 推导逻辑([`binding_layout.rs`](../src/rurixc/src/binding_layout.rs))+ 每条 ≥1 单测锚定**(不接线生产 emit、不改 registry、不落 golden、不 device);**PR-E2b(owner 闸门)= 生产 codegen 接线 + 6xxx 错误码落码 + golden/bless + device 真跑/run URL**(G-G2-3 闭环)。

---

## 4. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-28 | 新建 binding_layout.md（PR-E1 spec 脚手架，承 [RFC-0005](../rfcs/0005-binding-layout-inference.md)，owner Approved 2026-06-28）：登记文件名 + G2.3 绑定布局推导语义面说明 + **RXS-0163~0166 计划区间**（资源句柄→SPIR-V 资源绑定降级 / register-space 分配推导 / root signature 形态推导+RTS0 / 一致性校验门+strict-only）。**仅登记计划映射，不落带编号裸条款头**——条款体与每条 >=1 测试锚定随 PR-E2 同落，trace_matrix 维持全锚定。同步 owner 裁决摘要：Q-Space=B / Q-RootShape=B / Q-Sampler=B / Q-Bindless=A→RD-018 / Q-Gate=A / Q-Err=6xxx 续号策略 / Q-File=B / Q-Range=4 条 / Q-Inference-vs-Explicit=C。禁区不动：绑定二进制 ABI 布局 / 纹理路径内存模型 / DXIL-SPIR-V UB 边界只作边界声明。 | **Full RFC**（RFC-0005 / PR-E1） |
| v1.1 | 2026-06-28 | **PR-E2a：§2 计划映射升格为带编号条款体 `### RXS-0163 ~ ### RXS-0166`**（FLS 体例，按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节，**严禁 UB 节**，Legality 引 RX 码或占位「6xxx」，镜像 [dxil_backend.md](dxil_backend.md) RXS-0157~0162 先例）——RXS-0163 资源句柄→SPIR-V 资源绑定降级面（opaque 资源类型 + DescriptorSet/Binding 装饰，按 io_sig 声明序确定性）/ RXS-0164 register/space 分配推导（§9 Q-Space=B 按资源种类分轴，CBV/SRV/UAV/Sampler→b/t/u/s 各自从 0 递增，首期单 space0 + register/layout 冲突核验）/ RXS-0165 root signature 形态推导（§9 Q-RootShape=B CBV root descriptor + SRV/UAV/Sampler descriptor table；§9 Q-Sampler=B dynamic sampler）+ RTS0 容器机械序列化（D3D12 既定 DXBC+RTS0 容器格式，类比 RXS-0162 DXIL 容器，非 stable）/ RXS-0166 绑定布局推导一致性校验门（PSV0 反射 vs 推导意图比对）+ strict-only 推导失败。配套 **host 侧 safe 推导逻辑**落 [`src/rurixc/src/binding_layout.rs`](../src/rurixc/src/binding_layout.rs)（纯函数 over `mir::ResourceBinding`，零新 unsafe，照 RXS-0160 `check_stage_link` 范本）+ `mir.rs` 增资源种类建模（`ResourceClass`/`MirResourceType`/`ResourceCount`/`ResourceBinding`，仅数据建模，不改既有标量/向量 I/O 路径语义）+ 每条 ≥1 `//@ spec: RXS-####` 单测锚定（accept + reject 真实红绿，feature gate 复用 `dxil-backend`，**trace_matrix 162→166 全锚定**）。**本轮不接线生产 codegen emit**（emit_spirv 资源绑定装饰 / register-space 生产路径 / PSV0 校验门生产 emit 归 PR-E2b）、**不改 `registry/error_codes.json`**（错误码占位「6xxx」，落码归 PR-E2b/owner，避开 RX6014 与 RXS-0160 争号）、**不落 golden、不 device 真跑/bless**（归 PR-E2b/owner，G-G2-3）；bindless 维持 defer（RD-018），遇到即 host 推导 `Unmappable` 占位「6xxx」拒绝或结构不可达。禁区只作边界声明：register/space/mask/packing 数值物理布局 / descriptor table 偏移 / root parameter DWORD 物理布局 / 纹理路径内存模型 / DXIL·SPIR-V UB 不冻结为 stable（以「实现确定、gate 后、非 stable」表述）。deferred.json / spike_gating.json 不动，不开 SG-010；不碰 00–14。 | **Full RFC**（RFC-0005 / PR-E2a） |
