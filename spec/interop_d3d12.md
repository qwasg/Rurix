# Rurix 语言规范 — CUDA–D3D12 互操作呈现语义面（`ExternalBuffer` / `ExternalSemaphore` / 实时窗口呈现；G1.1 起）

> 条款:RXS-0140 起续号预留(G1.1 CUDA–D3D12 互操作呈现语义面:`ExternalBuffer`/`ExternalSemaphore` affine 类型与 import 句柄生命周期 / 生成式 context brand 与跨 context 编译期拦截 / `Ready→Acquired→Presentable` typestate 与共享 fence 偶奇值 handoff / D3D12 committed resource import ABI 与 present pass 布局)。体例见 [README.md](README.md)。
> 依据:**[RFC-0001](../rfcs/0001-cuda-d3d12-interop.md)**(CUDA–D3D12 interop 与软光栅实时窗口呈现,**owner 已批准定稿**,2026-06-18);06 §8.1(D-130:D3D12 创建 swapchain + 共享堆 → `cuImportExternalMemory`/`cuImportExternalSemaphore` 映射 backbuffer 等价纹理 → Rurix kernel 写入 → 信号量同步 present;`ExternalBuffer`/`ExternalSemaphore` affine 类型,D3D12 侧薄 C FFI 不进语言);06 §4.2(内存模型映射边界);06 §6(三阶段图形路线 G0→G1→G2);08 §1/§2(`rurix-rt` 运行时对象 / Driver API 薄层,D-230);spec/softraster.md:153(实时窗口呈现归 G1-1);01 §6(每阶段必有出图)。授权:[../milestones/g1/G1_CONTRACT.md](../milestones/g1/G1_CONTRACT.md)(`in_scope: d3d12_interop` / `realtime_present` / `spec_g1_clauses`,D-G1-1,G-G1-1 / G-G1-6)+ [../milestones/g1/G1_PLAN.md](../milestones/g1/G1_PLAN.md) §1 G1.1 第 1 项。
> 档位:**Full RFC**(RFC-0001;10 §3:本设计触 FFI ABI / 运行时语义 / unsafe 边界 / 内存模型映射,AGENTS 硬规则 5 / 10 §7.5 禁区,只能人类经 Full RFC 落笔)。RFC-0001 已由 owner 于 2026-06-18 人工落笔/批准 🔒 禁区章节(§4.2 FFI ABI / §4.3 内存模型映射·信号时序 / §4.4 安全包络 + Q1~Q5);spec 条款 PR 与实现 PR 均门控于 RFC-0001 合入之后。**AI 无权自判 Direct**,判档以 RFC-0001 与 G1_CONTRACT 授权为据,判档争议向上取严。任何偏离 RFC-0001 已批准设计、或触及 **G2 原生 D3D12+DXIL(D-131)** / **多后端(D-008/SG-003)** / **Python 原生嵌入(红线 1,SG-008)** / **const 泛型值运行期单态化(RD-007)** / **device 原子 lowering(D-406/RD-008)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):import 句柄生命周期 / 跨 API 信号时序 / external 资源所有权以 **affine 所有权 + 生成式 brand + typestate + 确定性诊断**定义,不以 UB 表述(RFC-0001 §4.3/§4.4)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.25 interop.md / v1.29 pipeline.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0140 起)与每条 ≥1 测试锚定随 G1.1 实现 PR(步骤 40)同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **CUDA–D3D12 互操作呈现语义面**的语义条款(G1.1+,D-G1-1)。覆盖语义面(RFC-0001 §4):

- **`ExternalBuffer<'ctx, T>` / `ExternalSemaphore<'ctx>` affine 类型与 import 句柄生命周期**:import 自 D3D12 共享 committed resource / 共享 fence 的 affine(move-only、非 `Copy`/非 `Clone`)资源;Drop 强制销毁序(mapped pointer `cuMemFree` → `cuDestroyExternalSemaphore` → `cuDestroyExternalMemory` → shim 销毁;D3D12 COM owner 不被 CUDA wrapper 释放)。镜像 `src/rurix-rt` 既有 `DeviceBuffer`/`InFlight` affine 纪律。
- **生成式 context brand 与跨 context 编译期拦截**:`D3D12Presenter::scope` 以高阶 `for<'ctx>` 闭包生成不可伪造、不可逃逸的新鲜 `'ctx`;interop context / stream / module / external memory / semaphore 全携不变 brand;两个独立 `scope` 的资源类型不同,跨 context 误用为编译期类型/借用错误(不依赖普通 `'ctx` 生命周期或 `Arc` 指针运行期身份)。
- **`Ready→Acquired→Presentable` typestate 与共享 fence 偶/奇值 handoff**:消费式状态机——`ReadyFrame::wait`(CUDA `cuWaitExternalSemaphoresAsync(2n)`)→ `AcquiredFrame`(暴露可写 backbuffer + `launch`)→ `signal`(CUDA `cuSignalExternalSemaphoresAsync(2n+1)`)→ `PresentableFrame::present`(shim queue wait `2n+1` → present pass → signal `2n+2`)→ `ReadyFrame`;未 wait 无可写 buffer、未 signal 无 `present`;私有 stream 被状态对象捕获,wait/launch/signal 同 stream 序。
- **D3D12 committed resource import ABI 与 present pass 布局**:`D3D12_RESOURCE` + `CUDA_EXTERNAL_MEMORY_DEDICATED` 导入(否决 HEAP);共享 buffer 固定行主序紧密 `f32 RGB`、分量 `0…255`(与 RXS-0121 `sr_tonemap` `ViewMut<global, f32>` 输出逐字节同义);shim 私有 fullscreen present pass 读 buffer 写 `R8G8B8A8_UNORM` backbuffer(`/255`),非 Rurix shader codegen、不扩张 G2。

全部 D3D12/DXGI 经**薄 C/C++ shim**(`cc` + `build.rs`,COM 留 C++,Rust 仅见版本化扁平 `extern "C"`,RFC-0001 §4.2)驱动,**不进语言**(D-130);device 分发维持 **PTX-only**(07 §7);G0 软光栅 kernel(`src/rurix-rt/kernels/sr_*.rx`,RXS-0118~0121)语义面 **0-byte**,仅新增呈现通路。import 句柄生命周期 / 跨 API 信号时序 / external 资源所有权以 **affine 所有权 + 生成式 brand + typestate + 确定性诊断**定义,**不以 UB 表述**(§4)。

**编号区间**:本文件条款自 **RXS-0140** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0139 @ [release.md](release.md))。本轮(脚手架)**仅登记区间预留 RXS-0140 ~ RXS-0143**,**不落带编号裸条款头**;条款体与每条 ≥1 测试锚定随 G1.1 实现 PR(步骤 40)同落。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款（计划骨架 — 非裸条款头，随实现 PR 落地带编号条款体）

> 沿 README v1.25 / v1.29 先例,本脚手架**不落 `### RXS-####` 裸条款头**(避免未锚定条款,trace_matrix 维持全锚定);下列为计划骨架,实现 PR(步骤 40)落地带编号条款体 + 每条 ≥1 `//@ spec: RXS-####` 测试锚定。每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**。

| 条款(计划) | 标题 | 测试锚定计划(每条 ≥1) | RFC-0001 来源 |
|---|---|---|---|
| RXS-0140 | `ExternalBuffer`/`ExternalSemaphore` affine 类型与 import 句柄生命周期 | `src/rurix-rt` 单测(move-only;Drop 销毁序 mapped `cuMemFree`→destroy semaphore→destroy memory→shim destroy;D3D12 COM owner 不被 CUDA wrapper 释放) | §4.1 / §4.4 |
| RXS-0141 | 生成式 context brand 与跨 context 编译期拦截 | conformance reject + UI golden(两个独立 `scope` 的资源/模块/stream brand 不匹配 → 编译期错误) | §4.1 |
| RXS-0142 | `Ready→Acquired→Presentable` typestate 与偶/奇 fence 协议 | 单测(`2n`/`2n+1`/`2n+2` 值序;溢出确定拒绝)+ conformance reject(未 wait 无可写 buffer;未 signal 无 `present`) | §4.1 / §4.3 |
| RXS-0143 | D3D12 committed resource import ABI 与 present pass 布局 | ABI size/offset 单测 + 带 GPU smoke(`allocation_size`/`mapping_size`/LUID/RGB 通道与像素对照) | §4.2 |

> 最终条款数与区间随实现 PR 收敛(若拆分/合并条款,README §4 区间与本节同步更新,修订行留痕)。

## 3. 错误码引用汇总（新段位说明 — 脚手架不预造）

> 三类编译期拦截(句柄生命周期 / 跨 context / 信号时序违例)由 **Rust 类型系统原生编译期诊断**拦截(affine move `E0382` / 生成式 brand 不匹配 / typestate 方法不存在 `E0599`),**不新增 RX 段位码**(对齐 M8.3 pipeline.md §3 零新码先例,RFC-0001 §5)。

**运行期诊断**(import 失败 / device LUID mismatch / shim 初始化失败 / fence 溢出等)按 07 §5 在 **7xxx 段位续接**(最高现存 RX7019 @ cublas;从 **RX7020** 起按实现中真实可达、用户可行动的错误类别分配,**脚手架不预留号码、不预造码数**);含义冻结(10 §6,`check_error_codes` 延续),`registry/error_codes.json` 只追加并同时落 [../src/rurixc/src/messages/en.messages](../src/rurixc/src/messages/en.messages) + [../src/rurixc/src/messages/zh.messages](../src/rurixc/src/messages/zh.messages) 双语 message-key(bilingual_coverage 门)。原始 `CUresult` / `HRESULT` 保留为诊断 detail,不直接充当稳定 RX 码(RFC-0001 §5)。本表随实现 PR 落地回填。

## 4. 升档 / 禁区留痕

- **本文件档位 = Full RFC(RFC-0001)**:不同于 M8 互操作/cublas/pipeline 的 Direct(契约 rfc_required:none 授权的初版条款化)——本设计触 **FFI ABI / 内存模型映射(信号时序) / 安全包络**(AGENTS 硬规则 5 / 10 §7.5 禁区),owner 经 AskQuestion 裁决 **Full RFC 前置**,🔒 禁区由 owner 于 2026-06-18 人工落笔/批准(RFC-0001 §4.2/§4.3/§4.4 + Q1~Q5)。**AI 不自判 Direct**,判档争议向上取严。
- **G0 软光栅 kernel 语义面 0-byte**:`src/rurix-rt/kernels/sr_*.rx` 与 RXS-0118~0121 字节不动;present shader 是 shim 私有固定资产(构建期 HLSL→DXBC 嵌入),**不进入 Rurix 语言/API/stable 面**,不扩张 G2(RFC-0001 §4.2.2 / §6)。
- **G2 原生 D3D12+DXIL(D-131)**:着色阶段进语言 + DXIL codegen 第二后端 → G2,不在本文件;触及即停下标注「需人工升档」。
- **多后端 / Python 原生嵌入 / const 泛型值运行期单态化 / device 原子 lowering**:分别为 D-008/SG-003、红线 1/SG-008、RD-007、D-406/RD-008,均不在本文件互操作呈现语义面登记;触及即停下标注「需人工升档」。
- **UB 节禁区**:import 句柄生命周期 / 跨 API 信号时序 / external 资源所有权以 **affine 所有权 + 生成式 brand + typestate + 确定性诊断**定义,**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5;RFC-0001 §4.3 已落笔保守保证边界)。

---

## 修订记录

| 版次 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-18 | 新建 spec/interop_d3d12.md(G1.1 CUDA–D3D12 互操作呈现语义面起始文件):登记编号区间 RXS-0140 起续号预留(RXS-0140 ~ RXS-0143 计划)+ 文件级前言 / 范围(`ExternalBuffer`/`ExternalSemaphore` affine 类型与 import 句柄生命周期 / 生成式 context brand 与跨 context 编译期拦截 / `Ready→Acquired→Presentable` typestate 与共享 fence 偶奇值 handoff / D3D12 committed resource import ABI 与 present pass 布局;薄 C/C++ shim 不进语言、PTX-only、G0 kernel 0-byte、affine+生成式 brand+typestate 不设 UB)/ 依据与授权(RFC-0001 owner 批准 + 06 §8.1/§4.2/§6 + 08 §1/§2 + spec/softraster.md:153 + 01 §6;G1_CONTRACT D-G1-1 / G-G1-1 / G-G1-6 + G1_PLAN §1)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0140 affine 类型与句柄生命周期 / RXS-0141 生成式 brand 跨 context 拦截 / RXS-0142 typestate 与偶奇 fence / RXS-0143 import ABI 与 present pass 布局)/ 错误码新段位说明(§3:编译期 rustc 原生零新码;运行期 7xxx RX7020+ 随实现按需,脚手架不预造)/ 升档·禁区留痕(§4:档位 Full RFC/RFC-0001、G0 0-byte、G2 D-131、多后端/红线1/RD-007/D-406、UB 节禁区)。**沿 README v1.25 / v1.29 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 G1.1 实现 PR(步骤 40)同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | **Full RFC**(RFC-0001) |
