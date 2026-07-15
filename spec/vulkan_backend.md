# Rurix 语言规范 — Vulkan/SPIR-V 跨端后端语义面（MIR → SPIR-V，承 RFC-0011；mb1 起）

> 条款:**RXS-0200 起续号**(mb1 Vulkan/SPIR-V 跨端第三后端语义面:codegen target 分发与 Vulkan 后端分叉 / MIR→SPIR-V compute 编码(执行模型·LocalSize·compute builtins·存储缓冲·控制流) / MIR→SPIR-V graphics 编码 / 数学 intrinsic → GLSL.std.450 映射 / 运行时 Backend trait 抽象 / Vulkan compute 运行时 / launch marshalling / artifact 泛化 / graphics+present / Android 移植缝 / toolchain 定位 / 绑定供应链)。区间 **RXS-0200 ~ RXS-0213** 随条款数定(见 [README.md](README.md) §4);跳号 RXS-0189~0199(MS1.2/MS1.2b 承接,feat/ms1.2b 在途)避撞维持(编号永不复用 10 §9.5)。体例见 [README.md](README.md)。
> 依据:**[RFC-0011](../rfcs/0011-vulkan-spirv-backend.md)**(Vulkan/SPIR-V 跨端第三后端,**Draft** 2026-07-15——**依赖 owner 裁决红线 3(D-008/SG-003)解除 + RFC 批准,未获裁决前本文件不合入 main**)+ RFC-0003/RFC-0004(第二后端 DXIL/SPIR-V 图形编码器先例,本后端抽取泛化其 `dxil_spirv.rs` 种子)+ RFC-0005(绑定布局推导,descriptor/set/binding 复用)+ RFC-0009(rxrt C ABI / launch marshalling 复用)+ RFC-0002(着色阶段类型面 RXS-0153~0156 前端复用);06 §8(codegen 第二后端设计预留);07 §7(device codegen 分发);11 §5(多后端解禁评估——红线 3 正式重审)。授权:[../milestones/mb1/MB1_CONTRACT.md](../milestones/mb1/MB1_CONTRACT.md)(D-MB1-*,G-MB1-*)+ [../milestones/mb1/MB1_PLAN.md](../milestones/mb1/MB1_PLAN.md) mb1 分期。
> 档位:**Full RFC**(RFC-0011;10 §3:本设计触 **codegen 第三后端 + target 分发 + 新运行时后端 Backend trait + FFI ABI descriptor-binding marshalling + 死亡路线红线 3 多后端**——四者并触,判档争议向上取严 硬规则 8)。任何触及 **🔒 launch marshalling FFI ABI 二进制布局(RFC-0011 §4.7)** / **Backend trait FFI 边界(§4.5)** / **dlopen 跨 OS 加载缝(§4.10)** / **纹理路径内存模型映射(06 §4.2)** 的条款,必须停下标注「需升档」,不在本文件自行落笔越禁区。**严禁 UB 节**(10 §7.5):目标不支持 / compute 子集外构造 / 数学 intrinsic 超集 / 降级失败以 **编译期 6xxx codegen 诊断(P-01 strict-only,无运行期 fallback)**定义,不以 UB 表述;运行期 Vulkan 失败走 cabi 确定性诊断 + 终止(不占 RX 码,工具层口径,对齐 spec/release.md §3)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架沿 README v1.0 dxil_backend.md / v1.51 edition.md 脚手架先例:仅登记新文件名 + 预留区间 RXS-0200~0213,不落带编号裸条款头**——条款体(RXS-0200 起)与每条 ≥1 测试锚定随 mb1 各 Phase 实现 PR(Phase 1~4,RFC-0011 批准 + 红线 3 解除后)同落。**本文件零 `### RXS-####` 条款头**,`ci/trace_matrix.py --check` 维持全锚定不变(无新增裸条款头、无悬空锚点、零新 RXS)。

---

## 0. 治理闸口(读在最前)

本文件为 **mb1 多后端新纪元 governance package 草案**的组成。mb1 方向 = Vulkan/SPIR-V 跨端后端,**正面触死亡路线红线 3**(多后端 AMD/Intel/Metal/Vulkan/SPIR-V;D-008/SG-003)。红线 3 解除是 owner 主动决策(10 §9.2),SG-003 现存记录(最近 2026-07-14)判定其前提『NVIDIA 单栈纵深完成』**未达**。因此:

- 本文件(及 RFC-0011、milestones/mb1 四件套、错误码、conformance、CI 步骤)**gated on** owner 裁决:① D-008 红线 3 解除(独立 errata PR)② SG-003 → triggered(RFC-0011)③ RFC-0011 批准。三者未获裁决前,**本文件不合入 main**。
- agent 起草并把待裁摊清,**不自签、不自翻**(见 [../milestones/mb1/OWNER_DECISION_PACKAGE.md](../milestones/mb1/OWNER_DECISION_PACKAGE.md))。

## 1. 范围与编号区间

本文件承载 **MIR → SPIR-V 跨端第三后端语义面**的语义条款(mb1)。承 RFC-0002 着色阶段类型面(RXS-0153~0156)、RFC-0003/0004 第二后端并列降级与 SPIR-V 图形编码器种子、RFC-0005 绑定布局推导、RFC-0009 rxrt C ABI,定义 Rurix 经 **单一 Vulkan/SPIR-V 后端**同覆盖 **AMD 桌面 + Android**、**compute + graphics(vertex/fragment)** 的语义。覆盖语义面(RFC-0011 §4):

- **codegen target 分发与 Vulkan 后端分叉**:MIR 之后 target 选择(`rx build --target vulkan`,与 `--target ptx`/`--target dxil` 并列),各后端独立降级、不共享 lowering(RFC-0003 §4.5 口径);gate `cargo feature vulkan-backend`(未启用 → 目标不可用 6xxx,PTX/DXIL 路径零漂移);无隐式多目标、无静默 fallback(P-01 strict-only)。
- **MIR→SPIR-V compute 编码**:`GLCompute` 执行模型 + `LocalSize` 执行模式 + compute builtins(`DeviceIntrinsic` → `GlobalInvocationId`/`LocalInvocationId`/`WorkgroupId`/`NumWorkgroups` + `Barrier`→`OpControlBarrier`)+ 存储/描述符缓冲(`StructuredBuffer`/`ConstantBuffer` → `OpTypeStruct`/`RuntimeArray`/`Block`/`Offset`/`ArrayStride`/`StorageBuffer` + `OpAccessChain`)+ `shared`→`Workgroup` 存储类 + 结构化控制流子集。抽取泛化 `dxil_spirv.rs` 既有 vertex/fragment 编码器骨架;compute 为新增主体。
- **MIR→SPIR-V graphics 编码**:vertex/fragment 复用 `dxil_spirv.rs` 编码器(execution model / Location·BuiltIn decoration / 采样链),面向 Vulkan 原生消费(`.spv` 直喂 `vkCreateShaderModule`,去 B 路 SPIRV-Cross→HLSL→dxc 转译链)。
- **数学 intrinsic → GLSL.std.450 ext-inst 映射**:`CallTarget::Libdevice{__nv_*}`(20 `DeviceMathFn`)→ `OpExtInst "GLSL.std.450"`;非 1:1(Cbrt/Log10)组合表达;超集 → 6xxx。
- **运行时 Backend trait 抽象**:rurix-rt 引入 `Backend`/`GpuDevice` trait(提升 `impl Cuda` 方法集),CUDA 收敛为首实现(NVIDIA 零回归),Vulkan 为并列实现;backend 选择器替 static CUDA,无隐式 fallback。
- **Vulkan compute 运行时**:instance/device/queue → shader module → compute pipeline + layout → descriptor set/pool → command buffer → dispatch + fence;内存 upload/download。VK_LAYER_KHRONOS_validation 零报错。
- **launch marshalling**:descriptor-binding(buffer→(set,binding)、scalar→push constant),保 MS1.2 `rxrt_launch` ABI 兼容(🔒 FFI,§4.7)。
- **artifact 泛化**:`ArtifactKind::Spirv` + `ArchKey{Sm/Gfx/SpirvPortable}` + 描述表 v2 + `rurix.lock` `kind="spirv"`,不破 NVIDIA cubin/ptx。
- **graphics + present**:render pass / graphics pipeline / swapchain / present(桌面 win32 surface / Android android surface);uc03/uc04 等价验收。
- **Android 移植缝**:dlopen `libvulkan.so` / 调用约定 / `aarch64-linux-android` + NDK / `ANativeWindow`;交叉构建绿,设备 pending-hardware。
- **toolchain 定位 + 供应链**:`glslang`/`spirv-val` 定位(缺工具 SKIP 非 RX 码);Vulkan Rust 绑定(ash vs 手写)pin。

**🔒 禁区不在本文件**:launch marshalling / Backend trait / dlopen 加载缝的 FFI ABI 二进制布局(RFC-0011 §4.5/§4.7/§4.10)、纹理路径内存模型映射(06 §4.2)只作边界声明,不落语义本体;触及即停手升 agent Full RFC。**对外通用多后端可移植抽象层承诺**(D-008/SG-003 红线 3 底层关切)**永不做**——本后端 explicit、单目标 per-build、无地址空间推断(RFC-0011 §7/§8)。

**编号区间**:本文件条款自 **RXS-0200** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;main 现最高 RXS-0188 @ [release.md](release.md);RXS-0189~0199 由 MS1.2/MS1.2b 承接[feat/ms1.2b 在途],跳号避撞维持——镜像 RXS-0181~0184 GRX 分支占用先例)。**已锁定预留区间 RXS-0200 ~ RXS-0213**(14 条,RFC-0011 §5 clause-mapping):RXS-0200 target 分发 / 0201 compute 执行模型 / 0202 compute builtins / 0203 存储缓冲+控制流 / 0204 graphics 编码 / 0205 数学 intrinsic 映射 / 0206 Backend trait / 0207 Vulkan compute 运行时 / 0208 launch marshalling / 0209 artifact 泛化 / 0210 graphics+present / 0211 Android 移植缝 / 0212 toolchain 定位 / 0213 绑定供应链。**本轮(脚手架)仅登记区间预留,不落带编号裸条款头**;条款体与每条 ≥1 测试锚定随 mb1 各 Phase 实现 PR(Phase 1~4,RFC-0011 批准 + 红线 3 解除后)同落。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 本节落带编号条款体(`### RXS-0200` 起),**随 mb1 各 Phase 实现 PR 同落**(条款 PR 先于/同实现 PR,硬规则 7;Phase 1 落 RXS-0200~0205 codegen 面 / Phase 2 落 RXS-0206~0209 运行时面 / Phase 3 落 RXS-0210 present / Phase 4 落 RXS-0211 Android;RXS-0212/0213 toolchain+供应链随 Phase 1 起)。各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(P-01 strict-only 编译期 6xxx 诊断定义,无运行期 fallback;10 §7.5)。**本片(MB1.1 walking skeleton)已落 RXS-0200 / RXS-0201**(codegen target 分发 + 最小 compute GLCompute 端到端 → spirv-val clean);RXS-0202~0205(compute body lowering / builtins / 存储缓冲+控制流 / graphics / 数学 intrinsic)与 RXS-0206~0213 随后续分片续落。**本片不碰** 🔒 launch marshalling FFI ABI(RFC-0011 §4.7)/ Backend trait(§4.5)/ 纹理内存模型映射(06 §4.2);触及即停手升档。

### RXS-0200 codegen target 分发与 Vulkan 后端分叉

MIR 之后按目标(target)选择 codegen 后端:现状 NVPTX→PTX 后端(D-207)/ DXIL 后端(D-131)/ Vulkan 后端(本条)**并存**。target 选择经 `rx build --target <ptx|dxil|vulkan>` 显式给定(RFC-0011 §4.1),无隐式多目标、无静默 fallback(P-01 strict-only)。Vulkan 后端 gate 于 cargo feature `vulkan-backend`(RFC-0011 §6);未启用时 Vulkan 后端不参与编译,`--target vulkan` 报目标不可用诊断,PTX/DXIL 路径不受影响。

本条覆盖 **codegen 分发与后端分叉的语义骨架** + **最小 compute kernel → SPIR-V 端到端**(空体 compute 入口 MIR → SPIR-V `GLCompute` 模块 → spirv-val 接受)。compute body lowering(RXS-0202/0203)、graphics 编码(RXS-0204)、数学 intrinsic 映射(RXS-0205)不在本条;本条 compute 路径以 RXS-0153 的 compute-via-kernel 着色为入口锚点。

#### Syntax

target 选择为工具链 CLI 面,非语言文法面:`rx build --target vulkan <input.rx>`(与 `--target ptx`/`--target dxil` 并列;省略 `--target` 维持现状默认 host/PTX 通道,零语义漂移)。kernel 源码不因 target 改写——同一份 compute 着色(`kernel fn`,RXS-0153 着色)经 Vulkan 后端降级为 SPIR-V GLCompute 模块。

#### Legality

- L1(后端可用性):`--target vulkan` 要求 cargo feature `vulkan-backend` 已启用;未启用 → **RX6026**(codegen 目标不可用,P-01 strict-only,不降级 host/PTX/DXIL)。
- L2(最小子集):本片 Vulkan 后端仅支持 compute 着色入口的**最小子集**(无 ABI 形参、空/平凡体 → SPIR-V `void` GLCompute 入口)。子集外构造(存储缓冲/资源句柄形参、非平凡体、控制流——需存储缓冲降级 / 描述符布局 / 结构化控制流)→ **RX6026**(属 RXS-0202/0203 后续分片)。
- L3(降级失败):SPIR-V emit / spirv-val 拒 → **RX6026** 编译期 codegen 诊断(无运行期 fallback)。工具链缺失(spirv-val 不可用)为开发环境降级 **SKIP**(非 RX6026,对齐 RXS-0073 ptxas 干验证 SKIP 纪律,真实红绿在带 Vulkan SDK 的环境)。

#### Dynamic Semantics

Vulkan 后端为 codegen/工具链面,本条无运行期语言语义(SPIR-V 在 Vulkan 管线的执行属运行时/RXS-0207+,不在本条)。降级管线为编译期确定性变换:给定 MIR 输入,SPIR-V 字流对相同输入确定(两次产出字节一致)。

#### Implementation Requirements

- IR1(分发点):target 分发在 MIR 之后(AST→HIR→TBIR→MIR 前沿对所有 target 共享);Vulkan 后端与 NVPTX / DXIL 后端并列、各自从 MIR 独立降级,不共享后端内部 lowering,不改 PTX/DXIL 路径(RFC-0003 §4.5 口径)。
- IR2(SPIR-V emit):MIR → SPIR-V `GLCompute` 入口(`OpEntryPoint GLCompute` + `OpExecutionMode LocalSize` + void `main`;SPIR-V 1.0,`Logical`/`GLSL450` 内存模型),小端字节 `.spv` 落盘。
- IR3(spirv-val gate):`.spv` 经 `spirv-val` **接受**方为合规产物(退出码判定,非 grep stdout,反 Godot 崩溃判定教训);不合规 → **RX6026**。工具缺失(spirv-val 不可用)→ **SKIP**(开发环境降级,真实红绿在带 Vulkan SDK 环境)。
- IR4(错误码):target 不可用 / 子集外构造 / 降级失败归 **RX6026**(6xxx codegen/目标段,跳 RX6024/RX6025=MS1.2b 在途占用避撞,只追加,registry/error_codes.json + en/zh message-key);工具链缺失为 SKIP 不发码。

### RXS-0201 MIR→SPIR-V compute 执行模型与最小 GLCompute 入口

compute 着色入口(`kernel fn`,RXS-0153 compute-via-kernel 着色)降级为 SPIR-V `GLCompute` 执行模型入口。本条覆盖**执行模型 + workgroup 维度(LocalSize)的最小切片**(空体 void `main`);compute builtins(`DeviceIntrinsic` → SPIR-V builtin,RXS-0202)、存储缓冲与结构化控制流(RXS-0203)不在本条。

#### Syntax

无语言文法面(codegen/工具链面)。

#### Legality

- L1(执行模型):compute 入口经 `OpEntryPoint GLCompute` 声明;首片 workgroup 维度取最小 `LocalSize 1,1,1`(launch bounds → LocalSize 降级属 RXS-0202 后续)。
- L2(子集):非平凡体 / 带形参 → **RX6026**(承 RXS-0200 L2)。

#### Dynamic Semantics

SPIR-V 模块结构确定:header(magic / version 1.0 / bound)+ `OpCapability Shader` + `OpMemoryModel Logical GLSL450` + `OpEntryPoint GLCompute %main "<entry>"` + `OpExecutionMode %main LocalSize 1 1 1` + `OpTypeVoid`/`OpTypeFunction` + 单基本块(`OpLabel` / `OpReturn`)。给定入口符号名,字流字节确定(两次产出逐字节一致)。

#### Implementation Requirements

- IR1(执行模型):`GLCompute`(区别于 RXS-0204 graphics 的 `Vertex`/`Fragment`);`Shader` capability 覆盖基本 GLCompute。
- IR2(spirv-val):`.spv` 经 spirv-val 接受(承 RXS-0200 IR3);篡改字流 → spirv-val 拒(真实红绿:篡改红 / 复原绿,RFC-0011 §6)。
- IR3(测试锚定):≥1 `//@ spec: RXS-0201` 覆盖 SPIR-V 结构(`GLCompute` + `LocalSize` 存在)+ spirv-val accept + 小端字节序。

> 锚定测试:conformance/vulkan/accept/vk_noop.rx(`--target vulkan` → spirv-val accept)+ src/rurixc/src/vulkan_codegen.rs 单测(header shape / little-endian)。

## 3. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-07-15 | 新建 vulkan_backend.md(mb1 spec 脚手架,承 RFC-0011 Draft):登记文件名 + 文件级语义面说明(MIR→SPIR-V 跨端第三后端,AMD 桌面 + Android,compute+graphics)+ §0 治理闸口(gated on 红线 3 解除 + RFC-0011 批准,owner 裁决)+ §1 范围与 **RXS-0200~0213 预留区间声明** + §2 条款占位(条款体随 mb1 各 Phase 实现 PR 同落)。**沿 README v1.0 dxil_backend.md / v1.51 edition.md 脚手架先例:仅登记文件名 + 预留区间,不落带编号裸条款头**——本文件**零 `### RXS-####` 条款头**,`ci/trace_matrix.py --check` 维持全锚定不变(无新增裸条款头、无悬空锚点、零新 RXS)。条款体(RXS-0200 起)与每条 ≥1 `//@ spec` 测试锚定随各 Phase 实现 PR(RFC-0011 批准 + 红线 3 解除后)同落。禁区声明:🔒 launch marshalling / Backend trait / dlopen FFI ABI 二进制布局(RFC-0011 §4.5/§4.7/§4.10)/ 纹理路径内存模型映射(06 §4.2)/ 通用多后端可移植抽象层承诺(D-008/SG-003)均不在本文件,触及即停手升档。错误码 **6xxx codegen 段**脚手架不预造、不预留,随各 Phase 按真实可达类别只追加(跳 MS1.2b 已占)。档位 **Full RFC**(RFC-0011;触 codegen 第三后端 + 新运行时后端 + FFI ABI + 红线 3,agent 自主判档,判档争议向上取严)。**gated on owner 裁决红线 3 解除 + RFC-0011 批准,未获裁决前不合入 main**,无体例变更 | **Full RFC**（RFC-0011） |
| v1.1 | 2026-07-15 | **MB1.1 walking skeleton:落带编号条款体 `### RXS-0200` / `### RXS-0201`**(codegen target 分发与 Vulkan 后端分叉 + 最小 compute GLCompute 端到端)+ 配套 rurixc 实现(`vulkan_codegen.rs` MIR→SPIR-V 最小 compute emitter + `driver.rs` `--target vulkan` 分发 + cargo feature `vulkan-backend` + `toolchain::spirv_val_gate` 缺工具 SKIP)。条款体按 FLS 分 Syntax / Legality(L1 后端可用性 / L2 最小子集 / L3 降级失败 → RX6026)/ Dynamic Semantics / Implementation Requirements,**严禁 UB 节**。配套 conformance accept(`conformance/vulkan/accept/vk_noop.rx` 空体 compute → GLCompute SPIR-V,`//@ spec: RXS-0200, RXS-0201`)+ vulkan_codegen 单测(header shape / 小端字节)。错误码新增 **RX6026**(`codegen.vulkan_unsupported`,6xxx 段跳 RX6024/6025=MS1.2b 避撞,只追加 + en/zh message-key)。**真实红绿**(本机 Vulkan SDK 1.3.296.0):`--target vulkan` 产 spirv-val-clean `.spv`(独立 spirv-val 退出码 0 accept);篡改 `.spv` 字节 → spirv-val 拒(退出码 1);子集外体 → RX6026(退出码 1);feature-off → RX6026(退出码 1)。`ci/trace_matrix.py` 全锚定 **184→186**(RXS-0200/0201 各 ≥1 `//@ spec`)。**本片不碰** 🔒 launch marshalling / Backend trait / 纹理内存模型;body lowering / builtins / 存储缓冲 / 控制流 / graphics / 数学 intrinsic 随 RXS-0202~0205 后续分片。档位 **Full RFC**(RFC-0011);**gated on owner 裁决红线 3 解除 + RFC-0011 批准,未获裁决前不合入 main**,无体例变更 | **Full RFC**（RFC-0011） |
