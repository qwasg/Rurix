# Rurix 语言规范 — DXIL 第二后端语义面（MIR → DXIL，承 RFC-0003；G2.2 起）

> 条款:**RXS-0157 起续号预留**(G2.2 DXIL 第二后端语义面:codegen target 分发与 DXIL 后端分叉(MIR 之后 target 选择,PTX/DXIL 并存,strict-only 无 fallback)/ 着色阶段着色 → DXIL 着色器类型降级对应 / 阶段 I/O → DXIL 签名·系统值语义降级 / 阶段间接口 → DXIL 阶段链接一致性核对)。区间大小未锁定(RFC-0003 §9 Q-Range:随路径裁定与条款数一并定),本脚手架仅声明 **RXS-0157~** 预留区间、**不落带编号裸条款头**。体例见 [README.md](README.md)。
> 依据:**[RFC-0003](../rfcs/0003-dxil-backend.md)**(MIR→DXIL 第二后端,**owner Approved** 2026-06-23;§9 Q-D131 路径裁决 = **A**(LLVM DirectX 后端直接 emit DXIL),经独立勘误回填 §9 / 13 §D-131);06 §8.2(codegen 第二后端设计预留);06 §4.2(纹理路径内存模型禁区,🔒);07 §7(device codegen 分发:MVP/G1 维持 NVPTX→PTX,DXIL 第二后端 G2 重评估);07 §5(错误码段位:6xxx codegen/目标);spec/shader_stages.md RXS-0153~0156(着色阶段类型面,DXIL 降级的上游类型面来源)。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)(D-G2-2,G-G2-2)+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.2 子里程碑。
> 档位:**Full RFC**(RFC-0003;10 §3:本设计触 **codegen 第二后端 + target 分发**,有别于 M8 互操作的 Direct)。**D-131 路径已裁 A**(owner 凭 G2.2 round-1~8 双路 spike 证据裁定,RFC-0003 §9 Q-D131 C→A + 13 §D-131 待决→A,经独立勘误 PR;A 路 dev 工具链偏差由 `registry/deferred.json` RD-011 跟踪)。**AI 无权自判 Direct**,判档以 RFC-0003 与 G2_CONTRACT 授权为据,判档争议向上取严。任何触及 **🔒 纹理路径内存模型映射(06 §4.2)** / **FFI ABI 二进制布局(RFC-0003 §4.6/§9 Q-Builtin)** / **多后端架构承诺(D-008/SG-003)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):target 不支持 / 降级失败 / 非法 I/O 映射以 **编译期 6xxx codegen 诊断(P-01 strict-only,无运行期 fallback)**定义,不以 UB 表述(RFC-0003 §3/§4)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR(PR-C1)沿 README v1.32 interop_d3d12.md / v1.33 async_buffer.md / v1.37 shader_stages.md 脚手架先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0157 起)与每条 ≥1 测试锚定随 **PR-C2**(DXIL 后端实现 PR)同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定 **156/156**,零新 RXS)。

---

## 1. 范围与编号区间

本文件承载 **MIR → DXIL 第二后端语义面**的语义条款(G2.2+,D-G2-2)。承 RFC-0002 着色阶段类型面(RXS-0153~0156)产出的着色阶段语义,定义其经工具链降级到 DXIL(可被 D3D12 PSO 消费的着色器对象)的语义。覆盖语义面(RFC-0003 §4):

- **codegen target 分发与 DXIL 后端分叉**:MIR 之后的 target 选择(`rx build --target dxil`,与现状 `--target ptx` 并列,RFC-0003 §9 Q-CLI),PTX 后端(D-207)与 DXIL 后端**并存**;target 不支持的构造 → **6xxx codegen 错误**(strict-only,无运行期 fallback,P-01)。DXIL 后端 gate 于 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate;未启用时 DXIL 后端不参与编译,PTX 路径不受影响)。
- **着色阶段着色 → DXIL 着色器类型降级对应**:RXS-0153 的着色阶段着色(`vertex`/`fragment`/`compute`/`mesh`/`task` + RT `raygen`/`closesthit`/`anyhit`/`miss`)降级为对应 DXIL 着色器类型(vertex/pixel/mesh/amplification/RT/compute shader)。精确对应表随 PR-C2 条款体落地。
- **阶段 I/O → DXIL 签名/系统值语义降级**:RXS-0154 的阶段专属 I/O(`#[interpolate]` 插值限定 / `#[builtin]` 内建变量)降级为 DXIL 输入/输出签名与系统值语义(SV_*)。内建变量 → DXIL 系统值语义名为**类型面映射**;其二进制 ABI 布局属 RFC-0003 §4.6/§9 Q-Builtin 的 🔒 FFI ABI 禁区,**不在本文件**,留 owner 后续独立 Full RFC。
- **阶段间接口 → DXIL 阶段链接一致性核对**:RXS-0155 的阶段间接口契约(vertex out → fragment in varying 兼容)在类型面已编译期校验(RXS-0155),DXIL 层为降级一致性核对。

全部 DXIL 后端语义维持 **D-131 = A 路径(LLVM DirectX 后端直接 emit DXIL)**:与 NVPTX 后端同构、D-205 LLVM 单栈、无第二中间 IR(RFC-0003 §9 Q-D131 裁 A)。**🔒 纹理/采样器内存模型映射**(06 §4.2 禁区:tex proxy / 采样 opcode / 描述符编码 / 缓存一致性 / UB)、**内建变量/根参数/常量缓冲二进制 ABI 布局**(RFC-0003 §4.6 FFI ABI 禁区)、**绑定布局推导实现**(G2.3,P-11)均**不在本文件**;DXIL golden 取**文本反汇编形态** + 经 dxc validator 验证后入 golden(RFC-0003 §9 Q-Golden)。target 不支持 / 降级失败 / 非法 I/O 映射以 **编译期 6xxx codegen 诊断(P-01 strict-only)**定义,**不以 UB 表述**(§4)。

**编号区间**:本文件条款自 **RXS-0157** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0156 @ [shader_stages.md](shader_stages.md))。**区间大小未锁定**(RFC-0003 §9 Q-Range:随 owner 与路径裁定一并定,路径选择可能影响降级面条款拆分粒度)。本轮(脚手架 PR-C1)**仅登记区间预留 RXS-0157~**,**不落带编号裸条款头**;条款体与每条 ≥1 测试锚定随 **PR-C2**(DXIL 后端实现 PR)同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定 156/156)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款（RXS-0157~ 预留，条款体随 PR-C2 实现 PR 同落）

> 本脚手架 PR-C1 **不落带编号裸条款头**(沿 README v1.32/v1.33/v1.37 脚手架先例)——本文件当前**零 `### RXS-####` 条款头**,故 `ci/trace_matrix.py --check` 维持全锚定 **156/156**(无新增裸条款头、无悬空锚点、零新 RXS)。
> 条款体(RXS-0157 起,区间随 RFC-0003 §9 Q-Range 与路径裁定一并锁定)与每条 ≥1 `//@ spec: RXS-####` 测试锚定随 **PR-C2**(DXIL 后端实现 PR)落地。各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(target 不支持 / 降级失败 / 非法 I/O 映射以编译期 6xxx codegen 诊断定义,P-01 strict-only,无运行期 fallback;10 §7.5)。**本批不碰** 🔒 纹理内存模型映射(06 §4.2 禁区)/ FFI ABI 二进制布局(RFC-0003 §4.6)/ 绑定布局推导(G2.3)。
> 计划条款映射(拟,区间待 §9 Q-Range 定;**非裸条款头,仅计划登记**):RXS-0157 codegen target 分发与 DXIL 后端分叉 / RXS-0158 着色阶段着色 → DXIL 着色器类型降级 / RXS-0159 阶段 I/O → DXIL 签名·系统值语义降级 / RXS-0160 阶段间接口 → DXIL 阶段链接一致性核对(详见 RFC-0003 §5 计划表)。错误码 **6xxx codegen 段**随 PR-C2 按真实可达类别只追加(脚手架不预造、不预留)。

## 3. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-24 | 新建 dxil_backend.md（PR-C1 spec 脚手架，承 RFC-0003 / D-131=A）:登记文件名 + 文件级语义面说明（MIR→DXIL 第二后端，承 RFC-0002 着色阶段类型面 RXS-0153~0156）+ §1 范围与 **RXS-0157~ 预留区间声明**（区间大小未锁定，随 RFC-0003 §9 Q-Range 与路径裁定一并定）+ §2 条款占位（条款体随 PR-C2 实现 PR 同落）。**沿 README v1.32 interop_d3d12.md / v1.33 async_buffer.md / v1.37 shader_stages.md 脚手架先例:仅登记文件名 + 预留区间，不落带编号裸条款头**——本文件**零 `### RXS-####` 条款头**，`ci/trace_matrix.py --check` 维持全锚定 **156/156**（无新增裸条款头、无悬空锚点、零新 RXS）。条款体（RXS-0157 起）与每条 ≥1 `//@ spec` 测试锚定随 PR-C2（DXIL 后端实现 PR）同落（条款 PR 先于实现 PR）。禁区声明:🔒 纹理路径内存模型映射（06 §4.2）/ FFI ABI 二进制布局（RFC-0003 §4.6 / §9 Q-Builtin）/ 绑定布局推导（G2.3，P-11）/ 多后端架构承诺（D-008/SG-003）均不在本文件，触及即停手升档。错误码 **6xxx codegen 段**脚手架不预造、不预留，随 PR-C2 按真实可达类别只追加。档位 **Full RFC**（RFC-0003;触 codegen 第二后端 + target 分发，AI 不自判 Direct，判档争议向上取严）。授权 G2_CONTRACT D-G2-2 / G-G2-2 + G2_PLAN G2.2 子里程碑，无体例变更 | **Full RFC**（RFC-0003） |
