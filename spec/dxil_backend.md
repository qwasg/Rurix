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

**编号区间**:本文件条款自 **RXS-0157** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0156 @ [shader_stages.md](shader_stages.md))。RFC-0003 §9 Q-Range 的初始区间未锁定状态已由 **RFC-0004 §9 Q-Range-B 锁定**为 RXS-0159 保号按 B 重构 + RXS-0160~0162 新增面;RXS-0158 维持 #96 HOLD。**PR-C2 分片1 起**已落首条带编号条款体 **RXS-0157**(codegen target 分发与 DXIL 后端分叉,§2);RXS-0158(着色阶段 → 着色器类型,#96 HOLD)、**RXS-0159(保号重构,按 B)** 与新增 **RXS-0160~0162** 仍为**计划映射 / 预留区间**(非裸条款头,详见 §2 计划映射区与 [RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md) §5/§9 Q-Range-B)。**D-131 v1.4 裁定混合 compute=A / 图形=B**(13 §D-131;RFC-0004 owner Approved 2026-06-25):图形 I/O 签名降级(RXS-0159)由 A 路类型面 stub 改 **B 路**(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL)重写,A 路签名产物 ISG1/OSG1 `elemcount=0` 不可达(上游 #90504/#57928,RFC-0004 §4.5),**#97 的 A 路 RXS-0159 不入 main,由 PR-D2 统一按 B 重写**。条款体与每条 ≥1 测试锚定随实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 全锚定 157/157,**本文件零新增裸条款头**)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 本节自 PR-C2 分片1 起落带编号条款体。本片**只落 RXS-0157**(codegen target 分发与 DXIL 后端分叉);RXS-0158(阶段着色器类型,#96 HOLD)仍为 RFC-0003 §9 Q-Range 计划映射;**RXS-0159(保号重构,按 B)与新增 RXS-0160~0162 的图形=B 计划映射见下「2.1 图形=B 条款计划映射(RFC-0004,预留区间)」**(非裸条款头,仅计划登记,条款体随 PR-D2 实现 PR 同落)。
> 各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(target 不支持 / 降级失败以编译期 6xxx codegen 诊断定义,P-01 strict-only,无运行期 fallback;10 §7.5)。**本片不碰** 🔒 纹理内存模型映射(06 §4.2 禁区)/ FFI ABI 二进制布局(RFC-0003 §4.6 / §9 Q-Builtin)/ 绑定布局推导(G2.3,P-11);触及即停手升档。

### 2.1 图形=B 条款计划映射(RFC-0004,预留区间)

> **本小节为 [RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md)(SPIR-V→DXIL 图形后端,混合 compute=A / 图形=B,owner Approved 2026-06-25)§5 下游条款计划表的 spec 侧登记。仅落计划映射 / 预留区间,不落条款体、不落裸条款头**——本小节**零 `### RXS-####` 三级标题**,trace_matrix 维持全锚定 **157/157**(无新增可锚条款)。条款体随 **PR-D2**(B 转译实现 PR)同落(条款 PR 先于实现 PR,硬规则 7)。区间裁决已锁定(RFC-0004 §9 Q-Range-B);feature gate **复用 `dxil-backend`**(Q-Gate-B,不新增子 gate)。错误码归 **6xxx 段**(承 RFC-0003 RX6007~,只追加,随实现 PR 按真实可达类别分配 + en/zh message-key)。

- **RXS-0159(保号重构,按 B)— 阶段 I/O → DXIL 签名/系统值语义降级**:A 路签名产物 ISG1/OSG1 `elemcount=0` **不可达**(上游 #90504 后端写空签名 + #57928 签名元数据 nullptr + 填充耦合 🔒 FFI ABI 禁区,RFC-0004 §4.5/§11)→ **D-131 v1.4 裁定图形=B** → **RXS-0159 保号**、按 B 路径(SPIR-V `BuiltIn`/`Location` decoration + 译后强制签名一致性校验门)在 **PR-D2 重写**。**#97 的 A 路 RXS-0159 类型面 stub 不入 main**,由 PR-D2 统一按 B 重写。测试锚定计划:dxil-sig accept(SV 真达,`elemcount>0`)+ reject(不可映射 / 签名不一致 → 6xxx)+ DXIL golden。
- **RXS-0160(新落,按 B)— 阶段间接口 → DXIL/SPIR-V 阶段链接一致性核对**:RXS-0155 阶段间接口契约(vertex out ↔ fragment in)在 B 路经 SPIR-V `Location` / 类型 / 插值匹配核对;错链即显式 6xxx(RFC-0004 §4.4 校验门)。测试锚定计划:dxil-sig accept + reject + golden。
- **RXS-0161(新增,B 路)— 图形着色阶段 MIR→SPIR-V 降级面**:着色阶段 → SPIR-V execution model / 阶段 I/O(RXS-0154 `#[builtin]`/`#[interpolate]`)→ SPIR-V `Location`/`BuiltIn` decoration / 资源句柄类型面(RXS-0156)→ SPIR-V 形态(RFC-0004 §4.2(a))。**精确映射随条款体落 PR-D2**;🔒 描述符编码 / 采样 opcode / 二进制布局不在本条(06 §4.2 / RFC-0004 §4.6)。测试锚定计划:SPIR-V/DXIL golden + conformance accept/reject。
- **RXS-0162(新增)— B 转译链确定性 + validator gate + 供应链 pin + strict-only 核验**:B 全链(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL)对给定输入确定(×N 容器 SHA256 一致);入 golden 前经 dxc validator 验证;**含强制签名一致性校验门**(译后 ISG1/OSG1 vs MIR 意图签名,不可裁剪,留不住即 6xxx,RFC-0004 §4.2/§4.4)+ golden 形态(仅 DXIL 文本反汇编入 golden,Q-Golden-B);供应链 SPIRV-Cross/dxc/glslang 经 lockfile `[[toolchain]]` + SHA256 pin(Q-Supply,env override 仅 dev/probe)+ 再分发审计(RD-014)。测试锚定计划:确定性核对 + validator gate + 篡改签名/转译输出真实红绿。

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

## 3. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-24 | 新建 dxil_backend.md（PR-C1 spec 脚手架，承 RFC-0003 / D-131=A）:登记文件名 + 文件级语义面说明（MIR→DXIL 第二后端，承 RFC-0002 着色阶段类型面 RXS-0153~0156）+ §1 范围与 **RXS-0157~ 预留区间声明**（区间大小未锁定，随 RFC-0003 §9 Q-Range 与路径裁定一并定）+ §2 条款占位（条款体随 PR-C2 实现 PR 同落）。**沿 README v1.32 interop_d3d12.md / v1.33 async_buffer.md / v1.37 shader_stages.md 脚手架先例:仅登记文件名 + 预留区间，不落带编号裸条款头**——本文件**零 `### RXS-####` 条款头**，`ci/trace_matrix.py --check` 维持全锚定 **156/156**（无新增裸条款头、无悬空锚点、零新 RXS）。条款体（RXS-0157 起）与每条 ≥1 `//@ spec` 测试锚定随 PR-C2（DXIL 后端实现 PR）同落（条款 PR 先于实现 PR）。禁区声明:🔒 纹理路径内存模型映射（06 §4.2）/ FFI ABI 二进制布局（RFC-0003 §4.6 / §9 Q-Builtin）/ 绑定布局推导（G2.3，P-11）/ 多后端架构承诺（D-008/SG-003）均不在本文件，触及即停手升档。错误码 **6xxx codegen 段**脚手架不预造、不预留，随 PR-C2 按真实可达类别只追加。档位 **Full RFC**（RFC-0003;触 codegen 第二后端 + target 分发，AI 不自判 Direct，判档争议向上取严）。授权 G2_CONTRACT D-G2-2 / G-G2-2 + G2_PLAN G2.2 子里程碑，无体例变更 | **Full RFC**（RFC-0003） |
| v1.1 | 2026-06-24 | **PR-C2 分片1:落首条带编号条款体 `### RXS-0157`**(codegen target 分发与 DXIL 后端分叉)+ 配套最小 compute kernel 端到端实现(rurixc `dxil_codegen` 模块 + `--target dxil` 分发 + cargo feature `dxil-backend` + patched llc 经 `RURIX_LLC` dev env 定位 RD-011 + dxc validator accept)。条款体按 FLS 体例分 Syntax / Legality(L1 后端可用性·L2 最小子集·L3 降级失败 → RX6007)/ Dynamic Semantics / Implementation Requirements(IR1 分发点·IR2 D-131=A 路径·IR3 golden 文本反汇编经 validator·IR4 错误码 RX6007),**严禁 UB 节**。配套 conformance accept(空体 compute kernel 产 DXIL,`//@ spec: RXS-0157`)+ reject(子集外构造 → RX6007)+ DXIL golden(文本反汇编 + bless)。错误码新增 **RX6007**(6xxx codegen/目标段续接 RX6006,只追加)+ en/zh message-key(双语覆盖)。`ci/trace_matrix.py --check` 全锚定 **157/157**(新增 RXS-0157 带测试锚定、无悬空)。RXS-0158/0159/0160 仍为 §9 Q-Range 计划映射(非裸条款头),随后续分片落地。本片不碰 🔒 纹理内存模型映射 / FFI ABI 布局 / 绑定布局推导(G2.3)。档位 **Full RFC**(RFC-0003),无体例变更 | **Full RFC**（RFC-0003） |
| v1.2 | 2026-06-25 | **PR-D1 图形=B spec 脚手架(承 [RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md),owner Approved 2026-06-25)**:§1 编号区间 + §2 新增 **2.1 图形=B 条款计划映射(预留区间)**,登记 **RXS-0159 保号重构(按 B)**(A 路签名 ISG1/OSG1 `elemcount=0` 不可达 → D-131 v1.4 裁定图形=B → RXS-0159 保号、PR-D2 按 B 路径 SPIR-V `BuiltIn`/`Location` + 译后强制签名一致性校验门重写;#97 A 路 RXS-0159 stub 不入 main)+ 新增 **RXS-0160**(阶段间接口 → DXIL/SPIR-V 阶段链接一致性核对)/ **RXS-0161**(图形着色阶段 MIR→SPIR-V 降级面:execution model / I/O decoration / 资源句柄)/ **RXS-0162**(B 转译链确定性 + validator gate + 供应链 pin `[[toolchain]]` + SHA256 + strict-only 核验,含签名一致性校验门与 golden 形态)预留区间与重构说明。区间裁决已锁定(RFC-0004 §9 Q-Range-B);feature gate 复用 `dxil-backend`(Q-Gate-B);错误码归 6xxx 段只追加。RXS-0158(阶段着色器类型,#96 HOLD)维持计划映射不动。**全部以计划映射 / 预留区间登记,不落条款体、零 `### RXS-####` 裸条款头**——`ci/trace_matrix.py --check` 维持全锚定 **157/157**(无新增可锚条款、无悬空锚点、零新 RXS)。条款体随 PR-D2(B 转译实现 PR)同落(条款 PR 先于实现 PR,硬规则 7)。**本片不碰** 🔒 签名二进制 ABI 布局(RFC-0004 §4.6(a))/ 纹理路径内存模型映射(06 §4.2)/ DXIL·SPIR-V UB 边界——只引边界声明,不落禁区语义本体。档位 **Full RFC**(RFC-0004),无体例变更 | **Full RFC**（RFC-0004） |
