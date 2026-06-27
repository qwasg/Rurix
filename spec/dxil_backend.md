# Rurix 语言规范 — DXIL 第二后端语义面（MIR → DXIL，承 RFC-0003；G2.2 起）

> 条款:**RXS-0157 起续号**(G2.2 DXIL 第二后端语义面:codegen target 分发与 DXIL 后端分叉(MIR 之后 target 选择,PTX/DXIL 并存,strict-only 无 fallback)/ 着色阶段着色 → DXIL 着色器类型降级对应 / 阶段 I/O → DXIL 签名·系统值语义降级 / 阶段间接口 → DXIL 阶段链接一致性核对)。RFC-0003 §9 Q-Range 初始未锁定;经 RFC-0004 §9 Q-Range-B 锁定,当前映射为 RXS-0157 既有条款体 + RXS-0158 HOLD + RXS-0159 保号按 B 重构 + RXS-0160~0162 预留。体例见 [README.md](README.md)。
> 依据:**[RFC-0003](../rfcs/0003-dxil-backend.md)**(MIR→DXIL 第二后端与 compute=A 基线,**owner Approved** 2026-06-23;§9 Q-D131 后经追加式勘误由 A 增补为混合路径)+ **[RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md)**(图形=B,**owner Approved** 2026-06-25;D-131 当前裁决 = **compute=A / 图形=B**);06 §8.2(codegen 第二后端设计预留);06 §4.2(纹理路径内存模型禁区,🔒);07 §7(device codegen 分发:MVP/G1 维持 NVPTX→PTX,DXIL 第二后端 G2 重评估);07 §5(错误码段位:6xxx codegen/目标);spec/shader_stages.md RXS-0153~0156(着色阶段类型面,DXIL 降级的上游类型面来源)。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)(D-G2-2,G-G2-2)+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.2 子里程碑。
> 档位:**Full RFC**(RFC-0003 + RFC-0004;10 §3:本设计触 **codegen 第二后端 + target 分发 + 图形=B 第二中间表示与外部转译依赖**,有别于 M8 互操作的 Direct)。**D-131 当前路径为混合 compute=A / 图形=B**:compute A 路 dev 工具链偏差由 `registry/deferred.json` RD-011 跟踪;图形 B 路供应链由 RD-014 跟踪。**AI 无权自判 Direct**,判档以 RFC-0003/RFC-0004 与 G2_CONTRACT 授权为据,判档争议向上取严。任何触及 **🔒 纹理路径内存模型映射(06 §4.2)** / **FFI ABI 二进制布局(RFC-0003 §4.6/§9 Q-Builtin;RFC-0004 §4.6)** / **多后端架构承诺(D-008/SG-003)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):target 不支持 / 降级失败 / 非法 I/O 映射以 **编译期 6xxx codegen 诊断(P-01 strict-only,无运行期 fallback)**定义,不以 UB 表述(RFC-0003 §3/§4;RFC-0004 §4.4/§4.6)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**PR-C2 分片1 起**已落首条带编号条款体 **RXS-0157**(codegen target 分发与 DXIL 后端分叉)+ 每条 ≥1 测试锚定(条款 PR 先于/同实现 PR,G2.1 PR-B2 先例),trace_matrix 全锚定 **157/157**;后续条款(RXS-0158~)随分片续落。

---

## 1. 范围与编号区间

本文件承载 **MIR → DXIL 第二后端语义面**的语义条款(G2.2+,D-G2-2)。承 RFC-0002 着色阶段类型面(RXS-0153~0156)产出的着色阶段语义,定义其经工具链降级到 DXIL(可被 D3D12 PSO 消费的着色器对象)的语义。覆盖语义面(RFC-0003 §4):

- **codegen target 分发与 DXIL 后端分叉**:MIR 之后的 target 选择(`rx build --target dxil`,与现状 `--target ptx` 并列,RFC-0003 §9 Q-CLI),PTX 后端(D-207)与 DXIL 后端**并存**;target 不支持的构造 → **6xxx codegen 错误**(strict-only,无运行期 fallback,P-01)。DXIL 后端 gate 于 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate;未启用时 DXIL 后端不参与编译,PTX 路径不受影响)。
- **着色阶段着色 → DXIL 着色器类型降级对应**:RXS-0153 的着色阶段着色(`vertex`/`fragment`/`compute`/`mesh`/`task` + RT `raygen`/`closesthit`/`anyhit`/`miss`)降级为对应 DXIL 着色器类型(vertex/pixel/mesh/amplification/RT/compute shader)。精确对应表随 PR-C2 条款体落地。
- **阶段 I/O → DXIL 签名/系统值语义降级**:RXS-0154 的阶段专属 I/O(`#[interpolate]` 插值限定 / `#[builtin]` 内建变量)降级为 DXIL 输入/输出签名与系统值语义(SV_*)。内建变量 → DXIL 系统值语义名为**类型面映射**;其二进制 ABI 布局属 RFC-0003 §4.6/§9 Q-Builtin 的 🔒 FFI ABI 禁区,**不在本文件**,留 owner 后续独立 Full RFC。
- **阶段间接口 → DXIL 阶段链接一致性核对**:RXS-0155 的阶段间接口契约(vertex out → fragment in varying 兼容)在类型面已编译期校验(RXS-0155),DXIL 层为降级一致性核对。

DXIL 后端语义维持 **D-131 = 混合 compute=A / 图形=B**:compute 经 LLVM DirectX 后端直接 emit DXIL,与 NVPTX 后端同构并维持 D-205 LLVM 单栈;图形经内部 MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL 链(RFC-0004),SPIR-V 仅为图形 B 路内部中间表示,不构成对外通用多后端承诺。**🔒 纹理/采样器内存模型映射**(06 §4.2 禁区:tex proxy / 采样 opcode / 描述符编码 / 缓存一致性 / UB)、**内建变量/根参数/常量缓冲二进制 ABI 布局**(RFC-0003 §4.6 / RFC-0004 §4.6 FFI ABI 禁区)、**绑定布局推导实现**(G2.3,P-11)均**不在本文件**;DXIL golden 取**文本反汇编形态** + 经 dxc validator 验证后入 golden(RFC-0003 §9 Q-Golden / RFC-0004 §9 Q-Golden-B)。target 不支持 / 降级失败 / 非法 I/O 映射以 **编译期 6xxx codegen 诊断(P-01 strict-only)**定义,**不以 UB 表述**。

**编号区间**:本文件条款自 **RXS-0157** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0156 @ [shader_stages.md](shader_stages.md))。区间裁决:**RFC-0004 §9 Q-Range-B 锁定** RXS-0159 保号按 B 重构 + RXS-0160~0162 新增面。**已落带编号条款体**:RXS-0157(target 分发)/ **RXS-0158**(着色阶段 → 着色器类型,PR-D2)/ **RXS-0159**(阶段 I/O 签名,按 B)/ **RXS-0160**(阶段间接口 → 阶段链接一致性核对,按 B)/ **RXS-0161**(MIR→SPIR-V 降级面)/ **RXS-0162**(B 转译链确定性 + validator gate + 供应链 pin)。**RXS-0160** 的 vertex out↔fragment in 链接核对 vertex+fragment 多阶段联编点接缝(`link_graphics_stages`)+ 链接核对入口(`check_stage_link`)已落,承 RXS-0159 语义名等价基件;**错链错误码归类待 owner 裁决**(RX6011 复用 / RX6014 新开,见 §2 RXS-0160 IR3),条款体先以占位「6xxx」表述。**D-131 v1.4 裁定混合 compute=A / 图形=B**(13 §D-131;RFC-0004 owner Approved 2026-06-25):图形 I/O 签名降级(RXS-0159)由 A 路类型面 stub 改 **B 路**(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL)重写,A 路签名产物 ISG1/OSG1 `elemcount=0` 不可达(上游 #90504/#57928,RFC-0004 §4.5),**#97 的 A 路 RXS-0159 不入 main,由 PR-D2 统一按 B 重写**。条款体与每条 ≥1 测试锚定随实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 全锚定 **161/161**)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 本节落带编号条款体。**已落**:RXS-0157(codegen target 分发与 DXIL 后端分叉)/ RXS-0158(着色阶段 → DXIL 着色器类型)/ RXS-0159(阶段 I/O → DXIL 签名/系统值语义,按 B)/ RXS-0160(阶段间接口 → 阶段链接一致性核对,按 B)/ RXS-0161(图形着色阶段 MIR→SPIR-V 降级面)/ RXS-0162(B 转译链确定性 + validator gate + 供应链 pin + strict-only 核验)。**RXS-0160 错链错误码归类待 owner 裁决**(RX6011 复用 / RX6014 新开,见 RXS-0160 IR3),条款体先以占位「6xxx」表述。
> 各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(target 不支持 / 降级失败以编译期 6xxx codegen 诊断定义,P-01 strict-only,无运行期 fallback;10 §7.5)。**本片不碰** 🔒 纹理内存模型映射(06 §4.2 禁区)/ FFI ABI 二进制布局(RFC-0003 §4.6 / §9 Q-Builtin)/ 绑定布局推导(G2.3,P-11);触及即停手升档。

### 2.1 图形=B 条款计划映射收口(RFC-0004)

> **收口**:RFC-0004 §5 下游条款(RXS-0159 / 0160 / 0161 / 0162)**条款体已全部落地(下文 §2)**,本小节无尚未落地的计划映射(零 `### RXS-####` 三级标题,trace_matrix 不计本小节)。承 [RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md) §5;feature gate 复用 `dxil-backend`(Q-Gate-B);错误码归 **6xxx 段**(只追加)。

- **RXS-0160(已落条款体,见 §2)— 阶段间接口 → 阶段链接一致性核对**:vertex out ↔ fragment in 链接核对的 **vertex+fragment 多阶段联编点**接缝([`dxil_codegen::link_graphics_stages`](../src/rurixc/src/dxil_codegen.rs))与链接核对入口([`signature_gate::check_stage_link`](../src/rurixc/src/dxil_sig_gate.rs))已落,承 RXS-0159 语义名等价 / 系统值匹配基件。**错链错误码归类待 owner 裁决**(RX6011 复用 / RX6014 新开,见 §2 RXS-0160 IR3「需人工升档」):本条款体先以占位「6xxx」表述,落码(`registry/error_codes.json` + 双语 message-key + 生产 emit 接线)与错链 conformance reject + golden 随 owner 裁码后的实现步落地(条款先于实现,硬规则 7)。

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

> **deferred 诚实标注(RD-012)**:`mesh`/`task` 着色器的合规 DXIL 需线程组维度 + 输出拓扑/`DispatchMesh` 声明(dxc validator 对空体 mesh/amplification 入口报缺失),RT 着色器为 **DXIL library 多入口形态**——两者的最小合规降级均越出「阶段→着色器类型(类型面)」、落入阶段 I/O(RXS-0159)/ library 多入口与 ABI 面,本片**不**实现,以 RD-012 显式登记承接后续子分片。本表对其映射**完整登记**(triple env / `hlsl.shader` / SM 下限),但**无 passing 测试锚定的规范性降级条款**:不支持阶段的 DXIL 降级请求 → **RX6007** 编译期诊断(下文 Legality L2;沿 RXS-0157 通道,`registry/deferred.json` RD-012 已预留专用码 RX6008,待 RD-012 实现时改接)。光栅(vertex/fragment)与 compute 阶段提供 passing 锚定(accept + DXIL golden,经 dxc validator 接受)。

#### Syntax

阶段→着色器类型降级为 codegen 面,非语言文法面:着色阶段源码(`<stage> fn`,RXS-0153 前缀式)不因 DXIL 降级改写。`--target dxil` 对同一着色阶段函数按其阶段类别产对应 DXIL 着色器类型的 DirectX 三元组 LLVM IR(`compute fn` 与 `kernel fn` 同产 compute shader,RXS-0153 compute-via-kernel)。

#### Legality

- L1(可降级阶段最小子集):本片仅 `compute`(及 `kernel`)/ `vertex` / `fragment` 着色阶段可降级,且沿 RXS-0157 最小子集——无 ABI 形参、平凡(空)体 → DXIL `void` 入口。子集外构造(I/O 签名形参 / 非平凡体——需 RXS-0159 阶段 I/O 签名或绑定布局推导 G2.3 / FFI ABI 禁区)→ **RX6007**(承 RXS-0157 L2,本条不重定义)。
- L2(deferred 阶段):`mesh` / `task` / RT(`raygen`/`closesthit`/`anyhit`/`miss`)着色阶段的 DXIL 降级**本片未实现**(RD-012;合规降级越出阶段→着色器类型类型面,见上表 deferred 标注)→ **RX6007**(沿 RXS-0157 通道显式拒绝「DXIL 着色阶段降级暂未支持」,P-01 strict-only,无静默 fallback、不降级为其他着色器类型;`registry/deferred.json` RD-012 已预留专用码 RX6008,待 RD-012 实现时改接)。
- L3(降级失败):同 RXS-0157 L3——DXIL 降级管线(IR emit / patched llc → DXIL 容器 / dxc validator)失败 → **RX6007**;工具链缺失为开发环境降级 **SKIP**(非发码,对齐 RXS-0073/RXS-0157)。

#### Dynamic Semantics

阶段→着色器类型降级为编译期确定性变换,本条无运行期语言语义(着色器在 D3D12/DXR 管线的执行属运行时/G2.3+,不在本条)。给定阶段着色 MIR 入口,其 DirectX 三元组 LLVM IR(triple 环境分量 + `hlsl.shader` 属性)对相同输入字节确定(两次产出一致)。

#### Implementation Requirements

- IR1(阶段→着色器类型映射):降级按上表将阶段类别映射为 triple 环境分量(`dxil-unknown-shadermodel<sm>-<env>`)+ 入口 `hlsl.shader` 属性值;`compute`/`mesh`/`task` 附 `hlsl.numthreads`(本片仅 compute 落地,取最小 `1,1,1`),`vertex`/`fragment` 不附 numthreads。映射在 [`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs) 由阶段标记(HIR `FnDecl::stage`,RXS-0153;`None` 取 compute)裁定;DXIL 收集根扩到含着色阶段入口(`build_dxil_crate`),不改 PTX 收集根(`build_device_crate` 维持排除着色阶段,D-207)。
- IR2(deferred 阶段发码):`mesh`/`task`/RT 阶段降级请求 → `RX6007`(message-key `codegen.dxil_unsupported`,附阶段名 + RD-012),不产任何 DXIL(strict-only);[`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs) 现沿 RXS-0157 RX6007 通道,`registry/deferred.json` RD-012 落码时改接预留 RX6008。
- IR3(SM 下限登记):上表 shader model 下限随阶段登记(光栅/compute SM6.0、mesh/amp SM6.5、RT SM6.3);本片落地阶段(SM6.0)的着色器类型映射经各自后端实测(compute=A 路;vertex/fragment=B 路 dxc,见 RXS-0161/0162),SM6.5/6.3 阶段(mesh/task/RT)为 deferred 阶段的映射登记值,无 passing 测试(RD-012);DXIL golden 经 dxc validator 验证,由 owner pin 环境 bless。
- IR4(错误码):本条**不新增错误码**——mesh/task/RT deferred 阶段暂沿 RXS-0157 **RX6007** 通道显式拒绝(`registry/deferred.json` RD-012 已预留专用码 RX6008,落码归 RD-012 实现里程碑,不在本片);子集外 / 降级失败亦归 RX6007(承 RXS-0157)。

### RXS-0159 阶段 I/O → DXIL 签名/系统值语义降级（B 路）

vertex/fragment 阶段 I/O(RXS-0154 `#[builtin]` / `#[interpolate]` + 字段名,spec/shader_stages.md)经 **B 路**(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL,RFC-0004 §4.2)降级为 DXIL ISG1/OSG1 签名。**D-131 v1.4 裁定图形=B**:A 路签名产物 `elemcount=0` 不可达(上游 #90504,RFC-0004 §4.5),本条按 B 路重写(承 §2.1 v1.2 计划)。本条只承诺**源码层签名元素的存在性 / 语义名 / 系统值 / 插值 / 方向**;🔒 寄存器号 / component mask / packing / 字节偏移**不属本条承诺**(RFC-0004 §4.6(a),外部 conformance,校验门不比对布局)。

#### Syntax

阶段 I/O 降级为 codegen 面,非语言文法面:阶段 I/O 由阶段函数签名(RXS-0154 `#[builtin]` / `#[interpolate]` 标注 + 字段名)给定,不因 DXIL 降级改写。

#### Legality

- L1(可降级 I/O 子集):`vertex` / `fragment` 的已建模 `#[builtin]` / `#[interpolate]` / 普通 varying(RXS-0154)+ 标量 / 向量类型可降级。子集外构造(资源句柄形参等非 I/O 签名面 / 未建模类型)→ **RX6013**(不可映射构造,`codegen.dxil_unmappable`)。
- L2(不可映射内建 / 类型):未建模 builtin / 越界向量宽度 / 阶段不符的系统值 → **RX6013**(由编码器 [`dxil_spirv`](../src/rurixc/src/dxil_spirv.rs) `DxilError::Unmappable` 透传)。
- L3(签名未保真,strict-only):B 链译后 ISG1/OSG1 与 MIR 意图签名经强制校验门比对——用户声明 / 可观察元素(输出方向语义名 / 系统值)缺失·改名·错配 → **RX6011**(`codegen.dxil_sig_mismatch`);声明的外部输入(`dir == In`)被消除且不可等价保留 → **RX6012**(`codegen.dxil_sig_dropped_input`)。无静默降级 / 丢弃(P-01,RFC-0004 §4.4)。
- 🔒(布局边界):承诺具体寄存器 / mask / 偏移值越出本条 = RFC-0004 §4.6(a) 禁区,**需人工升档**(owner 独立 Full RFC);本条仅作存在性 / 语义名 / 系统值边界声明,校验门**不**比对二进制布局。

#### Dynamic Semantics

阶段 I/O 降级为编译期确定性变换,本条无运行期语言语义。给定阶段 I/O MIR,SPIR-V `Location` / `BuiltIn` / `UserSemantic` decoration 与下游 DXIL 签名对相同输入字节确定(确定性核对见 RXS-0162)。

#### Implementation Requirements

- IR1(系统值 / 语义名映射):`#[builtin]` → DXIL 系统值(`position`→SV_Position、`vertex_index`→SV_VertexID、`instance_index`→SV_InstanceID、`frag_coord`→SV_Position、`frag_depth`/`depth`→SV_Depth 等,按阶段 + 方向),**系统值名经 B 链恒保真**(实测 SV_Position / SV_VertexID 真达,elemcount>0;B-over-A 核心)。用户命名 I/O 语义名保名**按方向收窄**(实测定结论,`evidence/dxil_b_strict_only_report.md` §3 + 本机 dxc 1.8.0.4739 / spirv-cross 复现):**(a) vertex 阶段输入**用户语义名 → **by-construction 保真**——[`dxil_spirv::emit_spirv`](../src/rurixc/src/dxil_spirv.rs) 按 io_sig 顺序 emit `Location` 装饰,SPIR-V→HLSL 段经 `spirv-cross --set-hlsl-vertex-input-semantic <location> <semantic>`(由 [`dxil_codegen::vertex_input_semantic_flags`](../src/rurixc/src/dxil_codegen.rs) 经 io_sig **导出、非硬编码**)按 location 覆盖,`POSITION` / `NORMAL` 端到端**不退化为通用 `TEXCOORD#`**(RFC-0004 §4.4 机制①,measured 顶点输入名存活);**(b) 输出 varying 与 fragment 输入 varying**用户语义名 → **当前不可保真**:spirv-cross HLSL 后端无输出 / 片元输入语义保名旗标,且**不消费** SPIR-V `UserSemantic` 装饰为 HLSL 语义(实测;`UserSemantic` 仅作 SPIR-V 层 provenance + 经 spirv-val 干净保留,非保名机制),退化为 `TEXCOORD#` → 经强制校验门 **RX6011 显式拒绝**(不静默通过,P-01);该输出 / 片元 varying 保名能力缺口 deferred 经 **RD-017**(回填条件 / 承接里程碑见 `registry/deferred.json`,status 留 owner)。映射在 [`dxil_spirv`](../src/rurixc/src/dxil_spirv.rs)(SPIR-V 装饰)+ [`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs)(顶点输入保名旗标导出)emit。
- IR2(强制签名一致性校验门,不可裁剪):[`signature_gate::check`](../src/rurixc/src/dxil_sig_gate.rs) 比对译后 `actual`(ISG1/OSG1)与 MIR 意图 `intent`,比较域 = 语义名 / 系统值 / 被用输入元素(**不**取寄存器 / mask / 顺序,ABI 中立);失败 → RX6011 / RX6012,strict-only 终止该入口产物、不产 golden。**不存在跳过校验的配置**(RFC-0004 §4.4 不可裁剪)。
- IR3(错误码):RX6011(签名不一致)/ RX6012(声明输入被消除)/ RX6013(不可映射)。RX6009 已被 `registry/deferred.json` RD-013(阶段 I/O 入口 body 数据流降级)预引占用,本条两类按段内不复用改派 RX6011 / RX6012(6xxx 段续接,只追加 + en/zh message-key)。
- IR4(入口 body deferred):本条覆盖阶段 I/O **签名类型面**;入口 body 数据流降级(真实读写 I/O 的语句级 codegen)属 **RD-013**,不在本条。

### RXS-0160 阶段间接口 → 阶段链接一致性核对（B 路）

RXS-0155 在类型面已编译期校验阶段间接口契约(vertex out → fragment in varying 名 / 类型 / 插值兼容,RX3012)。本条为该契约在 **DXIL 降级层**的一致性核对:vertex 阶段输出 varying 与 fragment 阶段输入 varying 经 **B 路**(RXS-0161/0162)降级后,以**语义名等价为链接键**核实跨阶段配对的语义名 / 类型 / 插值限定保真,错链即显式 6xxx(strict-only,无运行期 fallback,P-01)。本条承 RXS-0159 单阶段签名保真核对([`signature_gate::check`](../src/rurixc/src/dxil_sig_gate.rs))的语义名等价 / 系统值匹配基件,新增 **vertex+fragment 多阶段联编点**([`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs) 由单阶段编译扩到收集两阶段 io_sig 汇集到链接核对点)。本条只承诺**跨阶段 varying 的链接键存在性 / 语义名等价 / 类型 / 插值一致性**;🔒 寄存器号 / location 编号 / component mask / packing **不属本条承诺**(属 RFC-0004 §4.6(a) ABI 禁区,链接核对以语义名为键、不比对 location 数值)。

#### Syntax

阶段间接口链接核对为 codegen 面,非语言文法面:跨阶段 varying 由各阶段函数签名(RXS-0154 `#[interpolate]` / 字段名)给定,不因 DXIL 降级改写。链接核对不引入新文法。

#### Legality

- L1(可核对子集):`vertex` 输出方向(`dir == Out`)与 `fragment` 输入方向(`dir == In`)的已建模 varying / interpolate(RXS-0154)+ 标量 / 向量类型(RXS-0159 子集)。builtin 系统值(如 `position` / `frag_coord`)为阶段内系统值(经光栅器,非跨阶段用户 varying 链接键),不参与本核对。
- L2(链接键缺失,strict-only):fragment 输入 varying 在上游 vertex 输出中无同**语义名等价**链接键(错链:缺链接)→ **6xxx**(待 owner 裁 RX6011 复用 / RX6014 新开,见下 IR3 与 PR 描述「需人工升档」)。
- L3(链接键类型 / 插值失配,strict-only):语义名等价的链接键两端**类型不一致**或**插值限定不一致**(错链:类型 / 插值失配)→ **6xxx**(同 L2 待 owner 裁码)。
- 🔒(布局边界):承诺具体寄存器 / location 编号 / mask 越出本条 = RFC-0004 §4.6(a) 禁区,**需人工升档**;本条链接核对以语义名等价为键,**不**比对 location 数值(ABI 中立,对齐 RXS-0162 Property 7)。

#### Dynamic Semantics

阶段间链接核对为编译期确定性变换,本条无运行期语言语义(着色器在 D3D12 管线的跨阶段数据传递属运行时 / G2.3+,不在本条;运行期语义等价由 G-G2-2 device 真跑兑现)。给定两阶段 I/O 意图签名,链接核对结论(一致 / 错链分类)对相同输入确定。

#### Implementation Requirements

- IR1(链接核对入口,不可裁剪):[`signature_gate::check_stage_link`](../src/rurixc/src/dxil_sig_gate.rs)`(vs_out_sig, fs_in_sig)` 比对 vertex 输出方向与 fragment 输入方向的 varying / interpolate 元素,以语义名等价(大小写无关 + 剥语义 index 后缀,复用 RXS-0159 `semantic_name_matches`)为链接键:键缺失 / 两端类型或插值限定不一致 → 失败(strict-only,绝不静默通过)。比较域 = 语义名 / 类型 / 插值限定;**不**取 location 编号 / 寄存器 / mask(ABI 中立,RFC-0004 §4.6(a))。
- IR2(多阶段联编点):[`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs) 由单着色阶段编译扩到 **vertex+fragment 配对编译接缝**——收集两阶段 body 的 `io_sig` 汇集到链接核对点(`link_graphics_stages`);无 vertex+fragment 配对(单阶段编译 / 缺一阶段)→ 无链接核对(behavior 不变,A 路 / 单阶段零漂移,对齐 RXS-0157 R6.7)。
- IR3(错误码,待 owner 裁):错链 → **6xxx**(待 owner 裁定 **RX6011 复用**(签名不一致同语义类)**或 RX6014 新开**(6xxx 段下一空号;RX6008 / RX6009 分别由 RD-012 / RD-013 预留,不复用))——属语义归类裁决,**需人工升档**(PR 描述标注,落码归 owner 确认后的实现步,本条款体先以占位「6xxx」表述,不擅自落 `registry/error_codes.json`)。strict-only:错链必显式 6xxx,无运行期 fallback、无 skip 配置,校验失败终止该联编产物。
- IR4(测试锚定):本条 ≥1 `//@ spec: RXS-0160` 锚定——`check_stage_link` 单测(accept 链接一致 + reject 错链:缺链接键 / 类型失配 / 插值失配,工具无关恒跑)+ `link_graphics_stages` 多阶段联编接缝单测;conformance accept(链接一致 vertex+fragment 配对)host 侧确定性核对。错链 conformance reject 断言最终 6xxx + golden 随 owner 裁码后落(条款先于实现,golden bless 归 owner)。

### RXS-0161 图形着色阶段 MIR→SPIR-V 降级面

图形着色阶段经 **B 路**首段(MIR→SPIR-V,Rurix 自有,RFC-0004 §4.2(a))降级:着色阶段 → SPIR-V execution model;阶段 I/O(RXS-0154)→ SPIR-V `Location` / `BuiltIn` decoration;资源句柄类型面(RXS-0156)→ SPIR-V **opaque** 类型形态。SPIR-V 仅为 B 路内部 IR(D-008/SG-003),不作对外通用目标。

#### Syntax

MIR→SPIR-V 降级为 codegen 面,非语言文法面。

#### Legality

- L1(已建模子集):`vertex` / `fragment` execution model + 标量 / 向量 I/O + opaque 资源句柄类型(仅类型 / 传递,不涉访问语义)。
- L2(不可映射):未建模 builtin / 类型 / 阶段构造 → **RX6013**(`DxilError::Unmappable`,strict-only,不静默降级)。
- L3(deferred 阶段):`mesh` / `task` / RT execution model 本片不降级 → 承 RXS-0158 **RX6007** stub(RD-012)。
- 🔒(纹理禁区):纹理访问语义(描述符编码 / 采样·load·store opcode / 缓存 / LOD / 导数 / 越界)在本层**结构上不可达**(`MirIoType` 仅标量 / 向量,无法表达资源句柄 / 采样器);一旦类型面扩展触及,编码器在映射处发 RX6013 并标「需人工升档」,**不**发明 lowering / 二进制布局(RFC-0004 §4.6(b)、06 §4.2)。

#### Dynamic Semantics

MIR→SPIR-V 为编译期确定性变换,本条无运行期语言语义。给定图形阶段 MIR,SPIR-V 字流(execution model + decoration)对相同输入字节确定。

#### Implementation Requirements

- IR1(SPIR-V 编码器):[`dxil_spirv::emit_spirv`](../src/rurixc/src/dxil_spirv.rs) 以**纯 safe `Vec<u32>`** 手工 emit SPIR-V header(magic `0x07230203`)+ `Capability Shader` → `OpEntryPoint`(Vertex / Fragment execution model)→ `OpExecutionMode`(fragment `OriginUpperLeft`)→ 类型指令 → Input/Output 变量 → `Location` / `BuiltIn` / `UserSemantic` decoration → 平凡 passthrough `main`;无 `unsafe`。
- IR2(编码器合规):emit 产物经本机 `spirv-val` 验证无 error(Property 1)方进入 SPIR-V→HLSL 段;不合规 SPIR-V 不下传。
- IR3(收集根):DXIL 收集根(`build_device_crate`,feature `dxil-backend`)扩到含图形着色阶段入口并携 I/O 意图签名;非 feature 构建不收图形阶段(`io_sig` 空、PTX 路径零漂移,D-207)。
- IR4(资源句柄):仅 opaque 类型形态降级;访问语义未建模即 RX6013(不在本条)。

### RXS-0162 B 转译链确定性 + validator gate + 供应链 pin + strict-only 核验

图形着色阶段 B 全链(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL,RFC-0004 §4.2/§4.3)的确定性、validator gate、强制签名校验门叠加、供应链 pin 与 strict-only 核验。

#### Syntax

本条为 codegen / 工具链面,无语言文法面。

#### Legality

- L1(转译失败,strict-only):SPIR-V 不合规 / SPIRV-Cross 失败 / dxc validator reject → **RX6010**(`codegen.dxil_b_transpile_failed`,`DxilBError::Toolchain`,附失败阶段 + 原因);工具链缺失(定位 / spawn 失败)→ **SKIP**(非发码,环境降级,对齐 RXS-0073/RXS-0157)。
- L2(确定性):B 全链对给定 MIR 输入确定;同输入 ×N 容器 SHA256 须一致,漂移即红(golden 核对)。
- L3(strict-only,P-01):转译链任一段对用户声明 / 可观察签名元素静默降级或丢弃 = 违规;validator accept **不等于**用户签名意图保真,叠加 RXS-0159 IR2 强制签名校验门补足缺口,留不住即 6xxx,无例外、无 permissive 回退(RFC-0004 §4.4)。

#### Dynamic Semantics

B 全链为编译期确定性变换,本条无运行期语言语义(着色器在 D3D12 管线执行属运行时 / G2.3+,不在本条;运行期语义等价由 G-G2-2 device 真跑兑现)。

#### Implementation Requirements

- IR1(确定性核对):同 MIR 输入 ×N 容器 SHA256 全等(Property 3);纳入 golden 核对。
- IR2(validator gate):入 golden 前 DXIL 经 dxc validator 验证通过(不合规 DXIL 不得入 golden,对齐 RFC-0003 §9 Q-Golden)。
- IR3(校验门叠加):validator accept ≠ 用户签名保真,叠加 RXS-0159 强制签名一致性校验门(译后 ISG1/OSG1 vs MIR 意图,不可裁剪,RFC-0004 §4.4)。
- IR4(供应链 pin):SPIRV-Cross / dxc / glslang 经 lockfile `[[toolchain]]` + SHA256 pin(Q-Supply,env override 仅 dev/probe);再分发审计随 **RD-014**。**本片状态**:pin 段与 CI 转译链冒烟(`ci/dxil_codegen_smoke.py`,CI 步骤 46)随供应链落地子分片回填;本机实测工具(dxc 1.8.0.4739 / SPIRV-Cross / spirv-val,Vulkan SDK 1.3.296.0)为 pin 候选。
- IR5(golden 形态):仅 **DXIL 文本反汇编**入 golden(Q-Golden-B);`tests/dxil/graphics/*.dxil-disasm` 为 owner pin 环境 bless 后的文本基线,入库前须经签名 validator 接受。当前 `gfx_vs_min` 语料锁定已登记 RD-013/RD-017 缺口下的 `TEXCOORD` baseline,不声称 output varying 用户语义保真已兑现;device 真跑 / run URL / evidence 见 G-G2-2 远端 run。
- IR6(错误码):RX6010(B 链转译失败,`DxilBError::Toolchain`)。

## 3. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.6 | 2026-06-27 | **G-G2-2 owner 收口 + DXIL golden bless 落档**。owner 白栀于本工作会话监督确认 device 真跑 run URL、DXIL 文本 golden bless 与 G-G2-2 子里程碑签字;AI 代录机器事实,不代签 G2 整体 close-out。`tests/dxil/graphics/gfx_vs_min.dxil-disasm` 在 signed DXC pin 环境(`H:\dxc-round7\extracted\bin\x64` 含 `dxc.exe`/`dxv.exe`/`dxil.dll`)和显式 `spirv-cross.exe` 下经 `RURIX_BLESS=1 cargo test -p rurixc --features dxil-backend --test dxil_golden dxil_b_disasm_golden_matches_when_toolchain_present -- --exact --nocapture` 重 bless;入 golden 前 `dxv.exe` validator 接受,版本噪声规范化为 `OWNER-BLESSED-NORMALIZED`。远端 PR smoke [28284960733](https://github.com/qwasg/Rurix/actions/runs/28284960733) 全量 success,步骤 46 输出 `DXIL_DEVICE: ok adapter="NVIDIA GeForce RTX 4070 Ti" pixel=64,127,255,255 draw=ok`。当前 `gfx_vs_min` 仍为 RD-013/RD-017 缺口下的 TEXCOORD baseline,不关闭 deferred、不声称 output varying 用户语义保真。§2 仅更新当前 golden 形态说明;RXS-0160 计划映射、🔒 签名二进制 ABI 布局/纹理内存模型/DXIL·SPIR-V UB 边界仍不触及。| **Full RFC**（RFC-0004 / PR-D2） |
| v1.0 | 2026-06-24 | 新建 dxil_backend.md（PR-C1 spec 脚手架，承 RFC-0003 / D-131=A）:登记文件名 + 文件级语义面说明（MIR→DXIL 第二后端，承 RFC-0002 着色阶段类型面 RXS-0153~0156）+ §1 范围与 **RXS-0157~ 预留区间声明**（区间大小未锁定，随 RFC-0003 §9 Q-Range 与路径裁定一并定）+ §2 条款占位（条款体随 PR-C2 实现 PR 同落）。**沿 README v1.32 interop_d3d12.md / v1.33 async_buffer.md / v1.37 shader_stages.md 脚手架先例:仅登记文件名 + 预留区间，不落带编号裸条款头**——本文件**零 `### RXS-####` 条款头**，`ci/trace_matrix.py --check` 维持全锚定 **156/156**（无新增裸条款头、无悬空锚点、零新 RXS）。条款体（RXS-0157 起）与每条 ≥1 `//@ spec` 测试锚定随 PR-C2（DXIL 后端实现 PR）同落（条款 PR 先于实现 PR）。禁区声明:🔒 纹理路径内存模型映射（06 §4.2）/ FFI ABI 二进制布局（RFC-0003 §4.6 / §9 Q-Builtin）/ 绑定布局推导（G2.3，P-11）/ 多后端架构承诺（D-008/SG-003）均不在本文件，触及即停手升档。错误码 **6xxx codegen 段**脚手架不预造、不预留，随 PR-C2 按真实可达类别只追加。档位 **Full RFC**（RFC-0003;触 codegen 第二后端 + target 分发，AI 不自判 Direct，判档争议向上取严）。授权 G2_CONTRACT D-G2-2 / G-G2-2 + G2_PLAN G2.2 子里程碑，无体例变更 | **Full RFC**（RFC-0003） |
| v1.1 | 2026-06-24 | **PR-C2 分片1:落首条带编号条款体 `### RXS-0157`**(codegen target 分发与 DXIL 后端分叉)+ 配套最小 compute kernel 端到端实现(rurixc `dxil_codegen` 模块 + `--target dxil` 分发 + cargo feature `dxil-backend` + patched llc 经 `RURIX_LLC` dev env 定位 RD-011 + dxc validator accept)。条款体按 FLS 体例分 Syntax / Legality(L1 后端可用性·L2 最小子集·L3 降级失败 → RX6007)/ Dynamic Semantics / Implementation Requirements(IR1 分发点·IR2 D-131=A 路径·IR3 golden 文本反汇编经 validator·IR4 错误码 RX6007),**严禁 UB 节**。配套 conformance accept(空体 compute kernel 产 DXIL,`//@ spec: RXS-0157`)+ reject(子集外构造 → RX6007)+ DXIL golden(文本反汇编 + bless)。错误码新增 **RX6007**(6xxx codegen/目标段续接 RX6006,只追加)+ en/zh message-key(双语覆盖)。`ci/trace_matrix.py --check` 全锚定 **157/157**(新增 RXS-0157 带测试锚定、无悬空)。RXS-0158/0159/0160 仍为 §9 Q-Range 计划映射(非裸条款头),随后续分片落地。本片不碰 🔒 纹理内存模型映射 / FFI ABI 布局 / 绑定布局推导(G2.3)。档位 **Full RFC**(RFC-0003),无体例变更 | **Full RFC**（RFC-0003） |
| v1.2 | 2026-06-25 | **PR-D1 图形=B spec 脚手架(承 [RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md),owner Approved 2026-06-25)**:§1 编号区间 + §2 新增 **2.1 图形=B 条款计划映射(预留区间)**,登记 **RXS-0159 保号重构(按 B)**(A 路签名 ISG1/OSG1 `elemcount=0` 不可达 → D-131 v1.4 裁定图形=B → RXS-0159 保号、PR-D2 按 B 路径 SPIR-V `BuiltIn`/`Location` + 译后强制签名一致性校验门重写;#97 A 路 RXS-0159 stub 不入 main)+ 新增 **RXS-0160**(阶段间接口 → DXIL/SPIR-V 阶段链接一致性核对)/ **RXS-0161**(图形着色阶段 MIR→SPIR-V 降级面:execution model / I/O decoration / 资源句柄)/ **RXS-0162**(B 转译链确定性 + validator gate + 供应链 pin `[[toolchain]]` + SHA256 + strict-only 核验,含签名一致性校验门与 golden 形态)预留区间与重构说明。区间裁决已锁定(RFC-0004 §9 Q-Range-B);feature gate 复用 `dxil-backend`(Q-Gate-B);错误码归 6xxx 段只追加。RXS-0158(阶段着色器类型,#96 HOLD)维持计划映射不动。**全部以计划映射 / 预留区间登记,不落条款体、零 `### RXS-####` 裸条款头**——`ci/trace_matrix.py --check` 维持全锚定 **157/157**(无新增可锚条款、无悬空锚点、零新 RXS)。条款体随 PR-D2(B 转译实现 PR)同落(条款 PR 先于实现 PR,硬规则 7)。**本片不碰** 🔒 签名二进制 ABI 布局(RFC-0004 §4.6(a))/ 纹理路径内存模型映射(06 §4.2)/ DXIL·SPIR-V UB 边界——只引边界声明,不落禁区语义本体。档位 **Full RFC**(RFC-0004),无体例变更 | **Full RFC**（RFC-0004） |
| v1.3 | 2026-06-26 | **PR-D2:落条款体 `### RXS-0158`**(着色阶段着色 → DXIL 着色器类型降级对应),条款体**自 `origin/feat/g2.2-pr-c2-slice2-rxs0158:spec/dxil_backend.md` 整段照搬**(未改写措辞):含**着色器类型对应表**(compute/kernel→compute / vertex→vertex / fragment→pixel SM6.0 **已落**;mesh→mesh / task→amplification SM6.5、RT raygen·closesthit·anyhit·miss→library SM6.3 映射登记、实现 deferred)+ Syntax / Legality(L1 可降级阶段最小子集承 RXS-0157 / L2 deferred 阶段 → RX6008 / L3 降级失败 → RX6007)/ Dynamic Semantics / Implementation Requirements(IR1 阶段→着色器类型映射 + DXIL 收集根不改 PTX 根 / IR2 deferred 阶段 RX6008 / IR3 SM 下限登记 / IR4 错误码 RX6008),**严禁 UB 节、不定义 I/O 签名 ABI**。落地校正:**① 档位对齐 PR-D2 / RFC-0004**(承 D-131 混合裁决,非 slice2 旧 RFC-0003 语境);**② RD 编号分叉停手交 owner**:slice2 条款体引 **RD-012**(main `deferred.json` 已存在、已绑 RX6008,指 mesh/task/RT 缺口),本 spec requirements R5.7 要新开 **RD-016**(同一缺口)——**默认复用 RD-012、未擅自双开 RD-016**,在此分叉**标「需人工升档」交 owner 裁决**(deferred.json 未改、RD-016 未落);**③ RX6008 撞号核对**:main `registry/error_codes.json` 6xxx 段现为 RX6001~RX6007,**RX6008 未撞号**(deferred.json RD-012 已预引 RX6008 作此降级码,honor 既有引用不改派,registry 落条目 + status 翻转归 owner)。trace:本轮**仅落条款体**,`### RXS-0158` 三级标题需 ≥1 测试锚定(`//@ spec: RXS-0158`),锚定为后续任务,本轮不伪造锚定、不删条款头;若 trace 因新条款头无锚定而红,如实报告状态。**本片不碰** 🔒 纹理内存模型映射(06 §4.2)/ 内建变量·签名二进制 ABI 布局(RFC-0003 §4.6/§9 Q-Builtin)/ 绑定布局推导(G2.3,P-11)/ 阶段 I/O 签名 SV_*(RXS-0159)。档位 **Full RFC**(RFC-0004 / PR-D2),无体例变更 | **Full RFC**（RFC-0004 / PR-D2） |
| v1.4 | 2026-06-27 | **PR-D2:落图形=B 条款体 `### RXS-0159` / `### RXS-0161` / `### RXS-0162` + RXS-0158 实现一致性收口**。**RXS-0159**(阶段 I/O → DXIL 签名/系统值语义,按 B):by-construction `UserSemantic` 保名 + 强制签名一致性校验门(`signature_gate::check`,不可裁剪)+ 错误码 RX6011 签名不一致 / RX6012 声明输入被消除 / RX6013 不可映射(RX6009 已被 RD-013 预引,段内不复用改派);🔒 二进制 ABI 布局只边界声明(RFC-0004 §4.6(a))。**RXS-0161**(图形阶段 MIR→SPIR-V 降级面):`dxil_spirv::emit_spirv` 纯 safe `Vec<u32>` 编码器(execution model / Location·BuiltIn·UserSemantic decoration,经 spirv-val);纹理访问语义结构上不可达 → RX6013 升档(§4.6(b))。**RXS-0162**(B 转译链确定性 + validator gate + 供应链 pin + strict-only):RX6010 B 链转译失败;确定性 ×N SHA256 + dxc validator gate + 校验门叠加;供应链 `[[toolchain]]` pin + CI 步骤 46(`ci/dxil_codegen_smoke.py`)随供应链子分片回填,golden 仅 DXIL 文本反汇编、本机产物 NOT BLESSED 由 owner pin 环境重 bless。各条按 FLS 分 Syntax / Legality / Dynamic Semantics / Implementation Requirements,**严禁 UB 节**。错误码新增 **RX6010~RX6013**(6xxx 段续接 RX6007,只追加 + en/zh message-key;任务7 已分配)。**RXS-0158 收口**:经 owner 裁决**复用 RD-012**(不新开 RD-016,避同缺口双 RD),`dxil_codegen` 注释 RD-016→RD-012 对齐;mesh/task/RT deferred 阶段发码由 slice2 旧 A 框架的 **RX6008 改为实现实发的 RX6007**(沿 RXS-0157 通道,RX6008 维持 deferred.json RD-012 预留,落码归 RD-012 实现里程碑)——spec 与 impl 一致、无悬空码。**RXS-0160**(阶段间接口链接一致性核对)**本片不落条款体**:其 vertex out↔fragment in 链接核对需多阶段联编点,本片单着色阶段编译尚无该接缝,维持 §2.1 计划映射(非裸条款头,无悬空)、承后续里程碑。`ci/trace_matrix.py --check` 全锚定 **161/161**(RXS-0157~0159·0161·0162 各 ≥1 `//@ spec`,RXS-0160 无裸头不悬空)。**本片不碰** 🔒 签名二进制 ABI 布局(RFC-0004 §4.6(a))/ 纹理访问语义(§4.6(b)/06 §4.2)/ DXIL·SPIR-V UB 边界(§4.6(c));device 真跑 / 呈现对照 / run URL / evidence 归 owner(G-G2-2)。档位 **Full RFC**(RFC-0004 / PR-D2),无体例变更 | **Full RFC**（RFC-0004 / PR-D2） |
| v1.5 | 2026-06-27 | **RXS-0159 IR1 保名表述按实测收窄 + 顶点输入保名旗标接入生产(机制① 缺口闭合)**。背景:`dxil_codegen.rs` 生产 B 链调用 `spirv_cross_to_hlsl(.., &[])` 空 extra 旗标,致 RFC-0004 §4.4 机制① 顶点输入保名**未接入生产**——用户命名顶点输入退化为 `TEXCOORD#` → 校验门 RX6011 拒。实测定结论(本机 dxc 1.8.0.4739 / spirv-cross vulkan-sdk-1.3.296.0,贴真实 spirv-dis + dxc -dumpbin ISG1 输出):**(1)** spirv-cross **不消费** SPIR-V `UserSemantic` 装饰为 HLSL 语义(`OpDecorate ... UserSemantic` 经 spirv-val 干净但非保名机制);`--set-hlsl-named-vertex-input-semantic` 按 `OpName` 匹配,`emit_spirv` 不 emit `OpName` 故不命中;**保名经 `--set-hlsl-vertex-input-semantic <location> <semantic>` 按 `Location` 覆盖达成**,location 由 `emit_spirv` 按 io_sig 顺序确定性分配 → 经 io_sig 导出旗标(非硬编码),Rust-emit SPIR-V 路径下 **vertex 输入 `POSITION`/`NORMAL` 端到端存活**(measured)。**(2)** 输出 varying / fragment 输入 varying 用户语义名 **无保名旗标**(spirv-cross 无输出/片元语义旗标)→ 仍退化 `TEXCOORD#`。**处置(实测决定,非偏好)**:**接入生产**——`dxil_codegen::vertex_input_semantic_flags(stage, io_sig)` 经 io_sig 导出顶点输入保名旗标接入 `run_b_chain` 的 `spirv_cross_to_hlsl` 生产调用(原 `&[]`);**诚实收窄**——IR1 由「用户命名 varying/interpolate → by-construction UserSemantic 保名」收窄为「系统值名恒保真 + vertex 输入名 by-construction 保真(measured,location 覆盖)+ 输出/片元 varying 名当前不可保真 → RX6011 显式拒绝,缺口 deferred **RD-017**」。系统值 SV_Position/SV_VertexID 真达(elemcount>0)**不受影响**(B-over-A 核心,保留)。配套:`registry/deferred.json` 追加 **RD-017**(status 留 owner;**续号跳过 RD-016**——RD-016 经 `.kiro` requirements R5.7 owner 确认给 mesh/task/RT 缺口、已复用既有 **RD-012**、未创建 entry,按编号不复用续 RD-017)+ `dxil_spirv`/`dxil_codegen` 代码侧 `// STUB(RD-017)` 双侧标注 + 单测 `vertex_input_semantic_flags_derive_from_io_sig` + `toolchain` 冒烟 `b_chain_end_to_end_smoke` 升级为顶点输入名存活断言 + `ci/dxil_codegen_smoke.py` 顶点输入名保真断言绿检(输出 varying 退化维持 NOTE)。golden:既有图形语料(`gfx_vs_min.rx` 单 interpolate **输出** varying,无命名顶点输入)→ 保名旗标导出为空 → golden **不变**,无需重录。**不动 RFC-0004 措辞**(RFC §4.4 机制① 本已按顶点输入 measured 表述、未 over-broad,措辞勘误属 owner);不改 RXS-0160 计划映射;§3 既有行 0-byte。`ci/trace_matrix.py --check` 全锚定不变(RXS-0159 新单测携 `//@ spec: RXS-0159`)。**本片不碰** 🔒 寄存器/mask/packing 二进制 ABI 布局(§4.6(a))。device 真跑 / golden bless / RD-017 status 翻转归 owner(G-G2-2)。档位 **Full RFC**(RFC-0004 / PR-D2),无体例变更 | **Full RFC**（RFC-0004 / PR-D2） |
| v1.7 | 2026-06-27 | **落图形=B 条款体 `### RXS-0160`(阶段间接口 → 阶段链接一致性核对)+ 收口 §2.1 计划映射**。承 [RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md) §4.4/§5(owner Approved 2026-06-25),vertex out ↔ fragment in 跨阶段 varying 经 B 路降级后以**语义名等价为链接键**核实语义名 / 类型 / 插值一致性,错链 strict-only 必显式 6xxx(P-01,无运行期 fallback)。条款体按 FLS 分 Syntax / Legality(L1 可核对子集承 RXS-0159 / L2 链接键缺失 / L3 类型·插值失配 / 🔒 location 不比对)/ Dynamic Semantics / Implementation Requirements(IR1 `check_stage_link` 链接核对入口复用 RXS-0159 `semantic_name_matches` 语义名等价 / IR2 `link_graphics_stages` vertex+fragment 多阶段联编点接缝 / IR3 错误码待 owner 裁 / IR4 测试锚定),**严禁 UB 节**。**§2.1 收口**:RFC-0004 §5 下游条款 RXS-0159/0160/0161/0162 条款体全落地,无尚未落地计划映射。**判档点(需人工升档,owner 闸门)**:错链错误码语义归类——spec §2.1 旧文映射 RX6011,但 RX6011 现由 `codegen.dxil_sig_mismatch`(单阶段输出未保真)占用;6xxx 下一空号 = **RX6014**(RX6008/RX6009 分别由 RD-012/RD-013 预留不复用)。「复用 RX6011 同语义类」抑或「新开 RX6014」属语义归类裁决,**不擅自落码**——条款体先以占位「6xxx」表述,`check_stage_link` 返回类型化 `StageLinkError`(纯 host/safe,零新 unsafe),**不**接线生产 emit、**不**改 `registry/error_codes.json` / message-key;落码 + 错链 conformance reject + golden 归 owner 裁码后的实现步。配套:`check_stage_link` + `link_graphics_stages` 单测(accept 链接一致 + reject 缺链接键/类型失配/插值失配,工具无关恒跑,`//@ spec: RXS-0160`)+ conformance accept(`conformance/dxil/graphics/accept/` vertex+fragment 链接一致配对,host 侧确定性)。`ci/trace_matrix.py --check` 全锚定 **161→162**(RXS-0160 ≥1 `//@ spec` 锚定,无悬空)。**本片不碰** 🔒 寄存器/location 编号/mask/packing 二进制 ABI 布局(§4.6(a))/ 纹理访问语义(§4.6(b))/ DXIL·SPIR-V UB 边界(§4.6(c));device 真跑 / 呈现对照 / golden bless / 新错误码语义归类裁决归 owner(G-G2-2)。档位 **Full RFC**(RFC-0004 / PR-D2),无体例变更 | **Full RFC**（RFC-0004 / PR-D2） |
