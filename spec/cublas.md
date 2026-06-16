# Rurix 语言规范 — cublas 绑定语义面(GEMM/GEMV 三层绑定:raw FFI / safe wrapper / 高层 API;runtime DLL Attachment A 白名单约定;M8.2 起)

> 条款:RXS-0126 起续号预留(M8.2 cublas 绑定语义面:cublas raw FFI 边界 / safe wrapper / 高层 GEMM·GEMV API / cublas runtime DLL 按需附带与 Attachment A 白名单约定)。体例见 [README.md](README.md)。
> 依据:09(NVIDIA 库绑定:cublas 绑定包,GEMM/GEMV 三层绑定 raw FFI / safe wrapper / 高层 API,NVIDIA 组件按需附带 runtime DLL——UC-01/UC-02 性能路径的库后端);05 §(FFI 边界:复杂类型不透明句柄 + create/destroy/operate,语言不经语言级绑定走 C ABI;`cublasHandle_t` 不透明句柄);01 §6(MVP 成功判据:自研 / 绑定 kernel ≥ 手写 CUDA C++ 90%;克制声明——绑定既有高性能库而非重造);07 §7(device codegen 分发:M8 维持 PTX-only,cubin/fatbin 真分发 → G1);11 §3 M8(互操作、加固与 MVP 验收);许可红线 r6(NVIDIA 再分发白名单审计:cublas runtime DLL 按需附带须经 Attachment A 白名单最小集,完整 Toolkit/驱动/Nsight 永不捆绑)。授权:[../milestones/m8/M8_CONTRACT.md](../milestones/m8/M8_CONTRACT.md)(`in_scope: cublas_pkg` / `spec_m8_clauses`,D-M8-2,G-M8-2 / G-M8-7,`rfc_required: none`)+ [../milestones/m8/M8_PLAN.md](../milestones/m8/M8_PLAN.md) §2 M8.2 第 1 项。
> 档位:**Direct**(条款体)。本文是对 01/05/09 已锁定决策(cublas 绑定包 / 三层绑定 / C ABI 不透明句柄边界 / runtime DLL 按需附带白名单审计)的初版条款化、纯追加且尚无 stable 面;**AI 无权自判 Direct**,判档以 M8_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。本里程碑识别一处新决策面——**cublas FFI 边界 unsafe 策略**:带档位标记 **Mini**(对齐 `src/rurix-rt` / `src/rurix-interop` 注册式 unsafe 豁免先例 + M8_CONTRACT §5 guardrail 已锁口径:FFI 边界 crate 经裁决最小开 unsafe + 每块 `// SAFETY:` + `unsafe-audit/` 注册,safe wrapper 层对上全 safe,其余新 crate `unsafe_code=deny`)。任何偏离已锁定决策、或触及 **Python 原生嵌入(红线 1,SG-008 永久红线,仅 C ABI/PYD 通道)** / **cubin/fatbin 真分发(G1,M8 维持 PTX-only)** / **完整 Toolkit/驱动/Nsight 捆绑(许可红线 r6)** / **const 泛型值运行期单态化(RD-007)** / **device 原子 lowering(D-406/RD-008 人工落笔禁区)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔(10 §3,M8_CONTRACT §6 / out_of_scope)。**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5):cublas 绑定边界的设备指针生命周期 / 所有权语义以 affine 所有权 + 确定性诊断(RX 错误码)定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0126 起)与每条 ≥1 测试锚定随 M8.2 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **cublas 绑定语义面**的语义条款(M8.2+,D-M8-2)。覆盖语义面:

- **cublas raw FFI 边界**:Rurix↔cublas v2 C API 的 `extern "C"` 声明面(`cublasCreate_v2` / `cublasDestroy_v2` / `cublasSetStream_v2` / `cublasSgemm_v2` / `cublasSgemv_v2` 等)+ `cublasHandle_t` 不透明句柄(05 §FFI:不透明句柄 + create/destroy/operate)+ 句柄 / 流绑定生命周期;FFI 边界 unsafe 最小化(unsafe 策略见前言档位标记 Mini)。
- **cublas safe wrapper**:句柄 RAII(`CublasHandle` 创建 / 销毁)+ GEMM/GEMV 设备指针与维度合法性校验(空指针 / 维度不匹配 → 互操作 7xxx 段位诊断续接)+ 列主序 / 转置约定;`cublasStatus_t != SUCCESS` 映射确定性诊断;safe wrapper 层对上全 safe(签名无 `unsafe`)。
- **cublas 高层 GEMM/GEMV API**:复用 `rurix-rt` 共享 primary context + 借用外部设备指针缓冲(对接 UC-01/UC-02 零拷贝路径,所有权留外部)的高层算子接口;row-major(Rurix/PyTorch)↔ col-major(cublas)适配;C ABI 导出(i32 错误码)供互操作 / 冒烟消费。
- **cublas runtime DLL 按需附带与 Attachment A 白名单约定**:cublas runtime DLL(`cublas64_*.dll` / `cublasLt64_*.dll`)按需附带须经 `check_redistribution` + **Attachment A 白名单最小集**审计;完整 Toolkit / 驱动 / Nsight **永不捆绑**(许可红线 r6)。M8.2 期**链接系统 DLL + 审计留痕**,物理捆绑 / 再分发承接 M8.4 发布链路。

全部 cublas 绑定以 **C ABI / 不透明句柄通道**为对接面(语言**不经语言级绑定**,05 §FFI);device 分发维持 **PTX-only**(07 §7;cubin/fatbin 真分发 → G1,M8 out_of_scope);设备指针所有权 / 生命周期以 **affine 所有权 + 确定性诊断**定义,**不以 UB 表述**(§4)。cublas 为既有高性能库**绑定**,非重造(01 §6 克制声明)。

**编号区间**:本文件条款自 **RXS-0126** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0125 @ [interop.md](interop.md))。本轮规划落地 **RXS-0126 ~ RXS-0129**(cublas raw FFI 边界 / safe wrapper / 高层 GEMM·GEMV API / runtime DLL Attachment A 白名单约定),每条 ≥1 测试锚定(`//@ spec: RXS-####`,`src/rurix-cublas` crate 单测),随 M8.2 实现 PR 同落。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款(计划骨架 — 本脚手架 PR 预留,非裸条款头)

> 沿 README v1.15 / v1.20 / v1.25 脚手架先例:本 PR **不落带编号裸条款头**(`### RXS-####`),仅以计划骨架列出预留条款,使 `trace_matrix --check` 维持全锚定(无未锚定条款)。条款体(每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**,Legality 违例只引用错误码)与每条 ≥1 测试锚定随 M8.2 实现 PR 同落。

本文件 §2 规划如下四条(RXS-0126 ~ RXS-0129),随实现 PR 升格为条款体:

- **RXS-0126 cublas raw FFI 边界**:cublas v2 C API `extern "C"` 声明面 + `cublasHandle_t` 不透明句柄 + 句柄 / 流绑定生命周期;FFI unsafe 边界(档位 Mini,每块 `// SAFETY:` + `unsafe-audit/rurix-cublas.md` 注册)。
- **RXS-0127 cublas safe wrapper**:`CublasHandle` RAII + 设备指针 / 维度合法性校验(违例引用 7xxx 段位错误码)+ 列主序 / 转置约定 + `cublasStatus_t` 映射;对上全 safe。
- **RXS-0128 cublas 高层 GEMM/GEMV API**:复用 `rurix-rt` 共享 primary context + 借用外部设备指针缓冲;row-major ↔ col-major 适配;C ABI i32 返回码语义一致(与 safe wrapper 同义)。
- **RXS-0129 cublas runtime DLL 按需附带与 Attachment A 白名单约定**:`cublas64_*.dll` / `cublasLt64_*.dll` 按需附带须经 `check_redistribution` + Attachment A 白名单最小集审计;完整 Toolkit / 驱动 / Nsight 永不捆绑(许可红线 r6)。

## 3. 错误码引用汇总(新段位首批分配随实现 PR)

> 本节随 M8.2 实现 PR 落地 cublas 诊断错误码**引用**(07 §5 7xxx 链接 / 工具链段位续接,接 M8.1 互操作首批 RX7013~RX7015 之后,**续号 RX7016 起**,只追加、含义冻结),含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源;message-key 落 [../src/rurixc/src/messages/en.messages](../src/rurixc/src/messages/en.messages)(zh 双语全量属 M8.5 / RD-006)。**本脚手架 PR 不预造错误码**(M8_CONTRACT §5 / CI_GATES §5 第 2 项:新段位错误码首批分配随诊断 PR 留痕,开工脚手架不预造)。

## 4. 升档 / 禁区留痕

- **cublas FFI 边界 unsafe 策略(新决策面,档位 Mini)**:cublas 绑定 FFI 不可避免触 unsafe;口径已由 M8_CONTRACT §5 guardrail 锁定——FFI 边界 crate 经裁决最小开 unsafe + 每 unsafe 块 `// SAFETY:` + `unsafe-audit/` 注册条目(AGENTS 硬规则 9),safe wrapper 层对上全 safe,其余新 crate 默认 `unsafe_code=deny`。**AI 不自判 Direct**,该决策面带档位标记 Mini 落笔,判档争议向上取严。
- **NVIDIA 再分发白名单(许可红线 r6)**:cublas runtime DLL 按需附带须经 Attachment A 白名单最小集审计(`check_redistribution` 延续,M5.4 已激活);完整 Toolkit / 驱动 / Nsight **永不捆绑**;EULA 法律签署维持 pending-human-review,**AI 不代签**(对齐 M5 redistribution_audit 先例)。触及完整 Toolkit / 驱动 / Nsight 捆绑即停下标注「需人工升档」。
- **Python 原生嵌入(永久红线 1,SG-008)**:cublas 绑定仅保留 **C ABI / 不透明句柄通道**;Python 解释器宿主 / 原生嵌入为死亡路线红线,**永不实现**(SG-008 维持 not_triggered)。触及即停下标注「需人工升档」。
- **cubin/fatbin 真分发(G1,PTX-only)**:M8 维持 **PTX-only** 开发期产物(07 §7);cublas 为运行期库绑定(经 DLL),不改 device codegen 分发形态;cubin/fatbin 真分发 → G1(M8 out_of_scope)。
- **const 泛型值运行期单态化(RD-007)**:cublas 绑定作用面若触发数组长度类 const 泛型运行期单态化——**非 M8 验收门**(M8_CONTRACT out_of_scope / §6,inherited);本文件**不实现 RD-007**,亦不改 [consteval.md](consteval.md) RXS-0064 语义。遇硬需求**停下标注「需人工升档」**,按 14 §4 处置。
- **device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 人工落笔禁区)**:不在本文件 cublas 绑定语义面登记;触及即停下标注「需人工升档」。
- **UB 节禁区**:cublas 绑定边界的设备指针生命周期 / 所有权语义以 **affine 所有权 + 确定性诊断(RX 错误码)** 定义,**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-16 | 新建 spec/cublas.md(M8.2 cublas 绑定语义面起始文件):登记编号区间 RXS-0126 起续号预留 + 文件级前言 / 范围(cublas raw FFI 边界 / safe wrapper / 高层 GEMM·GEMV API / runtime DLL 按需附带与 Attachment A 白名单约定,C ABI·不透明句柄通道、永不 Python 原生嵌入、PTX-only、affine 所有权不设 UB、完整 Toolkit/驱动/Nsight 永不捆绑)/ 依据与授权(09 + 05 §FFI + 01 §6 + 07 §7 + 11 §3 M8 + 许可红线 r6;M8_CONTRACT D-M8-2 / G-M8-2 / G-M8-7 `rfc_required: none` + M8_PLAN §2)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0126 raw FFI 边界 / RXS-0127 safe wrapper / RXS-0128 高层 GEMM·GEMV API / RXS-0129 runtime DLL Attachment A 白名单)/ 错误码新段位预留说明(§3:cublas 诊断续接 7xxx RX7016 起随实现 PR 分配,脚手架不预造)/ 升档·禁区留痕(§4:cublas FFI unsafe 策略带档位标记 Mini、NVIDIA 再分发白名单 r6、Python 原生嵌入红线 1/SG-008、PTX-only/G1、RD-007、D-406/RD-008、UB 节禁区)。**沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 M8.2 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
