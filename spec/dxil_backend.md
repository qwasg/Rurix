# Rurix 语言规范 — DXIL 第二后端语义面（MIR → DXIL，承 RFC-0003；G2.2 起）

> 条款:**RXS-0157 起续号**(G2.2 DXIL 第二后端语义面:codegen target 分发与 DXIL 后端分叉(MIR 之后 target 选择,PTX/DXIL 并存,strict-only 无 fallback)/ 着色阶段着色 → DXIL 着色器类型降级对应 / 阶段 I/O → DXIL 签名·系统值语义降级 / 阶段间接口 → DXIL 阶段链接一致性核对)。RFC-0003 §9 Q-Range 初始未锁定;经 RFC-0004 §9 Q-Range-B 锁定,当前映射为 RXS-0157 既有条款体 + RXS-0158 HOLD + RXS-0159 保号按 B 重构 + RXS-0160~0162 预留。体例见 [README.md](README.md)。
> 依据:**[RFC-0003](../rfcs/0003-dxil-backend.md)**(MIR→DXIL 第二后端与 compute=A 基线,**owner Approved** 2026-06-23;§9 Q-D131 后经追加式勘误由 A 增补为混合路径)+ **[RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md)**(图形=B,**owner Approved** 2026-06-25;D-131 当前裁决 = **compute=A / 图形=B**);06 §8.2(codegen 第二后端设计预留);06 §4.2(纹理路径内存模型禁区,🔒);07 §7(device codegen 分发:MVP/G1 维持 NVPTX→PTX,DXIL 第二后端 G2 重评估);07 §5(错误码段位:6xxx codegen/目标);spec/shader_stages.md RXS-0153~0156(着色阶段类型面,DXIL 降级的上游类型面来源)。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)(D-G2-2,G-G2-2)+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.2 子里程碑。
> 档位:**Full RFC**(RFC-0003 + RFC-0004;10 §3:本设计触 **codegen 第二后端 + target 分发 + 图形=B 第二中间表示与外部转译依赖**,有别于 M8 互操作的 Direct)。**D-131 当前路径为混合 compute=A / 图形=B**:compute A 路 dev 工具链偏差由 `registry/deferred.json` RD-011 跟踪;图形 B 路供应链由 RD-014 跟踪。**agent 自主判档**,判档以 RFC-0003/RFC-0004 与 G2_CONTRACT 授权为据,判档争议向上取严。任何触及 **🔒 纹理路径内存模型映射(06 §4.2)** / **FFI ABI 二进制布局(RFC-0003 §4.6/§9 Q-Builtin;RFC-0004 §4.6)** / **多后端架构承诺(D-008/SG-003)** 的条款,必须停下标注「需升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):target 不支持 / 降级失败 / 非法 I/O 映射以 **编译期 6xxx codegen 诊断(P-01 strict-only,无运行期 fallback)**定义,不以 UB 表述(RFC-0003 §3/§4;RFC-0004 §4.4/§4.6)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**PR-C2 分片1 起**已落首条带编号条款体 **RXS-0157**(codegen target 分发与 DXIL 后端分叉)+ 每条 ≥1 测试锚定(条款 PR 先于/同实现 PR,G2.1 PR-B2 先例),trace_matrix 全锚定 **157/157**;后续条款(RXS-0158~)随分片续落。

---

## 1. 范围与编号区间

本文件承载 **MIR → DXIL 第二后端语义面**的语义条款(G2.2+,D-G2-2)。承 RFC-0002 着色阶段类型面(RXS-0153~0156)产出的着色阶段语义,定义其经工具链降级到 DXIL(可被 D3D12 PSO 消费的着色器对象)的语义。覆盖语义面(RFC-0003 §4):

- **codegen target 分发与 DXIL 后端分叉**:MIR 之后的 target 选择(`rx build --target dxil`,与现状 `--target ptx` 并列,RFC-0003 §9 Q-CLI),PTX 后端(D-207)与 DXIL 后端**并存**;target 不支持的构造 → **6xxx codegen 错误**(strict-only,无运行期 fallback,P-01)。DXIL 后端 gate 于 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate;未启用时 DXIL 后端不参与编译,PTX 路径不受影响)。
- **着色阶段着色 → DXIL 着色器类型降级对应**:RXS-0153 的着色阶段着色(`vertex`/`fragment`/`compute`/`mesh`/`task` + RT `raygen`/`closesthit`/`anyhit`/`miss`)降级为对应 DXIL 着色器类型(vertex/pixel/mesh/amplification/RT/compute shader)。精确对应表随 PR-C2 条款体落地。
- **阶段 I/O → DXIL 签名/系统值语义降级**:RXS-0154 的阶段专属 I/O(`#[interpolate]` 插值限定 / `#[builtin]` 内建变量)降级为 DXIL 输入/输出签名与系统值语义(SV_*)。内建变量 → DXIL 系统值语义名为**类型面映射**;其二进制 ABI 布局属 RFC-0003 §4.6/§9 Q-Builtin 的 🔒 FFI ABI 禁区,**不在本文件**,留 agent 后续独立 Full RFC。
- **阶段间接口 → DXIL 阶段链接一致性核对**:RXS-0155 的阶段间接口契约(vertex out → fragment in varying 兼容)在类型面已编译期校验(RXS-0155),DXIL 层为降级一致性核对。

DXIL 后端语义维持 **D-131 = 混合 compute=A / 图形=B**:compute 经 LLVM DirectX 后端直接 emit DXIL,与 NVPTX 后端同构并维持 D-205 LLVM 单栈;图形经内部 MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL 链(RFC-0004),SPIR-V 仅为图形 B 路内部中间表示,不构成对外通用多后端承诺。**🔒 纹理/采样器内存模型映射**(06 §4.2 禁区:tex proxy / 采样 opcode / 描述符编码 / 缓存一致性 / UB)、**内建变量/根参数/常量缓冲二进制 ABI 布局**(RFC-0003 §4.6 / RFC-0004 §4.6 FFI ABI 禁区)、**绑定布局推导实现**(G2.3,P-11)均**不在本文件**;DXIL golden 取**文本反汇编形态** + 经 dxc validator 验证后入 golden(RFC-0003 §9 Q-Golden / RFC-0004 §9 Q-Golden-B)。target 不支持 / 降级失败 / 非法 I/O 映射以 **编译期 6xxx codegen 诊断(P-01 strict-only)**定义,**不以 UB 表述**。

**编号区间**:本文件条款自 **RXS-0157** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0156 @ [shader_stages.md](shader_stages.md))。区间裁决:**RFC-0004 §9 Q-Range-B 锁定** RXS-0159 保号按 B 重构 + RXS-0160~0162 新增面。**已落带编号条款体**:RXS-0157(target 分发)/ **RXS-0158**(着色阶段 → 着色器类型,PR-D2)/ **RXS-0159**(阶段 I/O 签名,按 B)/ **RXS-0160**(阶段间接口 → 阶段链接一致性核对,按 B)/ **RXS-0161**(MIR→SPIR-V 降级面)/ **RXS-0162**(B 转译链确定性 + validator gate + 供应链 pin)/ **RXS-0171**(RD-013 body I/O 数据流降级最小切片)/ **RXS-0172**(输出 / fragment 输入 varying 用户语义名保名,RD-017,选项① HLSL 边界改写)/ **RXS-0173**(fragment 输出 → `SV_Target#` 渲染目标系统值映射,RD-017 片元输出 MRT 边界,G2.4)。**RXS-0160** 的 vertex out↔fragment in 链接核对 vertex+fragment 多阶段联编点接缝(`link_graphics_stages`)+ 链接核对入口(`check_stage_link`)已落,承 RXS-0159 语义名等价基件;**错链错误码 = RX6014**(agent 裁定方案 B 新开码,不复用 RX6011,见 §2 RXS-0160 IR3),G2.3 PR-E2b-2 已落码。**D-131 v1.4 裁定混合 compute=A / 图形=B**(13 §D-131;RFC-0004 owner Approved 2026-06-25):图形 I/O 签名降级(RXS-0159)由 A 路类型面 stub 改 **B 路**(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL)重写,A 路签名产物 ISG1/OSG1 `elemcount=0` 不可达(上游 #90504/#57928,RFC-0004 §4.5),**#97 的 A 路 RXS-0159 不入 main,由 PR-D2 统一按 B 重写**。条款体与每条 ≥1 测试锚定随实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 全锚定,RXS-0173 入后 **173/173**)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 本节落带编号条款体。**已落**:RXS-0157(codegen target 分发与 DXIL 后端分叉)/ RXS-0158(着色阶段 → DXIL 着色器类型)/ RXS-0159(阶段 I/O → DXIL 签名/系统值语义,按 B)/ RXS-0160(阶段间接口 → 阶段链接一致性核对,按 B)/ RXS-0161(图形着色阶段 MIR→SPIR-V 降级面)/ RXS-0162(B 转译链确定性 + validator gate + 供应链 pin + strict-only 核验)/ RXS-0171(RD-013 body I/O 数据流降级最小切片)/ RXS-0172(输出 / fragment 输入 varying 用户语义名保名,RD-017,选项① HLSL 边界改写)/ RXS-0173(fragment 输出 → SV_Target# 渲染目标系统值映射,RD-017 片元输出 MRT 边界)。**RXS-0160 错链错误码 = RX6014**(agent 裁定方案 B 新开码,不复用 RX6011,见 RXS-0160 IR3),G2.3 PR-E2b-2 已落码。
> 各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(target 不支持 / 降级失败以编译期 6xxx codegen 诊断定义,P-01 strict-only,无运行期 fallback;10 §7.5)。**本片不碰** 🔒 纹理内存模型映射(06 §4.2 禁区)/ FFI ABI 二进制布局(RFC-0003 §4.6 / §9 Q-Builtin)/ 绑定布局推导(G2.3,P-11);触及即停手升档。

### 2.1 图形=B 条款计划映射收口(RFC-0004)

> **收口**:RFC-0004 §5 下游条款(RXS-0159 / 0160 / 0161 / 0162)**条款体已全部落地(下文 §2)**,本小节无尚未落地的计划映射(零 `### RXS-####` 三级标题,trace_matrix 不计本小节)。承 [RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md) §5;feature gate 复用 `dxil-backend`(Q-Gate-B);错误码归 **6xxx 段**(只追加)。

- **RXS-0160(已落条款体,见 §2)— 阶段间接口 → 阶段链接一致性核对**:vertex out ↔ fragment in 链接核对的 **vertex+fragment 多阶段联编点**接缝([`dxil_codegen::link_graphics_stages`](../src/rurixc/src/dxil_codegen.rs))与链接核对入口([`signature_gate::check_stage_link`](../src/rurixc/src/dxil_sig_gate.rs))已落,承 RXS-0159 语义名等价 / 系统值匹配基件。**错链错误码 = RX6014**(agent 裁定方案 B 新开码,不复用 RX6011,RX6011 现由 `codegen.dxil_sig_mismatch` 单阶段输出未保真占用,语义不同):落码(`registry/error_codes.json` + 双语 message-key + 生产 emit 接线 [`dxil_codegen::emit_stage_link_error`](../src/rurixc/src/dxil_codegen.rs))已随 G2.3 PR-E2b-2 落地,错链 conformance reject + golden 随 agent 闸门后落(条款先于实现,硬规则 7)。

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
- IR3(SM 下限登记):上表 shader model 下限随阶段登记(光栅/compute SM6.0、mesh/amp SM6.5、RT SM6.3);本片落地阶段(SM6.0)的着色器类型映射经各自后端实测(compute=A 路;vertex/fragment=B 路 dxc,见 RXS-0161/0162),SM6.5/6.3 阶段(mesh/task/RT)为 deferred 阶段的映射登记值,无 passing 测试(RD-012);DXIL golden 经 dxc validator 验证,由 agent pin 环境 bless。
- IR4(错误码):本条**不新增错误码**——mesh/task/RT deferred 阶段暂沿 RXS-0157 **RX6007** 通道显式拒绝(`registry/deferred.json` RD-012 已预留专用码 RX6008,落码归 RD-012 实现里程碑,不在本片);子集外 / 降级失败亦归 RX6007(承 RXS-0157)。

### RXS-0159 阶段 I/O → DXIL 签名/系统值语义降级（B 路）

vertex/fragment 阶段 I/O(RXS-0154 `#[builtin]` / `#[interpolate]` + 字段名,spec/shader_stages.md)经 **B 路**(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL,RFC-0004 §4.2)降级为 DXIL ISG1/OSG1 签名。**D-131 v1.4 裁定图形=B**:A 路签名产物 `elemcount=0` 不可达(上游 #90504,RFC-0004 §4.5),本条按 B 路重写(承 §2.1 v1.2 计划)。本条只承诺**源码层签名元素的存在性 / 语义名 / 系统值 / 插值 / 方向**;🔒 寄存器号 / component mask / packing / 字节偏移**不属本条承诺**(RFC-0004 §4.6(a),外部 conformance,校验门不比对布局)。

#### Syntax

阶段 I/O 降级为 codegen 面,非语言文法面:阶段 I/O 由阶段函数签名(RXS-0154 `#[builtin]` / `#[interpolate]` 标注 + 字段名)给定,不因 DXIL 降级改写。

#### Legality

- L1(可降级 I/O 子集):`vertex` / `fragment` 的已建模 `#[builtin]` / `#[interpolate]` / 普通 varying(RXS-0154)+ 标量 / 向量类型可降级。子集外构造(资源句柄形参等非 I/O 签名面 / 未建模类型)→ **RX6013**(不可映射构造,`codegen.dxil_unmappable`)。
- L2(不可映射内建 / 类型):未建模 builtin / 越界向量宽度 / 阶段不符的系统值 → **RX6013**(由编码器 [`dxil_spirv`](../src/rurixc/src/dxil_spirv.rs) `DxilError::Unmappable` 透传)。
- L3(签名未保真,strict-only):B 链译后 ISG1/OSG1 与 MIR 意图签名经强制校验门比对——用户声明 / 可观察元素(输出方向语义名 / 系统值)缺失·改名·错配 → **RX6011**(`codegen.dxil_sig_mismatch`);声明的外部输入(`dir == In`)被消除且不可等价保留 → **RX6012**(`codegen.dxil_sig_dropped_input`)。无静默降级 / 丢弃(P-01,RFC-0004 §4.4)。
- 🔒(布局边界):承诺具体寄存器 / mask / 偏移值越出本条 = RFC-0004 §4.6(a) 禁区,**需升档**(owner 独立 Full RFC);本条仅作存在性 / 语义名 / 系统值边界声明,校验门**不**比对二进制布局。

#### Dynamic Semantics

阶段 I/O 降级为编译期确定性变换,本条无运行期语言语义。给定阶段 I/O MIR,SPIR-V `Location` / `BuiltIn` / `UserSemantic` decoration 与下游 DXIL 签名对相同输入字节确定(确定性核对见 RXS-0162)。

#### Implementation Requirements

- IR1(系统值 / 语义名映射):`#[builtin]` → DXIL 系统值(`position`→SV_Position、`vertex_index`→SV_VertexID、`instance_index`→SV_InstanceID、`frag_coord`→SV_Position、`frag_depth`/`depth`→SV_Depth 等,按阶段 + 方向),**系统值名经 B 链恒保真**(实测 SV_Position / SV_VertexID 真达,elemcount>0;B-over-A 核心)。用户命名 I/O 语义名保名**按方向收窄**(实测定结论,`evidence/dxil_b_strict_only_report.md` §3 + 本机 dxc 1.8.0.4739 / spirv-cross 复现):**(a) vertex 阶段输入**用户语义名 → **by-construction 保真**——[`dxil_spirv::emit_spirv`](../src/rurixc/src/dxil_spirv.rs) 按 io_sig 顺序 emit `Location` 装饰,SPIR-V→HLSL 段经 `spirv-cross --set-hlsl-vertex-input-semantic <location> <semantic>`(由 [`dxil_codegen::vertex_input_semantic_flags`](../src/rurixc/src/dxil_codegen.rs) 经 io_sig **导出、非硬编码**)按 location 覆盖,`POSITION` / `NORMAL` 端到端**不退化为通用 `TEXCOORD#`**(RFC-0004 §4.4 机制①,measured 顶点输入名存活);**(b) 输出 varying 与 fragment 输入 varying**用户语义名 → **当前不可保真**:spirv-cross HLSL 后端无输出 / 片元输入语义保名旗标,且**不消费** SPIR-V `UserSemantic` 装饰为 HLSL 语义(实测;`UserSemantic` 仅作 SPIR-V 层 provenance + 经 spirv-val 干净保留,非保名机制),退化为 `TEXCOORD#` → 经强制校验门 **RX6011 显式拒绝**(不静默通过,P-01);该输出 / 片元 varying 保名能力缺口 deferred 经 **RD-017**(回填条件 / 承接里程碑见 `registry/deferred.json`,status 留 agent;**保名机制条款见 RXS-0172**,选项① HLSL 边界改写,实现随 RD-017 实现 commit 落)。映射在 [`dxil_spirv`](../src/rurixc/src/dxil_spirv.rs)(SPIR-V 装饰)+ [`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs)(顶点输入保名旗标导出)emit。
- IR2(强制签名一致性校验门,不可裁剪):[`signature_gate::check`](../src/rurixc/src/dxil_sig_gate.rs) 比对译后 `actual`(ISG1/OSG1)与 MIR 意图 `intent`,比较域 = 语义名 / 系统值 / 被用输入元素(**不**取寄存器 / mask / 顺序,ABI 中立);失败 → RX6011 / RX6012,strict-only 终止该入口产物、不产 golden。**不存在跳过校验的配置**(RFC-0004 §4.4 不可裁剪)。
- IR3(错误码):RX6011(签名不一致)/ RX6012(声明输入被消除)/ RX6013(不可映射)。RX6009 已被 `registry/deferred.json` RD-013(阶段 I/O 入口 body 数据流降级)预引占用,本条两类按段内不复用改派 RX6011 / RX6012(6xxx 段续接,只追加 + en/zh message-key)。
- IR4(入口 body deferred):本条覆盖阶段 I/O **签名类型面**;入口 body 数据流降级(真实读写 I/O 的语句级 codegen)属 **RD-013**,不在本条。

### RXS-0160 阶段间接口 → 阶段链接一致性核对（B 路）

RXS-0155 在类型面已编译期校验阶段间接口契约(vertex out → fragment in varying 名 / 类型 / 插值兼容,RX3012)。本条为该契约在 **DXIL 降级层**的一致性核对:vertex 阶段输出 varying 与 fragment 阶段输入 varying 经 **B 路**(RXS-0161/0162)降级后,以**语义名等价为链接键**核实跨阶段配对的语义名 / 类型 / 插值限定保真,错链即显式 6xxx(strict-only,无运行期 fallback,P-01)。本条承 RXS-0159 单阶段签名保真核对([`signature_gate::check`](../src/rurixc/src/dxil_sig_gate.rs))的语义名等价 / 系统值匹配基件,新增 **vertex+fragment 多阶段联编点**([`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs) 由单阶段编译扩到收集两阶段 io_sig 汇集到链接核对点)。本条只承诺**跨阶段 varying 的链接键存在性 / 语义名等价 / 类型 / 插值一致性**;🔒 寄存器号 / location 编号 / component mask / packing **不属本条承诺**(属 RFC-0004 §4.6(a) ABI 禁区,链接核对以语义名为键、不比对 location 数值)。

#### Syntax

阶段间接口链接核对为 codegen 面,非语言文法面:跨阶段 varying 由各阶段函数签名(RXS-0154 `#[interpolate]` / 字段名)给定,不因 DXIL 降级改写。链接核对不引入新文法。

#### Legality

- L1(可核对子集):`vertex` 输出方向(`dir == Out`)与 `fragment` 输入方向(`dir == In`)的已建模 varying / interpolate(RXS-0154)+ 标量 / 向量类型(RXS-0159 子集)。builtin 系统值(如 `position` / `frag_coord`)为阶段内系统值(经光栅器,非跨阶段用户 varying 链接键),不参与本核对。
- L2(链接键缺失,strict-only):fragment 输入 varying 在上游 vertex 输出中无同**语义名等价**链接键(错链:缺链接)→ **RX6014** `codegen.dxil_stage_link_mismatch`(agent 裁定方案 B 新开码,见下 IR3)。
- L3(链接键类型 / 插值失配,strict-only):语义名等价的链接键两端**类型不一致**或**插值限定不一致**(错链:类型 / 插值失配)→ **RX6014**(同 L2,阶段间接口错链)。
- 🔒(布局边界):承诺具体寄存器 / location 编号 / mask 越出本条 = RFC-0004 §4.6(a) 禁区,**需升档**;本条链接核对以语义名等价为键,**不**比对 location 数值(ABI 中立,对齐 RXS-0162 Property 7)。

#### Dynamic Semantics

阶段间链接核对为编译期确定性变换,本条无运行期语言语义(着色器在 D3D12 管线的跨阶段数据传递属运行时 / G2.3+,不在本条;运行期语义等价由 G-G2-2 device 真跑兑现)。给定两阶段 I/O 意图签名,链接核对结论(一致 / 错链分类)对相同输入确定。

#### Implementation Requirements

- IR1(链接核对入口,不可裁剪):[`signature_gate::check_stage_link`](../src/rurixc/src/dxil_sig_gate.rs)`(vs_out_sig, fs_in_sig)` 比对 vertex 输出方向与 fragment 输入方向的 varying / interpolate 元素,以语义名等价(大小写无关 + 剥语义 index 后缀,复用 RXS-0159 `semantic_name_matches`)为链接键:键缺失 / 两端类型或插值限定不一致 → 失败(strict-only,绝不静默通过)。比较域 = 语义名 / 类型 / 插值限定;**不**取 location 编号 / 寄存器 / mask(ABI 中立,RFC-0004 §4.6(a))。
- IR2(多阶段联编点):[`dxil_codegen`](../src/rurixc/src/dxil_codegen.rs) 由单着色阶段编译扩到 **vertex+fragment 配对编译接缝**——收集两阶段 body 的 `io_sig` 汇集到链接核对点(`link_graphics_stages`);无 vertex+fragment 配对(单阶段编译 / 缺一阶段)→ 无链接核对(behavior 不变,A 路 / 单阶段零漂移,对齐 RXS-0157 R6.7)。
- IR3(错误码,agent 裁定方案 B 已落):错链 → **RX6014** `codegen.dxil_stage_link_mismatch`(6xxx 段当时下一空号;`RX6008` / `RX6009` 分别由 RD-012 / RD-013 预留,不复用)——agent 裁定**新开 RX6014**,**不**复用 RX6011(RX6011 = 单阶段输出签名不一致,语义不同)。落码 + 双语 message-key + 生产 emit 接线([`dxil_codegen::emit_stage_link_error`](../src/rurixc/src/dxil_codegen.rs))已随 G2.3 PR-E2b-2 落地。strict-only:错链必显式 RX6014,无运行期 fallback、无 skip 配置,校验失败终止该联编产物。
- IR4(测试锚定):本条 ≥1 `//@ spec: RXS-0160` 锚定——`check_stage_link` 单测(accept 链接一致 + reject 错链:缺链接键 / 类型失配 / 插值失配,工具无关恒跑)+ `link_graphics_stages` 多阶段联编接缝单测(reject 经 `emit_stage_link_error` 断言落 **RX6014**)+ `emit_stage_link_error_routes_to_rx6014` emit 分派单测;conformance accept(链接一致 vertex+fragment 配对)host 侧确定性核对。错链 conformance reject 断言 **RX6014** + golden 随 agent 闸门后落(golden bless 归 agent)。

### RXS-0161 图形着色阶段 MIR→SPIR-V 降级面

图形着色阶段经 **B 路**首段(MIR→SPIR-V,Rurix 自有,RFC-0004 §4.2(a))降级:着色阶段 → SPIR-V execution model;阶段 I/O(RXS-0154)→ SPIR-V `Location` / `BuiltIn` decoration;资源句柄类型面(RXS-0156)→ SPIR-V **opaque** 类型形态。SPIR-V 仅为 B 路内部 IR(D-008/SG-003),不作对外通用目标。

#### Syntax

MIR→SPIR-V 降级为 codegen 面,非语言文法面。

#### Legality

- L1(已建模子集):`vertex` / `fragment` execution model + 标量 / 向量 I/O + opaque 资源句柄类型(仅类型 / 传递,不涉访问语义)。
- L2(不可映射):未建模 builtin / 类型 / 阶段构造 → **RX6013**(`DxilError::Unmappable`,strict-only,不静默降级)。
- L3(deferred 阶段):`mesh` / `task` / RT execution model 本片不降级 → 承 RXS-0158 **RX6007** stub(RD-012)。
- 🔒(纹理禁区):纹理访问语义(描述符编码 / 采样·load·store opcode / 缓存 / LOD / 导数 / 越界)在本层**结构上不可达**(`MirIoType` 仅标量 / 向量,无法表达资源句柄 / 采样器);一旦类型面扩展触及,编码器在映射处发 RX6013 并标「需升档」,**不**发明 lowering / 二进制布局(RFC-0004 §4.6(b)、06 §4.2)。

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
- IR5(golden 形态):仅 **DXIL 文本反汇编**入 golden(Q-Golden-B);`tests/dxil/graphics/*.dxil-disasm` 为 agent pin 环境 bless 后的文本基线,入库前须经签名 validator 接受。当前 `gfx_vs_min` 语料锁定已登记 RD-013/RD-017 缺口下的 `TEXCOORD` baseline,不声称 output varying 用户语义保真已兑现;device 真跑 / run URL / evidence 见 G-G2-2 远端 run。
- IR6(错误码):RX6010(B 链转译失败,`DxilBError::Toolchain`)。

### RXS-0171 DXIL 图形=B 着色 body I/O 数据流降级（RD-013 最小切片）

本条解锁 RD-013 的最小语言语义切片:图形=B 路的 vertex/fragment 着色 body 可读取已声明 Input I/O 元素、执行白名单标量/向量表达式、并写出已声明 Output I/O 元素。owner 代表在 2026-06-29 本工作会话裁定 Q1–Q4 推荐组合:Q1=C1-a(形参/返回值值语义)、Q2=C2-a(字段序绑定)、Q3=C3-c(ABI 中立分层)、Q4=C4-c refined(白名单子集 + RX6013 strict-only)。本条只承诺源码层 I/O 数据流,不关闭 RD-013 的 golden/device/bless 义务,不签 G-G2-4。

#### Syntax

本条不新增语法。着色阶段函数按 RXS-0153/RXS-0154 既有语法书写:`vertex fn` / `fragment fn` 的 I/O 结构体形参表示输入 I/O,返回 I/O 结构体表示输出 I/O。函数 body 内对形参字段的读取是普通值读取;返回 I/O 结构体值是普通返回值。

#### Legality

- L1(值语义):输入 I/O 结构体作为普通形参按值读取;输出 I/O 结构体作为普通返回值写出。不得发明全局 I/O 变量、隐式 inout、隐式副作用通道或运行期 fallback。
- L2(字段序绑定):参数 I/O 结构体字段按源码声明序绑定到 `io_sig` 中 `dir == In` 的元素;返回 I/O 结构体字段按源码声明序绑定到 `dir == Out` 的元素。资源句柄形参与 `resources` 归 RXS-0163~0166,不参与本条字段序绑定。
- L3(ABI 中立边界):本条只承诺源码层 I/O 元素、方向、字段序、语义名/系统值、插值种类与标量/向量类型。**不**承诺 DXIL register、component mask、packing、字节布局、root signature 布局或稳定 SPIR-V `Location` 数值;触及显式 register/mask/packing/byte layout 即停手升 agent Full RFC(§4.6 禁区)。
- L4(最小 rvalue 白名单):首期 body lowering 仅支持 `Use`、f32/i32/u32 `Const`、标量或向量 f32/i32/u32 加/减/乘/除 `BinaryOp`,以及“声明的输出 I/O 聚合返回值”的机械分解。控制流分支/循环、调用、借用/引用、cast、unary、enum/variant/discriminant、资源/纹理/采样访问、非输出 I/O 聚合或其他 rvalue → **RX6013**(`codegen.dxil_unmappable`,strict-only)。
- L5(输出完整性):存在 `dir == Out` 元素时,body 必须在返回前写出所有 Output I/O 元素;缺失、字段数不一致或类型不一致 → **RX6013**。无 Output I/O 时仅允许 `unit` 返回。

#### Dynamic Semantics

图形=B body lowering 是编译期确定性变换。运行时可观察语义为:每次着色器调用读取当前 invocation 的输入 I/O 值,按白名单表达式求值,并把返回 I/O 结构体字段值写入对应输出 I/O 元素。本条不定义未建模构造的运行期行为;不支持即编译期 6xxx 诊断,不设 UB 节。

#### Implementation Requirements

- IR1(生产接线):图形=B 生产分发必须把完整 [`Body`](../src/rurixc/src/mir.rs)(`blocks` / `locals` / `arg_count` / `io_sig` / `resources`)送入 body-aware 降级入口;不得继续以只消费 `stage + io_sig + resources` 的 void stub 作为生产路径。
- IR2(SPIR-V lowering):Input place 读取降为 `OpLoad`;f32/i32/u32 常量降为 `OpConstant`;白名单算术降为对应 SPIR-V 算术 op;输出 I/O 聚合返回值逐 Out 元素降为 `OpStore`;函数仍以 `OpReturn` 收尾。产物必须保持 `spirv-val` 干净。
- IR3(签名门不旁路):RXS-0159 强制签名一致性校验门仍在 B 链末尾运行。body lowering 不能裁剪 `signature_gate::check`,不能以 validator accept 代替签名意图保真。
- IR4(strict-only 诊断):白名单外 MIR body 构造经 [`DxilBError::Spirv`](../src/rurixc/src/dxil_codegen.rs) 映射为 **RX6013**;工具链转译失败仍按 RXS-0162 使用 RX6010,签名门失败仍按 RXS-0159 使用 RX6011/RX6012。
- IR5(测试锚定):≥1 `//@ spec: RXS-0171` 覆盖字段序绑定、Input `OpLoad`、Output 聚合分解 `OpStore`、常量/二元算术、unsupported rvalue → RX6013,并在 `conformance/dxil/graphics/accept` 覆盖 vertex 输出写入与 fragment 输入读取后写出。

### RXS-0172 DXIL 图形=B 输出 varying / fragment 输入 varying 用户语义名保名（RD-017）

本条关闭 RXS-0159 IR1(b) 留下的语言层缺口:**输出 varying**(vertex/fragment 输出方向)与 **fragment 输入 varying** 的用户语义名经 B 链端到端保真。owner 代表在 2026-06-29 本工作会话裁定:采**选项①**(spirv-cross 产 HLSL 后于 HLSL 边界做受限、可验证的语义名 token 改写),**否决选项③**(不放宽 `signature_gate`、不以 location 等价冒充保名)。机制取证见 `evidence/rd017_varying_semantic_spike_20260629.json`(隔离 spike measured_local:改写后 dxc 接受 + 校验门不放宽也过 + 物理 ABI 不变 + 确定性)。本条只承诺源码层用户语义名的端到端存活;**不**放宽 RXS-0159 的 RX6011 标准,**不**关闭 RD-017 的 golden/device/bless 义务,**不**签 G-G2-4。

#### Syntax

本条不新增语法。输出/输入 varying 由阶段函数 I/O 结构体字段(RXS-0154 `#[interpolate]` / 普通 varying + 字段名)给定,不因保名改写改写源码。

#### Legality

- L1(保名范围):输出 varying(`dir == Out` 的 `Varying`/`Interpolate`)与 fragment 输入 varying(`dir == In`)的用户语义名经 B 链降级后,以**等价语义名**(`signature_gate::semantic_name_matches`:大小写无关 + 剥尾随语义 index 数字)出现于译后 DXIL ISG1/OSG1 签名,**不退化为通用 `TEXCOORD#`**。vertex 阶段输入保名维持 RXS-0159 IR1(a) 机制①;`#[builtin]` 系统值维持 RXS-0159 系统值映射,均不在本条改写面。
- L2(不放宽校验门):RXS-0159 的 **RX6011** 保名标准**不变**——语义名必须真实存活;**location 等价不算保名**(否决选项③)。保名失败 → RX6011 显式拒(不静默通过,P-01);本条**不得**为让保名测试通过而旁路或放宽 `signature_gate`(Property 5)。
- L3(ABI 中立边界):保名改写**只动** HLSL struct field 的 semantic token(从 `io_sig` 的 location→用户语义 provenance 恢复)。**不**冻结或承诺 DXIL register、component mask、packing、字节布局、稳定 SPIR-V `Location` 数值,**不**碰 resource/texture/sampler 语义。语义 index 后缀(如 `TEXCOORD0` 的 `0`)随保名恢复自然变化(改名的确定性后果,校验门**不比对**该维度,RXS-0159 口径),**非**物理 ABI 触碰。触及显式 register/mask/packing/byte layout/稳定 Location 即**停手升 agent Full RFC**(§4.6 禁区)。
- L4(fail-closed):保名改写**仅在** location→用户语义 provenance 明确映射时施加;provenance 不一致(回译 HLSL 的 `TEXCOORD#` 计数/位置与 `io_sig` 期望 varying 不符)时**不改写**该元素,退化名保留 → 经 RX6011 拒,**绝不静默放过**(strict-only 失败闭合)。

#### Dynamic Semantics

保名改写是编译期确定性变换:于 spirv-cross 产 HLSL 与 dxc 之间施加的纯文本 semantic token 替换,对相同输入字节确定(承 RXS-0162 确定性链,同输入两次产出一致)。本条无运行期语言语义;着色器运行期 I/O 语义承 RXS-0171 不变。

#### Implementation Requirements

- IR1(provenance 同源):location→用户语义名映射由 `io_sig` **导出、非硬编码**,与 [`dxil_codegen::vertex_input_semantic_flags`](../src/rurixc/src/dxil_codegen.rs) **同源**口径(varying/interpolate 按方向各自递增 `Location`,`#[builtin]` 不占 location;对齐 [`dxil_spirv::emit_spirv`](../src/rurixc/src/dxil_spirv.rs) 的 `next_in_location`/`next_out_location`)。输出方向取 `dir == Out`,fragment 输入方向取 `dir == In`。
- IR2(改写点):改写在 B 链 [`dxil_codegen::run_b_chain`](../src/rurixc/src/dxil_codegen.rs) 的 spirv-cross 产 HLSL 之后、dxc 之前施加;只替换目标 struct(输出方向 = spirv-cross 输出 struct;fragment 输入方向 = 输入 struct)的 field semantic token,不动行结构、类型、字段名、寄存器 packing。
- IR3(机制实测依据):spirv-cross HLSL 后端对 varying 默认发 `TEXCOORD<location>`(measured,`evidence/rd017_varying_semantic_spike_20260629.json`);改写按 location 复原用户名。机制依赖该约定 → 登记 **spirv-cross pin 复验点**(上游若改默认 varying 命名,改写探测侧须复验,留痕 `registry/deferred.json` RD-017 history)。
- IR4(门不旁路):RXS-0159 强制签名一致性校验门(`signature_gate::check`)仍在 B 链末尾运行;保名改写**不裁剪门、不放宽** `semantic_name_matches`。改写后名仍不等价 → RX6011 拒(L2/L4)。
- IR5(错误码):**不新增错误码**——保名失败的 backstop 是 RXS-0159 的 **RX6011**(`codegen.dxil_sig_mismatch`,SigMismatch)。改写为 fail-closed 生产侧步骤,其失败即退化名经门拒,无静默通过、无新 6xxx(strict-only,P-01)。
- IR6(测试锚定):≥1 `//@ spec: RXS-0172` 覆盖 (a) 输出 varying 保名后校验门(不放宽)过、(b) fragment 输入 varying 保名、(c) 改写只动 semantic token 的 ABI 中立断言(register/mask/类型/字段名不变)、(d) provenance 不一致 fail-closed → 退化名 → RX6011。conformance/golden 真链 bless 归 agent pin 环境(G-G2-4),不在本条测试义务。

### RXS-0173 DXIL 图形=B fragment 着色阶段输出 → `SV_Target#` 渲染目标系统值映射（RD-017 片元输出 MRT 边界）

本条补齐 RXS-0172 未覆盖的 **fragment 着色阶段输出方向(MRT 渲染目标)** 签名核对:fragment 着色器的输出 I/O 结构体字段(`dir == Out` 的 `Varying`/`Interpolate`)在 D3D12/DXIL 中**按渲染目标索引绑定**,经 B 链由 spirv-cross 忠实降级为 **`SV_Target<n>`** 系统值(render target system value),其中 `n` = 该输出字段在结构体内的**声明序索引**(0 起,与 SPIR-V 输出 `Location` 同源)。强制签名一致性校验门(RXS-0159 `signature_gate::check`)对 fragment 输出**以系统值类忠实匹配**(等价 `position→SV_Position` 的系统值映射),而**非**要求用户语义名(`albedo`/`normal`/`depth`)出现于 OSG1——因 D3D12 像素着色器输出无用户可见语义名通道,渲染目标绑定纯按 `SV_Target` 索引(外部 ABI,与 RTV 顺序一一对应)。本条**关闭** RD-017 次发现的 fragment 输出 MRT 边界(G2.4 RD-021 停手分支次发现:几何 pass FS 写 MRT 经 full B 链被签名门 strict-only 拒,因 RXS-0172 改写器只匹配 `TEXCOORD`)。

#### Syntax

本条不新增语法。fragment 输出 MRT 由阶段函数返回的输出 I/O 结构体字段(RXS-0171 输出聚合,按字段声明序)给定。

#### Legality

- L1(渲染目标映射):fragment 着色阶段(`ShaderStage::Fragment`)输出方向 varying 字段,按声明序 `n` 映射到 DXIL OSG1 的 `SV_Target<n>` 渲染目标系统值。`SV_Target` 是 D3D12 像素输出的合法系统值(非通用名退化),签名门以系统值类(`builtin_sv_tokens` `target → SV_TARGET`)忠实匹配,匹配则保真、缺失/数目不符则 strict-only 失败。
- L2(不放宽校验门):本条**不**放宽 RXS-0159 校验门——fragment 输出元素仍必须真实存活于 OSG1(以 `SV_Target` 系统值形态);删除/数目不符/未保真 → **RX6011**(`SigMismatch`,输出方向)。系统值类匹配是**忠实映射**(D3D12 ABI 既定),非"以等价冒充保名"(对比 RXS-0172 L2 否决的 location 冒充)。`frag_depth`/`depth`(`#[builtin]`)维持 RXS-0159 `SV_Depth` 系统值映射不变,不计入 `SV_Target` 渲染目标序。
- L3(声明序即渲染目标序):`SV_Target` 的索引 `n` 由 fragment 输出 varying 字段声明序确定(`#[builtin]` 不占渲染目标序),与 SPIR-V 输出 `Location` 同源(RXS-0172 `varying_provenance` 口径)。该索引是**外部 conformance(D3D12 RTV 绑定顺序)**,非本条冻结的自由 ABI;校验门按系统值类匹配,**不**额外比对索引数字本身越出 L1 范围。
- L4(方向限定):本条**只**作用于 fragment 阶段**输出**方向;vertex 输出 varying(inter-stage)维持 RXS-0172 用户语义名保名(`TEXCOORD#`→用户名)、fragment 输入 varying 维持 RXS-0172 保名、vertex 输入维持 RXS-0159 IR1(a)、各阶段 `#[builtin]` 系统值维持 RXS-0159 映射,均不在本条改写/匹配面。

#### Dynamic Semantics

本条无运行期语言语义:fragment 输出写出语义承 RXS-0171(输出 I/O 聚合返回 → 逐 `OpStore`);`SV_Target` 是编译期签名/绑定层的渲染目标系统值标注。着色器运行期 I/O 行为不因本条改变。

#### Implementation Requirements

- IR1(系统值 token):[`signature_gate`](../src/rurixc/src/dxil_sig_gate.rs) `builtin_sv_tokens` 增 `target → SV_TARGET`;`check` 引入着色阶段上下文(或由 [`dxil_codegen::emit_dxil_b_from_spv`](../src/rurixc/src/dxil_codegen.rs) 在调用前按声明序把 fragment `dir==Out` 的 `Varying`/`Interpolate` 意图元素转为 `Builtin("target<n>")` 期望),使 fragment 输出以 `SV_Target<n>` 系统值类匹配。vertex/compute 阶段输出方向**不**受影响(维持 RXS-0172 用户语义名匹配)。
- IR2(声明序绑定):`n` 由 fragment 输出 varying 字段声明序(`#[builtin]` 不占序)按 0 起递增确定,与 `varying_provenance` location 口径同源(确定性)。
- IR3(门不旁路):RXS-0159 强制签名一致性校验门仍在 B 链末尾运行且**不可裁剪**;本条只把 fragment 输出的"期望形态"由"用户语义名"改为"`SV_Target<n>` 系统值"(忠实于 D3D12 ABI),**不**新增旁路/skip、**不**放宽其余方向的保名标准。
- IR4(错误码):**不新增错误码**——fragment 输出未保真(缺失/数目不符)backstop 仍为 RXS-0159 的 **RX6011**(`codegen.dxil_sig_mismatch`,SigMismatch)。
- IR5(ABI 中立):本条**不**冻结或承诺 DXIL register、component mask、packing、字节布局、稳定 SPIR-V `Location`;`SV_Target` 索引 = D3D12 RTV 绑定顺序(外部 conformance,§4.6(a) 范畴),触及显式 register/mask/packing/byte layout 即停手升 agent Full RFC。**不**碰纹理/采样/资源访问语义(06 §4.2,RD-021 仍 defer)。
- IR6(测试锚定):≥1 `//@ spec: RXS-0173` 覆盖 (a) fragment 3 输出按声明序匹配 `SV_Target0/1/2` → 校验门(不放宽)过、(b) 删一输出 / 渲染目标数目不符 → RX6011 strict-only 拒、(c) vertex 输出仍走 RXS-0172 用户名保名(本条不误伤其他方向)。conformance/golden 真链 bless 经 agent pin 环境(G-G2-4),uc04_gbuffer_fs 几何 pass FS 写 MRT 经 full B 链不再被签名门拒。

## 3. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.6 | 2026-06-27 | **G-G2-2 owner 收口 + DXIL golden bless 落档**。owner 白栀于本工作会话监督确认 device 真跑 run URL、DXIL 文本 golden bless 与 G-G2-2 子里程碑签字;agent 代录机器事实,自主签署 G2 整体 close-out。`tests/dxil/graphics/gfx_vs_min.dxil-disasm` 在 signed DXC pin 环境(`H:\dxc-round7\extracted\bin\x64` 含 `dxc.exe`/`dxv.exe`/`dxil.dll`)和显式 `spirv-cross.exe` 下经 `RURIX_BLESS=1 cargo test -p rurixc --features dxil-backend --test dxil_golden dxil_b_disasm_golden_matches_when_toolchain_present -- --exact --nocapture` 重 bless;入 golden 前 `dxv.exe` validator 接受,版本噪声规范化为 `OWNER-BLESSED-NORMALIZED`。远端 PR smoke [28284960733](https://github.com/qwasg/Rurix/actions/runs/28284960733) 全量 success,步骤 46 输出 `DXIL_DEVICE: ok adapter="NVIDIA GeForce RTX 4070 Ti" pixel=64,127,255,255 draw=ok`。当前 `gfx_vs_min` 仍为 RD-013/RD-017 缺口下的 TEXCOORD baseline,不关闭 deferred、不声称 output varying 用户语义保真。§2 仅更新当前 golden 形态说明;RXS-0160 计划映射、🔒 签名二进制 ABI 布局/纹理内存模型/DXIL·SPIR-V UB 边界仍不触及。| **Full RFC**（RFC-0004 / PR-D2） |
| v1.0 | 2026-06-24 | 新建 dxil_backend.md（PR-C1 spec 脚手架，承 RFC-0003 / D-131=A）:登记文件名 + 文件级语义面说明（MIR→DXIL 第二后端，承 RFC-0002 着色阶段类型面 RXS-0153~0156）+ §1 范围与 **RXS-0157~ 预留区间声明**（区间大小未锁定，随 RFC-0003 §9 Q-Range 与路径裁定一并定）+ §2 条款占位（条款体随 PR-C2 实现 PR 同落）。**沿 README v1.32 interop_d3d12.md / v1.33 async_buffer.md / v1.37 shader_stages.md 脚手架先例:仅登记文件名 + 预留区间，不落带编号裸条款头**——本文件**零 `### RXS-####` 条款头**，`ci/trace_matrix.py --check` 维持全锚定 **156/156**（无新增裸条款头、无悬空锚点、零新 RXS）。条款体（RXS-0157 起）与每条 ≥1 `//@ spec` 测试锚定随 PR-C2（DXIL 后端实现 PR）同落（条款 PR 先于实现 PR）。禁区声明:🔒 纹理路径内存模型映射（06 §4.2）/ FFI ABI 二进制布局（RFC-0003 §4.6 / §9 Q-Builtin）/ 绑定布局推导（G2.3，P-11）/ 多后端架构承诺（D-008/SG-003）均不在本文件，触及即停手升档。错误码 **6xxx codegen 段**脚手架不预造、不预留，随 PR-C2 按真实可达类别只追加。档位 **Full RFC**（RFC-0003;触 codegen 第二后端 + target 分发，agent 自主判档，判档争议向上取严）。授权 G2_CONTRACT D-G2-2 / G-G2-2 + G2_PLAN G2.2 子里程碑，无体例变更 | **Full RFC**（RFC-0003） |
| v1.1 | 2026-06-24 | **PR-C2 分片1:落首条带编号条款体 `### RXS-0157`**(codegen target 分发与 DXIL 后端分叉)+ 配套最小 compute kernel 端到端实现(rurixc `dxil_codegen` 模块 + `--target dxil` 分发 + cargo feature `dxil-backend` + patched llc 经 `RURIX_LLC` dev env 定位 RD-011 + dxc validator accept)。条款体按 FLS 体例分 Syntax / Legality(L1 后端可用性·L2 最小子集·L3 降级失败 → RX6007)/ Dynamic Semantics / Implementation Requirements(IR1 分发点·IR2 D-131=A 路径·IR3 golden 文本反汇编经 validator·IR4 错误码 RX6007),**严禁 UB 节**。配套 conformance accept(空体 compute kernel 产 DXIL,`//@ spec: RXS-0157`)+ reject(子集外构造 → RX6007)+ DXIL golden(文本反汇编 + bless)。错误码新增 **RX6007**(6xxx codegen/目标段续接 RX6006,只追加)+ en/zh message-key(双语覆盖)。`ci/trace_matrix.py --check` 全锚定 **157/157**(新增 RXS-0157 带测试锚定、无悬空)。RXS-0158/0159/0160 仍为 §9 Q-Range 计划映射(非裸条款头),随后续分片落地。本片不碰 🔒 纹理内存模型映射 / FFI ABI 布局 / 绑定布局推导(G2.3)。档位 **Full RFC**(RFC-0003),无体例变更 | **Full RFC**（RFC-0003） |
| v1.2 | 2026-06-25 | **PR-D1 图形=B spec 脚手架(承 [RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md),owner Approved 2026-06-25)**:§1 编号区间 + §2 新增 **2.1 图形=B 条款计划映射(预留区间)**,登记 **RXS-0159 保号重构(按 B)**(A 路签名 ISG1/OSG1 `elemcount=0` 不可达 → D-131 v1.4 裁定图形=B → RXS-0159 保号、PR-D2 按 B 路径 SPIR-V `BuiltIn`/`Location` + 译后强制签名一致性校验门重写;#97 A 路 RXS-0159 stub 不入 main)+ 新增 **RXS-0160**(阶段间接口 → DXIL/SPIR-V 阶段链接一致性核对)/ **RXS-0161**(图形着色阶段 MIR→SPIR-V 降级面:execution model / I/O decoration / 资源句柄)/ **RXS-0162**(B 转译链确定性 + validator gate + 供应链 pin `[[toolchain]]` + SHA256 + strict-only 核验,含签名一致性校验门与 golden 形态)预留区间与重构说明。区间裁决已锁定(RFC-0004 §9 Q-Range-B);feature gate 复用 `dxil-backend`(Q-Gate-B);错误码归 6xxx 段只追加。RXS-0158(阶段着色器类型,#96 HOLD)维持计划映射不动。**全部以计划映射 / 预留区间登记,不落条款体、零 `### RXS-####` 裸条款头**——`ci/trace_matrix.py --check` 维持全锚定 **157/157**(无新增可锚条款、无悬空锚点、零新 RXS)。条款体随 PR-D2(B 转译实现 PR)同落(条款 PR 先于实现 PR,硬规则 7)。**本片不碰** 🔒 签名二进制 ABI 布局(RFC-0004 §4.6(a))/ 纹理路径内存模型映射(06 §4.2)/ DXIL·SPIR-V UB 边界——只引边界声明,不落禁区语义本体。档位 **Full RFC**(RFC-0004),无体例变更 | **Full RFC**（RFC-0004） |
| v1.3 | 2026-06-26 | **PR-D2:落条款体 `### RXS-0158`**(着色阶段着色 → DXIL 着色器类型降级对应),条款体**自 `origin/feat/g2.2-pr-c2-slice2-rxs0158:spec/dxil_backend.md` 整段照搬**(未改写措辞):含**着色器类型对应表**(compute/kernel→compute / vertex→vertex / fragment→pixel SM6.0 **已落**;mesh→mesh / task→amplification SM6.5、RT raygen·closesthit·anyhit·miss→library SM6.3 映射登记、实现 deferred)+ Syntax / Legality(L1 可降级阶段最小子集承 RXS-0157 / L2 deferred 阶段 → RX6008 / L3 降级失败 → RX6007)/ Dynamic Semantics / Implementation Requirements(IR1 阶段→着色器类型映射 + DXIL 收集根不改 PTX 根 / IR2 deferred 阶段 RX6008 / IR3 SM 下限登记 / IR4 错误码 RX6008),**严禁 UB 节、不定义 I/O 签名 ABI**。落地校正:**① 档位对齐 PR-D2 / RFC-0004**(承 D-131 混合裁决,非 slice2 旧 RFC-0003 语境);**② RD 编号分叉停手交 owner**:slice2 条款体引 **RD-012**(main `deferred.json` 已存在、已绑 RX6008,指 mesh/task/RT 缺口),本 spec requirements R5.7 要新开 **RD-016**(同一缺口)——**默认复用 RD-012、未擅自双开 RD-016**,在此分叉**标「需升档」交 agent 裁决**(deferred.json 未改、RD-016 未落);**③ RX6008 撞号核对**:main `registry/error_codes.json` 6xxx 段现为 RX6001~RX6007,**RX6008 未撞号**(deferred.json RD-012 已预引 RX6008 作此降级码,honor 既有引用不改派,registry 落条目 + status 翻转归 agent)。trace:本轮**仅落条款体**,`### RXS-0158` 三级标题需 ≥1 测试锚定(`//@ spec: RXS-0158`),锚定为后续任务,本轮不伪造锚定、不删条款头;若 trace 因新条款头无锚定而红,如实报告状态。**本片不碰** 🔒 纹理内存模型映射(06 §4.2)/ 内建变量·签名二进制 ABI 布局(RFC-0003 §4.6/§9 Q-Builtin)/ 绑定布局推导(G2.3,P-11)/ 阶段 I/O 签名 SV_*(RXS-0159)。档位 **Full RFC**(RFC-0004 / PR-D2),无体例变更 | **Full RFC**（RFC-0004 / PR-D2） |
| v1.4 | 2026-06-27 | **PR-D2:落图形=B 条款体 `### RXS-0159` / `### RXS-0161` / `### RXS-0162` + RXS-0158 实现一致性收口**。**RXS-0159**(阶段 I/O → DXIL 签名/系统值语义,按 B):by-construction `UserSemantic` 保名 + 强制签名一致性校验门(`signature_gate::check`,不可裁剪)+ 错误码 RX6011 签名不一致 / RX6012 声明输入被消除 / RX6013 不可映射(RX6009 已被 RD-013 预引,段内不复用改派);🔒 二进制 ABI 布局只边界声明(RFC-0004 §4.6(a))。**RXS-0161**(图形阶段 MIR→SPIR-V 降级面):`dxil_spirv::emit_spirv` 纯 safe `Vec<u32>` 编码器(execution model / Location·BuiltIn·UserSemantic decoration,经 spirv-val);纹理访问语义结构上不可达 → RX6013 升档(§4.6(b))。**RXS-0162**(B 转译链确定性 + validator gate + 供应链 pin + strict-only):RX6010 B 链转译失败;确定性 ×N SHA256 + dxc validator gate + 校验门叠加;供应链 `[[toolchain]]` pin + CI 步骤 46(`ci/dxil_codegen_smoke.py`)随供应链子分片回填,golden 仅 DXIL 文本反汇编、本机产物 NOT BLESSED 由 agent pin 环境重 bless。各条按 FLS 分 Syntax / Legality / Dynamic Semantics / Implementation Requirements,**严禁 UB 节**。错误码新增 **RX6010~RX6013**(6xxx 段续接 RX6007,只追加 + en/zh message-key;任务7 已分配)。**RXS-0158 收口**:经 agent 裁决**复用 RD-012**(不新开 RD-016,避同缺口双 RD),`dxil_codegen` 注释 RD-016→RD-012 对齐;mesh/task/RT deferred 阶段发码由 slice2 旧 A 框架的 **RX6008 改为实现实发的 RX6007**(沿 RXS-0157 通道,RX6008 维持 deferred.json RD-012 预留,落码归 RD-012 实现里程碑)——spec 与 impl 一致、无悬空码。**RXS-0160**(阶段间接口链接一致性核对)**本片不落条款体**:其 vertex out↔fragment in 链接核对需多阶段联编点,本片单着色阶段编译尚无该接缝,维持 §2.1 计划映射(非裸条款头,无悬空)、承后续里程碑。`ci/trace_matrix.py --check` 全锚定 **161/161**(RXS-0157~0159·0161·0162 各 ≥1 `//@ spec`,RXS-0160 无裸头不悬空)。**本片不碰** 🔒 签名二进制 ABI 布局(RFC-0004 §4.6(a))/ 纹理访问语义(§4.6(b)/06 §4.2)/ DXIL·SPIR-V UB 边界(§4.6(c));device 真跑 / 呈现对照 / run URL / evidence 归 agent(G-G2-2)。档位 **Full RFC**(RFC-0004 / PR-D2),无体例变更 | **Full RFC**（RFC-0004 / PR-D2） |
| v1.5 | 2026-06-27 | **RXS-0159 IR1 保名表述按实测收窄 + 顶点输入保名旗标接入生产(机制① 缺口闭合)**。背景:`dxil_codegen.rs` 生产 B 链调用 `spirv_cross_to_hlsl(.., &[])` 空 extra 旗标,致 RFC-0004 §4.4 机制① 顶点输入保名**未接入生产**——用户命名顶点输入退化为 `TEXCOORD#` → 校验门 RX6011 拒。实测定结论(本机 dxc 1.8.0.4739 / spirv-cross vulkan-sdk-1.3.296.0,贴真实 spirv-dis + dxc -dumpbin ISG1 输出):**(1)** spirv-cross **不消费** SPIR-V `UserSemantic` 装饰为 HLSL 语义(`OpDecorate ... UserSemantic` 经 spirv-val 干净但非保名机制);`--set-hlsl-named-vertex-input-semantic` 按 `OpName` 匹配,`emit_spirv` 不 emit `OpName` 故不命中;**保名经 `--set-hlsl-vertex-input-semantic <location> <semantic>` 按 `Location` 覆盖达成**,location 由 `emit_spirv` 按 io_sig 顺序确定性分配 → 经 io_sig 导出旗标(非硬编码),Rust-emit SPIR-V 路径下 **vertex 输入 `POSITION`/`NORMAL` 端到端存活**(measured)。**(2)** 输出 varying / fragment 输入 varying 用户语义名 **无保名旗标**(spirv-cross 无输出/片元语义旗标)→ 仍退化 `TEXCOORD#`。**处置(实测决定,非偏好)**:**接入生产**——`dxil_codegen::vertex_input_semantic_flags(stage, io_sig)` 经 io_sig 导出顶点输入保名旗标接入 `run_b_chain` 的 `spirv_cross_to_hlsl` 生产调用(原 `&[]`);**诚实收窄**——IR1 由「用户命名 varying/interpolate → by-construction UserSemantic 保名」收窄为「系统值名恒保真 + vertex 输入名 by-construction 保真(measured,location 覆盖)+ 输出/片元 varying 名当前不可保真 → RX6011 显式拒绝,缺口 deferred **RD-017**」。系统值 SV_Position/SV_VertexID 真达(elemcount>0)**不受影响**(B-over-A 核心,保留)。配套:`registry/deferred.json` 追加 **RD-017**(status 留 agent;**续号跳过 RD-016**——RD-016 经 `.kiro` requirements R5.7 owner 确认给 mesh/task/RT 缺口、已复用既有 **RD-012**、未创建 entry,按编号不复用续 RD-017)+ `dxil_spirv`/`dxil_codegen` 代码侧 `// STUB(RD-017)` 双侧标注 + 单测 `vertex_input_semantic_flags_derive_from_io_sig` + `toolchain` 冒烟 `b_chain_end_to_end_smoke` 升级为顶点输入名存活断言 + `ci/dxil_codegen_smoke.py` 顶点输入名保真断言绿检(输出 varying 退化维持 NOTE)。golden:既有图形语料(`gfx_vs_min.rx` 单 interpolate **输出** varying,无命名顶点输入)→ 保名旗标导出为空 → golden **不变**,无需重录。**不动 RFC-0004 措辞**(RFC §4.4 机制① 本已按顶点输入 measured 表述、未 over-broad,措辞勘误属 owner);不改 RXS-0160 计划映射;§3 既有行 0-byte。`ci/trace_matrix.py --check` 全锚定不变(RXS-0159 新单测携 `//@ spec: RXS-0159`)。**本片不碰** 🔒 寄存器/mask/packing 二进制 ABI 布局(§4.6(a))。device 真跑 / golden bless / RD-017 status 翻转归 agent(G-G2-2)。档位 **Full RFC**(RFC-0004 / PR-D2),无体例变更 | **Full RFC**（RFC-0004 / PR-D2） |
| v1.7 | 2026-06-27 | **落图形=B 条款体 `### RXS-0160`(阶段间接口 → 阶段链接一致性核对)+ 收口 §2.1 计划映射**。承 [RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md) §4.4/§5(owner Approved 2026-06-25),vertex out ↔ fragment in 跨阶段 varying 经 B 路降级后以**语义名等价为链接键**核实语义名 / 类型 / 插值一致性,错链 strict-only 必显式 6xxx(P-01,无运行期 fallback)。条款体按 FLS 分 Syntax / Legality(L1 可核对子集承 RXS-0159 / L2 链接键缺失 / L3 类型·插值失配 / 🔒 location 不比对)/ Dynamic Semantics / Implementation Requirements(IR1 `check_stage_link` 链接核对入口复用 RXS-0159 `semantic_name_matches` 语义名等价 / IR2 `link_graphics_stages` vertex+fragment 多阶段联编点接缝 / IR3 错误码待 agent 裁 / IR4 测试锚定),**严禁 UB 节**。**§2.1 收口**:RFC-0004 §5 下游条款 RXS-0159/0160/0161/0162 条款体全落地,无尚未落地计划映射。**判档点(需升档,agent 闸门)**:错链错误码语义归类——spec §2.1 旧文映射 RX6011,但 RX6011 现由 `codegen.dxil_sig_mismatch`(单阶段输出未保真)占用;6xxx 下一空号 = **RX6014**(RX6008/RX6009 分别由 RD-012/RD-013 预留不复用)。「复用 RX6011 同语义类」抑或「新开 RX6014」属语义归类裁决,**不擅自落码**——条款体先以占位「6xxx」表述,`check_stage_link` 返回类型化 `StageLinkError`(纯 host/safe,零新 unsafe),**不**接线生产 emit、**不**改 `registry/error_codes.json` / message-key;落码 + 错链 conformance reject + golden 归 agent 裁码后的实现步。配套:`check_stage_link` + `link_graphics_stages` 单测(accept 链接一致 + reject 缺链接键/类型失配/插值失配,工具无关恒跑,`//@ spec: RXS-0160`)+ conformance accept(`conformance/dxil/graphics/accept/` vertex+fragment 链接一致配对,host 侧确定性)。`ci/trace_matrix.py --check` 全锚定 **161→162**(RXS-0160 ≥1 `//@ spec` 锚定,无悬空)。**本片不碰** 🔒 寄存器/location 编号/mask/packing 二进制 ABI 布局(§4.6(a))/ 纹理访问语义(§4.6(b))/ DXIL·SPIR-V UB 边界(§4.6(c));device 真跑 / 呈现对照 / golden bless / 新错误码语义归类裁决归 agent(G-G2-2)。档位 **Full RFC**(RFC-0004 / PR-D2),无体例变更 | **Full RFC**（RFC-0004 / PR-D2） |
| v1.8 | 2026-06-28 | **PR-E2b-2:RXS-0160 错链错误码语义归类裁决落地(agent 裁定方案 B = 新开 RX6014)**。承 v1.7 判档点(`RX6011` 复用 vs `RX6014` 新开),agent 裁定**新开 RX6014**(不复用 RX6011——RX6011 = 单阶段输出签名不一致 `codegen.dxil_sig_mismatch`,与阶段间接口错链语义不同;6xxx 段当时下一空号,`RX6008`/`RX6009` 维持 RD-012/RD-013 预留不复用)。`registry/error_codes.json` append-only 追加 **RX6014** `codegen.dxil_stage_link_mismatch` + en/zh message-key(`ci/bilingual_coverage.py` PASS);RXS-0160 条款体 L2/L3 + IR3/IR4 占位「6xxx」换实码 RX6014;§1 编号区间 / §2 / §2.1 判档点叙述同步收口为已裁。生产 emit 接线 [`dxil_codegen::emit_stage_link_error`](../src/rurixc/src/dxil_codegen.rs)(`StageLinkError::Unlinked`/`LinkMismatch` 两类错链同落 RX6014),`dxil_sig_gate::StageLinkError` doc 同步实码;reject 真实红绿:`link_graphics_stages_mismatched_pair_is_link_error` 经 emit 接缝断言落 RX6014 + `emit_stage_link_error_routes_to_rx6014` emit 分派单测。**🔒 禁区不变**:错误码 message 只描述错链失败类别,不落 location/register/mask 物理布局值(§4.6(a))。本片与 binding_layout.md PR-E2b-2(RX6015/6016/6017 + RX6013 复用)同 commit:6xxx 段顺号 = RX6014(RXS-0160)→ RX6015/6016/6017(绑定布局,避让 RX6014)。`ci/trace_matrix.py --check` 全锚定维持(无新增/删除 RXS,RXS-0160 既有 `//@ spec` 锚点不变);device 真跑 / 呈现对照 / golden bless 归 agent(G-G2-2 / G-G2-3)。档位 **Full RFC**(RFC-0004 / PR-E2b-2),无体例变更 | **Full RFC**（RFC-0004 / PR-E2b-2） |
| v1.9 | 2026-06-29 | **RD-013 Q1–Q4 owner 代表裁决落库 + RXS-0171 条款体落地**。本轮采用用户在工作会话提供的推荐组合作为 owner/owner 代表裁决输入:Q1=C1-a 形参/返回值值语义、Q2=C2-a MIR place↔`io_sig` 字段序绑定、Q3=C3-c ABI 中立分层、Q4=C4-c refined 白名单子集 + RX6013 strict-only。新增 `### RXS-0171`(DXIL 图形=B 着色 body I/O 数据流降级最小切片):定义输入 I/O 结构体普通形参读取、输出 I/O 结构体普通返回写出;字段声明序绑定 In/Out;仅承诺源码层元素/方向/字段序/语义名/系统值/类型,不冻结 register/mask/packing/byte layout/稳定 Location;资源/纹理/采样/显式布局触及即升 agent Full RFC。实现侧接线 `emit_dxil_b_body`/`emit_spirv_body` 消费完整 Body,lower OpLoad/OpConstant/基础算术/OpStore;unsupported rvalue 与输出不完整均 RX6013。配套单测 + conformance RXS-0171 锚定;RD-013 status 仍 open,DXIL golden/device bless、CI step 48、G-G2-4 签字均归 agent 后续确认。| **Full RFC**（RFC-0004 / RD-013 agent 裁决） |
| v2.0 | 2026-06-29 | **RD-017 保名机制 agent 裁决落库 + RXS-0172 条款体落地(spec-first,先于实现 PR)**。owner 代表本工作会话裁定:采**选项①**(spirv-cross 产 HLSL 后于 HLSL 边界做受限、可验证的语义名 token 改写),**否决选项③**(不放宽 `signature_gate`、不以 location 等价冒充保名);#114 维持现状不拆 RD-013,RD-017 自 `af4ee25` 开 stacked 分支(base `feat/g2.4-uc04-pr-f2-impl`)。新增 `### RXS-0172`(输出 varying / fragment 输入 varying 用户语义名保名):关闭 RXS-0159 IR1(b) 缺口——`dir==Out` varying 与 fragment `dir==In` varying 用户语义名经 B 链端到端保真;provenance 由 `io_sig` 导出(与 `vertex_input_semantic_flags` 同源)、改写点在 `run_b_chain` 的 spirv-cross→dxc 之间;**只动 HLSL semantic token**,不冻结 register/mask/packing/byte layout/稳定 Location,semantic_index 后缀随保名自然变化非物理 ABI(校验门不比对);**不放宽 RX6011**(Property 5),fail-closed(provenance 不一致即退化名经门拒);不新增错误码(backstop = RXS-0159 RX6011)。机制取证 `evidence/rd017_varying_semantic_spike_20260629.json`(隔离 spike measured_local:改写后 dxc 接受 + 校验门不放宽也过 + 物理 ABI 不变 + 确定性;dxc 1.8.0.4739 / spirv-cross 1.3.290,签名 validator/golden/device 留 agent pin 环境)。**条款先于实现**(硬规则 7):RXS-0172 测试锚定与实现随本分支实现 commit 同落。RD-017 status 翻转 / golden bless / device 真跑归 agent(G-G2-4),本条不签收口。🔒 签名二进制 ABI 布局(RFC-0004 §4.6(a))/ 纹理内存模型(06 §4.2)/ DXIL·SPIR-V UB 边界不触及。档位 **Full RFC**(RFC-0004),无体例变更 | **Full RFC**（RFC-0004） |
| v2.1 | 2026-06-29 | **RD-017 片元输出 MRT 边界收口 + RXS-0173 条款体落地(spec-first,G2.4 选项 B)**。承 G2.4 RD-021 停手分支次发现(几何 pass FS 写 G-buffer MRT 经 full B 链被签名门 strict-only 拒:spirv-cross 把 fragment 输出降为 `SV_Target#` 渲染目标语义,而 RXS-0172 改写器只匹配 `TEXCOORD`)。新增 `### RXS-0173`(fragment 着色阶段输出 → `SV_Target#` 渲染目标系统值映射):fragment 输出 I/O 字段按声明序 `n` 忠实映射 OSG1 `SV_Target<n>` 渲染目标系统值(等价 `position→SV_Position` 系统值类匹配),**因 D3D12 像素输出按渲染目标索引绑定、无用户语义名通道** → 签名门以系统值类忠实匹配 fragment 输出而非要求用户名(`albedo`/`normal`/`depth`)出现于 OSG1。**机制取舍说明**:不采用"把 HLSL 里 `SV_Target#` 改名为用户名"(会破坏 D3D12 渲染目标按索引绑定、device draw 必坏),改为签名门系统值类识别(忠实于 D3D12 ABI,**非放宽门、非以 location/等价冒充保名**);RXS-0172 否决的"location 冒充"是把退化通用名当保名,本条是把渲染目标系统值忠实建模——二者不同。**不放宽 RXS-0159 校验门**(fragment 输出仍须真实存活于 OSG1,缺失/数目不符 → RX6011),**不新增错误码**(backstop = RX6011)。条款按 FLS 分 Syntax / Legality(L1 渲染目标映射 / L2 不放宽门 / L3 声明序即渲染目标序 / L4 方向限定)/ Dynamic Semantics / Implementation Requirements(IR1 `builtin_sv_tokens` 增 `target→SV_TARGET` + `check` 引入阶段上下文 / IR2 声明序绑定 / IR3 门不旁路 / IR4 RX6011 backstop / IR5 ABI 中立 / IR6 测试锚定),**严禁 UB 节**。`vertex` 输出维持 RXS-0172 用户名保名、fragment 输入维持 RXS-0172、vertex 输入维持 RXS-0159 IR1(a)、`#[builtin]`(含 `SV_Depth`)维持 RXS-0159 映射,均不受本条影响。`ci/trace_matrix.py --check` 全锚定 RXS-0173 入后 173/173(RXS-0173 ≥1 `//@ spec` 锚定)。🔒 register/mask/packing/byte layout(§4.6(a))/ 纹理采样内存模型(06 §4.2,RD-021 仍 defer)/ DXIL·SPIR-V UB 不触及。`Assisted-by: cursor:claude-opus-4.8`。档位 **Full RFC**(RFC-0004 / RFC-0006 G2.4),无体例变更 | **Full RFC**（RFC-0004 / RFC-0006 G2.4） |
