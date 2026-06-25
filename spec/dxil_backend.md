# Rurix 语言规范 — DXIL 第二后端语义面（MIR → DXIL，承 RFC-0003；G2.2 起）

> 条款:**RXS-0157 起续号预留**(G2.2 DXIL 第二后端语义面:codegen target 分发与 DXIL 后端分叉(MIR 之后 target 选择,PTX/DXIL 并存,strict-only 无 fallback)/ 着色阶段着色 → DXIL 着色器类型降级对应 / 阶段 I/O → DXIL 签名·系统值语义降级 / 阶段间接口 → DXIL 阶段链接一致性核对)。区间大小未锁定(RFC-0003 §9 Q-Range:随路径裁定与条款数一并定),**PR-C2 分片1 起**已落首条带编号条款体 **RXS-0157**(§2)。体例见 [README.md](README.md)。
> 依据:**[RFC-0003](../rfcs/0003-dxil-backend.md)**(MIR→DXIL 第二后端,**owner Approved** 2026-06-23;§9 Q-D131 路径裁决 = **A**(LLVM DirectX 后端直接 emit DXIL),经独立勘误回填 §9 / 13 §D-131);06 §8.2(codegen 第二后端设计预留);06 §4.2(纹理路径内存模型禁区,🔒);07 §7(device codegen 分发:MVP/G1 维持 NVPTX→PTX,DXIL 第二后端 G2 重评估);07 §5(错误码段位:6xxx codegen/目标);spec/shader_stages.md RXS-0153~0156(着色阶段类型面,DXIL 降级的上游类型面来源)。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)(D-G2-2,G-G2-2)+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.2 子里程碑。
> 档位:**Full RFC**(RFC-0003;10 §3:本设计触 **codegen 第二后端 + target 分发**,有别于 M8 互操作的 Direct)。**D-131 路径已裁 A**(owner 凭 G2.2 round-1~8 双路 spike 证据裁定,RFC-0003 §9 Q-D131 C→A + 13 §D-131 待决→A,经独立勘误 PR;A 路 dev 工具链偏差由 `registry/deferred.json` RD-011 跟踪)。**AI 无权自判 Direct**,判档以 RFC-0003 与 G2_CONTRACT 授权为据,判档争议向上取严。任何触及 **🔒 纹理路径内存模型映射(06 §4.2)** / **FFI ABI 二进制布局(RFC-0003 §4.6/§9 Q-Builtin)** / **多后端架构承诺(D-008/SG-003)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):target 不支持 / 降级失败 / 非法 I/O 映射以 **编译期 6xxx codegen 诊断(P-01 strict-only,无运行期 fallback)**定义,不以 UB 表述(RFC-0003 §3/§4)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**PR-C2 分片1 起**已落带编号条款体 **RXS-0157**(codegen target 分发)、**分片2 RXS-0158**(阶段→着色器类型)、**分片3 RXS-0159**(阶段 I/O → 签名/系统值语义,类型面)+ 每条 ≥1 测试锚定(条款 PR 先于/同实现 PR,G2.1 PR-B2 先例),trace_matrix 全锚定 **159/159**;后续条款(RXS-0160)随分片续落。

---

## 1. 范围与编号区间

本文件承载 **MIR → DXIL 第二后端语义面**的语义条款(G2.2+,D-G2-2)。承 RFC-0002 着色阶段类型面(RXS-0153~0156)产出的着色阶段语义,定义其经工具链降级到 DXIL(可被 D3D12 PSO 消费的着色器对象)的语义。覆盖语义面(RFC-0003 §4):

- **codegen target 分发与 DXIL 后端分叉**:MIR 之后的 target 选择(`rx build --target dxil`,与现状 `--target ptx` 并列,RFC-0003 §9 Q-CLI),PTX 后端(D-207)与 DXIL 后端**并存**;target 不支持的构造 → **6xxx codegen 错误**(strict-only,无运行期 fallback,P-01)。DXIL 后端 gate 于 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate;未启用时 DXIL 后端不参与编译,PTX 路径不受影响)。
- **着色阶段着色 → DXIL 着色器类型降级对应**:RXS-0153 的着色阶段着色(`vertex`/`fragment`/`compute`/`mesh`/`task` + RT `raygen`/`closesthit`/`anyhit`/`miss`)降级为对应 DXIL 着色器类型(vertex/pixel/mesh/amplification/RT/compute shader)。精确对应表随 PR-C2 条款体落地。
- **阶段 I/O → DXIL 签名/系统值语义降级**:RXS-0154 的阶段专属 I/O(`#[interpolate]` 插值限定 / `#[builtin]` 内建变量)降级为 DXIL 输入/输出签名与系统值语义(SV_*)。内建变量 → DXIL 系统值语义名为**类型面映射**;其二进制 ABI 布局属 RFC-0003 §4.6/§9 Q-Builtin 的 🔒 FFI ABI 禁区,**不在本文件**,留 owner 后续独立 Full RFC。
- **阶段间接口 → DXIL 阶段链接一致性核对**:RXS-0155 的阶段间接口契约(vertex out → fragment in varying 兼容)在类型面已编译期校验(RXS-0155),DXIL 层为降级一致性核对。

全部 DXIL 后端语义维持 **D-131 = A 路径(LLVM DirectX 后端直接 emit DXIL)**:与 NVPTX 后端同构、D-205 LLVM 单栈、无第二中间 IR(RFC-0003 §9 Q-D131 裁 A)。**🔒 纹理/采样器内存模型映射**(06 §4.2 禁区:tex proxy / 采样 opcode / 描述符编码 / 缓存一致性 / UB)、**内建变量/根参数/常量缓冲二进制 ABI 布局**(RFC-0003 §4.6 FFI ABI 禁区)、**绑定布局推导实现**(G2.3,P-11)均**不在本文件**;DXIL golden 取**文本反汇编形态** + 经 dxc validator 验证后入 golden(RFC-0003 §9 Q-Golden)。target 不支持 / 降级失败 / 非法 I/O 映射以 **编译期 6xxx codegen 诊断(P-01 strict-only)**定义,**不以 UB 表述**(§4)。

**编号区间**:本文件条款自 **RXS-0157** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0156 @ [shader_stages.md](shader_stages.md))。**区间大小未锁定**(RFC-0003 §9 Q-Range:随 owner 与路径裁定一并定,路径选择可能影响降级面条款拆分粒度)。**PR-C2 分片1~3 起**已落带编号条款体 **RXS-0157**(codegen target 分发与 DXIL 后端分叉,§2)/ **RXS-0158**(阶段→着色器类型)/ **RXS-0159**(阶段 I/O → 签名/系统值语义,类型面);RXS-0160(阶段间接口)仍为 §9 Q-Range 计划映射(非裸条款头),随后续分片落地。条款体与每条 ≥1 测试锚定随实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 全锚定 159/159)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 本节自 PR-C2 分片1 起落带编号条款体。分片1 落 **RXS-0157**(codegen target 分发与 DXIL 后端分叉);**分片2 落 RXS-0158**(着色阶段着色 → DXIL 着色器类型降级对应);**分片3 落 RXS-0159**(阶段 I/O → DXIL 签名/系统值语义降级,类型面);RXS-0160(阶段间接口)仍为 §9 Q-Range 待锁定的**计划映射**(非裸条款头,仅计划登记,详见 RFC-0003 §5),随后续分片落地。
> 各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(target 不支持 / 降级失败以编译期 6xxx codegen 诊断定义,P-01 strict-only,无运行期 fallback;10 §7.5)。**本片不碰** 🔒 纹理内存模型映射(06 §4.2 禁区)/ FFI ABI 二进制布局(RFC-0003 §4.6 / §9 Q-Builtin)/ 绑定布局推导(G2.3,P-11);触及即停手升档。

### RXS-0157 codegen target 分发与 DXIL 后端分叉

MIR 之后按目标(target)选择 codegen 后端:现状 NVPTX→PTX 后端(D-207)与 DXIL 后端(本条)**并存**。target 选择经 `rx build --target <ptx|dxil>` 显式给定(RFC-0003 §9 Q-CLI),无隐式多目标、无静默 fallback(P-01 strict-only)。DXIL 后端 gate 于 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate);未启用时 DXIL 后端不参与编译,`--target dxil` 报 target 不可用诊断,PTX 路径(D-207)不受影响。

本条覆盖 **codegen 分发与后端分叉的语义骨架** + **最小 compute kernel 端到端**(空体 compute 入口 MIR → DirectX 三元组 LLVM IR → DXIL 容器 → dxc validator 接受)。着色阶段着色 → DXIL 着色器类型的完整对应表(RXS-0158)、阶段 I/O 签名降级(RXS-0159)、阶段间接口核对(RXS-0160)不在本条;本条 compute 路径以 RXS-0153 的 compute-via-kernel 着色为入口锚点。

#### Syntax

target 选择为工具链 CLI 面,非语言文法面:`rx build --target dxil <input.rx>`(与 `--target ptx` 并列;省略 `--target` 维持现状默认 host/PTX 通道,零语义漂移)。着色阶段/kernel 源码不因 target 改写——同一份 compute 着色(`kernel fn` 或 `compute fn`,RXS-0153 着色)经 DXIL 后端降级为 DXIL compute shader。

#### Legality

- L1(后端可用性):`--target dxil` 要求 cargo feature `dxil-backend` 已启用;未启用 → **RX6007**(codegen 目标不可用,P-01 strict-only,不降级 host/PTX)。
- L2(最小子集):本片 DXIL 后端仅支持 compute 着色入口的**最小子集**(无 ABI 形参、空/平凡体 → DXIL `void` 入口)。子集外构造(View/资源句柄形参、非平凡体、需绑定布局推导或 FFI ABI 的语言面)→ **RX6007**(DXIL codegen 暂不支持构造;绑定布局推导属 G2.3、FFI ABI 属禁区,不在本片)。
- L3(降级失败):DXIL 降级管线(IR emit / patched llc → DXIL 容器 / dxc validator)失败 → **RX6007** 编译期 codegen 诊断(无运行期 fallback)。工具链缺失(patched llc / validator 不可用)为开发环境降级 **SKIP**(非 RX6007,对齐 RXS-0073 ptxas 干验证 SKIP 纪律,真实红绿在带工具链的环境)。

#### Dynamic Semantics

DXIL 后端为 codegen/工具链面,本条无运行期语言语义(着色器在 D3D12 管线的执行属运行时/G2.3+,不在本条)。降级管线为编译期确定性变换:给定 MIR 输入,DirectX 三元组 LLVM IR 文本与下游 DXIL 容器对相同输入确定(两次产出字节一致)。

#### Implementation Requirements

- IR1(分发点):target 分发在 MIR 之后(AST→HIR→TBIR→MIR 前沿对所有 target 共享,RFC-0003 §4.1);DXIL 后端与 NVPTX 后端并列、各自从 MIR 独立降级,不共享后端内部 lowering,不改 PTX 路径(RFC-0003 §4.5)。
- IR2(D-131=A 路径):DXIL 经 LLVM DirectX 后端直接 emit(`dxil-unknown-shadermodel6.0-compute` 三元组 + `hlsl.shader`/`hlsl.numthreads` 入口属性)→ patched llc `-filetype=obj` 产 DXIL 容器。patched llc 经 `RURIX_LLC` dev env 绝对路径定位(受控临时偏差,RD-011),不写死、不改 committed D-205 pin / toolchain.rs;缺 env / llc 不可用 → 清晰诊断(SKIP 或 RX6007),非静默 fallback。
- IR3(golden):DXIL golden 取**文本反汇编形态**(RFC-0003 §9 Q-Golden),**经 dxc validator 验证通过后**入 golden(不合规 DXIL 不得入 golden);确定性、纳入既有 bless 体系。
- IR4(错误码):target 不可用 / 子集外构造 / 降级失败归 **RX6007**(6xxx codegen/目标段,只追加,registry/error_codes.json + en/zh message-key);工具链缺失为 SKIP 不发码。

### RXS-0158 着色阶段着色 → DXIL 着色器类型降级对应

RXS-0153 着色阶段着色(`vertex`/`fragment`/`compute`/`mesh`/`task` + RT `raygen`/`closesthit`/`anyhit`/`miss`,spec/shader_stages.md)经 DXIL 后端降级为对应 **DXIL 着色器类型**:即 DirectX 三元组的 shader-stage 环境分量(`dxil-unknown-shadermodel<sm>-<env>`)、入口 `hlsl.shader` 属性值与 shader model 下限的精确对应。本条只覆盖**阶段 → 着色器类型 + shader profile**(结构/类型面);**不**定义阶段 I/O → 签名/系统值语义 SV_*(RXS-0159)、阶段间接口链接一致性(RXS-0160),亦**不碰** 🔒 纹理内存模型映射(06 §4.2)/ 内建变量·签名二进制 ABI 布局(RFC-0003 §4.6/§9 Q-Builtin)/ 绑定布局推导(G2.3,P-11)。

#### 着色器类型对应表

| Rurix 着色阶段 | DXIL 着色器类型 | triple 环境分量 `<env>` | `hlsl.shader` 属性值 | shader model 下限 | 本片状态 |
|---|---|---|---|---|---|
| `compute`(及 `kernel`,RXS-0153 compute-via-kernel) | compute shader | `compute` | `compute` | SM 6.0 | **已落**(承 RXS-0157;`hlsl.numthreads`) |
| `vertex` | vertex shader | `vertex` | `vertex` | SM 6.0 | **已落** |
| `fragment` | pixel shader | `pixel` | `pixel` | SM 6.0 | **已落** |
| `mesh` | mesh shader | `mesh` | `mesh` | SM 6.5 | 映射登记,实现 deferred(RD-012) |
| `task` | amplification shader | `amplification` | `amplification` | SM 6.5 | 映射登记,实现 deferred(RD-012) |
| `raygen` | RT raygeneration(library) | `library` | `raygeneration` | SM 6.3 | 映射登记,实现 deferred(RD-012) |
| `closesthit` | RT closesthit(library) | `library` | `closesthit` | SM 6.3 | 映射登记,实现 deferred(RD-012) |
| `anyhit` | RT anyhit(library) | `library` | `anyhit` | SM 6.3 | 映射登记,实现 deferred(RD-012) |
| `miss` | RT miss(library) | `library` | `miss` | SM 6.3 | 映射登记,实现 deferred(RD-012) |

> **deferred 诚实标注(RD-012)**:`mesh`/`task` 着色器的合规 DXIL 需线程组维度 + 输出拓扑/`DispatchMesh` 声明(dxc validator 对空体 mesh/amplification 入口报缺失),RT 着色器为 **DXIL library 多入口形态**——两者的最小合规降级均越出「阶段→着色器类型(类型面)」、落入阶段 I/O(RXS-0159)/ library 多入口与 ABI 面,本片**不**实现,以 RD-012 显式登记承接后续子分片。本表对其映射**完整登记**(triple env / `hlsl.shader` / SM 下限),但**无 passing 测试锚定的规范性降级条款**:不支持阶段的 DXIL 降级请求 → **RX6008** 编译期诊断(下文 Legality L2)。光栅(vertex/fragment)与 compute 阶段提供 passing 锚定(accept + DXIL golden,经 dxc validator 接受)。

#### Syntax

阶段→着色器类型降级为 codegen 面,非语言文法面:着色阶段源码(`<stage> fn`,RXS-0153 前缀式)不因 DXIL 降级改写。`--target dxil` 对同一着色阶段函数按其阶段类别产对应 DXIL 着色器类型的 DirectX 三元组 LLVM IR(`compute fn` 与 `kernel fn` 同产 compute shader,RXS-0153 compute-via-kernel)。

#### Legality

- L1(可降级阶段最小子集):本片仅 `compute`(及 `kernel`)/ `vertex` / `fragment` 着色阶段可降级,且沿 RXS-0157 最小子集——无 ABI 形参、平凡(空)体 → DXIL `void` 入口。子集外构造(I/O 签名形参 / 非平凡体——需 RXS-0159 阶段 I/O 签名或绑定布局推导 G2.3 / FFI ABI 禁区)→ **RX6007**(承 RXS-0157 L2,本条不重定义)。
- L2(deferred 阶段):`mesh` / `task` / RT(`raygen`/`closesthit`/`anyhit`/`miss`)着色阶段的 DXIL 降级**本片未实现**(RD-012;合规降级越出阶段→着色器类型类型面,见上表 deferred 标注)→ **RX6008**(DXIL 着色阶段降级暂未支持,P-01 strict-only,无静默 fallback、不降级为其他着色器类型)。
- L3(降级失败):同 RXS-0157 L3——DXIL 降级管线(IR emit / patched llc → DXIL 容器 / dxc validator)失败 → **RX6007**;工具链缺失为开发环境降级 **SKIP**(非发码,对齐 RXS-0073/RXS-0157)。

#### Dynamic Semantics

阶段→着色器类型降级为编译期确定性变换,本条无运行期语言语义(着色器在 D3D12/DXR 管线的执行属运行时/G2.3+,不在本条)。给定阶段着色 MIR 入口,其 DirectX 三元组 LLVM IR(triple 环境分量 + `hlsl.shader` 属性)对相同输入字节确定(两次产出一致)。

#### Implementation Requirements

- IR1(阶段→着色器类型映射):降级按上表将阶段类别映射为 triple 环境分量(`dxil-unknown-shadermodel<sm>-<env>`)+ 入口 `hlsl.shader` 属性值;`compute`/`mesh`/`task` 附 `hlsl.numthreads`(本片仅 compute 落地,取最小 `1,1,1`),`vertex`/`fragment` 不附 numthreads。映射在 [`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs) 由阶段标记(HIR `FnDecl::stage`,RXS-0153;`None` 取 compute)裁定;DXIL 收集根扩到含着色阶段入口(`build_dxil_crate`),不改 PTX 收集根(`build_device_crate` 维持排除着色阶段,D-207)。
- IR2(deferred 阶段发码):`mesh`/`task`/RT 阶段降级请求 → `RX6008`(message-key `codegen.dxil_stage_unsupported`,附阶段名 + RD-012),不产任何 DXIL(strict-only)。
- IR3(SM 下限登记):上表 shader model 下限随阶段登记(光栅/compute SM6.0、mesh/amp SM6.5、RT SM6.3);本片落地阶段(SM6.0)经 patched llc + dxc validator(1.9.2602.24)接受实测,SM6.5/6.3 阶段为 deferred 阶段的映射登记值,无 passing 测试(RD-012)。
- IR4(错误码):新增 **RX6008**(DXIL 着色阶段降级暂未支持;6xxx codegen/目标段续接 RX6007,只追加,registry/error_codes.json + en/zh message-key);子集外/降级失败仍归 RX6007(承 RXS-0157)。

### RXS-0159 阶段 I/O → DXIL 签名/系统值语义降级（类型面）

RXS-0154 阶段专属 I/O(`#[interpolate]` 插值限定 / `#[builtin]` 内建变量,spec/shader_stages.md)经 DXIL 后端降级为 **DXIL 输入/输出签名与系统值语义(SV_*)**。本条**只覆盖类型面映射**:内建变量 → DXIL 系统值语义**名**(按阶段 + 方向)、插值限定 → DXIL 插值限定符、vertex out / fragment in/out 签名的**结构**(哪个字段对应哪个语义元素)。

> **🔒 二进制 ABI 布局不在本条**:签名元素的**寄存器打包 / 字节偏移 / component mask / 作为 ABI 的根参数·常量缓冲二进制布局**属 RFC-0003 §4.6 / §9 Q-Builtin 的 **FFI ABI 禁区**,由 LLVM DirectX 后端 emit、经 dxc validator 验证;**Rurix 不定义、不冻结、不作为保证**。本条不写任何「偏移=N / 寄存器=Rn / 布局保证」字样;触及 ABI 二进制布局 / 🔒 纹理内存模型映射(06 §4.2)/ 绑定布局推导(G2.3,P-11)即停手标「需人工升档」。

#### 内建变量 → DXIL 系统值语义名映射表(类型面)

> 内建变量集来自 RXS-0154(已由着色阶段类型面校验已知性);本表只裁其在 DXIL 签名中的 **SV 语义名**对应,按**阶段 + 方向**(输入=形参 I/O 结构体字段 / 输出=返回 I/O 结构体字段)。表中无对应项(空白)= 该阶段/方向**不可映射** → `RX6009`(下文 Legality L2)。**仅语义名,无寄存器/偏移**。

| `#[builtin(..)]` | vertex 输入 | vertex 输出 | fragment 输入 | fragment 输出 |
|---|---|---|---|---|
| `position` | — | `SV_Position` | — | — |
| `vertex_id` | `SV_VertexID` | — | — | — |
| `instance_id` | `SV_InstanceID` | — | — | — |
| `frag_coord` | — | — | `SV_Position` | — |
| `front_facing` | — | — | `SV_IsFrontFace` | — |
| `primitive_id` | — | — | `SV_PrimitiveID` | — |
| `depth` | — | — | — | `SV_Depth` |
| `thread_id` | —(compute 专属 `SV_DispatchThreadID`,非图形签名) | — | — | — |

> fragment **输出 user varying**(非 builtin,以 `#[interpolate(..)]` 标注的颜色输出)→ **`SV_Target`**(渲染目标颜色输出;插值限定符对输出无光栅插值意义,本条以渲染目标语义裁定,具体 render-target 索引 SV_Target0/1/… 属后端 emit 的 ABI 不在本条)。

#### 插值限定 → DXIL 插值限定符映射表(类型面)

| `#[interpolate(..)]` | DXIL 插值限定符 |
|---|---|
| `perspective` / `linear` | `linear`(透视校正;HLSL 无独立 perspective 关键字) |
| `noperspective` | `noperspective` |
| `flat` | `nointerpolation` |
| `centroid` | `centroid` |
| `sample` | `sample` |

> **整数 varying 约束**:DXIL 要求整数类型的插值 varying 必须 `nointerpolation`(flat);整数 varying 携带非 flat 插值限定(`perspective`/`linear`/`noperspective`/`centroid`/`sample`)为**非法插值组合** → `RX6009`(下文 Legality L3)。

<!--RXS0159-MORE-->

#### Syntax

阶段 I/O → 签名/系统值语义降级为 codegen 面,非语言文法面:着色阶段 I/O 标注源码(`#[builtin(..)]` / `#[interpolate(..)]`,RXS-0154 属性式)不因 DXIL 降级改写。`--target dxil` 对带 I/O 签名的 vertex/fragment 入口,按其形参(输入)/ 返回(输出)的 I/O 结构体字段标注产对应 DXIL 输入/输出签名语义元数据(SV_* 语义名 + 插值限定符)。

#### Legality

- L1(可降级 I/O 子集):本片 `vertex` / `fragment` 入口的 I/O 经**命名 I/O 结构体**(形参 = 输入签名、返回 = 输出签名)表达,每字段 `#[builtin]` 或 `#[interpolate]`(承 RXS-0154)。非 I/O 结构体形参(标量 / 资源句柄 `Texture2D`/`Sampler` —— 绑定布局推导属 G2.3、FFI ABI 属禁区)/ 标量返回 → **RX6007**(承 RXS-0157 L2 子集外构造,本条不重定义)。`compute` 阶段无图形 I/O 签名(其线程系统值经 intrinsic,非签名),compute 入口带形参仍归 RX6007。
- L2(不可映射内建变量):`#[builtin(name)]` 在该阶段/方向无对应 DXIL 系统值(上表空白项,如 `thread_id` 在 vertex/fragment 入口、`vertex_id` 作输出、`frag_coord` 作 vertex 输出)→ **RX6009**(阶段 I/O 签名/系统值语义降级失败,P-01 strict-only,无静默 fallback)。
- L3(非法插值组合):整数类型 varying 携带非 `flat` 插值限定(DXIL 要求整数 varying 必须 `nointerpolation`)→ **RX6009**。未知插值限定 / 未标注 I/O 字段在全管线由 RXS-0154 `RX3011` 先行拦截(着色阶段类型面检查先于 codegen);codegen 侧 RX6009 仅裁类型面**可映射性**(已知 builtin/插值在 DXIL 签名中的 SV/限定符对应),不重复 RXS-0154 的已知性校验。

#### Dynamic Semantics

阶段 I/O → 签名/系统值语义降级为编译期确定性变换,本条无运行期语言语义(着色器在 D3D12 管线消费签名属运行时/G2.3+,不在本条)。给定带 I/O 签名的阶段入口,其 DirectX 三元组 LLVM IR(含类型面签名元数据)对相同输入字节确定(两次产出一致)。

#### Implementation Requirements

- IR1(SV 语义名映射):按上表将 `#[builtin]` 映射为 DXIL 系统值语义名(阶段 + 方向裁定),`#[interpolate]` 映射为 DXIL 插值限定符;fragment 输出 user varying → `SV_Target`。映射在 [`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs) 由 AST 阶段函数签名(形参 I/O 结构体 = 输入、返回 I/O 结构体 = 输出)+ 字段 `#[builtin]`/`#[interpolate]` 标注裁定;**只设语义名/插值,不算寄存器/偏移/component mask**(后者由 LLVM DirectX 后端 emit,§9 Q-Builtin 禁区)。
- IR2(类型面签名元数据):签名经命名元数据 `!rurix.dxil.sig.in` / `!rurix.dxil.sig.out` emit 入 DirectX 三元组 LLVM IR,元素 `!{!"<field>", !"<semantic>"}`(SV 名直出 / 插值出 `interp:<modifier>`),**仅语义名/插值,无二进制布局**。无 I/O 签名(空体 vertex/fragment 或 compute)→ 不 emit 签名元数据(保持 RXS-0157/0158 既有 golden 字节不变)。
- IR3(入口 body stub,RD-013):本片**仅落签名类型面**;带 I/O 签名的入口以 DXIL `void` 入口 + 签名元数据 emit,**入口 body 数据流降级**(输入读取/输出写入语句级 codegen)deferred(代码侧 // STUB(RD-013))——越出类型面、与签名 ABI 布局耦合且需 device codegen 语句降级扩展,随后续子分片承接。golden 取 `.dxil-ll` 文本(确定性、always-on 比对);`.dxil-disasm` + dxc validator 真验证为带工具链环境关卡(patched llc/validator 缺失 → SKIP,RD-011,真实红绿在带工具链环境)。
- IR4(错误码):不可映射内建变量 / 非法插值组合归 **RX6009**(`codegen.dxil_signature_unsupported`;6xxx codegen/目标段续接 RX6008,只追加,registry/error_codes.json + en/zh message-key);子集外构造仍归 RX6007(承 RXS-0157)、deferred 阶段仍归 RX6008(承 RXS-0158)。

## 3. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-24 | 新建 dxil_backend.md（PR-C1 spec 脚手架，承 RFC-0003 / D-131=A）:登记文件名 + 文件级语义面说明（MIR→DXIL 第二后端，承 RFC-0002 着色阶段类型面 RXS-0153~0156）+ §1 范围与 **RXS-0157~ 预留区间声明**（区间大小未锁定，随 RFC-0003 §9 Q-Range 与路径裁定一并定）+ §2 条款占位（条款体随 PR-C2 实现 PR 同落）。**沿 README v1.32 interop_d3d12.md / v1.33 async_buffer.md / v1.37 shader_stages.md 脚手架先例:仅登记文件名 + 预留区间，不落带编号裸条款头**——本文件**零 `### RXS-####` 条款头**，`ci/trace_matrix.py --check` 维持全锚定 **156/156**（无新增裸条款头、无悬空锚点、零新 RXS）。条款体（RXS-0157 起）与每条 ≥1 `//@ spec` 测试锚定随 PR-C2（DXIL 后端实现 PR）同落（条款 PR 先于实现 PR）。禁区声明:🔒 纹理路径内存模型映射（06 §4.2）/ FFI ABI 二进制布局（RFC-0003 §4.6 / §9 Q-Builtin）/ 绑定布局推导（G2.3，P-11）/ 多后端架构承诺（D-008/SG-003）均不在本文件，触及即停手升档。错误码 **6xxx codegen 段**脚手架不预造、不预留，随 PR-C2 按真实可达类别只追加。档位 **Full RFC**（RFC-0003;触 codegen 第二后端 + target 分发，AI 不自判 Direct，判档争议向上取严）。授权 G2_CONTRACT D-G2-2 / G-G2-2 + G2_PLAN G2.2 子里程碑，无体例变更 | **Full RFC**（RFC-0003） |
| v1.1 | 2026-06-24 | **PR-C2 分片1:落首条带编号条款体 `### RXS-0157`**(codegen target 分发与 DXIL 后端分叉)+ 配套最小 compute kernel 端到端实现(rurixc `dxil_codegen` 模块 + `--target dxil` 分发 + cargo feature `dxil-backend` + patched llc 经 `RURIX_LLC` dev env 定位 RD-011 + dxc validator accept)。条款体按 FLS 体例分 Syntax / Legality(L1 后端可用性·L2 最小子集·L3 降级失败 → RX6007)/ Dynamic Semantics / Implementation Requirements(IR1 分发点·IR2 D-131=A 路径·IR3 golden 文本反汇编经 validator·IR4 错误码 RX6007),**严禁 UB 节**。配套 conformance accept(空体 compute kernel 产 DXIL,`//@ spec: RXS-0157`)+ reject(子集外构造 → RX6007)+ DXIL golden(文本反汇编 + bless)。错误码新增 **RX6007**(6xxx codegen/目标段续接 RX6006,只追加)+ en/zh message-key(双语覆盖)。`ci/trace_matrix.py --check` 全锚定 **157/157**(新增 RXS-0157 带测试锚定、无悬空)。RXS-0158/0159/0160 仍为 §9 Q-Range 计划映射(非裸条款头),随后续分片落地。本片不碰 🔒 纹理内存模型映射 / FFI ABI 布局 / 绑定布局推导(G2.3)。档位 **Full RFC**(RFC-0003),无体例变更 | **Full RFC**（RFC-0003） |
| v1.2 | 2026-06-25 | **PR-C2 分片2:落 `### RXS-0158`**(着色阶段着色 → DXIL 着色器类型降级对应)。含**着色器类型对应表**(vertex→vertex / fragment→pixel / compute(及 kernel)→compute / mesh→mesh / task→amplification / RT raygen·closesthit·anyhit·miss→library 对应 RT 类型),每阶段登记 triple 环境分量 `<env>` + `hlsl.shader` 属性值 + shader model 下限(光栅/compute SM6.0 / mesh·amp SM6.5 / RT SM6.3)。条款体分 Syntax / Legality(L1 可降级阶段最小子集承 RXS-0157 / L2 deferred 阶段 → RX6008 / L3 降级失败 → RX6007)/ Dynamic Semantics / Implementation Requirements(IR1 阶段→着色器类型映射·DXIL 收集根扩到含着色阶段不改 PTX 根 / IR2 deferred 阶段 RX6008 / IR3 SM 下限登记 / IR4 错误码 RX6008),**严禁 UB 节、不定义 I/O 签名 ABI**。**实现取舍(诚实标注)**:vertex / fragment / compute 提供 passing 锚定(accept + DXIL golden,经 patched llc + dxc validator 1.9.2602.24 接受);mesh / task / RT 的合规 DXIL 降级越出阶段→着色器类型类型面(需线程组/DispatchMesh/输出拓扑或 library 多入口 + I/O 签名 ABI),本片**仅完整登记映射、不实现**,以 **RD-012** deferred 显式承接,reject 锚定(不支持阶段 → RX6008)。错误码新增 **RX6008**(`codegen.dxil_stage_unsupported`,6xxx 段续接 RX6007,只追加)+ en/zh message-key(双语覆盖)。配套 conformance accept(vertex/fragment/compute fn 各产对应 DXIL,`//@ spec: RXS-0158`)+ reject(mesh/task/raygen → RX6008)+ 各落地阶段 DXIL golden(`.dxil-ll` + 经 validator 接受的 `.dxil-disasm`,bless)。`ci/trace_matrix.py --check` 全锚定 **158/158**。本片不碰 🔒 纹理内存模型映射(06 §4.2)/ 内建变量·签名二进制 ABI 布局(RFC-0003 §4.6/§9 Q-Builtin)/ 绑定布局推导(G2.3,P-11)/ 阶段 I/O 签名 SV_*(RXS-0159)。档位 **Full RFC**(RFC-0003),无体例变更 | **Full RFC**（RFC-0003） |
| v1.3 | 2026-06-25 | **PR-C2 分片3:落 `### RXS-0159`**(阶段 I/O → DXIL 签名/系统值语义降级,**类型面**)。含**内建变量 → DXIL 系统值语义名映射表**(按阶段+方向:position→SV_Position〔vertex 输出〕/ vertex_id→SV_VertexID·instance_id→SV_InstanceID〔vertex 输入〕/ frag_coord→SV_Position·front_facing→SV_IsFrontFace·primitive_id→SV_PrimitiveID〔fragment 输入〕/ depth→SV_Depth〔fragment 输出〕/ thread_id 无图形签名对应;fragment 输出 user varying→SV_Target)+ **插值限定 → DXIL 插值限定符映射表**(perspective/linear→linear / noperspective→noperspective / flat→nointerpolation / centroid→centroid / sample→sample;整数 varying 必须 flat)。条款体分 Syntax / Legality(L1 可降级 I/O 子集承 RXS-0157·非 I/O 形参→RX6007 / L2 不可映射内建变量→RX6009 / L3 非法插值组合〔整数非 flat〕→RX6009)/ Dynamic Semantics / Implementation Requirements(IR1 SV 语义名映射·只设语义名不算寄存器/偏移 / IR2 类型面签名元数据 !rurix.dxil.sig.in·out / IR3 入口 body stub deferred RD-013 / IR4 错误码 RX6009),**严禁 UB 节、显式声明二进制 ABI 布局属 §9 Q-Builtin 禁区不在本条**。错误码新增 **RX6009**(`codegen.dxil_signature_unsupported`,6xxx 段续接 RX6008,只追加)+ en/zh message-key(双语覆盖)。配套 conformance accept(vertex_io SV_Position 输出 / fragment_io SV_Target 输出,`//@ spec: RXS-0159` + `//@ dxil-sig:`)+ reject(thread_id 不可映射 / 整数非 flat 插值 → RX6009)+ vertex/fragment I/O DXIL golden(`.dxil-ll` 含类型面签名元数据,bless;`.dxil-disasm` 待带工具链环境录入,dev SKIP RD-011)。新增 **RD-013**(入口 body 数据流降级 deferred)。`ci/trace_matrix.py --check` 全锚定 **159/159**。本片不碰 🔒 纹理内存模型映射(06 §4.2)/ 签名二进制 ABI 布局(RFC-0003 §4.6/§9 Q-Builtin)/ 绑定布局推导(G2.3,P-11)/ 阶段间接口(RXS-0160)。档位 **Full RFC**(RFC-0003),无体例变更 | **Full RFC**（RFC-0003） |
