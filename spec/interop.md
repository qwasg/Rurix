# Rurix 语言规范 — 互操作语义面(PYD 产出 / CAI v3 + DLPack 双协议零拷贝 / C ABI 边界;M8.1 起)

> 条款:RXS-0122 起续号预留(M8.1 互操作语义面:`rx build --emit=pyd` PYD 产出约定 / `__cuda_array_interface__` v3 + DLPack 双协议零拷贝零拷贝接入 / C ABI 边界)。体例见 [README.md](README.md)。
> 依据:09 §6(Python 互操作 D-307:`rx build --emit=pyd` 产 PYD,绑定层 nanobind + scikit-build-core,数据通道 `__cuda_array_interface__` v3 + DLPack 双协议零拷贝接入 PyTorch/CuPy——UC-01 的实现路径;Windows DLL 陷阱纪律;永不 Python 原生嵌入);02 §U1 / §4(UC-01 PyTorch 瓶颈算子替换:DLPack/`__cuda_array_interface__` 零拷贝、PYD 产出、性能 ≥ 手写 CUDA 90%、全程无裸指针);01 §6(MVP 成功判据:SAXPY/Reduction/GEMM kernel ≥ 手写 CUDA C++ 90%;克制声明——与 PyTorch 经 DLPack 零拷贝互操作,不是替代它);05 §(FFI 边界:复杂类型不透明句柄 + create/destroy/operate,Python 不经语言级绑定走 C ABI + nanobind);07 §7(device codegen 分发:M8 维持 PTX-only,cubin/fatbin 真分发 → G1);11 §3 M8(互操作、加固与 MVP 验收)。授权:[../milestones/m8/M8_CONTRACT.md](../milestones/m8/M8_CONTRACT.md)(`in_scope: uc01_pytorch_interop` / `spec_m8_clauses`,D-M8-1,G-M8-1 / G-M8-7,`rfc_required: none`)+ [../milestones/m8/M8_PLAN.md](../milestones/m8/M8_PLAN.md) §1 M8.1 第 1 项。
> 档位:**Direct**(条款体)。本文是对 01/02/05/09 已锁定决策(UC-01 PyTorch 算子替换 / PYD 产出 / nanobind + scikit-build-core / `__cuda_array_interface__` v3 + DLPack 双协议零拷贝 / C ABI 边界)的初版条款化、纯追加且尚无 stable 面;**agent 自主判档**,判档以 M8_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。本里程碑识别一处新决策面——**PYD/C ABI 边界 unsafe 策略**:带档位标记 **Mini**(对齐 `src/rurix-rt` 注册式 unsafe 豁免先例 + M8_CONTRACT §5 guardrail 已锁口径:FFI 边界 crate 经裁决最小开 unsafe + 每块 `// SAFETY:` + `unsafe-audit/` 注册,safe wrapper 层对上全 safe,其余新 crate `unsafe_code=deny`)。任何偏离已锁定决策、或触及 **Python 原生嵌入(红线 1,SG-008 永久红线,仅 C ABI/PYD 通道)** / **cubin/fatbin 真分发(G1,M8 维持 PTX-only)** / **const 泛型值运行期单态化(RD-007)** / **device 原子 lowering(D-406/RD-008 agent 自主落笔的高敏面)** 的条款,必须停下标注「需升档」,不在本文件自行落笔(10 §3,M8_CONTRACT §6 / out_of_scope)。**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5):互操作边界的指针生命周期 / 所有权语义以 affine 所有权 + 确定性诊断(RX 错误码)定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.15 toolchain.md / v1.20 stdlib.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0122 起)与每条 ≥1 测试锚定随 M8.1 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **互操作语义面**的语义条款(M8.1+,D-M8-1)。覆盖语义面:

- **`rx build --emit=pyd` PYD 产出约定**:`rx build` 新增 `--emit=pyd` 通道,经编译器 device codegen(PTX-only,07 §7)+ 运行时 + 绑定层(nanobind + scikit-build-core,09 §6)产出 Python 扩展模块(`.pyd`);产物形态 / 入口符号 / 模块装载约定。
- **`__cuda_array_interface__` v3 协议**:Rurix 显存缓冲对 PyTorch/CuPy 暴露 / 消费 `__cuda_array_interface__` v3(device 指针 `data`、`typestr`、`shape`、`strides`、`version`、`stream` 字段语义与版本协商),零拷贝接入既有 CUDA 张量。
- **DLPack 双协议零拷贝**:`__dlpack__` / `from_dlpack`(DLPack capsule 生产 / 消费),capsule 消费一次性语义 + 设备指针生命周期 / 所有权(affine 所有权,deleter 释放责任,跨框架共享不悬垂 / 不双重释放)。
- **C ABI 边界**:Rurix↔C↔nanobind 的 C ABI 导出约定(不透明句柄 + create/destroy/operate,05 §FFI);FFI 边界 unsafe 最小化 + safe wrapper 层对上全 safe(unsafe 策略见前言档位标记 Mini)。

全部互操作产物以 **C ABI / PYD 通道**为唯一对接面(Python **不经语言级绑定**,05 §FFI);**永不 Python 原生嵌入 / 解释器宿主**(死亡路线红线 1,SG-008 维持 not_triggered,见 §5)。device 分发维持 **PTX-only**(07 §7;cubin/fatbin 真分发 → G1,M8 out_of_scope);设备指针所有权 / 生命周期以 **affine 所有权 + 确定性诊断**定义,**不以 UB 表述**(§5)。

**编号区间**:本文件条款自 **RXS-0122** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0121 @ [softraster.md](softraster.md))。本轮落地 **RXS-0122 ~ RXS-0125**(`rx build --emit=pyd` PYD 产出约定 / `__cuda_array_interface__` v3 / DLPack 双协议零拷贝 + 设备指针所有权 / C ABI 边界),每条 ≥1 测试锚定(`//@ spec: RXS-####`,`src/rurix-interop` crate 单测)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。Legality 违例只**引用**错误码(§3 引用汇总),不在此定义其含义。互操作边界的设备指针生命周期 / 所有权语义以 **affine 所有权 + 确定性诊断(RX 错误码)** 定义,不以 UB 表述。

### RXS-0122 `rx build --emit=pyd` PYD 产出约定

**Syntax**(CLI 形态):

```
PydEmit ::= "rx" "build" "--emit=pyd" <entry> ["-o" <out_dir>]
```

**Legality**:

- `<entry>` 须含 ≥1 个 device `kernel fn`(作为零拷贝算子源);无 `kernel fn` → `RX7013`(互操作协议不支持:无可导出零拷贝算子)。
- `--emit` 目标须为已识别集合 `{check, mir, llvm-ir, nvptx-ir, ptx, pyd}`;未识别目标 → 工具链诊断(不静默落入 host EXE 路径)。
- `--emit=pyd` 以 `kernel fn` 为根,**不要求 host `main`**(对齐 device emit 通道 RXS-0070)。

**Dynamic Semantics**:

- 编译器侧把 `<entry>` 的 device `kernel fn` 全管线产 **PTX**(device codegen + `ptxas` 干验证,**PTX-only** 07 §7;cubin/fatbin → G1),写入 staging PTX 供打包消费。
- 打包侧经 **nanobind + scikit-build-core**(09 §6)产 Python 扩展模块(`.pyd`),链接 `rurix-interop` 运行时(C ABI,RXS-0125;复用 M5 自研 kernel 嵌入 PTX),导出 UC-01 算子替换接口(SAXPY/Reduction/GEMM)与内省 `operators()` / `protocols()`。
- 产物 `.pyd` 模块名稳定(Python 扩展模块按文件名导入,`PyInit_<module>`);拷贝至 `<out_dir>` 保留 ABI 标记名。

**Implementation Requirements**:

- 绑定层**规范性**为 nanobind + scikit-build-core(09 §6:相对 PyO3/maturin 的 C ABI 扩展编译速度 / 二进制体积 / 开销优势;Rurix 非 Rust 生态);device 分发维持 PTX-only。
- 编译(rurixc `--emit=pyd`)与打包(`rx build` 编排 cargo staticlib + scikit-build-core)分层;PYD 运行期经 `rurix-interop` safe wrapper(对上全 safe,RXS-0125)。

> 锚定测试:`src/rurix-interop`(`#[cfg(test)] pyd_project_template_present`:PYD 工程模板 pyproject.toml/CMakeLists.txt/binding.cpp + 算子内省存在)。

### RXS-0123 `__cuda_array_interface__` v3 零拷贝消费

**Syntax**(CAI v3 字段,消费 PyTorch/CuPy CUDA 张量):

```
CaiV3 ::= "{" "version" ":" 3 "," "data" ":" "(" DevPtr "," ReadOnly ")"
              "," "typestr" ":" Str "," "shape" ":" Tuple ["," "strides" ":" (Tuple | null)]
              ["," "stream" ":" Int] "}"
```

**Legality**:

- `data[0]`(设备指针)须为非空、本 `Context` 设备上有效、可读写、容纳算子所需元素数的设备地址;空指针 / 非设备地址 → `RX7014`。
- `shape` 维度须与算子契约相容且各维 > 0;维度为 0 / 算子维度不相容 → `RX7015`。
- 元素类型 M8.1 规范性收窄为 `f32`(`typestr` `<f4`);其余元素类型为加性后续。
- 合法性校验**先于任何 GPU 调用**(确定性诊断,纯 CPU 前置)。

**Dynamic Semantics**:

- 消费 `data` 设备指针**零拷贝**借用外部张量显存(不重分配、不主机往返),在与 PyTorch **共享的 device primary context** 内 launch 复用的 M5 kernel,结果写回 `out` 张量显存。
- 借用缓冲在算子调用期内有效;不取得所有权(所有权语义见 RXS-0124)。

**Implementation Requirements**:

- 设备指针经 `rurix-rt` 借用缓冲([`Context::from_device_ptr`])在共享 primary context([`Context::from_primary`])内消费;`data`/`shape` 合法性在 FFI 边界(`rurix-interop`)先于 GPU 校验,违例返回 RX7014/RX7015。

> 锚定测试:`src/rurix-interop`(`null_device_ptr_rejected` → RX7014;`zero_dim_rejected` → RX7015,先于 GPU)。

### RXS-0124 DLPack 双协议零拷贝与设备指针所有权

**Syntax**(DLPack capsule 生产 / 消费):

```
DlpackConsume ::= "from_dlpack" "(" Obj ")"      // Obj 实现 __dlpack__ → DLManagedTensor capsule
DlpackProduce ::= Obj "." "__dlpack__" "(" ")"   // 产 DLPack capsule(零拷贝)
```

**Legality**:

- 经 DLPack capsule 取得的设备指针须满足与 RXS-0123 同一设备指针合法性(非空 / 设备有效 / 容量足);违例 → `RX7014` / `RX7015`。
- DLPack 与 `__cuda_array_interface__` v3 为**双协议**:同一算子接口须能经任一协议零拷贝消费同一 PyTorch CUDA 张量,数值结果一致。

**Dynamic Semantics**:

- DLPack capsule 消费为**一次性**语义(消费后 capsule 标记 used);设备内存**所有权留在外部框架(PyTorch)deleter**——借用缓冲为 **affine 借用**,其 Drop **不释放**外部显存(不悬垂、不双重释放;`rurix-rt` 借用缓冲 owned=false)。
- 零拷贝路径数值语义与 RXS-0123(CAI v3)路径**同义**(同一 M5 kernel、同一 primary context)。

**Implementation Requirements**:

- 借用外部设备指针经 `rurix-rt` 借用缓冲(Drop 不 `cuMemFree`);所有 unsafe 借用块携 `// SAFETY:` + `unsafe-audit/` 注册(RXS-0125;M8_CONTRACT §5)。上层 nanobind `nb::ndarray<device::cuda>` 经 DLPack 导入抽取 `.data()` 设备指针,对上全 safe。

> 锚定测试:`src/rurix-interop`(`null_device_ptr_rejected`:双协议设备指针合法性前置 → RX7014;借用缓冲所有权 = 外部 deleter,Drop 不释放)。

### RXS-0125 C ABI 边界

**Syntax**(C ABI 导出,Windows x64 唯一 ABI):

```
CAbiExport ::= "extern" "\"C\"" "fn" "rurix_uc01_" <op> "(" DevPtrArgs "," DimArgs ")" "->" "i32"
```

**Legality**:

- 互操作经 **C ABI / PYD 通道**对接(Python **不经语言级绑定**,05 §FFI);**永不 Python 原生嵌入 / 解释器宿主**(红线 1,SG-008,§4)。
- C ABI 入口接受设备指针(不透明 `u64` 地址)+ 维度按值,返回 `i32` 错误码:`0` = 成功;互操作诊断段位 `RX7013`/`RX7014`/`RX7015`(07 §5,只追加、含义冻结);负 = 运行时/驱动失败。

**Dynamic Semantics**:

- C ABI 边界为 FFI **unsafe 边界**(经裁决最小开 unsafe,档位 Mini);其上 `rurix-interop` safe wrapper 与 `rurix-rt` safe 运行时层**对上全 safe**(签名无 `unsafe`);unsafe 仅在借用外部设备指针处(每块 `// SAFETY:` + `unsafe-audit/rurix-interop.md` / `rurix-rt.md` 注册)。
- 设备指针不在 C ABI 层解引用(仅前向 safe API);设备内存读写发生在 launch(`rurix-rt` U7 边界)。

**Implementation Requirements**:

- FFI 边界 crate(`rurix-interop`)`unsafe_code` 经裁决豁免 + `undocumented_unsafe_blocks = deny`;其余新 crate 默认 `unsafe_code = deny`(M8_CONTRACT §5 guardrail)。错误码 RX7013~RX7015 含义冻结(07 §5)。

> 锚定测试:`src/rurix-interop`(`ffi_thin_wrapper_codes_consistent`:C ABI 薄包返回码语义一致,段位常量 RX7013~7015 冻结)。

## 3. 错误码引用汇总

> 本表**引用**互操作诊断错误码(07 §5 7xxx 链接/工具链段位续接,M8.1 首批分配 RX7013~RX7015,只追加、含义冻结),含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源;message-key 落 [../src/rurixc/src/messages/en.messages](../src/rurixc/src/messages/en.messages)。zh 双语全量覆盖属 M8.5 / RD-006。

| 错误码 | 含义 | message-key | 条款 |
|---|---|---|---|
| RX7013 | 互操作协议不支持(对象未暴露 `__cuda_array_interface__` v3 / DLPack;或 `--emit=pyd` 输入无 `kernel fn`) | `interop.unsupported_protocol` | RXS-0122 / RXS-0125 |
| RX7014 | 互操作设备指针非法(空指针 / 非设备地址 / 非本 context 设备内存) | `interop.invalid_device_pointer` | RXS-0123 / RXS-0124 |
| RX7015 | 互操作形状不匹配(维度为 0 / 算子维度不相容) | `interop.shape_mismatch` | RXS-0123 |

## 4. 升档 / 禁区留痕

- **PYD/C ABI 边界 unsafe 策略(新决策面,档位 Mini)**:互操作(PYD / C ABI / DLPack 边界)FFI 不可避免触 unsafe;口径已由 M8_CONTRACT §5 guardrail 锁定——FFI 边界 crate 经裁决最小开 unsafe + 每 unsafe 块 `// SAFETY:` + `unsafe-audit/` 注册条目(AGENTS 硬规则 9),safe wrapper 层对上全 safe,其余新 crate 默认 `unsafe_code=deny`。**agent 自主判档**,该决策面带档位标记 Mini 落笔,判档争议向上取严。
- **Python 原生嵌入(永久红线 1,SG-008)**:仅保留 **C ABI / PYD 通道**;Python 解释器宿主 / 原生嵌入为死亡路线红线,**永不实现**(SG-008 维持 not_triggered)。触及即停下标注「需升档」。
- **cubin/fatbin 真分发(G1,PTX-only)**:M8 维持 **PTX-only** 开发期产物(07 §7);PYD 内嵌 device 代码沿用既有 PTX 装载协商(RXS-0076),cubin/fatbin 真分发 → G1(M8 out_of_scope)。
- **const 泛型值运行期单态化(RD-007)**:互操作 / 绑定作用面若触发数组长度类 const 泛型运行期单态化——**非 M8 验收门**(M8_CONTRACT out_of_scope / §6,inherited);本文件**不实现 RD-007**,亦不改 [consteval.md](consteval.md) RXS-0064 语义。遇硬需求**停下标注「需升档」**,按 14 §4 处置。
- **device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 agent 自主落笔的高敏面)**:不在本文件互操作语义面登记;触及即停下标注「需升档」。
- **UB 节禁区**:互操作边界的设备指针生命周期 / 所有权语义以 **affine 所有权 + 确定性诊断(RX 错误码)** 定义,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-16 | 新建 spec/interop.md(M8.1 互操作语义面起始文件):登记编号区间 RXS-0122 起续号预留 + 文件级前言 / 范围(`rx build --emit=pyd` PYD 产出约定 / `__cuda_array_interface__` v3 + DLPack 双协议零拷贝 / C ABI 边界,C ABI·PYD 唯一通道、永不 Python 原生嵌入、PTX-only、affine 所有权不设 UB)/ 依据与授权(09 §6 + 02 §U1 + 01 §6 + 05 §FFI + 07 §7 + 11 §3 M8;M8_CONTRACT D-M8-1 / G-M8-1 / G-M8-7 `rfc_required: none` + M8_PLAN §1)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0122 PYD 产出 / RXS-0123 CAI v3 / RXS-0124 DLPack 双协议 + 设备指针所有权 / RXS-0125 C ABI 边界)/ 错误码新段位预留说明(§3:互操作诊断续接 7xxx RX7013+ 随实现 PR 分配,脚手架不预造)/ 升档·禁区留痕(§4:PYD/C ABI unsafe 策略带档位标记 Mini、Python 原生嵌入红线 1/SG-008、PTX-only/G1、RD-007、D-406/RD-008、UB 节禁区)。**沿 README v1.15 toolchain.md / v1.20 stdlib.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 M8.1 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
| v1.1 | 2026-06-16 | 落地带编号条款体 RXS-0122 ~ RXS-0125(M8.1 实现 PR,条款体随实现 + 测试锚定同落):RXS-0122 `rx build --emit=pyd` PYD 产出约定(device kernel→PTX PTX-only + nanobind/scikit-build-core 打包链接 rurix-interop;无 kernel→RX7013)/ RXS-0123 `__cuda_array_interface__` v3 零拷贝消费(设备指针 + shape 合法性,空指针→RX7014、维度 0→RX7015,先于 GPU 校验)/ RXS-0124 DLPack 双协议零拷贝与设备指针所有权(capsule 一次性消费 + affine 借用,所有权留外部 deleter,借用缓冲 Drop 不释放,不悬垂/不双重释放;与 CAI v3 路径数值同义)/ RXS-0125 C ABI 边界(`extern "C"` 导出 + i32 错误码 RX7013~7015,FFI unsafe 边界经裁决最小开 unsafe 档位 Mini,safe wrapper 对上全 safe,永不 Python 原生嵌入)。每条 ≥1 锚定(`src/rurix-interop` crate 单测:pyd 工程模板存在 / RX7014·RX7015 先于 GPU 校验 / C ABI 薄包返回码一致;trace_matrix 维持全锚定)。§1 编号区间更新为 RXS-0122 ~ RXS-0125;§2 计划骨架升格为条款体;§3 错误码新段位首批分配 RX7013~RX7015 落 registry/error_codes.json(v1.20)+ en.messages(7xxx 续接、含义冻结,07 §5)。实现裁决:rurix-interop FFI 边界 crate unsafe_code 经裁决豁免 + unsafe-audit 注册(对齐 rurix-rt 先例),rurix-rt 增 primary context 共享([`Context::from_primary`])+ 借用外部设备指针缓冲([`Context::from_device_ptr`],Drop 不 free);PTX-only(cubin/fatbin→G1)、不触 const 泛型(RD-007)、不触 device 原子(D-406/RD-008)、永不 Python 原生嵌入(红线 1/SG-008)。新决策面 PYD/C ABI 边界 unsafe 策略档位 **Mini**(口径 M8_CONTRACT §5 锁定),agent 自主判档,判档争议向上取严。授权:09 §6 + 02 §U1 + 01 §6 + 05 §FFI + 07 §7 + 11 §3 M8,M8_CONTRACT D-M8-1 / G-M8-1 / G-M8-7 `rfc_required: none` | Direct |
