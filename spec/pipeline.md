# Rurix 语言规范 — UC-02 流水线类型化语义面(affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化;M8.3 起)

> 条款:RXS-0130 起续号预留(M8.3 UC-02 三 stream 重叠流水线类型化语义面:affine Context/Stream/Event/Buffer 所有权与销毁纪律 / Event 记录·等待与跨 stream 同步 / 流序分配类型化 / 跨线程所有权转移 / 资源生命周期错误类别与编译期拦截判据)。**复用既有 device/运行时条款(RXS-0066 着色 / RXS-0074 launch 类型契约 / RXS-0077 poisoned context 状态机),新增仅补缺口**。体例见 [README.md](README.md)。
> 依据:02 §U2(UC-02 三 stream 重叠流水线:H2D / compute / D2H 三 stream 重叠 + 资源生命周期错误类别 100% 编译期拦截);05 §1(device⊂host 安全子集:所有权 / 借用 / 生命周期规则在 device 路径同义延拓)+ 05 §(affine 资源所有权:GPU 资源 move-only、单一所有权、RAII 销毁纪律);06(GPU 执行模型:context / stream / event / 设备内存);08 §1/§2(`rurix-rt` 运行时对象:Context affine 根 / Stream / Buffer / 装载协商 / poisoned 状态机,D-230~D-234);01 §6(MVP 成功判据:UC-02 端到端 + 预设资源生命周期错误类别 100% 编译期拦截);07 §7(device codegen 分发:M8 维持 PTX-only)。授权:[../milestones/m8/M8_CONTRACT.md](../milestones/m8/M8_CONTRACT.md)(`in_scope: uc02_stream_pipeline` / `spec_m8_clauses`,D-M8-3,G-M8-3 / G-M8-7,`rfc_required: none`)+ [../milestones/m8/M8_PLAN.md](../milestones/m8/M8_PLAN.md) §3 M8.3 第 1 项。
> 档位:**Direct**(条款体)。本文是对 02/05/06/08 已锁定决策(UC-02 三 stream 重叠流水线 / affine 资源所有权 / context·stream·event·buffer 运行时对象 / 资源生命周期错误类别编译期拦截)的初版条款化、纯追加且尚无 stable 面;**AI 无权自判 Direct**,判档以 M8_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。本里程碑识别一处新决策面——**跨线程所有权转移的类型化机制(Send 化 primary-context 共享句柄 + 流序分配类型化)**:带档位标记 **Mini**(对齐 M8_CONTRACT §5 guardrail 已锁口径:`src/rurix-rt` 的 unsafe 边界维持 `undocumented_unsafe_blocks=deny`、FFI 凡落 unsafe 须每块 `// SAFETY:` + `unsafe-audit/` 注册;Event / 跨线程 `cuCtxSetCurrent` 新 FFI 落 `src/rurix-rt` 既有豁免 + 注册;safe 类型层对上全 safe,资源生命周期错误以 affine 类型 + rustc 原生诊断拦截)。任何偏离已锁定决策、或触及 **Python 原生嵌入(红线 1,SG-008 永久红线,仅 C ABI/PYD 通道)** / **cubin/fatbin 真分发(G1,M8 维持 PTX-only)** / **Tensor Core/WGMMA/TMA·cluster·动态并行·cooperative groups(11 §2 红线,SG-001/SG-002)** / **const 泛型值运行期单态化(RD-007)** / **device 原子 lowering(D-406/RD-008 人工落笔禁区)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔(10 §3,M8_CONTRACT §6 / out_of_scope)。**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5):跨 stream / 跨线程资源所有权与生命周期以 **affine 所有权 + 确定性诊断(rustc 编译期拦截)** 定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md / v1.27 cublas.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0130 起)与每条 ≥1 测试锚定随 M8.3 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **UC-02 流水线类型化语义面**的语义条款(M8.3+,D-M8-3)。覆盖语义面:

- **affine Context/Stream/Event/Buffer 所有权与销毁纪律**:GPU 资源(Context affine 根 / Stream / Event / DeviceBuffer / PinnedBuffer)为 **move-only**(非 `Copy` / 非 `Clone`)、单一所有权;生命周期 brand(`'ctx`)编码资源归属层级(跨 context 误用 / 逃逸为编译期借用错误,复用 RXS-0077 既有形态);RAII 确定性销毁序(stream / event 先于 context;owned buffer Drop 释放,borrowed buffer Drop 不释放,对齐 RXS-0124)。**Event 为本里程碑补缺的新运行时对象**(`cuEventCreate` / `cuEventRecord` / `cuEventDestroy`),其余复用 `rurix-rt` 既有对象。
- **Event 记录·等待与跨 stream 同步语义**:Event 在某 stream 上 `record`(`cuEventRecord`),另一 stream `wait`(`cuStreamWaitEvent`)以建立**跨 stream 流序依赖**——三 stream 重叠流水线(H2D stream record `evt_h2d` → compute stream wait `evt_h2d` → record `evt_compute` → D2H stream wait `evt_compute`)的同步骨架;不引入隐式同步 / 自动调度(P-05 薄层)。
- **流序分配类型化(stream-ordered allocation typing)**:缓冲句柄携 **stream brand**(产出 / 最后写入它的 stream);跨 stream 读取须经 `event.wait` 取得**重 brand** 后的句柄;**跳过同步即无重 brand 句柄 → 编译期类型错误**——使「跨 stream 未同步访问」成为 100% 编译期拦截的资源生命周期错误类别(而非运行期 UB / 数据竞争)。
- **跨线程所有权转移(Send 化 primary-context 共享句柄)**:`Context`(独占 / current context 线程绑定)维持 `!Send`;新增并行的 **shared primary-context 句柄**(`Send + Sync`,引用计数 retain/release),经其分配的 Buffer / Event 句柄为 **Send**,可经 affine `move` 跨线程转移(producer→compute→D2H 线程),worker 线程内 `cuCtxSetCurrent` 重绑 current context;单一所有权 + move 语义使 **use-after-free / double-free** 由 rustc 编译期排除,**跨线程非法转移**(送线程绑定的 `!Send` 守卫)为编译期 `Send` 约束错误。
- **资源生命周期错误类别与编译期拦截判据**:预设资源生命周期错误类别——**use-after-free**(move 后再用)/ **double-free**(重复 move·重复释放)/ **跨 stream 未同步访问**(缺 event 同步)/ **跨线程非法转移**(`!Send` 资源越界)——**100% 编译期拦截**(rustc affine move / 生命周期 brand / `Send`-`Sync` 约束原生诊断),MVP 验收判据(01 §6 / G-M8-3);reject 类别覆盖以 compile-fail 样例(应拦截却放行即红)核对。

全部 UC-02 流水线编排**复用 `src/rurix-rt` 运行时对象**(Context/Stream/DeviceBuffer/PinnedBuffer/Module/Kernel)+ 本里程碑补缺的 Event;device 分发维持 **PTX-only**(07 §7);跨 stream / 跨线程资源所有权 / 生命周期以 **affine 所有权 + 确定性诊断(rustc 编译期拦截)** 定义,**不以 UB 表述**(§4)。**资源生命周期错误类别由 Rust 类型系统原生拦截,本里程碑不新增 RX 错误码**(§3;rustc 编译期诊断而非 RX#### 段位码,「如需」按需——本轮判定为不需要)。

**编号区间**:本文件条款自 **RXS-0130** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0129 @ [cublas.md](cublas.md))。本轮计划落地 **RXS-0130 ~ RXS-0134**(affine Context/Stream/Event/Buffer 所有权与销毁纪律 / Event 记录·等待与跨 stream 同步 / 流序分配类型化 / 跨线程所有权转移 / 资源生命周期错误类别与编译期拦截判据),每条 ≥1 测试锚定(`//@ spec: RXS-####`,`src/rurix-rt` crate 单测)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款(计划骨架,随 M8.3 实现 PR 落地)

> 本脚手架 PR **不落带编号裸条款头**(`### RXS-####` 一经出现且无测试锚定即 `trace_matrix --check` 红)。以下为计划条款骨架(非裸条款头);条款体(Syntax / Legality / Dynamic Semantics / Implementation Requirements 按需分节,**严禁 UB 节**,10 §7.5)与每条 ≥1 测试锚定随 M8.3 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。Legality 违例由 rustc 原生编译期诊断拦截(§3),不引用新 RX 段位码。

- **RXS-0130 affine Context/Stream/Event/Buffer 所有权与 RAII 销毁纪律**(Event 补缺;复用 RXS-0066 着色 / RXS-0077 poisoned 状态机;move-only·单一所有权·生命周期 brand `'ctx`·确定性销毁序)。
- **RXS-0131 Event 记录·等待与跨 stream 同步语义**(`cuEventCreate` / `cuEventRecord` / `cuEventDestroy` / `cuStreamWaitEvent`;三 stream 重叠流序依赖骨架;薄层无隐式同步)。
- **RXS-0132 流序分配类型化(stream-ordered allocation typing)**(缓冲 stream brand + `event.wait` 重 brand;跨 stream 未同步访问 = 编译期类型错误)。
- **RXS-0133 跨线程所有权转移(Send 化 primary-context 共享句柄)**(shared context `Send + Sync` + Buffer/Event `Send`;线程绑定守卫 `!Send`;affine `move` 跨线程转移语义;worker `cuCtxSetCurrent` 重绑)。
- **RXS-0134 资源生命周期错误类别与编译期拦截判据**(use-after-free / double-free / 跨 stream 未同步 / 跨线程非法转移 四类 → rustc 类型级 100% 编译期拦截契约 + reject 样例锚定;MVP 验收判据 01 §6 / G-M8-3)。

## 3. 错误码引用汇总

> **本里程碑不新增 RX 错误码**(M8_PLAN §3 / 提示词「资源生命周期诊断错误码(如需)」——本轮判定为不需要)。UC-02 预设资源生命周期错误类别(use-after-free / double-free / 跨 stream 未同步 / 跨线程非法转移)由 **Rust 类型系统原生编译期拦截**(affine move → `E0382`、`Send` 约束 → `E0277`、生命周期 brand → 借用 / 生命周期错误、无 `Clone` → `E0599` 等 rustc 诊断),**而非 RX#### 段位码**;`registry/error_codes.json` 与 `src/rurixc/src/messages/en.messages` **本里程碑不动**(零追加)。
>
> 运行期失败(Event 创建 / 记录 / 等待的驱动错误)沿用 `rurix-rt` 既有 `CudaError` 错误值面 + poisoned 状态机(RXS-0077),不新增 RX 段位码。
>
> 若实现期发现某资源生命周期类别**无法以纯 Rust 类型拦截**而确需编译器侧 RX 诊断 / 运行期段位码,则**停手标注「需人工升档」**(§4),不在本文件自行预造错误码。

## 4. 升档 / 禁区留痕

- **跨线程所有权转移的类型化机制(新决策面,档位 Mini)**:Send 化 primary-context 共享句柄 + 流序分配类型化为本里程碑新决策面;口径已由 M8_CONTRACT §5 guardrail 锁定——`src/rurix-rt` 的 unsafe 边界维持 `undocumented_unsafe_blocks=deny`,Event / 跨线程 `cuCtxSetCurrent` 新 FFI 落 `src/rurix-rt` 既有豁免 + 每 unsafe 块 `// SAFETY:` + `unsafe-audit/rurix-rt.md` 注册条目(AGENTS 硬规则 9),safe 类型层对上全 safe(`uc02-demo` 默认 `unsafe_code=deny`)。**AI 不自判 Direct**,该决策面带档位标记 Mini 落笔,判档争议向上取严。
- **资源生命周期错误类别 100% 编译期拦截(MVP 验收判据)**:use-after-free / double-free / 跨 stream 未同步 / 跨线程非法转移四类以 **affine 类型 + rustc 原生编译期诊断**拦截(§3);reject 类别覆盖以 compile-fail 样例核对(应拦截却放行即红,反 YAML-only,run URL 归档)。**不以 UB 表述**(10 §7.5)。
- **Python 原生嵌入(永久红线 1,SG-008)**:UC-02 流水线仅保留 **C ABI / 自研 kernel 通道**;Python 解释器宿主 / 原生嵌入为死亡路线红线,**永不实现**(SG-008 维持 not_triggered)。触及即停下标注「需人工升档」。
- **cubin/fatbin 真分发(G1,PTX-only)**:M8 维持 **PTX-only** 开发期产物(07 §7);UC-02 复用 `rurix-rt` PTX 装载路径,不改 device codegen 分发形态;cubin/fatbin 真分发 → G1(M8 out_of_scope)。
- **高级 GPU intrinsics(11 §2 红线,SG-001/SG-002)**:UC-02 三 stream 重叠为**经典 stream/event 并发**编排,不触 Tensor Core / WGMMA / TMA / cluster / 动态并行 / cooperative groups;触及即停下标注「需人工升档」。
- **const 泛型值运行期单态化(RD-007)**:UC-02 流水线作用面若触发数组长度类 const 泛型运行期单态化——**非 M8 验收门**(M8_CONTRACT out_of_scope / §6,inherited);本文件**不实现 RD-007**,亦不改 [consteval.md](consteval.md) RXS-0064 语义。遇硬需求**停下标注「需人工升档」**,按 14 §4 处置。
- **device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 人工落笔禁区)**:UC-02 跨 stream 同步以 host 侧 event(`cuEventRecord` / `cuStreamWaitEvent`)表达,不触 device 原子 lowering;触及即停下标注「需人工升档」。
- **UB 节禁区**:跨 stream / 跨线程资源所有权 / 生命周期以 **affine 所有权 + 确定性诊断(rustc 编译期拦截)** 定义,**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-17 | 新建 spec/pipeline.md(M8.3 UC-02 流水线类型化语义面起始文件):登记编号区间 RXS-0130 起续号预留 + 文件级前言 / 范围(affine Context/Stream/Event/Buffer 所有权与销毁纪律 / Event 记录·等待与跨 stream 同步 / 流序分配类型化 / 跨线程所有权转移 / 资源生命周期错误类别与编译期拦截判据;**复用既有 RXS-0066/0074/0077 device·运行时条款,新增仅补缺口**;复用 src/rurix-rt 运行时对象、永不 Python 原生嵌入、PTX-only、affine 所有权不设 UB、不触高级 GPU intrinsics)/ 依据与授权(02 §U2 + 05 §1/§FFI + 06 + 08 §1/§2 + 01 §6 + 07 §7 M8;M8_CONTRACT D-M8-3 / G-M8-3 / G-M8-7 `rfc_required: none` + M8_PLAN §3)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0130 affine 所有权与销毁纪律 / RXS-0131 Event 记录·等待与跨 stream 同步 / RXS-0132 流序分配类型化 / RXS-0133 跨线程所有权转移 / RXS-0134 资源生命周期错误类别与编译期拦截判据)/ 错误码说明(§3:**本里程碑不新增 RX 码**——资源生命周期类别由 Rust 类型系统原生编译期拦截 rustc 诊断,registry/error_codes.json 与 en.messages 零追加;确需编译器侧 RX 诊断则停手升档)/ 升档·禁区留痕(§4:跨线程所有权转移类型化机制带档位标记 Mini、资源生命周期 100% 编译期拦截 MVP 判据、Python 原生嵌入红线 1/SG-008、PTX-only/G1、高级 GPU intrinsics SG-001/002、RD-007、D-406/RD-008、UB 节禁区)。**沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md / v1.27 cublas.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 M8.3 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
