# Rurix 语言规范 — 互操作语义面(PYD 产出 / CAI v3 + DLPack 双协议零拷贝 / C ABI 边界;M8.1 起)

> 条款:RXS-0122 起续号预留(M8.1 互操作语义面:`rx build --emit=pyd` PYD 产出约定 / `__cuda_array_interface__` v3 + DLPack 双协议零拷贝零拷贝接入 / C ABI 边界)。体例见 [README.md](README.md)。
> 依据:09 §6(Python 互操作 D-307:`rx build --emit=pyd` 产 PYD,绑定层 nanobind + scikit-build-core,数据通道 `__cuda_array_interface__` v3 + DLPack 双协议零拷贝接入 PyTorch/CuPy——UC-01 的实现路径;Windows DLL 陷阱纪律;永不 Python 原生嵌入);02 §U1 / §4(UC-01 PyTorch 瓶颈算子替换:DLPack/`__cuda_array_interface__` 零拷贝、PYD 产出、性能 ≥ 手写 CUDA 90%、全程无裸指针);01 §6(MVP 成功判据:SAXPY/Reduction/GEMM kernel ≥ 手写 CUDA C++ 90%;克制声明——与 PyTorch 经 DLPack 零拷贝互操作,不是替代它);05 §(FFI 边界:复杂类型不透明句柄 + create/destroy/operate,Python 不经语言级绑定走 C ABI + nanobind);07 §7(device codegen 分发:M8 维持 PTX-only,cubin/fatbin 真分发 → G1);11 §3 M8(互操作、加固与 MVP 验收)。授权:[../milestones/m8/M8_CONTRACT.md](../milestones/m8/M8_CONTRACT.md)(`in_scope: uc01_pytorch_interop` / `spec_m8_clauses`,D-M8-1,G-M8-1 / G-M8-7,`rfc_required: none`)+ [../milestones/m8/M8_PLAN.md](../milestones/m8/M8_PLAN.md) §1 M8.1 第 1 项。
> 档位:**Direct**(条款体)。本文是对 01/02/05/09 已锁定决策(UC-01 PyTorch 算子替换 / PYD 产出 / nanobind + scikit-build-core / `__cuda_array_interface__` v3 + DLPack 双协议零拷贝 / C ABI 边界)的初版条款化、纯追加且尚无 stable 面;**AI 无权自判 Direct**,判档以 M8_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。本里程碑识别一处新决策面——**PYD/C ABI 边界 unsafe 策略**:带档位标记 **Mini**(对齐 `src/rurix-rt` 注册式 unsafe 豁免先例 + M8_CONTRACT §5 guardrail 已锁口径:FFI 边界 crate 经裁决最小开 unsafe + 每块 `// SAFETY:` + `unsafe-audit/` 注册,safe wrapper 层对上全 safe,其余新 crate `unsafe_code=deny`)。任何偏离已锁定决策、或触及 **Python 原生嵌入(红线 1,SG-008 永久红线,仅 C ABI/PYD 通道)** / **cubin/fatbin 真分发(G1,M8 维持 PTX-only)** / **const 泛型值运行期单态化(RD-007)** / **device 原子 lowering(D-406/RD-008 人工落笔禁区)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔(10 §3,M8_CONTRACT §6 / out_of_scope)。**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5):互操作边界的指针生命周期 / 所有权语义以 affine 所有权 + 确定性诊断(RX 错误码)定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.15 toolchain.md / v1.20 stdlib.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0122 起)与每条 ≥1 测试锚定随 M8.1 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **互操作语义面**的语义条款(M8.1+,D-M8-1)。覆盖语义面:

- **`rx build --emit=pyd` PYD 产出约定**:`rx build` 新增 `--emit=pyd` 通道,经编译器 device codegen(PTX-only,07 §7)+ 运行时 + 绑定层(nanobind + scikit-build-core,09 §6)产出 Python 扩展模块(`.pyd`);产物形态 / 入口符号 / 模块装载约定。
- **`__cuda_array_interface__` v3 协议**:Rurix 显存缓冲对 PyTorch/CuPy 暴露 / 消费 `__cuda_array_interface__` v3(device 指针 `data`、`typestr`、`shape`、`strides`、`version`、`stream` 字段语义与版本协商),零拷贝接入既有 CUDA 张量。
- **DLPack 双协议零拷贝**:`__dlpack__` / `from_dlpack`(DLPack capsule 生产 / 消费),capsule 消费一次性语义 + 设备指针生命周期 / 所有权(affine 所有权,deleter 释放责任,跨框架共享不悬垂 / 不双重释放)。
- **C ABI 边界**:Rurix↔C↔nanobind 的 C ABI 导出约定(不透明句柄 + create/destroy/operate,05 §FFI);FFI 边界 unsafe 最小化 + safe wrapper 层对上全 safe(unsafe 策略见前言档位标记 Mini)。

全部互操作产物以 **C ABI / PYD 通道**为唯一对接面(Python **不经语言级绑定**,05 §FFI);**永不 Python 原生嵌入 / 解释器宿主**(死亡路线红线 1,SG-008 维持 not_triggered,见 §5)。device 分发维持 **PTX-only**(07 §7;cubin/fatbin 真分发 → G1,M8 out_of_scope);设备指针所有权 / 生命周期以 **affine 所有权 + 确定性诊断**定义,**不以 UB 表述**(§5)。

**编号区间**:本文件条款自 **RXS-0122** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0121 @ [softraster.md](softraster.md))。区间登记于 [README.md](README.md) §4 文件清单。**本脚手架 PR 不落带编号裸条款头**;计划条款骨架见 §2,条款体与锚定随 M8.1 实现 PR 同落。

## 2. 计划条款骨架(预留,非裸条款头)

> 下表为 M8.1 互操作语义面的**计划条款骨架**(自 RXS-0122 续号),仅作分解登记,**不构成 `### RXS-####` 裸条款头**(避免 `trace_matrix --check` 因无锚定 FAIL)。条款体(Syntax / Legality / Dynamic Semantics / Implementation Requirements,严禁 UB 节)+ 每条 ≥1 测试锚定(`//@ spec: RXS-####`)随 **M8.1 实现 PR** 同落,届时 §1 编号区间更新为实际落地区间、本节升格为「互操作条款落地说明」、[README.md](README.md) §4 行区间同步更新。

| 计划条款号(预留) | 主题 | 语义要点(规划) |
|---|---|---|
| RXS-0122(预留) | `rx build --emit=pyd` PYD 产出约定 | `--emit=pyd` 通道:device codegen PTX-only + 绑定层 nanobind + scikit-build-core 工程,产 `.pyd` 扩展模块;产物形态 / 模块入口符号 / 未知 emit 拒绝 |
| RXS-0123(预留) | `__cuda_array_interface__` v3 消费 / 产出 | CAI v3 字段语义(`data` device 指针 / `typestr` / `shape` / `strides` / `version` / `stream`)+ 版本协商;零拷贝消费 PyTorch/CuPy CUDA 张量 |
| RXS-0124(预留) | DLPack 双协议零拷贝(capsule 消费 + 设备指针所有权) | `__dlpack__` / `from_dlpack` capsule 生产 / 消费;capsule 一次性消费语义 + 设备指针生命周期 / affine 所有权 / deleter 释放责任(不悬垂 / 不双重释放) |
| RXS-0125(预留) | C ABI 边界 | C ABI 导出约定(不透明句柄 + create/destroy/operate)+ FFI 边界 unsafe 最小化 + safe wrapper 层对上全 safe;协议不支持 / 设备指针非法 / 形状不匹配诊断(新段位错误码 RX7013+,随实现 PR 分配) |

> 实际落地条款号 / 条目数随 M8.1 实现 PR 确定(可 1~N 条,自 RXS-0122 递增);上表主题与区间为规划,非承诺。

## 3. 错误码引用(预留)

> 互操作诊断(协议不支持 / 设备指针非法 / 形状不匹配 等)的**新段位错误码首批分配**(续接 7xxx 链接/工具链段位,RX7013 起,07 §5 分配制递增、含义冻结、只追加)+ message-key 随 **M8.1 实现 PR** 落地(M8_CONTRACT §5 / CI_GATES §5.2:开工脚手架不预造错误码)。本脚手架 PR **不新增 / 不预造错误码**,不改 [../registry/error_codes.json](../registry/error_codes.json) 与 [../src/rurixc/src/messages/en.messages](../src/rurixc/src/messages/en.messages)。

## 4. 升档 / 禁区留痕

- **PYD/C ABI 边界 unsafe 策略(新决策面,档位 Mini)**:互操作(PYD / C ABI / DLPack 边界)FFI 不可避免触 unsafe;口径已由 M8_CONTRACT §5 guardrail 锁定——FFI 边界 crate 经裁决最小开 unsafe + 每 unsafe 块 `// SAFETY:` + `unsafe-audit/` 注册条目(AGENTS 硬规则 9),safe wrapper 层对上全 safe,其余新 crate 默认 `unsafe_code=deny`。**AI 不自判 Direct**,该决策面带档位标记 Mini 落笔,判档争议向上取严。
- **Python 原生嵌入(永久红线 1,SG-008)**:仅保留 **C ABI / PYD 通道**;Python 解释器宿主 / 原生嵌入为死亡路线红线,**永不实现**(SG-008 维持 not_triggered)。触及即停下标注「需人工升档」。
- **cubin/fatbin 真分发(G1,PTX-only)**:M8 维持 **PTX-only** 开发期产物(07 §7);PYD 内嵌 device 代码沿用既有 PTX 装载协商(RXS-0076),cubin/fatbin 真分发 → G1(M8 out_of_scope)。
- **const 泛型值运行期单态化(RD-007)**:互操作 / 绑定作用面若触发数组长度类 const 泛型运行期单态化——**非 M8 验收门**(M8_CONTRACT out_of_scope / §6,inherited);本文件**不实现 RD-007**,亦不改 [consteval.md](consteval.md) RXS-0064 语义。遇硬需求**停下标注「需人工升档」**,按 14 §4 处置。
- **device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 人工落笔禁区)**:不在本文件互操作语义面登记;触及即停下标注「需人工升档」。
- **UB 节禁区**:互操作边界的设备指针生命周期 / 所有权语义以 **affine 所有权 + 确定性诊断(RX 错误码)** 定义,**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-16 | 新建 spec/interop.md(M8.1 互操作语义面起始文件):登记编号区间 RXS-0122 起续号预留 + 文件级前言 / 范围(`rx build --emit=pyd` PYD 产出约定 / `__cuda_array_interface__` v3 + DLPack 双协议零拷贝 / C ABI 边界,C ABI·PYD 唯一通道、永不 Python 原生嵌入、PTX-only、affine 所有权不设 UB)/ 依据与授权(09 §6 + 02 §U1 + 01 §6 + 05 §FFI + 07 §7 + 11 §3 M8;M8_CONTRACT D-M8-1 / G-M8-1 / G-M8-7 `rfc_required: none` + M8_PLAN §1)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0122 PYD 产出 / RXS-0123 CAI v3 / RXS-0124 DLPack 双协议 + 设备指针所有权 / RXS-0125 C ABI 边界)/ 错误码新段位预留说明(§3:互操作诊断续接 7xxx RX7013+ 随实现 PR 分配,脚手架不预造)/ 升档·禁区留痕(§4:PYD/C ABI unsafe 策略带档位标记 Mini、Python 原生嵌入红线 1/SG-008、PTX-only/G1、RD-007、D-406/RD-008、UB 节禁区)。**沿 README v1.15 toolchain.md / v1.20 stdlib.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 M8.1 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
