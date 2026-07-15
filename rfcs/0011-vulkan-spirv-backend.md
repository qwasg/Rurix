# RFC-0011 — Vulkan/SPIR-V 跨端第三后端（AMD 桌面 + Android；compute + graphics）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0011（4 位制，编号永不复用，10 §9.5） |
| 标题 | 单一 Vulkan/SPIR-V 后端：一条 MIR→SPIR-V codegen + 一个 Vulkan 运行时后端，同覆盖 AMD 桌面与 Android，compute 与 graphics(vertex/fragment) 皆支持 |
| 档位 | **Full RFC**（10 §3：**新 codegen 目标**(MIR→SPIR-V)+ **新运行时后端**(Vulkan Backend trait 抽象)+ **FFI ABI**(descriptor-binding marshalling / dlopen 移植缝)+ **触死亡路线红线 3**(多后端 D-008/SG-003)；AGENTS 硬规则 5，判档争议向上取严 硬规则 8） |
| 状态 | **Owner Approved（2026-07-15）**。owner（白栀）于本工作会话**明确指示解除多后端红线 3（D-008）并继续 Vulkan/SPIR-V 后端工作**（10 §9.2 owner 主动决策，非 close-out 自动触发）；SG-003 → triggered(RFC-0011) 同步(spike_gating errata)、D-008 §8 errata v2.1 记录、MB1 激活;可推进 mb1 下游实现 PR。agent 依明确授权代录机器事实,非自签。 |
| 承接里程碑 | MB1（milestones/mb1/MB1_CONTRACT.md，验收门 G-MB1-1 ~ G-MB1-6；多后端新纪元，含两道硬件尾门 open） |
| 关联条款 | 拟落 spec **RXS-0200 ~ RXS-0213**（区间随条款数定，见 §5）；新建 `spec/vulkan_backend.md`。跳号 RXS-0189~0199（MS1.2/MS1.2b 承接，feat/ms1.2b 在途）避撞维持 |
| 依据决策 | **D-008**（多后端红线 3 解除，触发时机「G2 完成后」已至，默认「维持红线直至 NVIDIA 纵深完成」——本 RFC 为其正式重审载体，待 owner 裁决）· **SG-003**（多后端 gating，conditional/not_triggered，本 RFC 请求 triggered）· **D-002/D-130**（历史否决 Vulkan 路线——本 RFC 正面重审其前提，见 §2/§7）· D-406 v2.0（agent 自主，但红线解除属 10 §9.2 owner 主动决策，carve-out）· RFC-0003（MIR→DXIL 第二后端并列降级先例）· RFC-0004（MIR→SPIR-V 图形编码器先例，本 RFC 抽取泛化其种子）· RFC-0005（绑定布局推导，descriptor/set/binding 复用）· D-113（FFI 战略；本 RFC 为**导入**方向，不触 `#[export(c)]`/RD-009）· D-205（LLVM pin）· 01 §1 §4 / 03 §4 / 11 §2 §5 / 14 §7 |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`。agent 起草并把待裁问题摊清；**红线 3 解除与本 RFC 批准绑定为 owner 裁决，agent 不自签** |
| Agent 批准 | **Owner Approved — 2026-07-15**。owner 明确指示解除红线 3 并继续,批准范围含 🔒 §4.5(Backend trait FFI)/ §4.7(launch marshalling)/ §4.10(dlopen 移植缝)+ §9 八项 Q(拟裁转定案);红线 3 解除为 owner 主动决策(10 §9.2,区别于 D-406 常规 RFC agent 自主批准的 carve-out);记录于本文件 + 13_DECISION_LOG §8 errata v2.1 + spike_gating SG-003 triggered + MB1_CONTRACT §8。agent 依明确授权代录,非自签 |

---

## 1. 摘要

新增**单一** Vulkan/SPIR-V 后端，用**一条 codegen**（MIR→SPIR-V，`rx build --target vulkan`）+ **一个运行时后端**（rurix-rt 新引 `Backend` trait，Vulkan 为其一实现，CUDA 收敛为并列实现）同时覆盖 **AMD 桌面**与 **Android**，**compute 与 graphics(vertex/fragment) 都做**（mesh/task/RT honest-defer）。SPIR-V 是唯一中间产物：AMD 桌面驱动与 Android `libvulkan.so` 都消费同一份 `.spv`，运行期由 Vulkan 驱动 JIT 到各家 ISA——**不新增每厂商 codegen 分叉**，一条路两平台。

```
single .rx ──rurixc──┬─ host 路:MIR→LLVM→link ── EXE
                     └─ device 路:kernel ──┬─ --target=ptx  → NVPTX→PTX(+cubin)   [现状,不动]
                                           ├─ --target=dxil → DXIL 容器            [G2.2,不动]
                                           └─ --target=vulkan → MIR→SPIR-V(本 RFC) → spirv-val
EXE/App ──rxrt_*(rurix-rt-cabi)── rurix-rt (Backend trait) ──┬─ Cuda impl   → nvcuda.dll        [现状,不回归]
                                                             └─ Vulkan impl → vulkan-1 / libvulkan.so
                                                                              (AMD 桌面 · Android)
```

**范围锁定（owner 已裁，勿扩勿缩）**：后端仅 Vulkan/SPIR-V 一条（AMD 桌面走 Vulkan 而非 D3D12/DXIL）；平台 AMD 桌面 + Android（不做 iOS/Metal）；阶段 compute + graphics。**两道硬件尾门明确 open**：① AMD 真卡最终验收 ② Android 真机 on-device smoke——缺硬件，只写 DoD 不声称已验证。本机 NVIDIA(RTX 4070 Ti，完整 Vulkan 1.3/1.4 实现)承担全部工程期真实红绿取证——**但 NVIDIA-Vulkan 跑通 ≠ 证明 AMD/Android**，此边界贯穿全 RFC。

## 2. 动机

**这是红线 3 的正式重审载体。** 11 §5 明列 G2 完成后有「多后端解禁评估（红线 3 的正式重审——仅当 NVIDIA 单栈纵深完成）」；D-008 触发时机「G2 完成后」已达（G-G2-1~G-G2-6 全闭环，2026-06-30）。本 RFC 不预设结论，把**解除 / 维持**两侧的实证摊给 owner 裁决(§9 Q-Redline)。

**诚实前置（owner 裁决须知）**：SG-003 现存 4 条 decisions 全 `not_triggered`，最近一条 2026-07-14 明确「**语言 1.0 发行 ≠ NVIDIA 纵深完成**，D-008 红线 3 解除条件未达，且解除属独立 one-at-a-time 决策非 close-out 自动触发」。即：**按项目自己的记录，红线 3 的解除前提『NVIDIA 单栈纵深完成』至今被判定为未达成。** 本 RFC 不改写这一判定，也不自行宣布其达成——是否认定前提已满足、是否解除，是 owner 的主动决策。agent 的职责是把技术面全部做完、把待裁摊清、停在闸口。

**维持红线的理据（03 §4 / 01 §1，须并陈）**：项目定位建立在红线 3 之上——WGSL/wgpu、SYCL、HIP 被点名为死亡路线（「跨平台优先 → 性能、能力暴露、生态深度全部让位；地址空间推断弱化 provenance」）；D-002 曾否决 Vulkan 路线（Windows 驱动黑洞）、D-130 择 D3D12 external memory 而非 Vulkan。红线 3 的**底层关切**是「跨平台优先牺牲性能/能力/provenance 的可移植抽象层」。

**解除的理据（须并陈）**：① 使命判据「生产级」的受众面，CUDA-only 触达不到 AMD 桌面与 Android——这是 GPU 生态两个最大的非 NVIDIA 面；② 本 RFC 的 Vulkan 后端在设计上**不犯 WGSL/SYCL 的错**——它是 explicit、单目标 per-build（`--target vulkan` 显式给定，无隐式多目标、无静默 fallback，P-01）、无地址空间推断（binding/set 由 RFC-0005 绑定布局推导显式产出，非弱化 provenance），因此它触的是红线 3 的**字面**（多后端），而非其**底层关切**（可移植抽象层牺牲控制）；③ SPIR-V 单一 IR 覆盖 AMD+Android，不引入 per-vendor codegen 蔓延，纵深可控。

**为何需要 Full RFC（而非 Direct/Mini）**：① 新 codegen 目标（MIR→SPIR-V，与 NVPTX/DXIL 并列的第三降级路）；② 新运行时后端（首次引入 `Backend` trait 抽象，改动 rurix-rt 核心分发）；③ FFI ABI（descriptor-binding marshalling、`dlopen` 跨 OS 加载缝）；④ 触死亡路线红线 3。任一均达 Full RFC 门（硬规则 5）；四者并触，判档争议向上取严（硬规则 8）。

## 3. 指导级解释（用户视角）

同一份 `.rx`，换目标即换后端——源码零改写：

```rx
// compute:与现状 kernel 语法 0 漂移(RXS-0153 compute-via-kernel 着色)
kernel fn saxpy(out: ViewMut<global, f32>, x: View<global, f32>,
                a: f32, n: usize, t: ThreadCtx<1>) {
    let i = t.global_id();
    if i < n { out[i] = a * x[i] + out[i]; }
}
```

```console
$ rx build --target vulkan saxpy.rx      # → saxpy.spv(spirv-val clean)；未启 vulkan-backend feature → RX60xx 目标不可用
$ rx build --target ptx    saxpy.rx      # → saxpy.ptx(现状,不动)
```

graphics（复用 G2.1 着色阶段类型面 RXS-0153~0156，与图形=B/DXIL 同一前端）：

```rx
vertex fn vs(#[builtin(position)] pos: vec4<f32>) -> VsOut { ... }
fragment fn fs(uv: vec2<f32>) -> #[builtin(color)] vec4<f32> { ... }
```

运行期：AMD 桌面上 `vulkan-1.dll` / Android 上 `libvulkan.so` 装载同一 `.spv`，经描述符集绑定缓冲、`vkCmdDispatch`（compute）或 render pass（graphics）执行。错误照旧编译期拦截：目标未启用（RX60xx）、compute 构造超子集（RX60xx）、着色阶段误用（RX3xxx）——全 strict-only，运行期 Vulkan 失败产确定性诊断后终止，无静默降级（P-01）。**NVIDIA 上今天即可真跑取证；AMD 真卡 / Android 真机为 open 尾门。**

## 4. 参考级设计

设计总纲：**加性、并列、不回归**。Vulkan 后端与 NVPTX(D-207)/DXIL(D-131) 后端**并列**，各自从 MIR 独立降级、不共享后端 lowering（RFC-0003 §4.5 口径）；gate 于新 cargo feature `vulkan-backend`（default off，镜像 `dxil-backend`）；未启用时 NVIDIA PTX/cubin 路径**字节零漂移**。

### 4.1 codegen target 分发与 Vulkan 后端分叉（RXS-0200）

- `driver.rs::compile` 在既有 `--target dxil` arm（`driver.rs:270`）之后追加 `--target vulkan` arm，`return compile_vulkan_target(...)`，cfg-gate 于 feature `vulkan-backend`（`#[cfg(not)]` reject stub 发 RX60xx，镜像 `driver.rs:1159`）。`device_emit` 谓词（`driver.rs:208-211`）扩为允许 `vulkan` 目标 main-less kernel-root 编译单元。
- CLI 合法目标集扩容：`src/rx/src/main.rs:146,153`（现 `ptx|dxil`）加 `vulkan`；`src/rurixc/src/bin/rurixc.rs` 无需改（不校验 `--target`）。
- 新模块 `src/rurixc/src/vulkan_codegen.rs`（`device_codegen.rs`/`dxil_codegen.rs` 之兄弟），注册于 `lib.rs`；feature 加于 `Cargo.toml`。

### 4.2 MIR→SPIR-V compute 编码（RXS-0201/0202/0203；新增主体）

RFC-0004 的 `dxil_spirv.rs` 已有 vertex/fragment SPIR-V 字流编码器（header/capability/memory-model/类型系统/`Builder`/id 分配/OpLoad-OpStore/算术）——**抽取为共享 emitter 骨架**（vertex/fragment 大量复用）。compute 是**全新主体**，须补齐 `dxil_spirv.rs` 明确缺失的每一块：

- **执行模型**：`OpEntryPoint GLCompute`（exec model 5，现无此常量）+ `OpExecutionMode LocalSize x y z`（mode 17，现无）；workgroup 维度来自 kernel launch 契约 / `#[workgroup(x,y,z)]` 标注（首期从 BlockDim 或默认标注取，具体标注面见 spec）。
- **compute builtins**：`DeviceIntrinsic`（hir.rs 13 变体：ThreadIndex/BlockIndex/BlockDim/GlobalId × xyz + Barrier）→ SPIR-V builtin：`LocalInvocationId`(27) / `WorkgroupId`(26) / `WorkgroupSize`(25 via NumWorkgroups 组合) / `GlobalInvocationId`(28) / `NumWorkgroups`(24)；`Barrier` → `OpControlBarrier`(现无)。复用 `dxil_spirv.rs` 既有类型-期望机制。
- **存储/描述符缓冲**：`MirResourceType::StructuredBuffer{read_only}` / `ConstantBuffer`（现 `dxil_spirv.rs` 只接受 Texture2D/Sampler，其余落 Unmappable）→ 补 `OpTypeStruct`/`OpTypeRuntimeArray`/`Block`|`BufferBlock` 装饰/`Offset`/`ArrayStride` 成员装饰/`StorageBuffer`|`Uniform` 存储类/`OpAccessChain`（索引）——全新 opcode/装饰常量。set/binding 由 `binding_layout::infer_spirv_bindings`(RFC-0005，已存在)显式产出，`OpDecorate DescriptorSet/Binding`（`dxil_spirv.rs` 已有此机制，复用）。
- **shared 内存**：`Local.shared`（mir.rs，addrspace(3)）→ `Workgroup` 存储类(4，现无)`OpVariable`。
- **控制流**：compute kernel 常需循环/分支——补 `OpBranch`/`OpBranchConditional`/`OpLoopMerge`/`OpSelectionMerge`/`OpPhi`（`dxil_spirv.rs` 现仅直线码，拒非 Goto/Return）。这是 compute 编码最大的新增面；首期子集边界（是否首版仅支持有界循环 / 结构化控制流子集）见 §9 Q-Stages，超子集 → RX60xx。
- 每个产出 `.spv` 过 `spirv-val`（已有 `locate_spirv_val`，`toolchain.rs:293`）；不合规不入 golden。

### 4.3 MIR→SPIR-V graphics 编码（RXS-0204；复用为主）

vertex/fragment 直接复用 `dxil_spirv.rs` 的 `emit_spirv`/`emit_spirv_body`（`EXEC_MODEL_VERTEX`/`FRAGMENT` + `OriginUpperLeft` + Location/BuiltIn 装饰 + Texture2D/Sampler 采样链已具备）。本 RFC 面向 **Vulkan 原生消费**（`.spv` 直喂 `vkCreateShaderModule`），去掉 B 路下游 SPIRV-Cross→HLSL→dxc→DXIL 转译链——SPIR-V 即终产物，非中间踏板。差异仅在：Vulkan graphics pipeline 的 execution model/decoration 需与 Vulkan render pass 约定对齐（vs DXIL 容器约定），首期 present 场景见 §4.9。

### 4.4 🔒 `__nv_*` → GLSL.std.450 ext-inst 映射（RXS-0205；数学 intrinsic 面）

`CallTarget::Libdevice{__nv_*}`（mir.rs:390，NVIDIA 专有 libdevice 外部符号）SPIR-V 无对应——建 `DeviceMathFn`(hir.rs 20 变体) → `GLSL.std.450` ext-inst 映射（`OpExtInstImport "GLSL.std.450"` + `OpExtInst`，`dxil_spirv.rs` 现完全无此面）。多数 1:1（Sqrt→Sqrt / Rsqrt→InverseSqrt / Sin/Cos/Tan / Exp/Exp2 / Ln→Log / Log2 / Floor/Ceil/Trunc/Round→RoundEven / Abs→FAbs / Powf→Pow / Min→FMin / Max→FMax / Fma→Fma）；**非 1:1 须组合**：`Cbrt`(GLSL.std.450 无) → `Pow(x, 1/3)` 或 `sign(x)*Pow(abs(x),1/3)`；`Log10`(无) → `Log2(x) * (1/log2(10))`。首期覆盖既有 20 个 `DeviceMathFn`；映射表完整体随实现 PR + spec 条款落地，超集 intrinsic → RX60xx（编译期，镜像 RX6003 子集纪律）。f32/f64 精度轴：SPIR-V ext-inst 按操作数类型自然分发（无 `f`/非 `f` 符号切分）。

### 4.5 🔒 `Backend`/`GpuDevice` trait 运行时抽象（RXS-0206；不回归 NVIDIA）

rurix-rt 现无任何 trait——CUDA Driver API 硬编码到底（`sys::cuda()` 全直调）。本 RFC 引入 `trait Backend`，把 `impl Cuda` 的扁平方法集（`sys.rs:297-876`：context 生命周期 / host+device alloc-free / memcpy sync+async / module load / get-function / launch / event / stream-wait / error / capability）提升为 trait 方法面；**CUDA 收敛为首个 `impl Backend for Cuda`，NVIDIA 功能零回归（硬约束）**。

- 句柄类型改为 backend 关联类型（`type DevicePtr`、`type Handle`、`type Module`、`type Kernel`）——现散落 `CuPtr`/`CuDevicePtr` 的 ~217 处字段/调用机械迁移。
- `static CUDA: OnceLock<Option<Cuda>>`（`sys.rs:274`）→ backend 选择器（首期由 feature + 运行期探测选定 Cuda 或 Vulkan；无隐式 fallback，P-01）。
- `lib.rs`（affine 单线程系）/`pipeline.rs`（Send 跨线程系，rurix-rt-cabi 驱动面）的 `Context`/`SharedContext`/`DeviceBox`/`SharedStream` 等泛化到 trait 之上。`unsafe impl Send/Sync` 的 CUDA context-thread 不变式改为 backend 声明。
- `error.rs` 的 `CUresult`-形错误 → backend-neutral 或 per-backend 码映射。
- `interop.rs`（D3D12，feature-gated）维持 NVIDIA-only，不进 Vulkan 后端首期。
- unsafe 新增集中于 Vulkan 后端实现文件，逐处 `// SAFETY:` + unsafe-audit **U26**；全仓其余 crate `unsafe_code=deny` 维持。

### 4.6 🔒 Vulkan compute 运行时（RXS-0207；本机 NVIDIA 真跑）

`impl Backend for Vulkan`：instance（`VkApplicationInfo`，开发期开 `VK_LAYER_KHRONOS_validation`）→ physical device 枚举 + queue family（compute）→ logical device + queue → SPIR-V `vkCreateShaderModule` → compute `VkPipeline` + `VkPipelineLayout`（descriptor-set layout + push-constant range）→ descriptor pool/set，`vkUpdateDescriptorSets` 绑缓冲 → command buffer 录 `vkCmdBindPipeline`/`vkCmdBindDescriptorSets`/`vkCmdPushConstants`/`vkCmdDispatch` → submit + fence 同步。内存：`VkBuffer` + `VkDeviceMemory`（host-visible staging + device-local），upload/download 经 map/copy 或 staging。CUDA `Context/Stream/Module/Kernel` 语义映射：ctx→instance+device、stream→queue+command pool、module→shader module、kernel→pipeline。

### 4.7 🔒 launch marshalling ABI（RXS-0208；保 MS1.2 ABI 兼容）

CUDA `rxrt_launch`（`lib.rs:701`）的 `slots[u64] + kinds[u8]`(0=buffer/1=scalar) 是 CUDA 扁平 kernelParams 模型（`void* params[]` 按序位+尺寸匹配）。Vulkan 是 descriptor-binding 模型（buffer→`(set,binding)`、scalar→push constant，无按序位 params 数组）。**MS1.2 已发布 `rxrt_launch` 符号面含义冻结（RXS-0194），不得破坏 NVIDIA 路。** 两个候选（§9 Q-Marshal）：
- **(主选) 保 `rxrt_launch` 签名字节不变，Vulkan 侧按 ordinal 推导绑定**：arg `i`（buffer）→ `(set=0, binding=i)`，scalar → push-constant block 顺序偏移；(set,binding) 布局由 codegen 侧 `binding_layout`(RFC-0005) 与 SPIR-V 描述符装饰**单一事实源**产出并随产物嵌入（§4.8 反射元数据），运行期 backend 消费。零 ABI 新增。
- **(备选) 新增 `rxrt_dispatch_*` 符号**承 descriptor-binding launch，`rxrt_launch` 保 CUDA-only 字节不变（RXS-0194「符号面只追加」口径）。

u64 句柄表语义、`diag` 失败行格式、handle-0 不变式、poisoned 传播为跨后端不变式，维持不变。

### 4.8 artifact 泛化（RXS-0209；不破 NVIDIA cubin/ptx）

- **变体类别（加性安全）**：`ArtifactKind`（fatbin.rs:17）加 `Spirv` + `as_str` 加 `"spirv"` arm。`rurix.lock` `[[artifact]]` schema（`lock.rs` `kind`/`sm_target` 皆自由 String）**零 pkg 码改动**即可承 `kind="spirv"`/`sm_target="gfx1100"`（§5 map 已证）。
- **arch key（真工作）**：`SmTarget(String)`（fatbin.rs:37，硬编 `sm_` 前缀 + `is_ascii_digit` 守卫——拒 `gfx1100`/`gfx90a`）泛化为 `ArchKey{ Sm(String), Gfx(String), SpirvPortable }`（或 newtype prefix-dispatch）。语义警示：NVIDIA 模型 PTX=可移植 JIT fallback / cubin=per-arch AOT；Vulkan 世界 **SPIR-V 占可移植槽**（驱动 JIT），`gfxNNNN` AOT(AMD `.hsaco`)占 per-arch 槽——`DeviceArtifactSet.ptx_fallback` 泛化为 `portable_fallback{kind,bytes}` 或加平行 `spirv_fallback` 字段。
- **描述表（版本门安全扩展）**：`@__rx_gpu_artifacts` blob（artifacts.rs，v1/48B）的 `version != 1` reject 即干净扩展缝——bump **v2** 加 `spirv_ptr`/`spirv_len` 槽 + 反射元数据（binding 布局）；v1 blob 零改动继续工作。`codegen.rs:1028` `emit_gpu_artifact_globals` 加 `@__rx_gpu_spirv` 全局，保留 sm_89 发射不动。
- **装载/协商（真分歧，backend-gated）**：`load_module_artifacts`/`try_load_cubin`（pipeline.rs，`cuModuleLoadData`-bound）加平行 `vkCreateShaderModule`/`VkPipeline` 路，gate 于 feature，NVIDIA `nvcuda.dll`-only 构建不受影响。

### 4.9 Vulkan graphics + present（RXS-0210；本机 NVIDIA 真跑）

render pass、graphics `VkPipeline`（vertex+fragment SPIR-V）、swapchain、`vkQueuePresentKHR`；复用 uc03/uc04 present 场景做等价验收（出图 + present）。VK_LAYER_KHRONOS_validation 零报错。像素/截图对照归档。Present surface：桌面 `VK_KHR_win32_surface`（NVIDIA 取证）/ Android `VK_KHR_android_surface`（§4.10，尾门）。

### 4.10 Android 交叉编译移植缝（RXS-0211；真机前全量）

OS 移植缝集中于 loader + 调用约定 + present surface：
- **dll 加载抽象**：`sys.rs:52-55` 的 `LoadLibraryA`/`GetProcAddress`（Win32-only）↔ Android `dlopen`/`dlsym`（`libc`，RTLD_NOW）；soname `vulkan-1.dll`(桌面) ↔ `libvulkan.so`(Android)。`Backend` trait 的 loader 即此缝。
- **调用约定**：`extern "system"`（Win x64 = MS x64）↔ aarch64-linux-android SysV（Rust off-Windows `"system"`→`"C"`，行为同）；显式化。
- **target**：`x86_64-pc-windows-msvc` ↔ `aarch64-linux-android`，接 NDK 交叉编译；`ANativeWindow` present 代码就位。
- **fail-closed**：NDK 缺失 → SKIP 标 dev-env degrade，不 fake success。平台无关的单元/逻辑测试须过；**设备 smoke 标 `pending-hardware` 并写 DoD**。DoD：android-arm64 交叉**构建绿**、可单测部分绿；on-device 运行 = open 尾门。

### 4.11 toolchain 定位与 fail-closed（RXS-0212）

- `locate_spirv_val`（`toolchain.rs:293`，现 dxil-backend-gated）复用 / 提级；加 `locate_glslang_validator`（env `RURIX_GLSLANG` → PATH-defer，镜像 `_val`/`_cross` never-None spawn-probe-SKIP 纪律）；Vulkan runtime 定位（`vulkan-1.dll`/`libvulkan.so`）为运行期 backend 探测，非 codegen 期 locator。
- **纪律**：缺工具（glslang/spirv-val/NDK/AMD 卡/Android 设备）→ SKIP 标 dev-env degrade 或 pending-hardware，**绝不 fake success**（P-01；Godot 教训——崩溃判定用退出码非 grep stdout）。`RURIX_REQUIRE_REAL=1` 在 GPU runner 上把 SKIP 翻硬红。
- **env 约定**：`VULKAN_SDK`（本机已装 1.3.296.0）；`RURIX_*` 前缀对齐既有 dxil/graphics 工具覆盖集。

### 4.12 Vulkan Rust 绑定选型（RXS-0213；供应链）

`ash`（成熟 Vulkan 绑定）vs 手写薄 `vulkan-1`/`libvulkan` FFI loader（仿 `sys.rs`）。**默认倾向手写薄 loader**——对齐 `sys.rs` 无外部绑定纪律（项目极简依赖、pin 一切）、unsafe 集中 U26、`dlopen` 缝天然适配 Android。若采 `ash`：须 `Cargo.lock` + `rurix.lock [[toolchain]]` pin 并在本 RFC 定案回填。见 §9 Q-Binding（owner 可裁）。

## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

新建 `spec/vulkan_backend.md`，自 **RXS-0200** 起续号（main 现最高 RXS-0188 @ release.md；RXS-0189~0199 由 MS1.2/MS1.2b 承接[feat/ms1.2b 在途]，跳号避撞维持——镜像 0181~0184 GRX 分支占用先例）。各条与 ≥1 测试锚定同 PR（硬规则 7，trace_matrix 维持全锚定）；stable 快照按新条款加性重 bless（RXS-0180 L2）。

| 条款(拟) | 标题 | 测试锚定计划(每条 ≥1) |
|---|---|---|
| RXS-0200 | codegen target 分发与 Vulkan 后端分叉（`--target vulkan`；feature `vulkan-backend`；RX60xx 目标不可用） | conformance/vulkan/accept 最小 compute + reject(feature-off / 非法目标) UI 语料 |
| RXS-0201 | MIR→SPIR-V compute 执行模型与 LocalSize（GLCompute/workgroup 维度） | conformance/vulkan compute golden + ci/vulkan_codegen_smoke spirv-val |
| RXS-0202 | compute builtins 降级（DeviceIntrinsic → SPIR-V builtin + OpControlBarrier） | conformance/vulkan/compute/builtins + codegen 单测 |
| RXS-0203 | 存储/描述符缓冲与控制流降级（StorageBuffer/AccessChain/结构化控制流子集；超子集 RX60xx） | conformance/vulkan/compute/buffers accept + reject/subset |
| RXS-0204 | MIR→SPIR-V graphics 编码（vertex/fragment，复用 RFC-0004 种子，Vulkan 原生消费） | conformance/vulkan/graphics vs/fs golden + spirv-val |
| RXS-0205 | 数学 intrinsic → GLSL.std.450 ext-inst 映射（超集 RX60xx） | conformance/vulkan/mathfn + codegen 映射单测 |
| RXS-0206 | 运行时 Backend trait 抽象与后端选择（无隐式 fallback，P-01；NVIDIA 零回归） | src/rurix-rt Backend 单测 + CUDA-路回归网 |
| RXS-0207 | Vulkan compute 运行时执行语义（pipeline/descriptor/dispatch；validation 零报错） | ci/vulkan_device_smoke.py NVIDIA 真跑 + lavapipe 第二 ICD |
| RXS-0208 | launch marshalling(descriptor-binding；MS1.2 rxrt_launch ABI 兼容，🔒 FFI) | cabi 单测 + rxrt_launch 字节不变回归 |
| RXS-0209 | artifact 泛化（ArtifactKind::Spirv + gfx ArchKey + 描述表 v2；rurix.lock） | fatbin/artifacts 单测 + rurix.lock roundtrip |
| RXS-0210 | Vulkan graphics + present 执行语义（swapchain；uc03/uc04 等价验收） | ci/vulkan_present_smoke.py NVIDIA 真跑 + 像素对照 |
| RXS-0211 | Android 移植缝（dlopen/libvulkan/ANativeWindow；交叉构建绿，设备 pending-hardware） | android-arm64 交叉构建门 + 平台无关单测 |
| RXS-0212 | toolchain 定位与 fail-closed（glslang/spirv-val；缺工具 SKIP，非 RX 码） | ci/vulkan_codegen_smoke SKIP 语义 + red_self_test |
| RXS-0213 | Vulkan 绑定供应链纪律（ash vs 手写；pin） | rurix.lock [[toolchain]] / Cargo.lock 校验 |

- **错误码策略**：编译期新码按真实可达类别分配——目标不可用 / compute 子集外 / 数学 intrinsic 超集归 6xxx codegen 段（next-free 6xxx，跳 MS1.2b 已占；不预留，实现实际可达为准）；工具链定位失败为 SKIP **不占 RX 码**（对齐 spec/release.md §3 工具层口径）；运行期 Vulkan 失败走 cabi 确定性诊断 + 终止，不占 RX 码。registry/error_codes.json 只追加 + en/zh message-key 成对。
- **numbering skip 纪律**：RXS 自 0200 起（跳 0189~0199 MS1 承接）；RD 自 **RD-029** 起（跳 027=交互模式/028=PT completeness，MS1 规划占用）；U26（U23 空号、U25 MS1.2b 占）；SG-003 flip（既有条目），新 spike gating(如需)自 SG-011。

## 6. feature gate / tracking / 实现序（10 §3 要件）

- **gate 形态 = cargo feature `vulkan-backend`（default off，镜像 `dxil-backend`/`shader-stages`）**：未启用时 `--target vulkan` 报目标不可用（RX60xx），NVIDIA PTX/cubin 路径字节零漂移、编译器不 bifurcate。成熟后随快照重 bless 进 stable 面。
- **失败测试先行**：本 RFC 提案时点，`ci/vulkan_codegen_smoke.py`、`spec/vulkan_backend.md`、`src/rurixc/src/vulkan_codegen.rs`、rurix-rt `Backend` trait 在 `main` 上**均不存在** = RED；实现 PR 落地后转绿。
- **栈式实现序（均门控于本 RFC 批准 + 红线 3 解除后；条款 commit 先于实现 commit，硬规则 7）**：
  1. **Phase 1 PR**：spec RXS-0200~0205 → `vulkan_codegen.rs`（`--target vulkan` + MIR→SPIR-V compute/graphics + 数学 intrinsic 映射，抽取泛化 `dxil_spirv.rs`）→ conformance SPIR-V golden(compute+vs+fs) → `ci/vulkan_codegen_smoke.py`(spirv-val，缺工具 SKIP) CI 步骤 → 错误码 en/zh → 快照重 bless → trace 再生。**纯 emit+验证，不需 GPU。**
  2. **Phase 2 PR**：spec RXS-0206~0209 → rurix-rt `Backend` trait（CUDA 收敛，零回归）→ Vulkan compute 后端 → marshalling → artifact 泛化 → `ci/vulkan_device_smoke.py` NVIDIA 真跑 + lavapipe 第二 ICD。
  3. **Phase 3 PR**：spec RXS-0210 → graphics + present → NVIDIA 出图/present 真跑取证。
  4. **Phase 4 PR**：spec RXS-0211 → Android 移植缝 + NDK 交叉构建门；设备 smoke pending-hardware。
- **真实红绿（反 YAML-only）**：Phase 1 内建——篡改 `.spv` 字节 → spirv-val 拒(红)；复原 → 绿。Phase 2+ 本机 NVIDIA(RTX 4070 Ti)+ lavapipe 双 ICD 真跑，validation layer 全程开、零报错。run URL 归档 MB1_CONTRACT §8。**AMD 真卡 / Android 真机验收为 open 尾门（DoD 见 MB1_CONTRACT §4），NVIDIA 跑通不充作 AMD/Android 已验证。**

## 7. 备选方案

- **维持红线 3 不解除（现状）**：project 定位默认选项，SG-003 记录支持。代价 = CUDA-only 永不触达 AMD 桌面/Android。本 RFC 不否决此选项——是否维持是 owner 裁决（§9 Q-Redline）。
- **D3D12/DXIL 覆盖 AMD（不走 Vulkan）**：DXIL 经 D3D12 可跑 AMD-on-Windows，但**不覆盖 Android/Linux**，且 owner 已裁「AMD 桌面走 Vulkan 而非 D3D12/DXIL」（范围锁定）。否决。
- **每平台独立 codegen（AMD ISA + Android 各一条）**：否决——SPIR-V 单一 IR 经驱动 JIT 覆盖两平台，不引入 per-vendor codegen 蔓延（红线 3 底层关切正是「蔓延失控」）。
- **Metal/iOS/MoltenVK**：明确 out-of-scope（owner 范围锁定）。
- **`ash` 重绑定为主**：降为 Q-Binding 备选——默认手写薄 loader 对齐 `sys.rs` 无外部绑定纪律；若 `ash` 生态收益压过依赖纪律，owner 可裁，pin 后回填。
- **把 Vulkan 做成通用可移植抽象层（WGSL/wgpu 式）**：**否决——这正是红线 3 的死亡路线本体**。本 RFC 的 Vulkan 后端 explicit、单目标 per-build、无地址空间推断、无静默 fallback，刻意不越此界。

## 8. 不做（范围红线）

- **AMD 真卡最终验收红绿**：缺硬件，open 尾门 + DoD（MB1_CONTRACT §4 G-MB1-5）；NVIDIA-Vulkan 跑通不充作已验证。
- **Android 真机 on-device smoke**：缺设备，交叉**构建**要绿，设备运行标 `pending-hardware`（G-MB1-6）。
- **Metal / iOS / D3D12 新路**：不触。
- **mesh/task/RT 着色阶段**：honest-defer，登记 **RD-029**（compute+graphics 首期；mesh/task/RT 随需按 10 §3 判档）。
- **通用可移植抽象层 / 地址空间推断 / 隐式多目标 fallback**：红线 3 底层关切，永不做（§7）。
- **NVIDIA(PTX/cubin/DXIL) 既有路任何功能回归**：硬约束，Backend trait 抽象须保 CUDA 零漂移。
- **`#[export(c)]`**（RD-009）不触。

## 9. 未决问题 / 关键裁决（**agent 起草，owner 裁决**——与既有 RFC 的 agent 自主签署不同）

| # | 裁决点 | 拟裁（待 owner）|
|---|---|---|
| **Q-Redline** | **红线 3(D-008)解除 + SG-003 → triggered** | **裁决 = 解除（owner 白栀,2026-07-15）。** owner 于本会话明确指示「把多端红线解除并继续工作」——10 §9.2 owner 主动决策,非 close-out 自动触发。诚实留痕:前提『NVIDIA 单栈纵深完成』先前(2026-07-14)判定未达,本次为 **owner 主动裁决解除**(其 prerogative),非 agent 宣布前提达成。同步 D-008 §8 errata v2.1 + SG-003 → triggered(RFC-0011)。下游 mb1 实现 PR 解锁。agent 依明确授权代录。 |
| Q-Binding | Vulkan Rust 绑定 | 拟：手写薄 `vulkan-1`/`libvulkan` FFI loader（对齐 sys.rs 无外部绑定 + U26 集中 + Android dlopen 天然适配）。owner 若偏好 ash，pin 后回填 |
| Q-Marshal | descriptor-binding launch ABI | 拟：保 `rxrt_launch` 签名字节不变，Vulkan 侧 ordinal→(set,binding) + push-constant，绑定布局随产物嵌入（主选）；`rxrt_dispatch_*` 新符号为备选。MS1.2 ABI 兼容硬约束 |
| Q-ArchKey | arch key 命名 | 拟：`ArchKey{Sm(String),Gfx(String),SpirvPortable}`；lock `kind="spirv"`/`sm_target="gfx1100"`(AMD)/`""`(portable) |
| Q-Trait | Backend trait 形态 | 拟：提升 `impl Cuda` 扁平方法集为 trait，句柄改关联类型，CUDA 首实现零回归；backend 选择器替 static CUDA（无隐式 fallback） |
| Q-Android | Android loader/present 策略 | 拟：dlopen libvulkan.so + NDK aarch64-linux-android + ANativeWindow/VK_KHR_android_surface；构建绿即达标，设备 pending-hardware |
| Q-Perf | 是否立性能门 | 拟：**不立**——首期 correctness（spirv-val clean + validation 零报错 + 数值对照），性能预算 mb1.bench.* 留占位，defer 至纵深期 |
| Q-Stages | compute 编码子集边界 | 拟：compute + graphics(vs/fs) 首期；compute 控制流首版支持结构化子集（有界/条件），超子集 RX60xx；mesh/task/RT → RD-029 |

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：feature `vulkan-backend` default off，成熟前不进 stable 面；条款随快照加性重 bless（RXS-0180 L2 同 edition 只增不破坏）。两道硬件尾门（AMD 真卡 / Android 真机）达成前，后端标 preview / 不承诺 stable。FCP-lite：本 RFC 为 Full RFC 触发面，且触红线 3——**合入通道非 advisory 自动，而是 owner 裁决红线解除后方开**。
- **Provenance**：`Assisted-by: claude-code:claude-opus-4-8`；agent 起草，**红线 3 解除与 RFC 批准为 owner 裁决，agent 不自签**（区别于 D-406 v2.0 常规 RFC 自主批准——红线 carve-out）。

## 11. 规范与实现依据

- 仓内：spec/shader_stages.md RXS-0153~0156（着色阶段前端复用）/ spec/dxil_backend.md RXS-0157~0162（第二后端并列降级先例）/ spec/binding_layout.md RXS-0163~（descriptor/set/binding 推导复用）/ spec/host_orchestration.md RXS-0189~0196（rxrt C ABI / launch marshalling）/ spec/release.md RXS-0150~0152（DeviceArtifactSet / lock [[artifact]]）/ spec/edition.md RXS-0180；src/rurixc/src/{driver,dxil_codegen,dxil_spirv,device_codegen,mir,hir,toolchain,codegen}.rs；src/rurix-rt/src/{sys,lib,pipeline,fatbin,artifacts}.rs;src/rurix-rt-cabi/src/{lib,artifacts}.rs;src/rurix-pkg/src/lock.rs；registry/{error_codes,deferred,spike_gating}.json；13_DECISION_LOG.md（D-002/D-008/D-130）；11_ROADMAP.md §2 §5；03_POSITIONING_AND_LANDSCAPE.md §4。
- 外部：Vulkan 1.3/1.4 规范（compute pipeline / descriptor set / VkShaderModule / swapchain / VK_KHR_android_surface）；SPIR-V 1.x 规范（GLCompute / LocalSize / StorageBuffer / OpControlBarrier / OpAccessChain）；GLSL.std.450 扩展指令集；Khronos glslang/spirv-val（VULKAN_SDK 1.3.296.0）；Android NDK（aarch64-linux-android / ANativeWindow）；（若采）ash 绑定 pin。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-07-15 | AI 起草初版(mb1 多后端新纪元；设计承 10 路子系统勘察：driver 分发 / dxil_spirv SPIR-V 种子 / MIR 模型+intrinsic / rurix-rt CUDA 缝 / artifact+cabi / toolchain / spec+RFC+治理+CI 体例)；**status Draft——依赖 owner 裁决红线 3 解除(§9 Q-Redline)，agent 不自签** | Full RFC(Draft) |
| Owner approval | 2026-07-15 | **owner（白栀）于本工作会话明确指示解除多后端红线 3(D-008)+ SG-003 → triggered(RFC-0011)+ 批准 RFC-0011 全文**(含 🔒 §4.5/§4.7/§4.10 FFI 面与 §9 八项 Q 裁决);批准后推进 mb1 Phase 1~4 实现(Phase 1 codegen + Phase 2 core 运行时已随本轮真实红绿落地)。10 §9.2 owner 主动决策;agent 依明确授权代录机器事实,非自签 | Full RFC(Owner Approved) |
