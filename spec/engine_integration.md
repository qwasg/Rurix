# Rurix 语言规范 — 引擎集成语义面（Rurix DLL 经 C ABI 嵌入 C++/D3D12 框架承担 compute pass；G1.3 起）

> 条款：RXS-0149 起续号预留（G1.3 引擎集成语义面：引擎集成 DLL 打包约定 + C ABI 头文件与导出 ABI 逐一对应）。体例见 [README.md](README.md)。
> 依据：06 §8.3（引擎级工作流 U5 服务承诺，UC-05 前奏：affine 资源 + 生命周期 brand + C ABI 嵌入现存引擎）；02 §U5（图形引擎开发者画像 + UC-05 最小 RHI/render graph；采纳判据 = C ABI FFI 成熟 + 增量 check `<5s`）；05 §11（D-113：C ABI 唯一，导出走 `#[export(c)]` + 编译器内建头文件生成，cbindgen 角色内置化，P-11）；[interop.md](interop.md) **RXS-0125**（M8.1 既有 C ABI 边界，**复用**，`src/rurix-interop`）；07 §7（device codegen 分发，PTX-only / cubin·fatbin → G1）；11 §4（G1 期：首个引擎集成）。授权：[../milestones/g1/G1_CONTRACT.md](../milestones/g1/G1_CONTRACT.md)（`in_scope: engine_integration` / `spec_g1_clauses`，D-G1-3，G-G1-3）+ [../milestones/g1/G1_PLAN.md](../milestones/g1/G1_PLAN.md) §3 + [../rfcs/mini-0002-engine-integration.md](../rfcs/mini-0002-engine-integration.md)（**MR-0002**）。
> 档位：**Mini-RFC**（[MR-0002](../rfcs/mini-0002-engine-integration.md)，owner 2026-06-20 经 AskUserQuestion 裁决）。本文**复用** RXS-0125 既有手写 `extern "C"` C ABI（语义 0-byte）+ `cdylib` DLL 打包 + 自建最小 C++/D3D12 harness，**不扩 C ABI / ABI 表面、不实现 `#[export(c)]` 编译器 codegen（defer，RD-009）、不触内存模型映射 / 新 FFI ABI 禁区**（区别于 G1.1 因新 FFI ABI 面裁 Full RFC/RFC-0001）。**AI 不自判 Direct**，判档以 MR-0002 + G1_CONTRACT 授权为据，判档争议向上取严。任何**扩 C ABI 表面 / 新导出语义 / 跨边界新所有权语义 / 实现 `#[export(c)]` codegen**，或触及 **Python 原生嵌入（红线 1，SG-008，仅 C ABI/PYD 通道）** / **图形着色阶段进语言（G2，D-131）** 的条款，必须停下标注「需人工升档」，不在本文件自行落笔（10 §3，MR-0002 §3）。**严禁 UB 节**（UB 为人类经 Full RFC 落笔的禁区，10 §7.5）：C ABI 边界的指针生命周期 / 所有权语义以 affine 所有权 + 确定性诊断（RX 错误码）定义，不以 UB 表述。
> 规范先行（AGENTS.md 硬规则第 7 条）：**条款 PR 先于实现 PR**；缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定（`//@ spec: RXS-####` 或单测注释）。**本脚手架 PR 沿 interop.md / async_buffer.md 先例：仅登记新文件名 + 预留区间，不落带编号裸条款头**——条款体（RXS-0149）与每条 ≥1 测试锚定随 G1.3 实现 PR（步骤 43）同落（条款 PR 先于实现 PR，trace_matrix 维持全锚定）。

---

## 1. 范围与编号区间

本文件承载 **引擎集成语义面**（G1.3+，D-G1-3）。覆盖语义面：

- **引擎集成 DLL 打包约定**：承担 compute pass 的 C ABI（**复用** RXS-0125 既有 `extern "C" fn … -> i32` 形态，语义 0-byte）编为 `cdylib` 产物（`rurix_engine.dll` + import lib，`rurix-cublas` cdylib 先例）；宿主 C++/D3D12 框架经头文件 + import lib 链接调用。
- **C ABI 头文件与导出 ABI 逐一对应**：随附头文件每个声明 ↔ 一个 DLL 导出符号（无悬空声明 / 无未声明导出）；头文件与导出 ABI 单一事实源对应（D-113「编译器内建头文件生成」方向的本期工程兑现——以与 ABI 1:1 的**随附**头文件兑现；`#[export(c)]` 编译器 codegen + 内建头文件生成实现 **defer**，RD-009）。

全部引擎集成产物以 **C ABI 通道**对接（**复用** M8.1 既有 C ABI，**不扩 ABI 表面**）；**永不 Python 原生嵌入 / 解释器宿主**（死亡路线红线 1，SG-008，见 §4）。本期 Rurix 仅承担 **compute pass**，**不进图形着色阶段 / DXIL 第二后端**（G2，06 §8.2 / D-131）；device 分发沿既有 PTX 装载协商（07 §7）。

**编号区间**：本文件条款自 **RXS-0149** 起续号（全 spec 唯一、分配制递增、永不复用，见 [README.md](README.md) §1；最高现存 RXS-0148 @ [async_buffer.md](async_buffer.md)）。本轮预留 **RXS-0149**（引擎集成 DLL 打包与 C ABI 头文件对应），条款体与 ≥1 测试锚定（`//@ spec: RXS-0149`，`src/rurix-engine` crate 单测）随 G1.3 实现 PR 同落。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款（计划骨架 — 脚手架不落裸条款头）

> 本脚手架 PR **不落**带编号裸条款头（沿 interop.md v1.0 / async_buffer.md 脚手架先例）；条款体随 G1.3 实现 PR（步骤 43）同落，每条 ≥1 测试锚定，trace_matrix 维持全锚定。

- **RXS-0149（拟落）引擎集成 DLL 打包与 C ABI 头文件对应**：`cdylib` 产物形态（`rurix_engine.dll` + import lib）+ 导出符号**复用** RXS-0125 既有 `extern "C" fn … -> i32` 形态；导出符号集与随附头文件声明**逐一对应**（无悬空声明 / 无未声明导出）；宿主 C++/D3D12 框架经头文件 + import lib 链接调 compute pass C ABI 入口（设备指针 + 维度按值 → `i32` 错误码 0/RX7013~7015/负），compute pass 复用既有 device kernel（saxpy/reduce 等，**语义 0-byte**）；引擎边界 crate `unsafe_code` 经裁决最小开（C ABI 导出边界）+ unsafe-audit 注册，safe wrapper 对上全 safe。锚定测试：`src/rurix-engine`（`c_abi_header_matches_exports`：随附头文件声明 ↔ 导出符号一致）。

## 3. 错误码引用汇总

> 引擎集成 compute pass C ABI **复用** M8.1 既有互操作诊断错误码（RXS-0125，RX7013~RX7015，07 §5 7xxx 段位，只追加、含义冻结），含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源。**本文件零新增错误码**（对齐 G1.1 RXS-0140~0143 / G1.2 RXS-0144~0148 零新码先例）。compute pass C ABI 入口返回 `i32`：`0` = 成功；`RX7013`/`RX7014`/`RX7015` = 互操作诊断（协议不支持 / 设备指针非法 / 形状不匹配）；负 = 运行时/驱动失败。若 device 段实现期某类别确需**新**运行期诊断段位码，按 14 §4 + RX 段位制（7xxx 从 **RX7020** 起，M8.2 止于 RX7019）处置并停手标注，**不预造**。

## 4. 升档 / 禁区留痕

- **C ABI 复用 vs `#[export(c)]` codegen（新决策面，档位 Mini-RFC/MR-0002）**：owner 裁决复用 RXS-0125 既有 `extern "C"` C ABI（不扩 ABI 表面）+ 随附 1:1 头文件；`#[export(c)]` 编译器 codegen + 内建头文件生成实现 **defer**（**RD-009**，registry/deferred.json）。**AI 不自判 Direct**，判档以 MR-0002 + G1_CONTRACT 授权为据，争议向上取严。
- **扩 C ABI 表面 / 新导出语义 / 跨边界新所有权语义（FFI ABI 禁区，AGENTS 硬规则 5）**：若实现期确需，**停手升 Full RFC**（向上取严，镜像 G1.1 RFC-0001），不在本文件自行落笔。
- **宿主 C++/D3D12 框架选型（G1.3 执行期新决策面）**：owner 裁决 = **自建最小 C++/D3D12 harness**（CI_GATES §1 留痕；COM 复杂度留 C++ 不进语言，对齐 D-130 shim 纪律）。
- **Python 原生嵌入（永久红线 1，SG-008）**：仅保留 C ABI 通道；Python 解释器宿主 / 原生嵌入永不实现。触及即停下标注「需人工升档」。
- **图形着色阶段进语言 / DXIL 第二后端（G2，D-131 待决）**：本期 Rurix 仅承担 compute pass；触及即停下标注「需人工升档」。
- **UB 节禁区**：C ABI 边界的指针生命周期 / 所有权语义以 **affine 所有权 + 确定性诊断（RX 错误码）** 定义，**严禁 UB 节**（10 §7.5）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-20 | 新建 spec/engine_integration.md（G1.3 引擎集成语义面起始文件）：登记编号区间 RXS-0149 起续号预留 + 文件级前言 / 范围（引擎集成 DLL 打包约定 + C ABI 头文件与导出 ABI 逐一对应；**复用** M8.1 RXS-0125 既有手写 `extern "C"` C ABI 语义 0-byte、`cdylib` 产 DLL、自建最小 C++/D3D12 harness、仅承担 compute pass 不进图形着色阶段、永不 Python 原生嵌入、PTX-only、affine 所有权不设 UB）/ 依据与授权（06 §8.3 + 02 §U5 + 05 §11 D-113 + interop.md RXS-0125 + 07 §7 + 11 §4；G1_CONTRACT D-G1-3 / G-G1-3 + G1_PLAN §3 + MR-0002）/ 计划条款骨架（§2 预留，非裸条款头：RXS-0149 引擎集成 DLL 打包与 C ABI 头文件对应）/ 错误码零新增说明（§3：复用 RX7013~7015，7xxx 新段位若需从 RX7020 起随实现 PR 分配，脚手架不预造）/ 升档·禁区留痕（§4：C ABI 复用 vs `#[export(c)]` codegen 带档位标记 Mini-RFC + `#[export(c)]` codegen defer RD-009、扩 ABI 表面升 Full RFC、宿主自建 harness 裁决留痕、Python 原生嵌入红线 1/SG-008、图形着色阶段 G2/D-131、UB 节禁区）。**沿 interop.md / async_buffer.md 脚手架先例：本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 G1.3 实现 PR（步骤 43）同落（条款 PR 先于实现 PR，trace_matrix 维持全锚定），无体例变更 | **Mini-RFC**（MR-0002） |
