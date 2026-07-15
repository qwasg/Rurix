# Rurix 语言规范 — cublas 绑定语义面(GEMM/GEMV 三层绑定:raw FFI / safe wrapper / 高层 API;runtime DLL Attachment A 白名单约定;M8.2 起)

> 条款:RXS-0126 起续号预留(M8.2 cublas 绑定语义面:cublas raw FFI 边界 / safe wrapper / 高层 GEMM·GEMV API / cublas runtime DLL 按需附带与 Attachment A 白名单约定)。体例见 [README.md](README.md)。
> 依据:09(NVIDIA 库绑定:cublas 绑定包,GEMM/GEMV 三层绑定 raw FFI / safe wrapper / 高层 API,NVIDIA 组件按需附带 runtime DLL——UC-01/UC-02 性能路径的库后端);05 §(FFI 边界:复杂类型不透明句柄 + create/destroy/operate,语言不经语言级绑定走 C ABI;`cublasHandle_t` 不透明句柄);01 §6(MVP 成功判据:自研 / 绑定 kernel ≥ 手写 CUDA C++ 90%;克制声明——绑定既有高性能库而非重造);07 §7(device codegen 分发:M8 维持 PTX-only,cubin/fatbin 真分发 → G1);11 §3 M8(互操作、加固与 MVP 验收);许可红线 r6(NVIDIA 再分发白名单审计:cublas runtime DLL 按需附带须经 Attachment A 白名单最小集,完整 Toolkit/驱动/Nsight 永不捆绑)。授权:[../milestones/m8/M8_CONTRACT.md](../milestones/m8/M8_CONTRACT.md)(`in_scope: cublas_pkg` / `spec_m8_clauses`,D-M8-2,G-M8-2 / G-M8-7,`rfc_required: none`)+ [../milestones/m8/M8_PLAN.md](../milestones/m8/M8_PLAN.md) §2 M8.2 第 1 项。
> 档位:**Direct**(条款体)。本文是对 01/05/09 已锁定决策(cublas 绑定包 / 三层绑定 / C ABI 不透明句柄边界 / runtime DLL 按需附带白名单审计)的初版条款化、纯追加且尚无 stable 面;**agent 自主判档**,判档以 M8_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。本里程碑识别一处新决策面——**cublas FFI 边界 unsafe 策略**:带档位标记 **Mini**(对齐 `src/rurix-rt` / `src/rurix-interop` 注册式 unsafe 豁免先例 + M8_CONTRACT §5 guardrail 已锁口径:FFI 边界 crate 经裁决最小开 unsafe + 每块 `// SAFETY:` + `unsafe-audit/` 注册,safe wrapper 层对上全 safe,其余新 crate `unsafe_code=deny`)。任何偏离已锁定决策、或触及 **Python 原生嵌入(红线 1,SG-008 永久红线,仅 C ABI/PYD 通道)** / **cubin/fatbin 真分发(G1,M8 维持 PTX-only)** / **完整 Toolkit/驱动/Nsight 捆绑(许可红线 r6)** / **const 泛型值运行期单态化(RD-007)** / **device 原子 lowering(D-406/RD-008 agent 自主落笔的高敏面)** 的条款,必须停下标注「需升档」,不在本文件自行落笔(10 §3,M8_CONTRACT §6 / out_of_scope)。**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5):cublas 绑定边界的设备指针生命周期 / 所有权语义以 affine 所有权 + 确定性诊断(RX 错误码)定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0126 起)与每条 ≥1 测试锚定随 M8.2 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **cublas 绑定语义面**的语义条款(M8.2+,D-M8-2)。覆盖语义面:

- **cublas raw FFI 边界**:Rurix↔cublas v2 C API 的 `extern "C"` 声明面(`cublasCreate_v2` / `cublasDestroy_v2` / `cublasSetStream_v2` / `cublasSgemm_v2` / `cublasSgemv_v2` 等)+ `cublasHandle_t` 不透明句柄(05 §FFI:不透明句柄 + create/destroy/operate)+ 句柄 / 流绑定生命周期;FFI 边界 unsafe 最小化(unsafe 策略见前言档位标记 Mini)。
- **cublas safe wrapper**:句柄 RAII(`CublasHandle` 创建 / 销毁)+ GEMM/GEMV 设备指针与维度合法性校验(空指针 / 维度不匹配 → 互操作 7xxx 段位诊断续接)+ 列主序 / 转置约定;`cublasStatus_t != SUCCESS` 映射确定性诊断;safe wrapper 层对上全 safe(签名无 `unsafe`)。
- **cublas 高层 GEMM/GEMV API**:复用 `rurix-rt` 共享 primary context + 借用外部设备指针缓冲(对接 UC-01/UC-02 零拷贝路径,所有权留外部)的高层算子接口;row-major(Rurix/PyTorch)↔ col-major(cublas)适配;C ABI 导出(i32 错误码)供互操作 / 冒烟消费。
- **cublas runtime DLL 按需附带与 Attachment A 白名单约定**:cublas runtime DLL(`cublas64_*.dll` / `cublasLt64_*.dll`)按需附带须经 `check_redistribution` + **Attachment A 白名单最小集**审计;完整 Toolkit / 驱动 / Nsight **永不捆绑**(许可红线 r6)。M8.2 期**链接系统 DLL + 审计留痕**,物理捆绑 / 再分发承接 M8.4 发布链路。

全部 cublas 绑定以 **C ABI / 不透明句柄通道**为对接面(语言**不经语言级绑定**,05 §FFI);device 分发维持 **PTX-only**(07 §7;cubin/fatbin 真分发 → G1,M8 out_of_scope);设备指针所有权 / 生命周期以 **affine 所有权 + 确定性诊断**定义,**不以 UB 表述**(§4)。cublas 为既有高性能库**绑定**,非重造(01 §6 克制声明)。

**编号区间**:本文件条款自 **RXS-0126** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0125 @ [interop.md](interop.md))。本轮落地 **RXS-0126 ~ RXS-0129**(cublas raw FFI 边界 / safe wrapper / 高层 GEMM·GEMV API / runtime DLL Attachment A 白名单约定),每条 ≥1 测试锚定(`//@ spec: RXS-####`,`src/rurix-cublas` crate 单测)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。Legality 违例只**引用**错误码(§3 引用汇总),不在此定义其含义。cublas 绑定边界的设备指针生命周期 / 所有权语义以 **affine 所有权 + 确定性诊断(RX 错误码)** 定义,不以 UB 表述。

### RXS-0126 cublas raw FFI 边界

**Syntax**(cublas v2 C API `extern "C"` 声明面 + 不透明句柄):

```
CublasHandle  ::= "*mut c_void"                       // cublasHandle_t 不透明句柄(05 §FFI)
CublasStatus  ::= "c_int"                             // cublasStatus_t,0 = CUBLAS_STATUS_SUCCESS
RawFfiSym     ::= "cublasCreate_v2" | "cublasDestroy_v2" | "cublasSgemm_v2" | "cublasSgemv_v2"
DllCandidate  ::= "cublas64_" <ver> ".dll"            // Attachment A 白名单最小集(RXS-0129)
```

**Legality**:

- cublas runtime DLL 候选名须限 **Attachment A 白名单最小集**形态(`cublas64_*.dll`);完整 Toolkit / 驱动(`nvcuda*`)/ Nsight / 静态导入库(`*.lib`)/ libdevice 不得作为加载候选(许可红线 r6,RXS-0129)。
- raw FFI 符号集为 cublas v2 C API(`*_v2` ABI);符号缺失 / DLL 不可用 → 上层映射 `RX7016`(safe wrapper 层裁定,见 RXS-0127)。

**Dynamic Semantics**:

- cublas runtime DLL 经 `LoadLibraryA` / `GetProcAddress` **运行期动态加载**(非链接期绑定,对齐 `rurix-rt` `nvcuda.dll` 先例:开发机无 Toolkit 时仍可编译,host-only CI 不致链接死);进程内单次加载(失败 → `None`)。
- `cublasHandle_t` 句柄绑定 **current context**——由 [`rurix-rt` `Context::from_primary`] 设置的与 PyTorch 共享的 device primary context(句柄生命周期不晚于其创建时的 current context)。

**Implementation Requirements**:

- C ABI = `extern "C"`(Windows x64 唯一 ABI,D-113);设备指针(A/B/C/x/y)以 `u64` 设备地址按值传参(x64 GP 寄存器与 `const float*` 同宽,ABI 等价,FFI 层不解引用),alpha/beta 为主机标量指针(`CUBLAS_POINTER_MODE_HOST` 默认)。
- FFI 边界为 **unsafe 边界**(经裁决最小开 unsafe,档位 Mini);每 unsafe 块 `// SAFETY:` + `unsafe-audit/rurix-cublas.md` 注册(原语 C1~C5)。

> 锚定测试:`src/rurix-cublas`(`raw_ffi_dll_candidates_attachment_a`:候选 DLL 名全部匹配 `cublas64_*.dll` Attachment A 形态 + `cublasHandle_t` 为指针宽度不透明句柄)。

### RXS-0127 cublas safe wrapper

**Syntax**(safe wrapper 签名,对上全 safe):

```
CublasHandle::create() -> Result<CublasHandle, i32>      // RAII,Drop → cublasDestroy
gemm(c, a, b, m, n, k: usize) -> i32                     // 0 = 成功;否则 RX70xx
gemv(y, a, x, m, n: usize) -> i32
```

**Legality**:

- 句柄创建失败(cublas runtime DLL 不可用 / `cublasCreate` 非 `SUCCESS`)→ `RX7016`。
- 设备指针(`c`/`a`/`b`/`y`/`x`)任一为空(`0`)→ `RX7017`(空指针 / 非设备地址)。
- 维度任一为 `0`(或算子维度不相容)→ `RX7018`。
- 合法性校验**先于任何 cublas 调用**(确定性诊断,纯 CPU 前置)。

**Dynamic Semantics**:

- 设备指针 / 维度校验通过后方调 cublas;`cublasStatus_t != SUCCESS` → `RX7019`(运行时失败,RXS-0128)。
- `CublasHandle` 为 **RAII** 守卫:Drop 调 `cublasDestroy`(句柄非空早查,错误吞掉,Drop 无 panic)。
- 列主序 / 转置约定见 RXS-0128(行主序 ↔ cublas 列主序适配)。

**Implementation Requirements**:

- safe wrapper 层**对上全 safe**(`create`/`gemm`/`gemv` 签名无 `unsafe`);unsafe 仅在 raw FFI 调用处(RXS-0126)。
- 错误码 `RX7016`~`RX7019` 含义冻结(07 §5),与 C ABI 层(RXS-0128)返回码语义一致。

> 锚定测试:`src/rurix-cublas`(`safe_wrapper_validates_before_cublas`:空指针 → RX7017、维度 0 → RX7018,先于任何 cublas 调用)。

### RXS-0128 cublas 高层 GEMM/GEMV API(row-major ↔ col-major 适配)

**Syntax**(高层 API + C ABI 导出):

```
CAbiExport ::= "extern" "\"C\"" "fn" "rurix_cublas_" ("gemm" | "gemv") "(" DevPtrArgs "," DimArgs ")" "->" "i32"
```

**Legality**:

- 设备指针 / 维度合法性同 RXS-0127(违例 → `RX7017` / `RX7018`);句柄初始化失败 → `RX7016`;cublas 执行失败 → `RX7019`。
- 元素类型 M8.2 规范性收窄为 `f32`(`cublasSgemm` / `cublasSgemv`);其余元素类型为加性后续。

**Dynamic Semantics**:

- 复用 [`rurix-rt` `Context::from_primary`] 取与 PyTorch **共享的 primary context** + [`Context::from_device_ptr`] **借用**外部设备指针缓冲(**affine 借用**,Drop **不**释放,所有权留外部 deleter;不悬垂 / 不双重释放,对齐 RXS-0124 / `rurix-rt` U10),零拷贝直接喂入 cublas。
- **行主序 ↔ cublas 列主序适配**(cublas 为列主序):
  - GEMM `C[M,N]=A[M,K]·B[K,N]`(行主序):行主序 `C(M×N)` 在内存中 ≡ 列主序 `C^T(N×M)`,经参数交换 `cublasSgemm(OP_N, OP_N, N, M, K, B, N, A, K, C, N)` 直接产行主序结果(不做显式转置 kernel)。
  - GEMV `y[M]=A[M,N]·x[N]`(行主序):行主序 `A(M×N)` ≡ 列主序 `A_cm(N×M)`,经 `cublasSgemv(OP_T, N, M, A, N, x, 1, y, 1)` 转置直接产行主序 `y=A·x`。
- cublas 调用后经 `ctx.synchronize()`(`cuCtxSynchronize`)阻塞至完成方返回。

**Implementation Requirements**:

- C ABI 入口接受设备指针(不透明 `u64` 地址)+ 维度按值,返回 `i32` 错误码(`0` = 成功;cublas 诊断段位 `RX7016`~`RX7019`,07 §5);返回码语义与 safe wrapper(RXS-0127)**一致**。
- 数值语义:与手写 CUDA C++ 对照(`bench/cuda_ref`)数值一致(冒烟 `ci/cublas_binding_smoke.py` 与 `torch.matmul` / `torch.mv` 对照,容差内);性能 ≥ 手写 CUDA C++ 90%(01 §6 UC-01 判据,`m8.ratio.cublas_*_vs_cuda`)。

> 锚定测试:`src/rurix-cublas`(`ffi_thin_wrapper_codes_consistent`:C ABI 薄包返回码与 safe API 一致,段位常量 RX7016~7019 冻结)。

### RXS-0129 cublas runtime DLL 按需附带与 Attachment A 白名单约定

**Legality**:

- cublas runtime DLL 按需附带仅限 **Attachment A 白名单最小集**(`cublas64_*.dll` / `cublasLt64_*.dll`,运行期库);完整 CUDA Toolkit / 驱动 / Nsight / 静态库 / headers / libdevice **永不捆绑**(许可红线 r6)。触及即停下标注「需升档」(§4)。
- NVIDIA EULA 法律签署维持 `pending-human-review`(**agent 自主签署**,对齐 M5 redistribution_audit 先例)。

**Dynamic Semantics**:

- M8.2 期**链接系统 DLL**(开发机 / runner 已安装 CUDA Toolkit 的 `cublas64_*.dll`,经动态加载,RXS-0126)+ `check_redistribution` 审计留痕;**物理捆绑 / 再分发**承接 M8.4 发布链路(rurixup/MSI/winget 分离打包)。
- 加载成功的 runtime DLL 名经 `loaded_dll` 内省接口暴露(审计留痕,机器事实)。

**Implementation Requirements**:

- 候选 DLL 名集仅 Attachment A 白名单形态;`check_redistribution`(M5.4 已激活)延续审计(cublas runtime DLL ∈ Attachment A 白名单,无禁止组件)。

> 锚定测试:`src/rurix-cublas`(`runtime_dll_attachment_a_whitelist`:`loaded_dll` 审计内省 + 候选集不含驱动 / Nsight / 静态库 / libdevice 禁止组件)。

## 3. 错误码引用汇总

> 本表**引用**cublas 绑定诊断错误码(07 §5 7xxx 链接 / 工具链段位续接,接 M8.1 互操作 RX7013~RX7015 之后,M8.2 首批分配 RX7016~RX7019,只追加、含义冻结),含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源;message-key 落 [../src/rurixc/src/messages/en.messages](../src/rurixc/src/messages/en.messages)。zh 双语全量覆盖属 M8.5 / RD-006。

| 错误码 | 含义 | message-key | 条款 |
|---|---|---|---|
| RX7016 | cublas 句柄初始化失败(cublas runtime DLL 不可用 / `cublasCreate` 失败 / 无 GPU 或共享 primary context) | `cublas.handle_init_failed` | RXS-0126 / RXS-0127 |
| RX7017 | cublas 设备指针非法(空指针 / 非设备地址 / 非本 context 设备内存) | `cublas.invalid_device_pointer` | RXS-0127 |
| RX7018 | cublas 维度不匹配(维度为 0 / GEMM·GEMV 算子维度不相容) | `cublas.dimension_mismatch` | RXS-0127 |
| RX7019 | cublas 执行运行时失败(`cublasStatus_t != SUCCESS` / context 同步失败) | `cublas.runtime_failed` | RXS-0128 |

## 4. 升档 / 禁区留痕

- **cublas FFI 边界 unsafe 策略(新决策面,档位 Mini)**:cublas 绑定 FFI 不可避免触 unsafe;口径已由 M8_CONTRACT §5 guardrail 锁定——FFI 边界 crate 经裁决最小开 unsafe + 每 unsafe 块 `// SAFETY:` + `unsafe-audit/` 注册条目(AGENTS 硬规则 9),safe wrapper 层对上全 safe,其余新 crate 默认 `unsafe_code=deny`。**agent 自主判档**,该决策面带档位标记 Mini 落笔,判档争议向上取严。
- **NVIDIA 再分发白名单(许可红线 r6)**:cublas runtime DLL 按需附带须经 Attachment A 白名单最小集审计(`check_redistribution` 延续,M5.4 已激活);完整 Toolkit / 驱动 / Nsight **永不捆绑**;EULA 法律签署维持 pending-human-review,**agent 自主签署**(对齐 M5 redistribution_audit 先例)。触及完整 Toolkit / 驱动 / Nsight 捆绑即停下标注「需升档」。
- **Python 原生嵌入(永久红线 1,SG-008)**:cublas 绑定仅保留 **C ABI / 不透明句柄通道**;Python 解释器宿主 / 原生嵌入为死亡路线红线,**永不实现**(SG-008 维持 not_triggered)。触及即停下标注「需升档」。
- **cubin/fatbin 真分发(G1,PTX-only)**:M8 维持 **PTX-only** 开发期产物(07 §7);cublas 为运行期库绑定(经 DLL),不改 device codegen 分发形态;cubin/fatbin 真分发 → G1(M8 out_of_scope)。
- **const 泛型值运行期单态化(RD-007)**:cublas 绑定作用面若触发数组长度类 const 泛型运行期单态化——**非 M8 验收门**(M8_CONTRACT out_of_scope / §6,inherited);本文件**不实现 RD-007**,亦不改 [consteval.md](consteval.md) RXS-0064 语义。遇硬需求**停下标注「需升档」**,按 14 §4 处置。
- **device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 agent 自主落笔的高敏面)**:不在本文件 cublas 绑定语义面登记;触及即停下标注「需升档」。
- **UB 节禁区**:cublas 绑定边界的设备指针生命周期 / 所有权语义以 **affine 所有权 + 确定性诊断(RX 错误码)** 定义,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-16 | 新建 spec/cublas.md(M8.2 cublas 绑定语义面起始文件):登记编号区间 RXS-0126 起续号预留 + 文件级前言 / 范围(cublas raw FFI 边界 / safe wrapper / 高层 GEMM·GEMV API / runtime DLL 按需附带与 Attachment A 白名单约定,C ABI·不透明句柄通道、永不 Python 原生嵌入、PTX-only、affine 所有权不设 UB、完整 Toolkit/驱动/Nsight 永不捆绑)/ 依据与授权(09 + 05 §FFI + 01 §6 + 07 §7 + 11 §3 M8 + 许可红线 r6;M8_CONTRACT D-M8-2 / G-M8-2 / G-M8-7 `rfc_required: none` + M8_PLAN §2)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0126 raw FFI 边界 / RXS-0127 safe wrapper / RXS-0128 高层 GEMM·GEMV API / RXS-0129 runtime DLL Attachment A 白名单)/ 错误码新段位预留说明(§3:cublas 诊断续接 7xxx RX7016 起随实现 PR 分配,脚手架不预造)/ 升档·禁区留痕(§4:cublas FFI unsafe 策略带档位标记 Mini、NVIDIA 再分发白名单 r6、Python 原生嵌入红线 1/SG-008、PTX-only/G1、RD-007、D-406/RD-008、UB 节禁区)。**沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 M8.2 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
| v1.1 | 2026-06-16 | 落地带编号条款体 RXS-0126 ~ RXS-0129(M8.2 实现 PR,条款体随实现 + 测试锚定同落):RXS-0126 cublas raw FFI 边界(cublas v2 C API `extern "C"` 声明面 + `cublasHandle_t` 不透明句柄 + runtime DLL 动态加载对齐 rurix-rt nvcuda 先例,候选限 Attachment A 白名单形态;FFI unsafe 边界档位 Mini)/ RXS-0127 cublas safe wrapper(`CublasHandle` RAII + 设备指针 / 维度合法性先于任何 cublas 调用,空指针→RX7017、维度 0→RX7018、句柄失败→RX7016、执行失败→RX7019;对上全 safe)/ RXS-0128 cublas 高层 GEMM/GEMV API(复用 rurix-rt 共享 primary context + 借用外部设备指针缓冲零拷贝,行主序 ↔ cublas 列主序适配:GEMM 参数交换、GEMV `CUBLAS_OP_T`;`ctx.synchronize` 后返回;C ABI i32 返回码与 safe API 同义;数值 ≥ 手写 CUDA C++ 90% UC-01 判据)/ RXS-0129 cublas runtime DLL 按需附带与 Attachment A 白名单约定(M8.2 链接系统 DLL + check_redistribution 审计留痕,物理捆绑 / 再分发承接 M8.4;完整 Toolkit/驱动/Nsight 永不捆绑,r6;EULA pending-human-review,agent 自主签署)。每条 ≥1 锚定(`src/rurix-cublas` crate 单测:DLL 候选 Attachment A 形态 / 设备指针 + 维度先于 cublas / C ABI 薄包返回码一致 / loaded_dll 审计内省;trace_matrix 维持全锚定)。§1 编号区间更新为 RXS-0126 ~ RXS-0129;§2 计划骨架升格为条款体;§3 错误码新段位首批分配 RX7016~RX7019 落 registry/error_codes.json(v1.21)+ en.messages(7xxx 续接、含义冻结,07 §5)。实现裁决:rurix-cublas FFI 边界 crate unsafe_code 经裁决豁免 + unsafe-audit 注册(对齐 rurix-rt / rurix-interop);PTX-only、不触 const 泛型(RD-007)、不触 device 原子(D-406/RD-008)、永不 Python 原生嵌入(红线 1/SG-008)、完整 Toolkit 永不捆绑(r6)。新决策面 cublas FFI 边界 unsafe 策略档位 **Mini**(口径 M8_CONTRACT §5 锁定),agent 自主判档,判档争议向上取严。授权:09 + 05 §FFI + 01 §6 + 07 §7 + 11 §3 M8,M8_CONTRACT D-M8-2 / G-M8-2 / G-M8-7 `rfc_required: none` | Direct |
