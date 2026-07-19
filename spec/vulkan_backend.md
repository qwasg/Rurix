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

**编号区间**:本文件条款自 **RXS-0200** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;main 现最高 RXS-0188 @ [release.md](release.md);RXS-0189~0199 由 MS1.2/MS1.2b 承接[feat/ms1.2b 在途],跳号避撞维持——镜像 RXS-0181~0184 GRX 分支占用先例)。**已锁定预留区间 RXS-0200 ~ RXS-0213**(14 条,RFC-0011 §5 clause-mapping):RXS-0200 target 分发 / 0201 compute 执行模型 / 0202 compute builtins / 0203 存储缓冲+控制流 / 0204 graphics 编码 / 0205 数学 intrinsic 映射 / 0206 Backend trait / 0207 Vulkan compute 运行时 / 0208 launch marshalling / 0209 artifact 泛化 / 0210 graphics+present / 0211 Android 移植缝 / 0212 toolchain 定位 / 0213 绑定供应链。**本轮(脚手架)仅登记区间预留,不落带编号裸条款头**;条款体与每条 ≥1 测试锚定随 mb1 各 Phase 实现 PR(Phase 1~4,RFC-0011 批准 + 红线 3 解除后)同落。区间登记于 [README.md](README.md) §4 文件清单。**续登(RFC-0013 独立续号,超 mb1 区间)**:RXS-0230(§4.B7 graphics descriptor 底座,G3.3)/ **RXS-0246 mesh/task SPIR-V 编码 + RXS-0247 RT 六模型编码 + SPIR-V 1.4 per-entry 分叉(§4.E5/E6,G3.6)**。

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

> **勘误(承 RXS-0210,2026-07-15)**:IR3 原断言「`SPV_GOOGLE_hlsl_functionality1` + `UserSemantic` … Vulkan 驱动忽略但合规」在 **device 面被证伪**——`spirv-val --target-env vulkan1.0` 接受(codegen 校验门为真),但 `vkCreateShaderModule` 在**未启用** device 扩展 `VK_GOOGLE_hlsl_functionality1` 时按 **VUID-VkShaderModuleCreateInfo-pCode-08742 拒**(声明了 SPIR-V 扩展却未满足对应 device 要求)。修订:provenance 装饰(`UserSemantic`→`SPV_GOOGLE`)自 RXS-0210 起改为 **target-conditional**——**Vulkan 原生路不 emit**(`emit_spirv_body_vulkan`,`.spv` 免 device 扩展依赖、跨 ICD 直喂),**DXIL 路保名字节不变**(`emit_spirv_body`,provenance=true,B 路 SPIRV-Cross→HLSL→dxc 边界消费;A/B sha256 逐字节相等实证)。详见 RXS-0210 L2。

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

### RXS-0208 launch marshalling(descriptor-binding;与 RXS-0203 codegen 单一事实源;MS1.2 ABI 前瞻兼容)

运行期把 kernel 实参序位(ordinal)marshalling 为 Vulkan 描述符绑定与 push-constant 布局:`buffers[i]` → `(set 0, binding i)` StorageBuffer;标量按序位 → 单一 push-constant 块的顺排偏移。**序位是唯一分派依据**(无按名绑定、无按类型推断),运行期布局与 codegen 侧(RXS-0203)SPIR-V 描述符装饰**同源于形参出现序**,是同一事实源的两侧,不各自约定。🔒 MS1.2 `rxrt_launch` 扁平 kernelParams ABI 二进制布局属 RFC-0011 §4.7 升档禁区,本条只声明映射义务、不定义其字节本体。

#### Syntax

无语言文法面(运行时 / FFI 面)。

#### Legality

- L1(marshalling 面):运行期把 `buffers[i]` 序位 marshalling 为 `(set = 0, binding = i)` StorageBuffer;标量形参按序位 marshalling 为单一 push-constant 块内的顺排偏移(4 字节对齐)。**序位(ordinal)是唯一分派依据**——无按名绑定、无按类型推断、无隐式重排。
- L2(单一事实源):运行期 `(set, binding)` 与 push-constant `offset` **必须**与 codegen(RXS-0203)产出的 SPIR-V 描述符装饰一致——`OpDecorate Binding` = buffer 形参出现序、push-constant 成员 `OpMemberDecorate Offset` = 标量形参出现序 × 4。两侧**同源于形参出现序**,非各自约定的两份可漂移拷贝。不一致 → 见 L3。
- L3(fail-closed):marshalling 与 SPIR-V 装饰不符时,由 Vulkan validation 在运行期**确定性拒绝**(pipeline `pName` 不符 → `VUID-VkPipelineShaderStageCreateInfo-pName-00707`;binding 数 / 类型不符 → descriptor VUID)并返回 `Err`(非 panic、非静默,承 RXS-0207 L2,P-01 strict-only)。**不占 RX 码**(运行期工具层口径,对齐 spec/release.md §3;非编译期 6xxx)。

#### Dynamic Semantics

`run_compute(spv, entry, buffers, push_constants, groups)`(`src/rurix-rt/src/vk.rs`)——`buffers[i]` → `(set 0, binding i)` StorageBuffer(descriptor set layout binding `i` + write descriptor set `dst_binding = i`,in/out 原位回写),`push_constants` 字节整块经 `vkCmdPushConstants`(offset 0)喂入 shader push-constant 块(标量顺排 4 字节对齐,同 RXS-0203)。序位化 marshalling 对相同 `(buffers, push_constants)` 布局确定。

#### Implementation Requirements

- IR1(ordinal 映射):运行时 descriptor set layout binding `i` = buffer 序 `i`,与 codegen `classify_param` 的 `next_binding` 递增(`src/rurixc/src/vulkan_codegen.rs`)**同序**;push-constant 块单块、成员按标量序 `Offset i×4`(codegen `emit_push_constants`)。两侧唯一事实源 = 形参出现序。
- IR2(🔒 边界声明,不落本体):MS1.2 `rxrt_launch` 的 `slots[u64] + kinds[u8]` 扁平 kernelParams ABI **二进制布局**属 RFC-0011 §4.7 🔒 升档禁区;本条只声明「Vulkan 侧按 ordinal 从 slots 推导 `(set, binding)` / push-constant」的**映射义务**,不定义 `rxrt_launch` 字节布局本体。
- IR3(前瞻兼容,条件):当 `rxrt_launch` / `rurix-rt-cabi` 合入本分支后,Vulkan backend 消费其序位化 slots 时**必须**保持 CUDA 路 `rxrt_launch` 符号面字节不变(RXS-0194「符号面只追加」口径;主选零 ABI 新增,备选 `rxrt_dispatch_*` 新符号)。**mb1 base 无该符号 → ABI-字节回归测试对象缺席 → honest-defer RD-030**(backfill:MS1.2 rxrt_launch / rurix-rt-cabi 合入本分支)。
- IR4(锚定):≥1 `//@ spec: RXS-0208`,host 可测,覆盖「ordinal → `(set, binding)` 映射 + 与 RXS-0203 codegen binding / push-constant offset 序一致」——测试消费 `rurixc --target vulkan` 对 saxpy 的**真** `.spv`,解析实际 `OpDecorate Binding` / `OpMemberDecorate Offset` 装饰值核对,非内联复刻绑定规则。

> 锚定测试:src/rurix-rt/src/vk.rs 单测(`marshalling_ordinal_matches_codegen_binding`——解析 build.rs 经 vulkan_codegen 产的真 saxpy `.spv` 的 binding / offset 序,核对 = 运行时 descriptor-binding 构造序);device 真跑沿用 RXS-0207 `bin/vk_saxpy`(错入口名 → VUID 红 / 正确 → 绿)。

### RXS-0209 device 产物泛化(SPIR-V 变体 + ArchKey 三槽 + lock format-generic)

分发产物模型泛化以承 Vulkan:`ArtifactKind` 加 `Spirv`(可移植 device 产物)、架构键 `SmTarget` 泛化为 `ArchKey{Sm/Gfx/SpirvPortable}`(per-arch AOT 键 `sm_89`/`gfx1100` + 可移植槽),`rurix.lock` `[[artifact]]` 天然承 `kind="spirv"` / `sm_target="gfx1100"`(format-generic,零 schema 改)。**NVIDIA cubin/ptx 路径逐字节不变**(加性:`new`/`ptx_fallback`/`from_capability` 签名与语义 0-byte)。

#### Syntax

无语言文法面(host 产物模型类型面)。

#### Legality

- L1(变体类别加性):`ArtifactKind` 加 `Spirv`(`as_str() = "spirv"`),Vulkan 可移植 device 产物(驱动 JIT 装载,占可移植槽);既有 `Ptx`/`Cubin`/`Fatbin` 字面量与语义 0-byte。
- L2(ArchKey 三槽):架构键 `ArchKey{Sm(String), Gfx(String), SpirvPortable}` prefix-dispatch 解析——`sm_<digits>` → `Sm`(NVIDIA cubin AOT)/ `gfx<alnum>` → `Gfx`(AMD hsaco AOT,G1.5 `SmTarget` 因 `sm_` 硬前缀守卫**误拒**的形态,正是泛化点)/ `""` → `SpirvPortable`(可移植槽,无 per-arch 键)。**`SpirvPortable` 与 `Ptx` 是同一可移植槽的两个厂商实现**(Vulkan SPIR-V / NVIDIA PTX,驱动 JIT);`Sm`/`Gfx` 是 per-arch AOT 槽两厂商实现。
- L3(lock format-generic,零码改):`rurix.lock` `[[artifact]]` `kind`/`sm_target` 皆自由 `String`,序列化 / 解析 / 排序键 `(package, kind, sm_target)` 对 `"spirv"`/`"gfx1100"` 天然工作——**零 schema、零码改**即锁定 Vulkan 变体;未知前缀 → `ArchKey::parse` 返 `None` → 装载协商降级(非致命,同 RXS-0151),无编译期诊断、不占 RX 码。

#### Dynamic Semantics

`DeviceArtifactSet` 平行加 SPIR-V 可移植槽 `spirv_fallback: Option<Vec<u8>>`(`with_spirv_fallback` builder / `spirv_fallback` accessor),不动 NV `ptx_fallback` 构造签名。装载协商 `select_load_variant`:per-arch AOT 命中 → `Cubin(ArchKey)`;未命中且存在 SPIR-V 槽 → `SpirvPortable`;否则 → `PtxFallback`。**NVIDIA 零回归**:NV-only 集 `spirv_fallback = None` → 未命中恒回 `PtxFallback`(逐字节等价 G1.5)。

#### Implementation Requirements

- IR1(fatbin.rs 加性 + ripple):`src/rurix-rt/src/fatbin.rs` `ArtifactKind::Spirv` + `SmTarget → ArchKey` 泛化 + `CubinVariant`/`with_cubin`/`cubin_for`/`cubin_targets`/`LoadChoice`/`select_load_variant` 键类型 ripple + `DeviceArtifactSet` SPIR-V 槽;`lib.rs`/`bin/fatbin_saxpy.rs` 名替(NVIDIA 运行时路径逻辑 0-byte)。
- IR2(描述表 v2 blob,honest-defer):device 描述表 v2(`@__rx_gpu_artifacts` blob bump + `@__rx_gpu_spirv` 段 + `emit_gpu_artifact_globals` codegen)在 mb1 base **无对象**(base 无 artifacts blob / emit_gpu_artifact_globals,MS1.2 codegen 面)→ **honest-defer RD-031**(backfill:MS1.2 artifacts blob / codegen 合入本分支);本条只落 host 产物模型泛化,不伪造描述表本体。
- IR3(lock 诚实登记):`src/rurix-pkg/src/lock.rs` 仅 doc-comment 泛化(`kind` 加 `"spirv"`、`sm_target` 泛化为 per-arch AOT 键 + 可移植槽空),**schema 零码改**;roundtrip 测试加 `kind="spirv"`/`sm_target="gfx1100"` 变体断言。
- IR4(锚定):≥1 `//@ spec: RXS-0209`,纯 host 类型(回归网不依赖 GPU 而绿),覆盖 `ArtifactKind::Spirv` + `ArchKey` prefix-dispatch(Sm/Gfx/SpirvPortable) + `with_spirv_fallback` roundtrip + lock spirv/gfx 变体 roundtrip。

> 锚定测试:src/rurix-rt/src/fatbin.rs 单测(`artifact_kind_and_archkey_spirv_generalization`)+ src/rurix-pkg/src/lock.rs 单测(`lock_artifact_spirv_and_gfx_key_roundtrip`)。

### RXS-0210 Vulkan graphics 运行时 + offscreen present

graphics 出图运行时最小面:`vk::run_graphics_offscreen` 在本机 Vulkan 设备 offscreen 渲染一帧(render pass 单 color attachment CLEAR→STORE + graphics pipeline vertex+fragment + framebuffer + 顶点缓冲 + `vkCmdDraw` + `vkCmdCopyImageToBuffer` 回读)→ 紧凑 RGBA8 像素,数值对照校验(覆盖/背景/插值)。**配套 codegen provenance 微调**:Vulkan 原生 SPIR-V 去 `UserSemantic`/`SPV_GOOGLE`(承 RXS-0204 勘误),`.spv` 免 device 扩展依赖直喂 `vkCreateShaderModule`。swapchain/窗口 present(平台 surface)为 open 尾门 honest-defer。

#### Syntax

无语言文法面(运行时/FFI 面 + codegen provenance 微调;着色阶段语法承 RFC-0002 RXS-0153~0156)。

#### Legality

- L1(offscreen 必要面):render pass(单 color attachment,loadOp=CLEAR / storeOp=STORE / finalLayout=TRANSFER_SRC_OPTIMAL)+ graphics pipeline(vertex+fragment 双 stage,pName 恒 `"main"`——`OpEntryPoint` 名恒 `"main"`,不走 compute mangled 路径)+ framebuffer + 顶点缓冲绑定(`vkCmdBindVertexBuffers`)+ `vkCmdDraw` + `vkCmdCopyImageToBuffer` 回读;host 侧像素数值对照为 device 必要证据。
- L2(provenance 去除,承 RXS-0204 勘误):Vulkan 原生 SPIR-V **不 emit** `UserSemantic` / `SPV_GOOGLE_hlsl_functionality1`(`emit_spirv_body_vulkan`,provenance=false)→ 去后 `vkCreateShaderModule` 免 device 扩展 `VK_GOOGLE_hlsl_functionality1` 依赖(修 VUID-...-08742),跨 ICD(NVIDIA/AMD/Android/lavapipe)可移植;**DXIL 路(`emit_spirv_body`,provenance=true)保名字节不变**(target-conditional,零回归;A/B sha256 逐字节相等实证 + `dxil-backend` 单独启用 test 404 不变)。去装饰只减不增 → `spirv-val` 仍接受两变体(修复是「去装饰」非「产非法 SPIR-V」)。
- L3(fail-closed):缺 Vulkan 驱动 / 无 graphics queue / pipeline 创建失败 / image 格式不支持 → 确定性 `Err`(非 panic,P-01,无静默 fallback,不占 RX 码);开发期 `RURIX_VK_VALIDATION=1` 装 `VK_EXT_debug_utils` messenger,`VK_LAYER_KHRONOS_validation` 的 **ERROR 级校验消息经回调翻 `Err`**(退出码判红,承 red_self_test 反证)。
- L4(present,win32 已落地 / android+AMD 尾门):swapchain / 窗口 present(平台 surface + `VK_KHR_swapchain` + `vkAcquireNextImageKHR`/`vkQueuePresentKHR` + semaphore)。**W6:win32 present 已在本机 NVIDIA/Windows 真跑落地并数值校验**——`vk::run_graphics_present` 建隐藏 win32 窗口(`VK_KHR_win32_surface`)+ swapchain(imageUsage `COLOR_ATTACHMENT|TRANSFER_SRC`)→ 渲染 N 帧居中三角形到 swapchain image → **`vkCmdCopyImageToBuffer` 回读像素断言**(反证「present 无 headless 数值校验」的 defer 理由:swapchain-image readback 即可数值对照)→ 转 `PRESENT_SRC_KHR` → `vkQueuePresentKHR` 逐帧 `VK_SUCCESS`/`SUBOPTIMAL` + validation 零报错。**RD-032 的 code-deferral(win32 present 代码面)由此 discharge**;**尾门维持 open**:AMD 真卡 present 像素校验 = **G-MB1-6**、Android surface present(`VK_KHR_android_surface` on-device 出图循环)= **G-MB1-7**(缺硬件,不设 CI 硬门)。非 Windows → `run_graphics_present` 确定性 `Err`(windows-only)。

#### Dynamic Semantics

`run_graphics_offscreen(vs, fs, vertices, vertex_stride, attrs, W, H, clear)` 确定性渲染到 device-local color image(`R8G8B8A8_UNORM`,usage COLOR_ATTACHMENT|TRANSFER_SRC)→ renderpass finalLayout 转 TRANSFER_SRC_OPTIMAL → `vkCmdCopyImageToBuffer` 到 host-visible+coherent buffer → 回读紧凑 RGBA8(`W*H*4`);单 graphics queue 同步(`vkQueueWaitIdle`)后像素确定。good 路 `VK_LAYER_KHRONOS_validation` 零报错(stderr 静默)。

#### Implementation Requirements

- IR1(手写 FFI,U27):graphics VkStruct `#[repr(C)]` 逐字节对齐 + 句柄(image/imageView/renderPass/framebuffer/buffer/memory/shaderModule/pipeline/commandPool/messenger)线性配对 create/destroy(逆序销毁,无泄漏/双释放);gate feature `vulkan` 默认关闭,**CUDA 路零回归**(`cargo build/test -p rurix-rt` 默认零改动)。unsafe 边界注册 `unsafe-audit/rurix-rt.md` **U27**。
- IR2(codegen provenance gate):`Builder.emit_provenance`(DXIL 路 `true` / Vulkan 原生路 `false`)门两处 `UserSemantic` emit 点(I/O + 资源)→ `used_user_semantic` 保持 false → `OpExtension SPV_GOOGLE` 自然不 emit;`emit_spirv_body_vulkan`(provenance=false)路由,`vulkan_codegen.rs` 图形阶段改调之(唯一路由改)。dxil-backend 单独启用 test 字节不变(反证:diff `.spv` 仅少 `UserSemantic`/`OpExtension`)。
- IR3(真跑校验):本机 NVIDIA RTX 4070 Ti offscreen 居中三角形 → readback → 像素断言(背景角 == clear / 中心覆盖非背景 / 覆盖计数 > 0)+ validation 零报错;经 `bin/vk_triangle` + `ci/vulkan_graphics_smoke.py`(step 56,`RURIX_REQUIRE_REAL=1` GPU runner)。red_self_test 反证:provenance-带保名 `.spv` 喂同管线 → VUID-...-08742 → 退出码判红(证方案真实 + validation-vs-runtime 诚实:两变体 spirv-val 皆 accept)。**W6 present 真跑校验**:本机 NVIDIA/Windows win32 swapchain present(`vk::run_graphics_present`)渲染 N 帧 → swapchain-image readback 同像素断言 + `vkQueuePresentKHR` 逐帧成功 + validation 零报错(present 路同 messenger fail-closed,red_self_test 经 VUID-...-08742 判红);经 `bin/vk_present` + `ci/vulkan_present_smoke.py`(step 58,`RURIX_REQUIRE_REAL=1` GPU runner)。AMD 真卡 present = **G-MB1-6**、Android surface present = **G-MB1-7** open 尾门(RD-032)。
- IR4(锚定):≥1 `//@ spec: RXS-0210`。

> 锚定测试:conformance/vulkan/accept/vk_tri_vs.rx + conformance/vulkan/accept/vk_tri_fs.rx(codegen 面,`spirv-val --target-env vulkan1.0` accept 且无 `SPV_GOOGLE`)+ src/rurixc/src/dxil_spirv.rs 单测(`vulkan_variant_omits_user_semantic_and_extension` / `dxil_variant_keeps_user_semantic_and_extension`)+ src/rurix-rt/src/bin/vk_triangle.rs(本机 NVIDIA offscreen 真跑,像素断言)+ src/rurix-rt/src/vk.rs 单测(`present_swapchain_negotiation_helpers`——win32 swapchain extent/format/image-count 协商纯 host)+ src/rurix-rt/src/bin/vk_present.rs(本机 NVIDIA/Windows win32 present 真跑,swapchain-image readback 像素断言)。

### RXS-0211 Android 移植缝与交叉构建

跨端第三后端的 Android 落地缝:把运行时的链接期 OS 符号抽为 per-OS cfg 分叉的 `loader` 子模块,消除 aarch64-android 链接期未定义符号缝。运行时有**两处** per-OS 链接期缝,均须 cfg 分叉:① **Vulkan loader**(`vk.rs`,feature `vulkan`;`vulkan-1.dll`/`libvulkan.so` 动态加载原语 `LoadLibraryA`/`GetProcAddress` vs `dlopen`/`dlsym`;其余 ~35 Vulkan 命令均经 `vkGetInstanceProcAddr` 运行时解析,零链接期 Vulkan 符号);② **CUDA Driver 装载器**(`sys.rs`,**默认恒编译**;`nvcuda.dll`/`libcuda.so` 同 `LoadLibraryA`/`GetProcAddress` vs `dlopen`/`dlsym`——经 `backend.rs` `CudaBackend`→`Context`→`Cuda::load` 拉入,**每个 bin 均触**,故其链接期缝 W3 SKIP 时被遮蔽,真 NDK 交叉构建方暴露〔勘误 W8,v1.12〕)。`extern "system"` 在 aarch64-android == AAPCS64 == `extern "C"`,故 Vulkan/CUDA 函数指针类型零改动即在 Android ABI 正确;android 无 `libcuda.so` → `dlopen` 返回 null → `Cuda::load` 返回 `None` → CUDA 运行期不可用(诚实降级,android 本无 NVIDIA CUDA)。**构建绿即达标**;真机 on-device saxpy 数值回读 + ANativeWindow 出图为 open 尾门 G-MB1-7(无 android runner)。

#### Syntax

无新语法(运行时/工具链面;着色阶段语法承 RFC-0002 RXS-0153~0156,不在本条)。

#### Legality

- L1(OS 加载缝,per-OS 唯一确定):**两处**加载缝同构 cfg 分叉。① Vulkan(`vk.rs`,feature `vulkan`):`cfg(windows)` → 库名 `vulkan-1.dll` + 加载原语 `LoadLibraryA`/`GetProcAddress`(Win32 kernel32);`cfg(not(windows))` → 库名 `libvulkan.so` + `dlopen(RTLD_NOW)`/`dlsym`(POSIX;Android/Linux libc 直接提供)。库名与加载原语由 OS 唯一确定;缺库/缺 `vkGetInstanceProcAddr` → 确定性 `Err` 非 panic(承 RXS-0207 L2,P-01)。② **CUDA Driver(`sys.rs`,默认恒编译)**:`cfg(windows)` → 库名 `nvcuda.dll` + `LoadLibraryA`/`GetProcAddress`;`cfg(not(windows))` → 库名 `libcuda.so` + `dlopen(RTLD_NOW)`/`dlsym`。库名/原语由 OS 唯一确定;缺库(含 android 无 `libcuda.so`)→ `Cuda::load` 返回 `None` → CUDA 运行期不可用(承既有「缺 nvcuda → CUDA 不可用」诚实降级,08 §2.5;非 panic)。两缝的 `dlopen`/`dlsym` 由 libc 提供,消除 aarch64-android 链接期未定义符号。
- L2(Android present 缝):`cfg(target_os = "android")` 经扩展 `VK_KHR_surface` + `VK_KHR_android_surface` 从 `ANativeWindow*` 建 `VkSurfaceKHR`;compute 路径**不启用** surface 扩展(`run_compute` 的 InstanceCreateInfo 维持 `enabled_extension_count = 0`),故 present 复杂度与 compute 正交。
- L3(构建降级):缺 NDK / 缺 `aarch64-linux-android` rustup target → 交叉构建门 **SKIP**(dev-env 降级,非 fake pass);专用 android-build runner 经 `RURIX_REQUIRE_ANDROID=1` 把「缺 NDK/target」翻硬红(仍不覆盖 on-device,on-device 恒 G-MB1-7 open)。

#### Dynamic Semantics

加载缝对 `run_compute` **语义中性**:同一 Phase 1 `.spv` 在桌面(`vulkan-1.dll`)与 Android(`libvulkan.so`)消费,SPIR-V 字流与 compute 结果不因 OS 改变(承 RXS-0207/0208 marshalling 单一事实源)。`load_vulkan_loader` 仅按 OS 选库名 + 加载原语,不 `close`/`FreeLibrary`(进程常驻,镜像 sys.rs nvcuda.dll 纪律)。Android present 为 on-device 语义,属尾门 G-MB1-7。

#### Implementation Requirements

- IR1(cfg-gated loader,Windows 零漂移):**两处** `mod loader` cfg 分叉,`cfg(windows)` 分支逐调用等价现行实现——① `vk.rs` `load_vulkan_loader`(`open`=`LoadLibraryA` / `sym`=`GetProcAddress` / `VULKAN_LIB`=`vulkan-1.dll`);② `sys.rs` `Cuda::load`(`open`=`LoadLibraryA` / `sym`=`GetProcAddress` / `CUDA_LIB`=`nvcuda.dll`),NVIDIA/桌面 CUDA + Vulkan 路径行为字节不变(内 `unsafe` 块纯 lint 构造,零码生成差异)。
- IR2(交叉 build 绿):`cargo build -p rurix-rt --features vulkan --target aarch64-linux-android` 链接无未定义符号——**真 NDK(r27d)交叉构建 lib + 全部 bin(`vk_saxpy`/`vk_triangle`/`vk_present`/`saxpy`/`fatbin_saxpy`)均产 aarch64 ELF**(勘误 W8:含拉入 `sys.rs` CUDA 装载器的 CUDA-demo bin,其链接期缝经 ② cfg 分叉消除;W3 honest-SKIP 时被遮蔽)。
- IR3(平台无关单测 + present 编译):`entry_point_name`(承 RXS-0207)+ 加载缝库名选择 host 绿;`#[cfg(target_os="android")] android_present`(`vkCreateAndroidSurfaceKHR` FFI stub,`#[repr(C)]` 逐字节对齐)随 android target 编译绿。
- IR4(锚定):≥1 `//@ spec: RXS-0211`(`ci/trace_matrix.py --check` 全锚定 +1)。
- IR5(NVIDIA/CUDA 零回归):default 构建/测试字节不变(feature `vulkan` 默认关闭,`cargo build/test -p rurix-rt` 零改动)。

> 锚定测试:src/rurix-rt/src/vk.rs 单测(`loader_seam_selects_platform_lib`——加载缝库名 per-OS 唯一 + 平台无关 entry-name 编排,纯 host 无设备)+ CI `ci/vulkan_android_build_smoke.py`(NDK+target 在位 → 交叉 build 绿;缺 → SKIP dev-env 降级)。on-device saxpy 数值精确回读 + ANativeWindow present 出图 = G-MB1-7 open 尾门。

### RXS-0212 SPIR-V/glslang 工具链定位与 fail-closed gate 三态

Vulkan 后端 codegen 期外部工具链(SPIR-V 校验器 `spirv-val`、若用 `glslang`)的**定位顺序**与**验证 gate 三态**语义。定位承 DXIL 第二后端 `RURIX_DXC`/`RURIX_LLC`(RXS-0157)与 `locate_spirv_val`(RXS-0161)既有 env>PATH never-None spawn-probe 先例;gate 承 RXS-0073 ptxas 干验证「缺工具 SKIP 非 fake pass」纪律。工具缺失是**开发环境降级**(dev-env degrade),不是编译失败——不占 RX 码(工具层口径,对齐 spec/release.md §3);真实红绿在带 Vulkan SDK 的 dev/CI 环境。本条为 codegen 期 locator 语义,Vulkan **运行期**驱动定位(`vulkan-1.dll`/`libvulkan.so`)属 RXS-0207/0211 加载缝,不在本条。

#### Syntax

无语言文法面(codegen/工具链面;工具 env 键 `RURIX_SPIRV_VAL`/`RURIX_GLSLANG` 承既有 `RURIX_*` 覆盖集,不入语言文法)。

#### Legality

- L1(定位顺序,承 dxil 先例):`spirv-val` 定位序 = env `RURIX_SPIRV_VAL`(绝对路径,`.is_file` 命中)> PATH `spirv-val`(按名 never-None,交由 spawn 判定);`glslang`(若用)同理(env `RURIX_GLSLANG` > PATH)。locator **恒不返回 `None`**(PATH-defer 候选按名返回,可用性由 spawn 阶段裁定),与 `locate_dxc`/`locate_spirv_cross` 纪律一致。
- L2(gate 三态,fail-closed):`SpirvValGate{Accepted, Rejected, Skipped}`——工具在位 + 产物合规(退出码 0)→ `Accepted`;工具在位 + 产物违规(退出码非 0)→ `Rejected`(codegen 红,driver 归 **RX6026**,承 RXS-0200 L3);**工具缺失(spawn 失败)→ `Skipped`(dev-env degrade,非 fake pass,不占 RX 码)**。`RURIX_REQUIRE_REAL=1`(GPU/CI runner)把 `Skipped` 翻硬红(runner 应有 Vulkan SDK,缺即环境异常)。
- L3(退出码判定):gate 判定用进程**退出码**,**非 grep stdout**(反 Godot 崩溃判定教训——文本匹配 / 连环 device-removal 污染态致误判;崩溃/合规判定恒用退出码)。

#### Dynamic Semantics

driver `--target vulkan` 消费 gate(`compile_vulkan_target`):`Accepted` → 静默产 `.spv`(退出 0);`Rejected(reason)` → **RX6026** 编译期 codegen 诊断携原因、无运行期 fallback(P-01 strict-only);`Skipped` → 打印提示「set RURIX_SPIRV_VAL / install Vulkan SDK … validator gate SKIPPED」后按已产 `.spv` 落盘退出 0(dev-env degrade,真实红绿延后到带 SDK 环境)。给定同一 `.spv` 与同一工具,gate 判定确定(退出码确定)。CI `ci/vulkan_codegen_smoke.py` 复用同定位/SKIP 语义(缺 spirv-val → 校验段 SKIP exit 0;`RURIX_REQUIRE_REAL=1` → 缺工具翻硬红)。

#### Implementation Requirements

- IR1(locator + gate):`src/rurixc/src/toolchain.rs` `locate_spirv_val`(env>PATH never-None,`any(dxil-backend, vulkan-backend)` 提级)+ `spirv_val_gate(spv) -> SpirvValGate`(feature `vulkan-backend`);driver `compile_vulkan_target` 三态分派(`src/rurixc/src/driver.rs`)。
- IR2(fail-closed 纪律):缺工具恒 `Skipped` 标 dev-env degrade,**绝不 fake success**(P-01;对齐 RXS-0073 ptxas 干验证 SKIP);`RURIX_REQUIRE_REAL=1` 由 CI 层(`ci/vulkan_codegen_smoke.py`)兑现翻硬红,codegen 库层维持三态。
- IR3(锚定):≥1 `//@ spec: RXS-0212` host 测,覆盖定位顺序(env 绝对路径 > PATH,never-None)+ gate 三态(`Accepted`/`Rejected`/`Skipped`)+ **缺工具 → `Skipped` 非 fake pass**(非法字节恒不 `Accepted`;退出码判定,反 grep)。

> 锚定测试:src/rurixc/src/toolchain.rs 单测(`spirv_val_locate_order_and_gate_tristate`——最小合法 GLCompute 模块 → `Accepted`〔工具在位〕/ 非法字节 → `Rejected`〔工具在位真跑拒绝〕/ 工具缺失 → `Skipped`〔恒不 fake-accept〕+ `locate_spirv_val` 定位序)+ CI `ci/vulkan_codegen_smoke.py`(spirv-val 缺 → 校验 SKIP exit 0;`RURIX_REQUIRE_REAL=1` 翻硬红)。

### RXS-0213 Vulkan 绑定供应链纪律(手写薄 loader,零外部绑定 crate)

Vulkan 运行时绑定与 SPIR-V codegen 的**供应链纪律**:运行时绑定 = 手写薄 `vulkan-1`/`libvulkan` FFI loader(仿 `sys.rs` nvcuda.dll 动态加载纪律),codegen 侧 SPIR-V 自包含(纯 Rust MIR→SPIR-V emitter)——两侧均**零外部 Vulkan/SPIR-V 绑定 crate**。承 RFC-0011 §4.12 / §9 Q-Binding 默认(默认倾向手写薄 loader:对齐 `sys.rs` 无外部绑定纪律、unsafe 集中 U26/U27、`dlopen` 缝天然适配 Android)。供应链 provenance 是**静态构建期事实**(依赖图属性),可由 manifest / lockfile 机械校验,非运行期语言语义。

#### Syntax

无语言文法面(运行时 / 构建工程面)。

#### Legality

- L1(手写薄 loader,零外部绑定):Vulkan 运行时绑定 = 手写薄 `vulkan-1`/`libvulkan` FFI loader(`src/rurix-rt/src/vk.rs`,unsafe-audit U26/U27),**无外部 Vulkan 绑定 crate**——不引入 `ash`/`vulkano`/`erupt`/`gpu-alloc`;codegen 侧 SPIR-V 由 `src/rurixc/src/vulkan_codegen.rs` 纯 Rust emitter 产出,**无外部 SPIR-V crate**(rspirv/spirv-tools/…)。
- L2(供应链 provenance,极简依赖):零新增外部运行时依赖 → 攻击面 / 审计面最小(项目极简依赖、pin 一切纪律);`vulkan` feature `= []`(空依赖集,不引入 dep),`rurix-rt` `[dependencies]` 仅可选 `rurix-d3d12`(G1.1 互操作,与 Vulkan 后端正交、默认关闭)。
- L3(前瞻 pin 策略,非 defer):若未来改采 `ash`(RFC-0011 §9 Q-Binding,owner 可裁)→ **必** pin 于 `Cargo.lock` + 记 `rurix.lock [[toolchain]]` + SHA256(承 G1.5 `[[artifact]]` digest / RXS-0157 dxc pin 先例)。当前无该依赖对象 → **直接声明策略**(前瞻性口径,**非 honest-defer**,零新 RD:无缺席对象须 backfill)。

#### Dynamic Semantics

无运行期语言语义:绑定供应链为构建期依赖图属性 + 运行期 `dlopen` 加载缝(加载缝语义属 RXS-0211)。静态事实:`rurix-rt` / `rurixc` 依赖图不含外部 Vulkan/SPIR-V 绑定 crate,确定性可由 `Cargo.toml` / `Cargo.lock` 校验(同一 manifest 校验结果确定)。

#### Implementation Requirements

- IR1(rurix-rt 零外部 Vulkan 绑定):`src/rurix-rt/src/vk.rs` 手写 FFI(dispatchable/non-dispatchable 句柄类型 + `#[repr(C)]` 逐字节对齐,~35 Vulkan 命令经 `vkGetInstanceProcAddr` 运行时解析)+ `src/rurix-rt/Cargo.toml` `[dependencies]` 无 `ash`/`vulkano`/`erupt`/`gpu-alloc`;`vulkan = []` 空依赖集。
- IR2(rurixc codegen 自包含):`src/rurixc/src/vulkan_codegen.rs` 纯 Rust SPIR-V emitter(`emit`/`words_to_bytes` 手写字流),`src/rurixc/Cargo.toml` 无外部 SPIR-V crate。
- IR3(锚定):≥1 `//@ spec: RXS-0213` host 测——解析 `rurix-rt`(`env!("CARGO_MANIFEST_DIR")`)+ `rurixc` 的**真** `Cargo.toml` 依赖清单,断言不含外部 Vulkan 绑定(`ash`/`vulkano`/`erupt`/`gpu-alloc`)/ SPIR-V crate,且 `vulkan = []` 空依赖集(非内联复刻,直接校验真 manifest)。

> 锚定测试:src/rurix-rt/src/vk.rs 单测(`binding_supply_chain_no_external_vulkan_crate`——解析真 `rurix-rt`/`rurixc` Cargo.toml 依赖行,断言零外部 Vulkan/SPIR-V 绑定 crate + `vulkan` feature 空依赖)。

### RXS-0230 Vulkan graphics descriptor 运行时建面（`run_graphics_offscreen_v2` 加性；RFC-0013 §4.B7，后续三面共用底座）

> **编号续号说明**:本条 RXS-0230 由 **RFC-0013**(§4.B7)新增,超 mb1 的 RXS-0200~0213 区间(独立 RFC 续号,跳 RXS-0214~0229 = 采样章其余落点 / EI1 earmark 避撞)。现状:`run_graphics_offscreen`(src/rurix-rt/src/vk.rs)**零 descriptor 面**,纹理/采样器/storage image 在 Vulkan 腿运行时结构不可达(vk.rs 无 resource 形参)。本面新建 graphics descriptor 运行时底座——**采样/bindless/graph/present 四面单点关键依赖**(PR-S0 独立先落地,E-5)。

#### Syntax

无语言文法面(运行时 FFI 面)。

#### Legality

- L1(加性 API,命名律 §4.0-5):新增 `run_graphics_offscreen_v2`——**v1(`run_graphics_offscreen`)签名与行为 0-byte 保留**(MB1 语料零回归,步骤 56 offscreen 三角形仍真跑 PASS);v2 追加 `resources: &[GraphicsResource]`(纹理含逐层 mip 数据、SamplerDesc、storage image)。既有调用方零重编译(加性)。
- L2(绑定方案 = 单一 binding-号事实源 + 两套 set 分配策略,Q-S-BindingScheme,E-3):现行 `infer_spirv_bindings`(binding_layout.rs:139)硬编码 `set:0` + per-class binding——B 链经 spirv-cross 映射 register 正确,但原生 Vulkan 消费下四类轴 binding 0 互撞。拟裁:**单一 binding-号事实源**(binding 号一处推导,同 RXS-0164 register 事实源)+ **按目标选择的两套 set 分配策略**——**B 链形态维持现装饰字节不动**(零 golden 重 bless);**Vk-native 形态** `set = 类别轴(0=CBV/1=SRV/2=UAV/3=Sampler)、binding = 类内序`(bindless 无界表自 set4 起,§4.0-1)。目标选择由既有 provenance/mode 旗标承载(现门控 UserSemantic 发射,本面显式扩为亦门控 descriptor-set 装饰)。
- L3(static/immutable sampler):静态 sampler(RXS-0224)降级 Vulkan immutable sampler(`VkDescriptorSetLayoutBinding.pImmutableSamplers`);`serialize_rts0` 扩 `NumStaticSamplers`(现恒 0)对应 D3D12 static sampler。
- L4(SKIP 三态):无显示 / 无 Vulkan → SKIP 三态(`RURIX_REQUIRE_REAL=1` 翻硬红);底座数值见证滑动时下游四面走尾门降级路径不阻塞条款合入。

#### Dynamic Semantics

`run_graphics_offscreen_v2` 内部建 `VkDescriptorSetLayout`(含 immutable samplers)/ `VkDescriptorPool` / `vkUpdateDescriptorSets` / `vkCmdBindDescriptorSets`,mip 链经 staging 逐层 upload + layout 迁移;storage image(UAV 轴)走 `VK_DESCRIPTOR_TYPE_STORAGE_IMAGE`。给定同一 `resources` 与 `.spv`,descriptor 建面 / 绑定序确定(binding 号单一事实源)。运行期 present / readback 语义承 RXS-0210(offscreen)不变。

#### Implementation Requirements

- IR1(descriptor 建面):`run_graphics_offscreen_v2` 建 sampled image(SRV 轴)/ sampler(Sampler 轴,immutable 者经 layout binding)/ storage image(UAV 轴)descriptor;set 分配按 L2 Vk-native set-per-class;mip 链 staging 逐层 upload + `vkCmdPipelineBarrier` layout 迁移。
- IR2(单一 binding-号事实源两策略):binding 号由 `binding_layout`(同 RXS-0164 register 事实源)一处推导;set/space 分配为按目标(B 链 / Vk-native)选择的两套策略(E-3;非「一处推导两形态」的含糊)。SPIR-V 装饰与 RTS0 序列化共同消费。
- IR3(unsafe 折叠 U27):新 unsafe FFI 逐处 `// SAFETY:` **折叠进 U27 扩注**(graphics FFI 边界内,**0 新号**,§6.4/E-2)。
- IR4(合入门 = B 链字节 diff golden,E-3):**「混合有界+无界并存 + 多表 + 四类别齐全」压测语料的 B 链 SPIR-V 字节 diff golden**(机核 B 链装饰字节不动,承 binding_layout.rs:144-147 device bug 教训),**非仅 UI golden**;conformance 另断言两形态除 `Decorate` 外指令流逐字相等(反双产物漂移)。
- IR5(测试锚定):≥1 `//@ spec: RXS-0230` 覆盖 vk.rs host 单测(建面/上传纯 host 可测部分 + set-per-class 分配)+ v1 语料回归(offscreen 三角形 0-byte)+ 步骤 63 Vulkan 腿同判据 + 混合有界+无界 B 链字节 diff golden。

### RXS-0246 MIR→SPIR-V mesh/task 编码（MeshEXT/TaskEXT + SPV_EXT_mesh_shader，RFC-0013 §4.E5）

> **落点(E-4 采纳,钉死)**:mesh/task 为 **workgroup 语义**(`#[numthreads]`/LocalSize/`TaskPayloadWorkgroupEXT`),编码**复用 [`vulkan_codegen`](../src/rurixc/src/vulkan_codegen.rs) 的 GLCompute/LocalSize/workgroup 基建**,不在 `dxil_spirv.rs` 重复实现 LocalSize/workgroup;mesh 的 vertex-out→fs-in I/O 装饰面承既有装饰机制。零漂移门显式跨两个发射器界定(`dxil_spirv.rs` 变 / `vulkan_codegen.rs` 既有 GLCompute golden 不动)。

#### Syntax

无用户面语法(codegen 内部);消费 RXS-0243 mesh/task 入口契约。

#### Legality

- **执行模型**:`OpEntryPoint MeshEXT` / `TaskEXT` + capability `MeshShadingEXT` + `OpExtension "SPV_EXT_mesh_shader"`。
- **execution modes**:`LocalSize`(承 `#[numthreads]`)+ `OutputVertices N` + `OutputPrimitivesEXT M` + `OutputTrianglesEXT`(承 `#[outputs]`)。
- 子集外构造 / SPIR-V emit 失败 / spirv-val 拒 → `RX6026` 类别扩充(RXS-0200 L2/L3 既有语义面)。

#### Dynamic Semantics

- **mesh 输出**:`Position` builtin(per-vertex Block 成员)+ varying Location 数组(Output 存储类)+ `PrimitiveTriangleIndicesEXT`(uvec3 数组);`set_mesh_outputs` → `OpSetMeshOutputsEXT`。
- **task→mesh payload**:`TaskPayloadWorkgroupEXT` 存储类变量;`emit_mesh_tasks` → `OpEmitMeshTasksEXT`(块终结子,无 `OpReturn`)。
- **SPIR-V 版本**:mesh/task 入口随 RXS-0247 版本分叉走 **1.4** 口径 emit(interface 全量枚举);全部产物过 `spirv-val --target-env vulkan1.2`/`spv1.4` 三态 gate(承 RXS-0212)。

#### Implementation Requirements

[`vulkan_codegen`](../src/rurixc/src/vulkan_codegen.rs) `emit_mesh_min`/`emit_task_min`(`ExtBuilder`,header 恒 [`SPIRV_VERSION_1_4`]);首期产**固定最小合规模块**(单三角形非空输出 / payload 写 + EmitMeshTasks,库级见证镜像 sampling/bindless corpus 无 CLI 见证),从真实 `.rx` MIR 体的 SetMeshOutputs/EmitMeshTasks intrinsic 降级接线归后续 PR(诚实标注)。

> 锚定测试:[`vulkan_codegen::tests::mesh_entry_point_is_mesh_ext_model`](../src/rurixc/src/vulkan_codegen.rs)（MeshEXT 执行模型）+ [`tests/mesh_rt_vulkan_spirv_val.rs`](../src/rurixc/tests/mesh_rt_vulkan_spirv_val.rs)（mesh/task .spv → `spirv-val --target-env vulkan1.2`/`spv1.4` accept）。

### RXS-0247 MIR→SPIR-V RT 六执行模型编码 + SPIR-V 1.4 per-entry 分叉（RFC-0013 §4.E6）

#### Syntax

无用户面语法(codegen 内部);消费 RXS-0244/0245 RT 类型面契约。

#### Legality

- **六执行模型**:`RayGenerationKHR` / `IntersectionKHR` / `AnyHitKHR` / `ClosestHitKHR` / `MissKHR` / `CallableKHR`,capability `RayTracingKHR` + `OpExtension "SPV_KHR_ray_tracing"`。
- **存储类**:`RayPayloadKHR` / `IncomingRayPayloadKHR` / `HitAttributeKHR` / `CallableDataKHR` / `IncomingCallableDataKHR`;`ShaderRecordBufferKHR` 不进首期(§8)。
- **指令族**:`OpTraceRayKHR` / `OpReportIntersectionKHR` / `OpExecuteCallableKHR`;`OpTypeAccelerationStructureKHR` + descriptor 装饰(承 RXS-0163 推导新类别,SRV 轴 UniformConstant)。
- **校验轴**:合规判定以 `spirv-val` **退出码**为准,**不以驱动宽容度为准**(NVIDIA 驱动可能接受不合规组合,不得据此免分叉)。

#### Dynamic Semantics

- **1.4 分叉(硬边界,Q-M-SpirvVersion)**:RT 腿硬性要求 SPIR-V **1.4**(`VK_KHR_ray_tracing_pipeline` 依赖 `VK_KHR_spirv_1_4`);1.4 起 `OpEntryPoint` interface 须枚举**全部**被引用全局变量。分叉形态 = **per-entry 版本轴**:mesh/RT 入口 emit **1.4** + interface 全量;**既有 compute/vertex/fragment 入口维持 1.0 emit,产物字节零漂移**(既有 vulkan golden 不重 bless、DXIL B 路消费的 SPIR-V 字节不变)。
- **零回归门**:dxil 套件恒定 + vulkan 既有 golden 字节 diff 空 + `spirv-val --target-env vulkan1.2` 双口径皆 accept。分叉落**发射函数级**:compute [`assemble`](../src/rurixc/src/vulkan_codegen.rs)(1.0)/ vertex·fragment [`dxil_spirv::emit_spirv_inner`](../src/rurixc/src/dxil_spirv.rs)(1.0)不变;mesh/task/RT [`ExtBuilder::finish`](../src/rurixc/src/vulkan_codegen.rs)(1.4)。

#### Implementation Requirements

[`vulkan_codegen`](../src/rurixc/src/vulkan_codegen.rs) `emit_raygen_min`/`emit_miss_min`/`emit_closesthit_min`/`emit_anyhit_min`/`emit_intersection_min`/`emit_callable_min`(六模型全覆盖);header 恒 [`SPIRV_VERSION_1_4`];intersection/callable/anyhit 首期 accept-only(§8,device 端到端见证 defer RD-034)。

> 锚定测试:[`vulkan_codegen::tests`](../src/rurixc/src/vulkan_codegen.rs)`::mesh_rt_entries_emit_1_4_and_full_interface`（1.4 版本轴 + compute 1.0 零漂移锚点）+ `::raygen_entry_point_is_ray_generation_khr` + [`tests/mesh_rt_vulkan_spirv_val.rs`](../src/rurixc/tests/mesh_rt_vulkan_spirv_val.rs)（六模型 .spv × `vulkan1.2`/`spv1.4` accept + 1.4 header 机核）。

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
| v1.7 | 2026-07-15 | **MB1.2 marshalling + artifact 泛化:落带编号条款体 `### RXS-0208` / `### RXS-0209`**。RXS-0208(launch marshalling):运行期 ordinal → `(set 0, binding i)` StorageBuffer + 标量 push-constant 顺排,与 codegen RXS-0203 描述符装饰**单一事实源**(同源于形参出现序);L3 fail-closed 经 Vulkan validation 拒(不占 RX 码);IR2 🔒 `rxrt_launch` 字节本体 §4.7 禁区不落;IR3 前瞻兼容 **honest-defer RD-030**(mb1 base 无 rxrt_launch → ABI 回归对象缺席)。RXS-0209(artifact 泛化):`ArtifactKind::Spirv` + `SmTarget → ArchKey{Sm/Gfx/SpirvPortable}` prefix-dispatch(泛化 G1.5 硬 `sm_` 守卫误拒 `gfx1100` 的点)+ `DeviceArtifactSet` SPIR-V 可移植槽 + `rurix.lock` format-generic 承 `kind="spirv"`/`sm_target="gfx1100"`(零 schema 改);IR2 描述表 v2 blob **honest-defer RD-031**(mb1 base 无 @__rx_gpu_artifacts blob / emit_gpu_artifact_globals)。配套 `src/rurix-rt/src/fatbin.rs`(ArtifactKind/ArchKey/DeviceArtifactSet 加性 + ripple)/ `lib.rs`·`bin/fatbin_saxpy.rs`(名替)/ `vk.rs`(marshalling 单测)/ `rurix-rt/build.rs`(经 vulkan_codegen 产真 saxpy `.spv` 供锚点解析)/ `src/rurix-pkg/src/lock.rs`(doc-comment + roundtrip 变体,schema 零码改)。条款体按 FLS 分 Syntax/Legality/Dynamic Semantics/Implementation Requirements,**严禁 UB 节**。**真实红绿**:RXS-0208 锚点消费 `rurixc --target vulkan` 对 saxpy 的**真** `.spv`,解析实际 `OpDecorate Binding` = [0,1,2] / `OpMemberDecorate Offset` = [0,4],核对 = 运行时 descriptor-binding 构造序(非内联复刻规则);RXS-0209 纯 host 类型回归网不依赖 GPU 而绿。**NVIDIA 零回归**:`ArchKey::from_capability` 恒产 `Sm`、NV-only 集 `spirv_fallback=None` → 未命中恒 `PtxFallback`(cubin/ptx 路径逐字节不变),`cargo test -p rurixc --features dxil-backend` 404 不变。`ci/trace_matrix.py` 全锚定 **192→194**(RXS-0208 vk.rs 单测 + RXS-0209 fatbin.rs/lock.rs 单测)。**第二 ICD(lavapipe)+ AMD 真卡维持 open 尾门**;RXS-0210 present / RXS-0211 Android 随后续分片。档位 **Full RFC**(RFC-0011)。无体例变更 | **Full RFC**（RFC-0011） |
| v1.8 | 2026-07-15 | **MB1 Phase 3 graphics + offscreen present:落带编号条款体 `### RXS-0210`**(Vulkan graphics 运行时 + offscreen present)+ 承 **RXS-0204 勘误**(provenance 装饰改 target-conditional)。**codegen 方案 B**(`dxil_spirv.rs`):`Builder.emit_provenance`(DXIL 路 true / Vulkan 原生路 false)门两处 `UserSemantic` emit → `emit_spirv_body_vulkan`(去 `UserSemantic`/`SPV_GOOGLE`)+ `vulkan_codegen.rs` 图形阶段改调之(唯一路由改);修 `vkCreateShaderModule` VUID-...-08742(去 device 扩展 `VK_GOOGLE_hlsl_functionality1` 依赖,跨 ICD 可移植)。**graphics 运行时**(`src/rurix-rt/src/vk.rs`,feature `vulkan`):新增 render pass(单 color attachment CLEAR→STORE→TRANSFER_SRC)+ graphics pipeline(vertex+fragment,pName `"main"`)+ framebuffer + 顶点缓冲 + `vkCmdDraw` + `vkCmdCopyImageToBuffer` 回读 `pub fn run_graphics_offscreen`(unsafe-audit **U27**)+ `VK_EXT_debug_utils` messenger fail-closed(ERROR 级校验消息翻 `Err`,退出码判红)+ `bin/vk_triangle` 真跑 demo。条款体按 FLS 分 Syntax/Legality/Dynamic Semantics/Implementation Requirements,**严禁 UB 节**;present 窗口/平台 surface → honest-defer **RD-032**。配套 conformance accept `vk_tri_vs.rx`/`vk_tri_fs.rx`。**真实红绿(本机 NVIDIA RTX 4070 Ti,Vulkan 1.4.351)**:offscreen 居中三角形 → readback 像素断言(背景角==clear / 中心覆盖非背景 / covered=968)+ `VK_LAYER_KHRONOS_validation` **零报错**(stderr 静默);**red_self_test 反证**:provenance-带保名 `.spv` 喂同管线 → VUID-VkShaderModuleCreateInfo-pCode-08742 → 退出码判红(两变体 spirv-val 皆 accept,证「去装饰」非「产非法」)。**DXIL 字节不变实证**:同 vertex+fragment 经 DXIL 路(provenance=true)A/B sha256 逐字节相等 + `cargo test -p rurixc --features dxil-backend` **404 不变**。**NVIDIA(CUDA)零回归**(feature vulkan 默认关闭)。`ci/trace_matrix.py` 全锚定 **194→195**(RXS-0210:dxil_spirv 单测 ×2 + conformance vk_tri_{vs,fs} + bin/vk_triangle)。新增 CI **步骤 56**(`ci/vulkan_graphics_smoke.py`,G-MB1-4,`RURIX_REQUIRE_REAL=1`)。**AMD 真卡 + 窗口 present 维持 open 尾门(G-MB1-6 / RD-032)**;RXS-0211 Android 随后续分片。零新 RX 码、零新 RFC。档位 **Full RFC**(RFC-0011)。无体例变更 | **Full RFC**（RFC-0011） |
| v1.9 | 2026-07-15 | **MB1 Phase 4 Android 移植缝 + 交叉构建:落带编号条款体 `### RXS-0211`**(Android 移植缝与交叉构建)。**加载缝 cfg 分叉**(`src/rurix-rt/src/vk.rs`,feature `vulkan`):运行时唯一链接期 OS 符号(`LoadLibraryA`/`GetProcAddress`)抽为 per-OS `mod loader`——`#[cfg(windows)]` = `vulkan-1.dll` + `LoadLibraryA`/`GetProcAddress`(**Windows 路径逐调用等价旧实现、零漂移**)、`#[cfg(not(windows))]` = `libvulkan.so` + `dlopen(RTLD_NOW)`/`dlsym`(`unsafe extern "C"`,Android/Linux libc);`load_vulkan_loader` 只调抽象 `loader::open`/`sym` + 库名常量 `VULKAN_LIB`,消除 aarch64-android 链接期未定义符号缝(其余 ~35 Vulkan 命令经 `vkGetInstanceProcAddr` 运行时解析,`extern "system"`==AAPCS64==`extern "C"` 零改动)。**Android present 缝**(`#[cfg(target_os="android")] pub mod android_present`):`vkCreateAndroidSurfaceKHR` FFI stub(`#[repr(C)] AndroidSurfaceCreateInfoKHR` 逐字节对齐 + `ANativeWindow` opaque + safe `create_android_surface(...) -> Result`),compute 路径不启用 surface 扩展(与 present 正交),仅 android target 编译绿;on-device 出图 = 尾门 G-MB1-7。unsafe-audit **U26 扩注**(无新 U 号,同一 feature `vulkan` FFI 边界)。新增 `.cargo/config.toml`(`[target.aarch64-linux-android]` linker/ar,桌面 target 零影响)+ CI **步骤 57** `ci/vulkan_android_build_smoke.py`(NDK+target 在位 → 交叉 build 绿;缺 → SKIP dev-env 降级,非 fake;`RURIX_REQUIRE_ANDROID=1` 翻硬红;**不设 `RURIX_REQUIRE_REAL`**——NVIDIA runner 无 NDK 须干净 SKIP,G-MB1-5)。条款体按 FLS 分 Syntax/Legality(L1 OS 加载缝 per-OS / L2 android present / L3 构建降级 SKIP)/Dynamic Semantics(加载缝对 `run_compute` 语义中性)/Implementation Requirements(IR1 Windows 零漂移 / IR2 交叉 build 绿 / IR3 平台无关单测 + present 编译 / IR4 锚定 / IR5 NVIDIA 零回归),**严禁 UB 节**。**真实红绿(本机 NVIDIA RTX 4070 Ti,无 NDK)**:加载缝 cfg 分叉后**Windows 零漂移双证**——`ci/vulkan_device_smoke.py`(compute saxpy)+ `ci/vulkan_graphics_smoke.py`(offscreen 三角形)**均仍真跑 PASS**;新单测 `loader_seam_selects_platform_lib`(库名 per-OS 唯一 + entry-name 编排,纯 host)绿,`rurix-rt --features vulkan --lib` 20→21;`ci/vulkan_android_build_smoke.py` → **SKIP**(NDK 缺,达标非 fake)。**NVIDIA(CUDA)零回归**(feature vulkan 默认关闭,`rurix-rt --lib` 17 不变;`rurixc --features dxil-backend --lib` 404 不变)。`ci/trace_matrix.py` 全锚定 **195→196**(RXS-0211 vk.rs 单测)。**Android on-device saxpy/present 维持 open 尾门 G-MB1-7**(非新 RD:on-device 属既有硬件尾门,cross-build-green@NDK = 本条交付物)。零新 RX 码、零新 RD、零新 U、零新 RFC。档位 **Full RFC**(RFC-0011)。无体例变更 | **Full RFC**（RFC-0011） |
| v1.10 | 2026-07-15 | **MB1 W5 toolchain 定位 + 绑定供应链:落带编号条款体 `### RXS-0212` / `### RXS-0213`**(两条 formalize 既落实现,零新运行期行为)。RXS-0212(SPIR-V/glslang 工具链定位与 fail-closed gate 三态):`spirv-val` 定位序 env `RURIX_SPIRV_VAL`(绝对路径 `.is_file`)> PATH `spirv-val`(never-None,承 dxil `RURIX_DXC`/RXS-0157 先例)+ `SpirvValGate{Accepted/Rejected/Skipped}` 三态(工具在位+合规→Accepted / 在位+违规→Rejected〔driver RX6026〕/ **缺工具→Skipped dev-env degrade 非 fake pass,不占 RX 码**;`RURIX_REQUIRE_REAL=1` 翻硬红)+ **退出码判定非 grep stdout**(反 Godot 教训);消费点 driver `compile_vulkan_target` + CI `ci/vulkan_codegen_smoke.py`。RXS-0213(Vulkan 绑定供应链纪律):运行时绑定 = 手写薄 `vulkan-1`/`libvulkan` FFI loader(`vk.rs` U26/U27,仿 sys.rs)+ codegen SPIR-V 自包含(`vulkan_codegen.rs` 纯 Rust)——两侧**零外部 Vulkan/SPIR-V 绑定 crate**(无 `ash`/`vulkano`/`erupt`/`gpu-alloc`);`vulkan = []` 空依赖集;前瞻 pin 策略(若采 ash → Cargo.lock + rurix.lock [[toolchain]] + SHA256,**非 defer** 零新 RD)。承 RFC-0011 §4.11/§4.12/§9 Q-Binding。条款体按 FLS 分 Syntax/Legality/Dynamic Semantics/Implementation Requirements,**严禁 UB 节**。**真实红绿(本机 Vulkan SDK)**:RXS-0212 锚点 `spirv_val_locate_order_and_gate_tristate` 真跑 spirv-val——最小合法 GLCompute 模块→Accepted / 非法字节→Rejected(退出码非 0 真拒)/ 缺工具→Skipped(恒不 fake-accept);RXS-0213 锚点 `binding_supply_chain_no_external_vulkan_crate` 解析**真** `rurix-rt`/`rurixc` Cargo.toml 断言零外部绑定 crate + `vulkan = []`。`ci/trace_matrix.py` 全锚定 **196→198**(RXS-0212 toolchain.rs 单测 + RXS-0213 vk.rs 单测)。**零新 RX 码、零新 RD、零新 U、零新 RFC、零新 CI 步骤**(既有 `ci/vulkan_codegen_smoke.py` 覆盖 RXS-0212 SKIP 语义)。**NVIDIA(dxil-backend)零回归**(`rurixc --features dxil-backend --lib` 404 不变;RXS-0213 锚在 feature `vulkan` 门内,`rurix-rt --lib` 17 不变)。档位 **Full RFC**(RFC-0011)。无体例变更 | **Full RFC**（RFC-0011） |
| v1.11 | 2026-07-15 | **MB1 W6 win32 swapchain present 落地:discharge `### RXS-0210` L4 present-defer(NVIDIA/Windows)**,**零新条款号 / 零新 RX / 零新 RD / 零新 RFC**(present 完成既有 RXS-0210 的 L4,承 RD-032 code-deferral)。**present 运行时**(`src/rurix-rt/src/vk.rs`,feature `vulkan` + `#[cfg(windows)]`):新增 `pub fn run_graphics_present`——创建隐藏 win32 窗口(user32/kernel32 FFI:`RegisterClassW` + `CreateWindowExW` WS_POPUP 不显示 + `DestroyWindow`/`UnregisterClassW` 拆除)+ `VkSurfaceKHR`(`VK_KHR_win32_surface` `vkCreateWin32SurfaceKHR`)+ surface support/caps/formats/present-modes 协商(FIFO + `B8G8R8A8`/`R8G8B8A8` 优选 + extent from caps)+ `VkSwapchainKHR`(`VK_KHR_swapchain`,imageUsage `COLOR_ATTACHMENT|TRANSFER_SRC`)+ per-image view/framebuffer(复用 render pass,+ EXTERNAL 子通道依赖)+ semaphore×2(imageAvailable/renderFinished)+ N 帧循环(`vkAcquireNextImageKHR`→render→barrier→`vkCmdCopyImageToBuffer` 回读→转 `PRESENT_SRC_KHR`→`vkQueueSubmit`→`vkQueuePresentKHR`→`vkQueueWaitIdle`);`#[cfg(not(windows))]` stub 返回确定性 `Err`(windows-only)。复用 offscreen 的 `VK_EXT_debug_utils` messenger fail-closed。unsafe-audit **U27 扩注**(present FFI:win32 surface + swapchain + semaphore + user32 窗口,同 feature `vulkan` 边界,**无新 U 号**)+ `bin/vk_present` 真跑 demo。**真实红绿(本机 NVIDIA RTX 4070 Ti,Vulkan 1.4.351)**:win32 隐藏窗口 swapchain present 渲染 3 帧 → swapchain-image readback 像素断言(背景角==clear / 中心覆盖非背景 / covered=968,与 offscreen 同)+ `vkQueuePresentKHR` 逐帧成功 + `VK_LAYER_KHRONOS_validation` **零报错**(stderr 0 字节);**red_self_test 反证**:provenance-带保名 `.spv` 喂同 present 管线 → VUID-VkShaderModuleCreateInfo-pCode-08742 → 退出码判红(证 present 路同样 fail-closed)。**offscreen graphics + compute 零漂移**(`ci/vulkan_graphics_smoke.py` + `ci/vulkan_device_smoke.py` 仍真跑 PASS,`run_graphics_offscreen`/`run_compute` 字节行为不变)。**NVIDIA(CUDA)零回归**(feature vulkan 默认关闭,`rurix-rt --lib` 17 不变;`rurixc --features dxil-backend --lib` 404 不变);新 host 单测 `present_swapchain_negotiation_helpers`(纯 host,`rurix-rt --features vulkan --lib` 22→23)。`ci/trace_matrix.py` 全锚定 **198 不变**(present 锚 RXS-0210,`bin/vk_present.rs` + vk.rs 单测,无新条款)。新增 CI **步骤 58**(`ci/vulkan_present_smoke.py`,G-MB1-4,`RURIX_REQUIRE_REAL=1`;非 Windows SKIP)。**RD-032 code-deferral discharge;AMD 真卡 present = G-MB1-6、Android surface present = G-MB1-7 维持 open 尾门**。档位 **Full RFC**(RFC-0011)。无体例变更 | **Full RFC**（RFC-0011） |
| v1.12 | 2026-07-15 | **MB1 W8 Android 交叉构建真绿 + `### RXS-0211` 勘误(sys.rs = 第二链接期缝)**,**零新条款号 / 零新 RX / 零新 RD / 零新 U / 零新 RFC**。**勘误**:RXS-0211 v1.9 称加载缝为运行时「**唯一**链接期 OS 符号」——**不完整**;真 NDK(r27d)交叉构建暴露**第二**链接期缝:`src/rurix-rt/src/sys.rs` CUDA Driver 装载器(`nvcuda.dll` 经 `LoadLibraryA`/`GetProcAddress`,**默认恒编译**,经 `backend.rs` `CudaBackend`→`Context`→`Cuda::load` 拉入,**每个 bin 均触**;W3 SKIP 时被遮蔽)。lib 编译历来通过(rc 0),但 bin 链接期 `ld.lld: undefined symbol: LoadLibraryA/GetProcAddress` 未定义符号红。**修复**(镜像 vk.rs W3 加载缝):`sys.rs` 的 `LoadLibraryA`/`GetProcAddress` extern + `Cuda::load` 调用点抽为 per-OS cfg 分叉 `mod loader`——`#[cfg(windows)]` = 库名 `nvcuda.dll` + `LoadLibraryA`/`GetProcAddress`(`unsafe extern "system"`,**逐调用等价旧实现、字节零漂移**,内 `unsafe` 块纯 lint 构造无码生成差异)、`#[cfg(not(windows))]` = 库名 `libcuda.so` + `dlopen(RTLD_NOW)`/`dlsym`(`unsafe extern "C"`,libc 提供 → aarch64-android 链接期符号可解析);`Cuda::load` 只调抽象 `loader::open`/`sym` + 库名常量 `CUDA_LIB`,承既有「缺库 → `None` → CUDA 不可用」诚实降级(android 无 `libcuda.so` → `dlopen` 返回 null → CUDA 运行期不可用,android 本无 NVIDIA CUDA)。amend RXS-0211 开篇/L1/IR1/IR2(两处加载缝 cfg 分叉);unsafe-audit **U1 扩注**(无新 U 号,同 nvcuda FFI 边界,加 `#[cfg(not(windows))]` `dlopen`/`dlsym` 分支)。**真实红绿(本机 NVIDIA RTX 4070 Ti + NDK r27d)**:`cargo build -p rurix-rt --features vulkan --target aarch64-linux-android` **链接绿 rc 0——lib + 全部 5 bin(`vk_saxpy`/`vk_triangle`/`vk_present`/`saxpy`/`fatbin_saxpy`)均产 aarch64 ELF(e_machine=0xB7)**,含拉入 CUDA 装载器的 `saxpy`/`fatbin_saxpy`;`RURIX_REQUIRE_ANDROID=1 ci/vulkan_android_build_smoke.py` phase1 host 23 + phase2 真交叉构建 **PASS rc 0**(不再 SKIP)。**NVIDIA/Windows 零回归**:`rurixc --features dxil-backend --lib` 404、`rurix-rt --lib` 17、`rurix-rt --features vulkan --lib` 23 全不变;`cargo build -p rurix-rt` 默认净、clippy(默认 + `--features vulkan`)`-D warnings` 绿、`cargo fmt --check` 净;真跑证 CUDA 装载器不改——`tests/gpu_roundtrip.rs`(真 nvcuda.dll 装载 saxpy 回读)+ `ci/fatbin_dist_smoke.py`(真 cubin 设备 SAXPY out==a*x+y,n=1048576,sm_89)+ Vulkan `device`(saxpy max_err=0)/`graphics`/`present` 三 smoke 全 PASS。`ci/trace_matrix.py --check` 全锚定 **198 不变**(勘误无新条款,RXS-0211 锚定不变)。**cross-build-green@NDK = G-MB1-5 由 honest-SKIP 升真交叉构建**;on-device saxpy/present 维持 open 尾门 G-MB1-7。档位 **Full RFC**(RFC-0011)。无体例变更 | **Full RFC**（RFC-0011） |
| v1.16 | 2026-07-19 | **RFC-0013 §4.E5/E6 mesh/task/RT SPIR-V 编码 + RXS-0246/RXS-0247 条款体(spec-first,G3.6 PR-Mc/Md 编码腿)**。承 RFC-0013(Agent Approved 2026-07-18)。新增 `### RXS-0246`(MIR→SPIR-V mesh/task 编码,超 RXS-0200~0213/0230 区间独立 RFC 续号):MeshEXT/TaskEXT + capability MeshShadingEXT + `SPV_EXT_mesh_shader` + execution modes(LocalSize/OutputVertices/OutputPrimitivesEXT/OutputTrianglesEXT)+ OpSetMeshOutputsEXT + mesh 输出(Position/varying/PrimitiveTriangleIndicesEXT)+ TaskPayloadWorkgroupEXT + OpEmitMeshTasksEXT;**编码复用 vulkan_codegen.rs GLCompute/LocalSize workgroup 基建(E-4 钉死落点)**,不在 dxil_spirv.rs 重复。新增 `### RXS-0247`(RT 六执行模型编码 + SPIR-V 1.4 per-entry 分叉):RayGenerationKHR/IntersectionKHR/AnyHitKHR/ClosestHitKHR/MissKHR/CallableKHR + capability RayTracingKHR + `SPV_KHR_ray_tracing` + 存储类族(RayPayloadKHR/IncomingRayPayloadKHR/HitAttributeKHR/CallableDataKHR/IncomingCallableDataKHR)+ OpTraceRayKHR/OpReportIntersectionKHR 族 + OpTypeAccelerationStructureKHR(SRV 轴);**1.4 分叉 = per-entry 版本轴(Q-M-SpirvVersion)**:mesh/RT 入口 emit 1.4 + interface 全量,既有 compute/vertex/fragment 维持 1.0 字节零漂移(分叉落发射函数级:compute assemble 1.0 / vertex·fragment emit_spirv_inner 1.0 / mesh·RT ExtBuilder::finish 1.4)。**codegen 腿实现落地**:`vulkan_codegen.rs` `emit_mesh_min`/`emit_task_min`/`emit_{raygen,miss,closesthit,anyhit,intersection,callable}_min`(六模型全覆盖 + ExtBuilder,header 恒 SPIRV_VERSION_1_4);**真实红绿**(本机 Vulkan SDK 1.3.296.0):八阶段 .spv 全过 `spirv-val --target-env vulkan1.2` **且** `spv1.4`(退出码 0 accept);1.0 路零回归(dxil_spirv 版本单测恒 1.0 + vulkan compute assemble 常量不变)。子集外 → RX6026 扩类别。FLS 分节 **严禁 UB 节**。首期产固定最小合规模块(库级见证镜像 sampling/bindless corpus 无 CLI),从真实 `.rx` MIR 体的 intrinsic 降级接线 + device 端到端见证(vk 运行时 mesh pipeline/AS/SBT/TraceRays)归后续 PR + 主循环活驱动(诚实标注);intersection/callable/anyhit device 语料 accept-only(§8)。类型面见 spec/shader_stages.md RXS-0242~0245;DXIL 腿见 spec/dxil_backend.md RXS-0249。每条 ≥1 `//@ spec` 测试锚定 | **Full RFC**（RFC-0013） |
| v1.14 | 2026-07-18 | **RFC-0013 §4.B7 Vulkan graphics descriptor 运行时建面 + RXS-0230 条款体(spec-first,G3.3 PR-S0 descriptor 底座)**。承 RFC-0013(Agent Approved 2026-07-18,E-3/E-5)。新增 `### RXS-0230`(Vulkan graphics descriptor 运行时建面,超 RXS-0200~0213 区间独立 RFC 续号,跳 RXS-0214~0229):`run_graphics_offscreen`(vk.rs)零 descriptor 面 → 新建 graphics descriptor 底座(采样/bindless/graph/present 四面单点关键依赖,PR-S0 独立先落地)。`run_graphics_offscreen_v2` 加性 API(v1 签名/行为 0-byte 保留,MB1 语料零回归)+ `resources: &[GraphicsResource]`(纹理逐层 mip / SamplerDesc / storage image);内部 `VkDescriptorSetLayout`(含 immutable samplers)/`VkDescriptorPool`/`vkUpdateDescriptorSets`/`vkCmdBindDescriptorSets` + mip 链 staging 逐层 upload + layout 迁移。**绑定方案(Q-S-BindingScheme,E-3)= 单一 binding-号事实源 + 按目标两套 set 分配策略**——B 链形态装饰字节不动(零 golden 重 bless);Vk-native 形态 set=类别轴(0=CBV/1=SRV/2=UAV/3=Sampler,bindless 自 set4);目标选择由既有 provenance/mode 旗标承载(现门控 UserSemantic,扩为亦门控 descriptor-set 装饰)。static sampler(RXS-0224)→ Vulkan immutable sampler + `serialize_rts0` NumStaticSamplers 扩。新 unsafe **折叠 U27 扩注**(0 新号,§6.4/E-2)。**合入门 = 混合有界+无界并存+多表+四类别齐全语料的 B 链 SPIR-V 字节 diff golden**(机核 B 链字节不动,承 :144-147 device bug 教训,E-3;非仅 UI golden);SKIP 三态(RURIX_REQUIRE_REAL=1 硬红)。FLS 分节,**严禁 UB 节**。宿主 SamplerDesc 见 spec/host_orchestration.md RXS-0225;类型面见 spec/shader_stages.md RXS-0223;codegen 见 spec/dxil_backend.md RXS-0226~0229。每条 ≥1 `//@ spec` 测试锚定随实现 commit 同落 | **Full RFC**(RFC-0013) |
| v1.15 | 2026-07-19 | **RXS-0207/RXS-0210 加性修订行(G3.5 render graph Vulkan 执行器引用,零改条款体):** [render_graph.md](render_graph.md) **`### RXS-0240`**(双后端 barrier 映射与执行器语义,RFC-0013 §4.D5b,Agent Approved 2026-07-18)的 Vulkan 执行器新入口 **`run_graph`**——多 pass command buffer 录制(逐 pass render pass begin/end + 边界 `vkCmdPipelineBarrier`,layout/stage/access 全取自 render_graph.md RXS-0238 `graph.rs` 同源表,执行器**逐字重放禁二次推导**),承 `### RXS-0207`(Vulkan compute 运行时)/ `### RXS-0210`(graphics+offscreen present)执行语义地基。现 `run_graphics_offscreen` / `run_graphics_offscreen_v2` / `run_graphics_present` 手写定点 barrier 路径 **0-byte 保留**(步骤 48 offscreen 入口 / 既有入口字节不动);新 FFI unsafe 沿 **U27 扩注**(graphics FFI 边界,**0 新 U 号**,RFC-0013 §6.4)。**RXS-0207/RXS-0210 条款体字节 0-byte**(render graph 多 pass 执行语义由 render_graph.md RXS-0240 承载,vulkan_backend.md 不落 graph 执行条款本体);零新条款号 / 零新 RX 码 / 零新 RD / 零新 RFC。Vulkan 数值常量(layout/stage/access)与 `graph.rs` 逐值一致(执行器逐字重放单一事实源)。device 段真跑归主循环活驱动;host 段(D6 互证 + 映射一致性单测)恒跑。档位 **Full RFC**(RFC-0013 §4.D / render_graph.md RXS-0240)。无体例变更 | **Full RFC**（RFC-0013） |
| v1.13 | 2026-07-18 | **RXS-0210 加性修订行(G3.2 present 收尾引用,零改条款体):** `### RXS-0210` L4 win32 swapchain present(RD-032 discharge,W6 落地)的 `run_graphics_present` 呈现循环由 [d3d12_runtime.md](d3d12_runtime.md) **`### RXS-0221`**(swapchain 失效与重建,RFC-0013 §4.A2,Agent Approved 2026-07-18)**加性收尾**——`vkAcquireNextImageKHR`/`vkQueuePresentKHR` 返回 `VK_ERROR_OUT_OF_DATE_KHR`(与可选 `SUBOPTIMAL_KHR`)时,由既有仅接受 `VK_SUCCESS`/`SUBOPTIMAL_KHR` 的终止路收窄为「`vkDeviceWaitIdle` → 重建 swapchain/imageView/framebuffer(重查 surface caps extent)→ 重录后续帧 + 重建后首帧 readback 再断言」。**跨后端重建不变式的单一事实源 = RXS-0221**(Q-P-RebuildHome:重建序为后端无关不变式,单条防两文件语义漂移,本行仅引用不复述);实现落 `src/rurix-rt/src/vk.rs`(present 协商 helper 纯 host 可单测 + OUT_OF_DATE/SUBOPTIMAL 分支),新 unsafe 沿 **U27 扩注**(graphics FFI 边界,**0 新 U 号**,RFC-0013 §6.4)。**RXS-0210 条款体字节 0-byte**(present 循环语义扩充由 RXS-0221 承载,vulkan_backend.md 不落 present 重建条款本体);零新条款号 / 零新 RX 码 / 零新 RD / 零新 RFC。**NVIDIA/Windows 零回归**:`OUT_OF_DATE` 在 NVIDIA/Windows 极难自然触发,既有 offscreen/present 三 smoke 绿路字节行为不变;AMD 真卡 present = G-MB1-6、Android surface present = G-MB1-7 维持 open 尾门(不 claim)。档位 **Full RFC**(RFC-0013 §4.A / d3d12_runtime.md RXS-0221)。无体例变更 | **Full RFC**（RFC-0013） |
