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

### RXS-0202 compute builtins 与 device intrinsic 降级

compute body 内 `ThreadCtx` 方法(`global_id()`/`thread_index()`/`block_index()`,MIR `CallTarget::DeviceIntrinsic`)降级为 SPIR-V builtin 变量读取;`sync()`(Barrier)降级为 `OpControlBarrier`。本条覆盖 compute 线程上下文 intrinsic 的 SPIR-V 映射;数学 intrinsic(`__nv_*` → GLSL.std.450)属 RXS-0205,不在本条。

#### Syntax

无语言文法面(codegen 面;`ThreadCtx` 方法名与语义承 RXS-0072 device intrinsic 集,不因 target 改写)。

#### Legality

- L1(支持集):`global_id`(→ `GlobalInvocationId`)/ `thread_index`(→ `LocalInvocationId`)/ `block_index`(→ `WorkgroupId`)/ `sync`(→ `OpControlBarrier`)。首期不支持 `block_dim`(→ WorkgroupSize/LocalSize,需 launch bounds 降级)→ **RX6026**。
- L2(轴):`.{x,y,z}` 分量经 `OpCompositeExtract` 取 0/1/2。

#### Dynamic Semantics

builtin 变量为 `Input` 存储类 `vec3<uint>`,懒发 + `OpDecorate BuiltIn <enum>` + 入 `OpEntryPoint` interface(SPIR-V 1.0 仅 Input/Output 变量入 interface)。索引类 intrinsic:`OpLoad v3uint <builtin>` + `OpCompositeExtract uint _ <axis>` → 存入结果 local。`Barrier` → `OpControlBarrier Workgroup Workgroup (AcquireRelease|WorkgroupMemory)`。确定性:给定 MIR 字流确定。

#### Implementation Requirements

- IR1(映射表):`GlobalId{X,Y,Z}`→`GlobalInvocationId`(28)/ `ThreadIndex{X,Y,Z}`→`LocalInvocationId`(27)/ `BlockIndex{X,Y,Z}`→`WorkgroupId`(26);返回值经 `OpCompositeExtract` 取 usize(u32)分量。
- IR2(barrier):`sync()` → `OpControlBarrier`(Execution/Memory scope = `Workgroup`,semantics = `AcquireRelease | WorkgroupMemory`)。
- IR3(锚定):≥1 `//@ spec: RXS-0202` 覆盖 global_id → GlobalInvocationId 降级 + spirv-val accept。

> 锚定测试:conformance/vulkan/accept/vk_fill.rx(`out[global_id()] = 1.0` → GlobalInvocationId + StorageBuffer,spirv-val accept)。

### RXS-0203 存储缓冲、标量算术与结构化控制流

`View`/`ViewMut<global,T>` 形参降级为 **StorageBuffer 描述符**;标量形参降级为 **push constant**;compute body 的 local 采**内存式**(Function `OpVariable` + `OpLoad`/`OpStore`,镜像 NVPTX);标量算术/比较与**结构化 `if`** 降级为 SPIR-V 算术/比较/结构化控制流。本条是 compute lowering 的主体(saxpy 规范 UC 端到端)。

#### Syntax

无语言文法面(codegen 面)。

#### Legality

- L1(缓冲/标量元素):StorageBuffer 元素与 push-constant 标量首期 `f32`/`i32`/`u32`/`usize`(`usize` 建模为 32-bit u32);`F64`/`I64`/`U64` 需 `Float64`/`Int64` capability → **RX6026**(后续分片)。
- L2(控制流):首期支持**结构化 `if`**(SwitchBool,分支收敛于唯一 merge 块);循环、提前 `return`(分支不收敛)→ **RX6026**(结构化循环 `OpLoopMerge` 属后续分片)。
- L3(子集外):位运算/逻辑运算、非标量 local、`Cast`/`UnaryOp`/`Ref`/`Aggregate`、纹理采样、device fn 调用 → **RX6026**。

#### Dynamic Semantics

- **存储缓冲**(SPIR-V 1.0 SSBO):`OpTypeStruct{OpTypeRuntimeArray T}` 装饰 `BufferBlock`(member 0 `Offset 0`,runtime array `ArrayStride sizeof(T)`),变量 `Uniform` 存储类装饰 `DescriptorSet 0`/`Binding <序>`;`buf[i]` → `OpAccessChain(var, %uint_0, i)`。
- **push constant**:标量形参聚为单 `OpTypeStruct` 装饰 `Block`(member `Offset`),`PushConstant` 存储类;entry 处 `OpAccessChain`+`OpLoad` 拷入其 Function local。
- **local 内存式**:非 ZST、非 buffer 形参、非 ret slot 的 local 各建 Function `OpVariable`,读写经 `OpLoad`/`OpStore`(规避 SSA/phi)。
- **算术/比较**:`+−*/%` → `OpFAdd`/`OpIAdd`/…(浮点/有符号/无符号分派);`<`/`==` 等 → `OpFOrd*`/`OpULessThan`/… 产 `OpTypeBool`,经 `OpSelect` 存回 u32(0/1)。
- **结构化 if**:`SwitchBool` → 载 discr(u32)`OpINotEqual 0` 得 bool → `OpSelectionMerge <merge> None` + `OpBranchConditional cond <then> <else>`;`merge` = 前向可达(then)∩ 前向可达(else)取最小块。

确定性:给定 MIR,SPIR-V 字流字节确定。

#### Implementation Requirements

- IR1(SSBO 布局):`BufferBlock` + `ArrayStride`/`Offset` + `DescriptorSet 0`/`Binding`(按 buffer 形参序);binding 序即形参内 buffer 出现序。
- IR2(push constant):单 `Block` push-constant 块,成员按标量形参序 `Offset`(4 字节标量顺排);entry 拷入 Function local 后 body 统一按 local 处理。
- IR3(merge 计算):结构化 `if` 的 `OpSelectionMerge` merge 块 = 前向可达交集最小块;无收敛(循环/提前 return)→ RX6026(结构化循环属后续)。
- IR4(校验):`.spv` 经 `spirv-val --target-env vulkan1.0` **严格 Vulkan 校验接受**(saxpy 规范 UC:多 SSBO + push constant + GlobalInvocationId + fmul/fadd + 结构化 if 端到端);篡改字流 → 拒(真实红绿)。
- IR5(锚定):≥1 `//@ spec: RXS-0203` 覆盖 saxpy 端到端 + spirv-val vulkan1.0 accept。

> 锚定测试:conformance/vulkan/accept/vk_saxpy.rx(saxpy 规范 UC)+ conformance/vulkan/accept/vk_fill.rx(存储缓冲最小)。

### RXS-0204 MIR→SPIR-V graphics 编码(vertex/fragment,复用 RFC-0004 种子)

vertex/fragment 着色阶段(`Body.stage = Some(Vertex/Fragment)`)经 `--target vulkan` 复用 RFC-0004 的 `dxil_spirv` SPIR-V 编码器(RXS-0161),产 **Vulkan 原生 SPIR-V**(`.spv` 直喂 `vkCreateShaderModule`)——去 B 路 SPIRV-Cross→HLSL→dxc→DXIL 转译链,SPIR-V 即终产物、非中间踏板。graphics io_sig/resources 收集(`attach_graphics_io_sig` / `dxil_io`)与 `dxil_spirv`/`binding_layout` 模块的 feature gate 由 `dxil-backend` 扩为 **`any(dxil-backend, vulkan-backend)`**(NVPTX/DXIL 路零漂移:`any` 含 dxil-backend,dxil 行为不变)。

#### Syntax

无语言文法面(codegen 面;着色阶段语法承 RFC-0002 RXS-0153~0156,不因 target 改写)。

#### Legality

- L1(复用面):vertex/fragment 复用 `dxil_spirv::emit_spirv_body`(execution model `Vertex`/`Fragment` + `OriginUpperLeft`〔fragment〕 + `Location`/`BuiltIn`/`UserSemantic` 装饰 + 采样链);其可映射子集即 RXS-0161 面。
- L2(阶段边界):mesh/task/RT 着色阶段不在本条 → honest-defer **RD-029**;编码器不可映射构造(承 `dxil_spirv` `DxilError`)→ **RX6026**。

#### Dynamic Semantics

`--target vulkan` 对图形阶段:`build_and_emit_vulkan` 按 `Body.stage` 路由——`Some(Vertex/Fragment)` → `dxil_spirv::emit_spirv_body`;`None`(compute)→ compute lowerer(RXS-0201~0203)。产 `OpEntryPoint Vertex/Fragment` + Location 装饰的 SPIR-V 字流;确定性(同 io_sig ×N 字节全等,承 RXS-0162 host 可达确定性面)。

#### Implementation Requirements

- IR1(复用):图形阶段直调 `dxil_spirv::emit_spirv_body(stage, body)`,不重复实现 vertex/fragment 编码(RFC-0011 §4.3 抽取泛化)。
- IR2(feature gate):`dxil_spirv`/`binding_layout`/`attach_graphics_io_sig`/`dxil_io`/`collectable_stage` 的 cfg 由 `dxil-backend` 扩为 `any(dxil-backend, vulkan-backend)`;dxil-backend 单独启用时行为字节不变(`any` 超集)。
- IR3(校验):图形 `.spv` 经 `spirv-val --target-env vulkan1.0` 接受(vertex/fragment 端到端);`SPV_GOOGLE_hlsl_functionality1` + `UserSemantic` 为 dxil_spirv 编码器既有产物,Vulkan 驱动忽略但合规。
- IR4(锚定):≥1 `//@ spec: RXS-0204` 覆盖 vertex + fragment → SPIR-V + spirv-val vulkan1.0 accept。

> 锚定测试:conformance/vulkan/accept/vk_vertex.rx(vertex)+ conformance/vulkan/accept/vk_fragment.rx(fragment)。

### RXS-0205 数学 intrinsic → GLSL.std.450 ext-inst 映射

f32 数学方法(`sqrt`/`sin`/`pow`/`fma`/…,MIR `CallTarget::Libdevice{__nv_*}`,承 RXS-0081)降级为 SPIR-V `OpExtInst "GLSL.std.450" <op>`。`CallTarget::Libdevice` 是 NVIDIA 专有 libdevice 外部符号,SPIR-V 无对应——本条建 `__nv_*` → GLSL.std.450 ext-inst 映射。

#### Syntax

无语言文法面(codegen 面;数学方法名与语义承 RXS-0081 device 数学 intrinsic 集)。

#### Legality

- L1(可映射集):20 个 `DeviceMathFn` 中 1:1 可映射项——`sqrt`→`Sqrt` / `rsqrt`→`InverseSqrt` / `exp`→`Exp` / `exp2`→`Exp2` / `ln`(log)→`Log` / `log2`→`Log2` / `sin`/`cos`/`tan` / `floor`/`ceil`/`trunc` / `round`→`RoundEven` / `abs`(fabs)→`FAbs` / `powf`(pow,2 元)→`Pow` / `min`(fmin,2 元)→`FMin` / `max`(fmax,2 元)→`FMax` / `fma`(3 元)→`Fma`。
- L2(需组合项):`cbrt`(GLSL.std.450 无,需 `Pow(x,1/3)` 组合)、`log10`(需 `Log2·(1/log2 10)` 组合)→ **RX6026**(后续分片)。
- L3(精度轴):符号形态 `__nv_<base>`(f64)/ `__nv_<base>f`(f32);base 无一以 'f' 结尾,strip 尾 'f' 唯一恢复 base;ext-inst 按操作数类型分发(f32/f64 同一编号)。首期仅 f32(F64 需 Float64 capability,RXS-0203 L1)。

#### Dynamic Semantics

`OpExtInstImport "GLSL.std.450"`(懒发,layout 在 memory-model 之前)得 ext-inst-set id;调用点 `OpExtInst <result_type=float> <result_id> <set> <instruction> <arg0..>`(operand 经 operand 载入),结果存入目标 local。确定性:给定 MIR 字流确定。

#### Implementation Requirements

- IR1(映射表):见 L1;arity 1/2/3 通用处理(operand 逐个载入入 `OpExtInst` 操作数)。
- IR2(ext-import):单次 `OpExtInstImport "GLSL.std.450"` 懒发 + 缓存 ext-inst-set id。
- IR3(锚定):≥1 `//@ spec: RXS-0205` 覆盖 sqrt/max → GLSL.std.450 降级 + spirv-val vulkan1.0 accept;未映射(cbrt)→ RX6026(真实红绿)。

> 锚定测试:conformance/vulkan/accept/vk_math.rx(`x[i].sqrt().max(0.0)` → OpExtInst Sqrt/FMax,spirv-val vulkan1.0 accept)。

### RXS-0206 Compute 后端抽象（Backend trait；CUDA 收敛为一实现，Vulkan 并列）

运行时引入 `ComputeBackend` trait,把「跑一个 compute:artifact → module → buffers 上传 → launch → 同步 → readback」收敛为单一后端无关抽象;**CUDA 收敛为一实现**(组合既有 `Context`/`Module`/`Stream`/`DeviceBuffer` public API,**零改其类型**)、**Vulkan 并列实现**(委托 `vk::run_compute`,RXS-0207)。后端选择**显式**(`RURIX_BACKEND` = cuda|vulkan),**无隐式 fallback**(P-01)。

#### Syntax

无语言文法面(运行时/库 API 面)。

#### Legality

- L1(抽象面):`trait ComputeBackend { type Session; open() -> Session; dispatch(session, job) }` 覆盖 open→dispatch 最小 orchestration;句柄经**关联类型 `Session`**(CUDA=拥有 `CUcontext` 的 `Context` / Vulkan=一次性 ZST)零成本区分。`ComputeJob{artifact,entry,buffers(in/out 原位回写),scalars,groups,block}` 后端无关。
- L2(选择,P-01):`RURIX_BACKEND` 未设 = 默认 CUDA(核心后端);显式选定不可用 / 未知取值 / 未编译后端 → **确定性 `Err`**(`NotCompiled`/`Unknown`/`Run`),**绝不自动改跑另一后端**(默认选择 ≠ 运行期 fallback)。
- L3(零回归):CUDA 实现只调既有 pub 方法,**不改 `Context`/`Stream`/`DeviceBuffer`/`Module` 语义、不触 sys.rs/pipeline.rs/vk.rs**;NVPTX/cubin 路不经 backend.rs(NVIDIA 零回归,硬约束)。

#### Dynamic Semantics

`run_job(kind, job)` 枚举分派:选定后端 `open()` → `dispatch()`——CUDA:PTX `load_module` → `function` → `alloc`+H2D → `Stream::launch(grid,block,params)` → `synchronize` → D2H;Vulkan:SPIR-V 字节→字流 → `vk::run_compute`。`job.buffers` 原位回写。纯 host 薄层,确定性。

#### Implementation Requirements

- IR1(零 unsafe):`src/rurix-rt/src/backend.rs` 纯 host 薄层组合各后端 safe public API + safe 字节转换,**零 unsafe**——不入 unsafe-audit、不新增 U 号;`undocumented_unsafe_blocks=deny` 不触发。
- IR2(收敛/委托):CUDA `CudaBackend::dispatch` 只组合既有 API(标量 marshalling ABI 装配 = RXS-0208 后续);Vulkan `VulkanBackend::dispatch` 1:1 委托 `vk::run_compute`(签名逐参对齐,vk.rs 零改)。
- IR3(真跑校验):saxpy 经 `run_job(BackendKind::Vulkan, ..)` 在本机 NVIDIA 真跑数值**精确**(max_err=0)——证 trait 端到端;default(无 vulkan)构建 `CudaBackend` 编译 + 既有 CUDA test 零漂移。
- IR4(锚定):≥1 `//@ spec: RXS-0206` 覆盖 `parse_backend`(显式选择、无 fallback、未知/未编译确定性 Err)。

> 锚定测试:src/rurix-rt/src/backend.rs 单测(`parse_backend`)+ src/rurix-rt/src/bin/vk_saxpy.rs(经 `run_job(Vulkan)` 真 NVIDIA GPU saxpy 精确)。

### RXS-0207 Vulkan compute 运行时执行语义

Phase 1 `--target vulkan` 产 SPIR-V 经手写 `vulkan-1`/`libvulkan` FFI 薄 loader(RFC-0011 §9 Q-Binding 默认:零外部绑定,镜像 `sys.rs` nvcuda.dll 纪律)在 Vulkan 设备(NVIDIA / AMD 桌面 / Android)真跑:instance/device/compute queue → `vkCreateShaderModule` → compute `VkPipeline` + `VkPipelineLayout` → descriptor set(StorageBuffer)+ push constant → command buffer `vkCmdDispatch` → 单 queue 同步(`vkQueueWaitIdle`)→ 回读。运行期 marshalling(buffer→(set,binding)、scalar→push constant)与 codegen 侧描述符布局(RXS-0203)**单一事实源一致**。

#### Syntax

无语言文法面(运行时/FFI 面)。

#### Legality

- L1(首期 compute 面):host-visible+coherent StorageBuffer(免 flush/invalidate)+ 单 push constant 块 + 单 queue 同步提交(`vkQueueWaitIdle`)。
- L2(fail-closed):缺 Vulkan 驱动 / 无 compute queue / pipeline 创建失败 → 确定性 `Err`(非 panic,P-01 strict-only,无静默 fallback);pipeline `pName` 须 = SPIR-V `OpEntryPoint` 名(codegen mangled 符号名),不符 → Vulkan 拒(validation VUID)。

#### Dynamic Semantics

`run_compute(spv, entry, buffers, push_constants, groups)`:`buffers[i]` 绑 `(set 0, binding i)` StorageBuffer(in/out 原位回写),`push_constants` 布局 = shader push constant 块(标量顺排 4 字节对齐),`groups` = `vkCmdDispatch` 工作组数。host-visible+coherent 内存免显式同步;单 queue 同步执行,`vkQueueWaitIdle` 后回读确定。开发期 `VK_LAYER_KHRONOS_validation`(env `RURIX_VK_VALIDATION=1`)零报错。

#### Implementation Requirements

- IR1(手写 FFI,U26):`vulkan-1.dll` 动态加载 + `#[repr(C)]` 结构逐字节对齐 + 句柄线性配对 create/destroy;gate feature `vulkan` 默认关闭,**CUDA(NVPTX/cubin)路零回归**(硬约束)。
- IR2(真跑校验):本机 NVIDIA(RTX 4070 Ti)saxpy 端到端真跑数值**精确**(`out = a*x + out`,max_err=0)+ `VK_LAYER_KHRONOS_validation` 零报错(反证:错入口名触发 VUID,证 layer 生效);**第二 ICD**(lavapipe/SwiftShader)跨厂商回归为 open 尾门(本机无软件 ICD,vendor 或 follow-up)。AMD 真卡为 open 尾门(G-MB1-6)。
- IR3(锚定):≥1 `//@ spec: RXS-0207` 覆盖 entry 名解析(pipeline `pName` 一致性前置);device 真跑证据经 `bin/vk_saxpy` + `ci/vulkan_device_smoke.py`(GPU runner)。

> 锚定测试:src/rurix-rt/src/vk.rs 单测(`entry_point_name` 解析)+ src/rurix-rt/src/bin/vk_saxpy.rs(本机 NVIDIA 真跑 demo,数值精确)。

## 3. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-07-15 | 新建 vulkan_backend.md(mb1 spec 脚手架,承 RFC-0011 Draft):登记文件名 + 文件级语义面说明(MIR→SPIR-V 跨端第三后端,AMD 桌面 + Android,compute+graphics)+ §0 治理闸口(gated on 红线 3 解除 + RFC-0011 批准,owner 裁决)+ §1 范围与 **RXS-0200~0213 预留区间声明** + §2 条款占位(条款体随 mb1 各 Phase 实现 PR 同落)。**沿 README v1.0 dxil_backend.md / v1.51 edition.md 脚手架先例:仅登记文件名 + 预留区间,不落带编号裸条款头**——本文件**零 `### RXS-####` 条款头**,`ci/trace_matrix.py --check` 维持全锚定不变(无新增裸条款头、无悬空锚点、零新 RXS)。条款体(RXS-0200 起)与每条 ≥1 `//@ spec` 测试锚定随各 Phase 实现 PR(RFC-0011 批准 + 红线 3 解除后)同落。禁区声明:🔒 launch marshalling / Backend trait / dlopen FFI ABI 二进制布局(RFC-0011 §4.5/§4.7/§4.10)/ 纹理路径内存模型映射(06 §4.2)/ 通用多后端可移植抽象层承诺(D-008/SG-003)均不在本文件,触及即停手升档。错误码 **6xxx codegen 段**脚手架不预造、不预留,随各 Phase 按真实可达类别只追加(跳 MS1.2b 已占)。档位 **Full RFC**(RFC-0011;触 codegen 第三后端 + 新运行时后端 + FFI ABI + 红线 3,agent 自主判档,判档争议向上取严)。**gated on owner 裁决红线 3 解除 + RFC-0011 批准,未获裁决前不合入 main**,无体例变更 | **Full RFC**（RFC-0011） |
| v1.1 | 2026-07-15 | **MB1.1 walking skeleton:落带编号条款体 `### RXS-0200` / `### RXS-0201`**(codegen target 分发与 Vulkan 后端分叉 + 最小 compute GLCompute 端到端)+ 配套 rurixc 实现(`vulkan_codegen.rs` MIR→SPIR-V 最小 compute emitter + `driver.rs` `--target vulkan` 分发 + cargo feature `vulkan-backend` + `toolchain::spirv_val_gate` 缺工具 SKIP)。条款体按 FLS 分 Syntax / Legality(L1 后端可用性 / L2 最小子集 / L3 降级失败 → RX6026)/ Dynamic Semantics / Implementation Requirements,**严禁 UB 节**。配套 conformance accept(`conformance/vulkan/accept/vk_noop.rx` 空体 compute → GLCompute SPIR-V,`//@ spec: RXS-0200, RXS-0201`)+ vulkan_codegen 单测(header shape / 小端字节)。错误码新增 **RX6026**(`codegen.vulkan_unsupported`,6xxx 段跳 RX6024/6025=MS1.2b 避撞,只追加 + en/zh message-key)。**真实红绿**(本机 Vulkan SDK 1.3.296.0):`--target vulkan` 产 spirv-val-clean `.spv`(独立 spirv-val 退出码 0 accept);篡改 `.spv` 字节 → spirv-val 拒(退出码 1);子集外体 → RX6026(退出码 1);feature-off → RX6026(退出码 1)。`ci/trace_matrix.py` 全锚定 **184→186**(RXS-0200/0201 各 ≥1 `//@ spec`)。**本片不碰** 🔒 launch marshalling / Backend trait / 纹理内存模型;body lowering / builtins / 存储缓冲 / 控制流 / graphics / 数学 intrinsic 随 RXS-0202~0205 后续分片。档位 **Full RFC**(RFC-0011);**gated on owner 裁决红线 3 解除 + RFC-0011 批准,未获裁决前不合入 main**,无体例变更 | **Full RFC**（RFC-0011） |
| v1.2 | 2026-07-15 | **MB1.1 compute body lowering:落带编号条款体 `### RXS-0202` / `### RXS-0203`**(compute builtins + 存储缓冲/标量算术/结构化控制流)+ 配套 `vulkan_codegen.rs` 全 body 降级(镜像 NVPTX 内存式 local:Function `OpVariable` + load/store):`View/ViewMut<global,T>`→StorageBuffer 描述符(SSBO;BufferBlock + set0/binding序 + OpAccessChain)/ 标量形参→push constant(Block+Offset)/ `ThreadCtx.global_id`→`GlobalInvocationId` builtin(OpCompositeExtract)/ 算术 fmul·fadd·比较 OpULessThan / 结构化 `if`→OpSelectionMerge+OpBranchConditional(merge=前向可达交集)。条款体按 FLS 分 Syntax/Legality/Dynamic Semantics/Implementation Requirements,**严禁 UB 节**;子集外(BlockDim / device fn / 数学 intrinsic〔RXS-0205〕/ 循环 / 非标量 / F64·I64 / 位运算)→ RX6026。配套 conformance accept:`vk_fill.rx`(RXS-0202:global_id+SSBO+OpAccessChain 写)+ `vk_saxpy.rx`(RXS-0203:saxpy 规范 UC = 多 SSBO+push constant+builtin+算术+结构化 if)。**真实红绿**(本机 Vulkan SDK 1.3.296.0):`fill`/`saxpy` 经 `--target vulkan` 产 SPIR-V,**`spirv-val --target-env vulkan1.0` 严格 Vulkan 校验接受**(exit 0);比较结果 Bool 内存式建模为 u32(OpSelect)。`ci/trace_matrix.py` 全锚定 **186→188**(RXS-0202/0203 各 ≥1 `//@ spec`)。**本片不碰** 🔒 launch marshalling / Backend trait / 纹理内存模型;graphics(RXS-0204)/ 数学 intrinsic→GLSL.std.450(RXS-0205)/ 结构化循环随后续分片。档位 **Full RFC**(RFC-0011);**gated on owner 裁决红线 3 解除 + RFC-0011 批准,未获裁决前不合入 main**,无体例变更 | **Full RFC**（RFC-0011） |
| v1.3 | 2026-07-15 | **MB1.1 数学 intrinsic:落带编号条款体 `### RXS-0205`**(`__nv_*` → GLSL.std.450 ext-inst 映射)+ 配套 `vulkan_codegen.rs` `emit_call` Libdevice 臂 + `glsl_ext_op` 映射表 + `OpExtInstImport "GLSL.std.450"` 懒发。覆盖 20 `DeviceMathFn` 中 18 个 1:1 项(sqrt/rsqrt/exp/exp2/log/log2/sin/cos/tan/floor/ceil/trunc/round/fabs/pow/fmin/fmax/fma;arity 1/2/3 通用),`cbrt`/`log10`(需组合)→ RX6026 诚实 defer。条款体 FLS,严禁 UB。配套 conformance accept `vk_math.rx`(`x[i].sqrt().max(0.0)`)。**真实红绿**(本机 Vulkan SDK 1.3.296.0):sqrt(1 元 `OpExtInst Sqrt`)/ max(2 元 `FMax`)/ fma(3 元 `Fma`)经 `--target vulkan` → `spirv-val --target-env vulkan1.0` accept(exit 0);cbrt→RX6026(exit 1)。`ci/trace_matrix.py` 全锚定 **188→189**(RXS-0205 ≥1 `//@ spec`)。**本片不碰** 🔒 launch marshalling / Backend trait / 纹理内存模型;graphics(RXS-0204)/ 结构化循环 / cbrt·log10 组合随后续分片。档位 **Full RFC**(RFC-0011);**gated on owner 裁决红线 3 解除 + RFC-0011 批准,未获裁决前不合入 main**,无体例变更 | **Full RFC**（RFC-0011） |
| v1.4 | 2026-07-15 | **MB1.1 graphics 编码:落带编号条款体 `### RXS-0204`**(MIR→SPIR-V vertex/fragment,复用 RFC-0004 `dxil_spirv` 种子)+ 配套 `build_and_emit_vulkan` 按 `Body.stage` 路由(graphics→`dxil_spirv::emit_spirv_body` / compute→lower_compute)+ feature gate `dxil_spirv`/`binding_layout`/`attach_graphics_io_sig`/`dxil_io`/`collectable_stage` 由 `dxil-backend` 扩为 **`any(dxil-backend, vulkan-backend)`**。SPIR-V 即 Vulkan 原生终产物(`.spv`→`vkCreateShaderModule`),去 B 路 SPIRV-Cross→HLSL→dxc 转译链。条款体 FLS,严禁 UB;mesh/task/RT → RD-029 defer。配套 conformance accept `vk_vertex.rx`/`vk_fragment.rx`。**真实红绿**(本机 Vulkan SDK 1.3.296.0):vertex(`OpEntryPoint Vertex`)/ fragment(`OpEntryPoint Fragment`+`OriginUpperLeft`)经 `--target vulkan` → `spirv-val --target-env vulkan1.0` accept(exit 0)。**零回归**:dxil-backend 单独启用 test 404 passed、default test 318 passed(`any` 超集,dxil 行为字节不变)。`ci/trace_matrix.py` 全锚定 **189→190**(RXS-0204 ≥1 `//@ spec`)。**本片不碰** 🔒 launch marshalling / Backend trait / 纹理内存模型;present(RXS-0210)/ 多阶段 .spv 分文件输出随后续分片。档位 **Full RFC**(RFC-0011);**gated on owner 裁决红线 3 解除 + RFC-0011 批准,未获裁决前不合入 main**,无体例变更 | **Full RFC**（RFC-0011） |
| v1.5 | 2026-07-15 | **MB1.2 Vulkan compute 运行时:落带编号条款体 `### RXS-0207`**(Vulkan compute 执行语义)+ 配套 `src/rurix-rt/src/vk.rs`(feature `vulkan` 默认关闭)手写 `vulkan-1` FFI 薄 loader(RFC §9 Q-Binding 默认零外部绑定,镜像 sys.rs;~35 Vulkan 命令 + `#[repr(C)]` 结构逐字节对齐 + 句柄线性生命周期,unsafe-audit **U26**〔跳 U23 空号 / U25=MS1.2b 避撞〕)+ `bin/vk_saxpy` 真跑 demo。instance/device/compute queue → shader module → compute pipeline → descriptor(StorageBuffer)+ push constant → `vkCmdDispatch` → 单 queue 同步。条款体 FLS,严禁 UB。**真实红绿(本机 NVIDIA RTX 4070 Ti,Vulkan 1.4.351)**:`rurixc --target vulkan saxpy.rx` 产 `.spv` → `bin/vk_saxpy` 真跑 saxpy=a*x+out **数值精确 max_err=0**(n=1024,a=2)+ `VK_LAYER_KHRONOS_validation` **零报错**(反证:错入口名触发 VUID-VkPipelineShaderStageCreateInfo-pName-00707,证 layer 生效);缺 Vulkan 驱动 → 确定性 Err 非 panic。**NVIDIA(CUDA)零回归**(feature vulkan 默认关闭,cargo build/test -p rurix-rt 零改动)。`ci/trace_matrix.py` 全锚定 **190→191**(RXS-0207 vk.rs 单测 + bin/vk_saxpy)。**第二 ICD(lavapipe)+ AMD 真卡为 open 尾门**;RXS-0206 Backend trait 抽象(CUDA 收敛)/ RXS-0208 marshalling ABI / RXS-0209 artifact 泛化随后续分片。档位 **Full RFC**(RFC-0011);**gated on owner 裁决红线 3 解除 + RFC-0011 批准,未获裁决前不合入 main**,无体例变更 | **Full RFC**（RFC-0011） |
| v1.6 | 2026-07-15 | **MB1.2 Backend trait:落带编号条款体 `### RXS-0206`**(Compute 后端抽象,CUDA 收敛为一实现,Vulkan 并列)+ 配套 `src/rurix-rt/src/backend.rs`(**纯 host 薄层,零 unsafe,不新增 U 号**):`trait ComputeBackend{type Session; open; dispatch}` + `ComputeJob`(artifact/entry/buffers/scalars/groups/block)+ `CudaBackend`(组合既有 Context/Module/Stream/DeviceBuffer pub API,**零改其类型 / 零触 sys·pipeline·vk**)+ `VulkanBackend`(1:1 委托 vk::run_compute)+ `parse_backend`/`select_backend`/`run_job`(RURIX_BACKEND 显式选择,**无隐式 fallback** P-01:未知/未编译 → 确定性 Err)。条款体 FLS,严禁 UB。**真实红绿**:saxpy 经 `run_job(BackendKind::Vulkan)` 在本机 NVIDIA RTX 4070 Ti 真跑数值精确 max_err=0(bin/vk_saxpy 改经 backend 抽象,非直调 vk::run_compute)+ `parse_backend` 单测(cuda/未知/未编译 各确定性)。**NVIDIA(CUDA)零回归**:default(无 vulkan)构建 CudaBackend 编译 + rurix-rt lib 16 test 零漂移;clippy 双 feature clean。`ci/trace_matrix.py` 全锚定 **191→192**(RXS-0206 backend.rs 单测 + bin/vk_saxpy)。RXS-0208 marshalling ABI / RXS-0209 artifact 泛化 / lavapipe 第二 ICD 随后续分片。档位 **Full RFC**(RFC-0011)。**红线 3 已 owner 解除(2026-07-15),RFC-0011 Owner Approved**,mb1 下游实现解锁。无体例变更 | **Full RFC**（RFC-0011） |
